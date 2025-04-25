mod image_processing;

use eframe::egui;
use eframe::icon_data::from_png_bytes;

use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;
use image_processing::{min_max_normalize, standardize, log_min_max_normalize, fft};
use std::env;
use log::{info, error};


const ICON: &[u8] = include_bytes!("../assets/icon.png");


struct ImageViewerApp {
    image: Option<DynamicImage>,
    image_path: Option<PathBuf>,
    scale: f32,
    normalization: NormalizationType,
    texture: Option<egui::TextureHandle>,
    offset: egui::Vec2,
    dragging: bool,
    last_mouse_pos: Option<egui::Pos2>,
}

// TODO: FFT is not queite Normalization, but it is a transformation, need to be fixed
#[derive(PartialEq)]
enum NormalizationType {
    None,
    MinMax,
    LogMinMax,
    Standard,
    FFT,
}


impl Default for ImageViewerApp {
    fn default() -> Self {
        Self {
            image: None,
            image_path: None,
            scale: 1.0,
            normalization: NormalizationType::None,
            texture: None,
            offset: egui::Vec2::ZERO,
            dragging: false,
            last_mouse_pos: None,
        }
    }
}

impl ImageViewerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn load_image(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let img = image::open(&path)?;
        // Resize image to 768x768 while maintaining aspect ratio
        let resized = img.resize(768, 768, image::imageops::FilterType::Lanczos3);
        self.image = Some(resized);
        self.image_path = Some(path);
        self.offset = egui::Vec2::ZERO;
        self.scale = 1.0;
        self.texture = None;
        Ok(())
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(img) = &self.image {
            let normalized_img = match self.normalization {
                NormalizationType::None => img.clone(),
                NormalizationType::MinMax => min_max_normalize(img),
                NormalizationType::LogMinMax => log_min_max_normalize(img),
                NormalizationType::Standard => standardize(img),
                NormalizationType::FFT => fft(img),
            };

            let (width, height) = normalized_img.dimensions();
            
            let rgba8 = normalized_img.to_rgba8();
            let pixels = rgba8.into_raw();
            
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                &pixels,
            );

            self.texture = Some(ctx.load_texture(
                "image-texture",
                color_image,
                egui::TextureOptions::default(),
            ));
        }
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle file drops
        if !ctx.input(|i| i.raw.dropped_files.is_empty()) {
            if let Some(path) = ctx.input(|i| i.raw.dropped_files[0].path.clone()) {
                info!("Dropped file: {:?}", path);
                if let Err(e) = self.load_image(path) {
                    error!("Failed to load dropped image: {}", e);
                } else {
                    self.update_texture(ctx);
                    ctx.request_repaint();
                }
            }
        }

        // Handle mouse wheel zooming centered on cursor
        if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let zoom_delta = ctx.input(|i| i.zoom_delta());
            
            if zoom_delta != 1.0 {
                let old_scale = self.scale;
                let new_scale = (self.scale * zoom_delta).clamp(0.1, 5.0);
                
                if old_scale != new_scale {
                    // Calculate the point we're zooming around in screen space
                    let screen_point = pointer_pos;
                    
                    // Convert to image space before zoom
                    let image_point = (screen_point - self.offset) / old_scale;
                    
                    // Apply zoom
                    self.scale = new_scale;
                    
                    // Calculate new offset to keep the point under cursor
                    let new_screen_point = image_point * new_scale;
                    self.offset = screen_point - new_screen_point;
                    
                    self.texture = None;
                    ctx.request_repaint();
                }
            }
        }

        // Handle panning with left mouse button
        if ctx.input(|i| i.pointer.primary_pressed()) {
            self.dragging = true;
        }
        if !ctx.input(|i| i.pointer.primary_down()) {
            self.dragging = false;
        }
        
        if self.dragging {
            let delta = ctx.input(|i| i.pointer.delta());
            self.offset += delta;
            ctx.request_repaint();
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open Image").clicked() {
                    // Create a file dialog with image filters
                    let file_dialog = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tif", "tiff"]);
                    
                    // Try to set a sensible default directory
                    let file_dialog = if let Ok(home_dir) = env::var("HOME") {
                        let pictures_dir = PathBuf::from(home_dir).join("Pictures");
                        if pictures_dir.exists() {
                            file_dialog.set_directory(pictures_dir)
                        } else {
                            file_dialog.set_directory(env::current_dir().unwrap_or_default())
                        }
                    } else {
                        file_dialog.set_directory(env::current_dir().unwrap_or_default())
                    };
                    
                    if let Some(path) = file_dialog.pick_file() {
                        info!("Opening image from path: {:?}", path);
                        if let Err(e) = self.load_image(path) {
                            error!("Failed to load image: {}", e);
                        }
                    }
                }

                ui.separator();

                ui.label("Scale:");
                if ui.add(egui::Slider::new(&mut self.scale, 0.1..=5.0)).changed() {
                    self.texture = None; // Reset texture when scale changes
                }

                ui.separator();

                ui.label("Normalization:");
                let mut changed = false;
                changed |= ui.radio_value(&mut self.normalization, NormalizationType::None, "None").changed();
                changed |= ui.radio_value(&mut self.normalization, NormalizationType::MinMax, "Min-Max").changed();
                changed |= ui.radio_value(&mut self.normalization, NormalizationType::LogMinMax, "Log Min-Max").changed();
                changed |= ui.radio_value(&mut self.normalization, NormalizationType::Standard, "Standard").changed();
                changed |= ui.radio_value(&mut self.normalization, NormalizationType::FFT, "FFT").changed();


                if changed {
                    self.texture = None; // Reset texture when normalization changes
                }
            });
        });

        if self.texture.is_none() {
            self.update_texture(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                let size = texture.size_vec2();
                let scaled_size = size * self.scale;
                
                // Calculate the rect for the image
                let rect = egui::Rect::from_min_size(
                    egui::pos2(
                        ui.min_rect().min.x + self.offset.x,
                        ui.min_rect().min.y + self.offset.y
                    ),
                    scaled_size
                );
                
                // Create a new image with the correct scale
                let mut image = egui::Image::new(texture);
                image = image.fit_to_exact_size(scaled_size);
                ui.put(rect, image);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No image loaded. Click 'Open Image' to load an image.");
                });
            }
        });
    }
}
//TODO: Add a way to save the image
fn main() -> Result<(), eframe::Error> {
    let icon_data = from_png_bytes(ICON).unwrap();
    env_logger::init();
    info!("Starting Image Viewer application");

    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    info!("Command line arguments: {:?}", args);
    
    // Check for file path in arguments
    let initial_image = if args.len() > 1 {
        let path = &args[1];
        info!("Found file path in arguments: {}", path);
        Some(path.clone())
    } else {
        info!("No file path provided in arguments");
        None
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 800.0])
            .with_min_inner_size([400.0, 400.0])
            .with_drag_and_drop(true)
            .with_icon(icon_data),
        ..Default::default()
    };

    eframe::run_native(
        "Image Viewer",
        native_options,
        Box::new(move |cc| {
            let mut app = ImageViewerApp::new(cc);
            
            // Load initial image if provided
            if let Some(path) = initial_image {
                info!("Loading initial image: {}", path);
                match app.load_image(PathBuf::from(path)) {
                    Ok(_) => info!("Successfully loaded initial image"),
                    Err(e) => error!("Failed to load initial image: {}", e),
                }
            }
            
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
