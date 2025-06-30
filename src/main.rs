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
    base_scale: f32, // Scale to fit image in window
    normalization: NormalizationType,
    channel: ChannelType,
    texture: Option<egui::TextureHandle>,
    offset: egui::Vec2,
    dragging: bool,
    last_mouse_pos: Option<egui::Pos2>,
    texture_needs_update: bool,
    last_texture_scale: f32,
    last_normalization: NormalizationType,
    last_channel: ChannelType,
}

// TODO: FFT is not queite Normalization, but it is a transformation, need to be fixed
#[derive(PartialEq, Clone, Copy)]
enum NormalizationType {
    None,
    MinMax,
    LogMinMax,
    Standard,
    FFT,
}

#[derive(PartialEq, Clone, Copy)]
enum ChannelType {
    RGB,
    Red,
    Green,
    Blue,
}

impl ChannelType {
    fn as_str(&self) -> &'static str {
        match self {
            ChannelType::RGB => "RGB",
            ChannelType::Red => "Red",
            ChannelType::Green => "Green",
            ChannelType::Blue => "Blue",
        }
    }
}


impl Default for ImageViewerApp {
    fn default() -> Self {
        Self {
            image: None,
            image_path: None,
            scale: 1.0,
            base_scale: 1.0,
            normalization: NormalizationType::None,
            channel: ChannelType::RGB,
            texture: None,
            offset: egui::Vec2::ZERO,
            dragging: false,
            last_mouse_pos: None,
            texture_needs_update: false,
            last_texture_scale: 1.0,
            last_normalization: NormalizationType::None,
            last_channel: ChannelType::RGB,
        }
    }
}

impl ImageViewerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn load_image(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let img = image::open(&path)?;
        
        // Calculate base scale to fit image in window
        let (img_width, img_height) = img.dimensions();
        let max_display_size = 1024.0 - 100.0; // Account for UI
        let scale_w = max_display_size / img_width as f32;
        let scale_h = max_display_size / img_height as f32;
        self.base_scale = scale_w.min(scale_h).min(1.0);
        
        // Store original image without resizing
        self.image = Some(img);
        self.image_path = Some(path);
        self.offset = egui::Vec2::ZERO;
        self.scale = 1.0; // Reset user scale
        self.texture = None;
        self.texture_needs_update = true;
        // Reset cached values
        self.last_texture_scale = 1.0;
        self.last_normalization = self.normalization;
        self.last_channel = self.channel;
        Ok(())
    }
    
    fn calculate_window_size(&self) -> (f32, f32) {
        if let Some(img) = &self.image {
            let (width, height) = img.dimensions();
            let (w, h) = (width as f32, height as f32);
            
            // Add space for UI elements (top panel)
            let ui_height = 80.0;
            let ui_padding = 40.0;
            
            let scaled_width = (w * self.base_scale + ui_padding).max(400.0).min(1024.0);
            let scaled_height = (h * self.base_scale + ui_height + ui_padding).max(400.0).min(1024.0);
            
            (scaled_width, scaled_height)
        } else {
            (800.0, 800.0) // Default size
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(img) = &self.image {
            // Check if we need to regenerate texture
            let needs_regenerate = self.texture.is_none() || 
                self.last_normalization != self.normalization ||
                self.last_channel != self.channel ||
                (self.last_texture_scale - self.scale).abs() > 0.2; // Only regenerate on significant scale changes
            
            if !needs_regenerate {
                return;
            }
            
            // Calculate the final display size based on current scaling
            let (orig_width, orig_height) = img.dimensions();
            let final_scale = self.base_scale * self.scale;
            
            // Only resize if the final display size is smaller than original
            // This preserves quality when zooming in
            let display_width = (orig_width as f32 * final_scale) as u32;
            let display_height = (orig_height as f32 * final_scale) as u32;
            
            let working_img = if final_scale < 1.0 {
                // Scale down for performance when displaying smaller
                img.resize(display_width, display_height, image::imageops::FilterType::Lanczos3)
            } else {
                // Use original image when zooming in to preserve quality
                img.clone()
            };
            
            let normalized_img = match self.normalization {
                NormalizationType::None => working_img,
                NormalizationType::MinMax => min_max_normalize(&working_img),
                NormalizationType::LogMinMax => log_min_max_normalize(&working_img),
                NormalizationType::Standard => standardize(&working_img),
                NormalizationType::FFT => fft(&working_img),
            };

            let (width, height) = normalized_img.dimensions();
            let rgba8 = normalized_img.to_rgba8();
            
            // Apply channel filtering
            let filtered_pixels = match self.channel {
                ChannelType::RGB => rgba8.into_raw(),
                ChannelType::Red => {
                    rgba8.pixels().flat_map(|p| [p[0], 0, 0, p[3]]).collect()
                },
                ChannelType::Green => {
                    rgba8.pixels().flat_map(|p| [0, p[1], 0, p[3]]).collect()
                },
                ChannelType::Blue => {
                    rgba8.pixels().flat_map(|p| [0, 0, p[2], p[3]]).collect()
                },
            };
            
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                &filtered_pixels,
            );

            self.texture = Some(ctx.load_texture(
                "image-texture",
                color_image,
                egui::TextureOptions::default(),
            ));
            
            // Update cached values
            self.last_texture_scale = self.scale;
            self.last_normalization = self.normalization;
            self.last_channel = self.channel;
        }
    }
}

impl eframe::App for ImageViewerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle file drops
        let mut file_dropped = false;
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    info!("Dropped file: {:?}", path);
                    if let Err(e) = self.load_image(path.clone()) {
                        error!("Failed to load dropped image: {}", e);
                    } else {
                        file_dropped = true;
                        break; // Only load the first valid image
                    }
                }
            }
        });
        
        if file_dropped {
            // Resize window to fit the new image
            let (width, height) = self.calculate_window_size();
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
            ctx.request_repaint();
        }

        // Store zoom info for use in central panel
        let mut zoom_info: Option<(egui::Pos2, f32, f32)> = None;
        if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let zoom_delta = ctx.input(|i| i.zoom_delta());
            
            if zoom_delta != 1.0 {
                let old_scale = self.scale;
                let new_scale = (self.scale * zoom_delta).clamp(0.1, 5.0);
                
                if old_scale != new_scale {
                    zoom_info = Some((pointer_pos, old_scale, new_scale));
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
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif"]);
                    
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
                        } else {
                            // Resize window to fit the new image
                            let (width, height) = self.calculate_window_size();
                            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
                        }
                    }
                }

                ui.separator();

                ui.label("Scale:");
                if ui.add(egui::Slider::new(&mut self.scale, 0.1..=5.0)).changed() {
                    self.texture_needs_update = true;
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
                    self.texture_needs_update = true;
                }
                
                ui.separator();
                
                ui.label("Channel:");
                let mut channel_changed = false;
                egui::ComboBox::from_label("")
                    .selected_text(self.channel.as_str())
                    .show_ui(ui, |ui| {
                        channel_changed |= ui.selectable_value(&mut self.channel, ChannelType::RGB, "RGB").changed();
                        channel_changed |= ui.selectable_value(&mut self.channel, ChannelType::Red, "Red").changed();
                        channel_changed |= ui.selectable_value(&mut self.channel, ChannelType::Green, "Green").changed();
                        channel_changed |= ui.selectable_value(&mut self.channel, ChannelType::Blue, "Blue").changed();
                    });
                    
                if channel_changed {
                    self.texture_needs_update = true;
                }
            });
        });

        if (self.texture.is_none() || self.texture_needs_update) && self.image.is_some() {
            self.update_texture(ctx);
            self.texture_needs_update = false;
        }

        // Handle zoom outside of the panel to avoid borrowing issues
        if let Some((pointer_pos, old_scale, new_scale)) = zoom_info {
            if let Some(img) = &self.image {
                let old_final_scale = self.base_scale * old_scale;
                let (orig_width, orig_height) = img.dimensions();
                let old_display_size = egui::vec2(
                    orig_width as f32 * old_final_scale,
                    orig_height as f32 * old_final_scale
                );
                
                // Calculate where image would be positioned
                let available_size = ctx.screen_rect().size();
                let center_x = available_size.x / 2.0;
                let center_y = (available_size.y - 80.0) / 2.0 + 80.0; // Account for top panel
                
                let old_image_pos = egui::pos2(
                    center_x - old_display_size.x / 2.0 + self.offset.x,
                    center_y - old_display_size.y / 2.0 + self.offset.y
                );
                
                let old_image_rect = egui::Rect::from_min_size(old_image_pos, old_display_size);
                
                // Check if pointer is over the image
                if old_image_rect.contains(pointer_pos) {
                    // Convert pointer position to image-relative coordinates
                    let image_center = old_image_rect.center();
                    
                    // Calculate the point in image space (relative to image center)
                    let pointer_offset_from_center = pointer_pos - image_center;
                    let image_point = pointer_offset_from_center / old_final_scale;
                    
                    // Apply new scale
                    self.scale = new_scale;
                    let new_final_scale = self.base_scale * new_scale;
                    
                    // Calculate where that point should be now
                    let new_pointer_offset = image_point * new_final_scale;
                    
                    // Adjust offset to keep the point under cursor
                    let desired_center = pointer_pos - new_pointer_offset;
                    self.offset += desired_center - image_center;
                } else {
                    // If not over image, just apply zoom
                    self.scale = new_scale;
                }
                
                // Mark texture for update if scale changed significantly
                if (new_scale - old_scale).abs() > 0.1 {
                    self.texture_needs_update = true;
                }
                ctx.request_repaint();
            }
        }
        
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(img) = &self.image {
                if let Some(texture) = &self.texture {
                    let _texture_size = texture.size_vec2();
                    let final_scale = self.base_scale * self.scale;
                    
                    // Calculate display size based on original image dimensions
                    let (orig_width, orig_height) = img.dimensions();
                    let display_size = egui::vec2(
                        orig_width as f32 * final_scale,
                        orig_height as f32 * final_scale
                    );
                    
                    // Center the image in the available space
                    let available_rect = ui.available_rect_before_wrap();
                    let center_x = available_rect.center().x;
                    let center_y = available_rect.center().y;
                    
                    // Calculate position to center the image
                    let image_pos = egui::pos2(
                        center_x - display_size.x / 2.0 + self.offset.x,
                        center_y - display_size.y / 2.0 + self.offset.y
                    );
                    
                    let image_rect = egui::Rect::from_min_size(image_pos, display_size);
                    
                    // Only draw the image if it intersects with the visible area
                    if image_rect.intersects(available_rect) {
                        let image = egui::Image::new(texture)
                            .fit_to_exact_size(display_size);
                        ui.put(image_rect, image);
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("Loading image...");
                    });
                }
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
                    Ok(_) => {
                        info!("Successfully loaded initial image");
                        // Set initial window size based on image
                        let (width, height) = app.calculate_window_size();
                        cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
                    },
                    Err(e) => error!("Failed to load initial image: {}", e),
                }
            }
            
            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
