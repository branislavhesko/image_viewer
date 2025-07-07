mod image_processing;

use eframe::egui;
use eframe::icon_data::from_png_bytes;

use image::{DynamicImage, GenericImageView, ImageBuffer};
use std::path::PathBuf;
use image_processing::{min_max_normalize, standardize, log_min_max_normalize, fft};
use std::env;
use log::{info, error, warn};
use std::io::BufReader;
use std::fs::File;


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
    pixel_info: Option<(u32, u32, u8, u8, u8)>, // (x, y, r, g, b)
    show_pixel_tool: bool,
    click_pos: Option<egui::Pos2>,
    is_floating_point_image: bool,
    original_data_range: Option<(f32, f32)>, // (min, max) of original floating point data
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
            pixel_info: None,
            show_pixel_tool: false,
            click_pos: None,
            is_floating_point_image: false,
            original_data_range: None,
        }
    }
}

impl ImageViewerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn load_image(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let (img, is_fp, data_range) = self.load_image_with_fallback(&path)?;
        
        // Calculate base scale to fit image in window
        let (img_width, img_height) = img.dimensions();
        let max_display_size = 1024.0 - 100.0; // Account for UI
        let scale_w = max_display_size / img_width as f32;
        let scale_h = max_display_size / img_height as f32;
        self.base_scale = scale_w.min(scale_h).min(1.0);
        
        // Store original image without resizing
        self.image = Some(img);
        self.image_path = Some(path);
        self.is_floating_point_image = is_fp;
        self.original_data_range = data_range;
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
    
    fn load_image_with_fallback(&self, path: &PathBuf) -> anyhow::Result<(DynamicImage, bool, Option<(f32, f32)>)> {
        // Try the standard image crate first
        match image::open(path) {
            Ok(img) => {
                info!("Successfully loaded image using standard image crate");
                return Ok((img, false, None));
            }
            Err(e) => {
                warn!("Standard image loading failed: {}", e);
                
                // Check if it's a TIFF file and try direct TIFF loading
                if let Some(ext) = path.extension() {
                    if ext.to_string_lossy().to_lowercase() == "tiff" || ext.to_string_lossy().to_lowercase() == "tif" {
                        info!("Attempting to load TIFF file with direct TIFF decoder");
                        return self.load_tiff_direct(path);
                    }
                }
                
                // If not TIFF or TIFF loading failed, return the original error
                return Err(e.into());
            }
        }
    }
    
    fn load_tiff_direct(&self, path: &PathBuf) -> anyhow::Result<(DynamicImage, bool, Option<(f32, f32)>)> {
        let file = File::open(path)?;
        let mut decoder = tiff::decoder::Decoder::new(BufReader::new(file))?;
        
        // Read the image
        let (width, height) = decoder.dimensions()?;
        let colortype = decoder.colortype()?;
        
        info!("TIFF dimensions: {}x{}, colortype: {:?}", width, height, colortype);
        
        match colortype {
            tiff::ColorType::Gray(8) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U8(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageLuma8(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for Gray(8) TIFF")),
                }
            }
            tiff::ColorType::Gray(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageLuma16(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for Gray(16) TIFF")),
                }
            }
            tiff::ColorType::RGB(8) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U8(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgb8(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGB(8) TIFF")),
                }
            }
            tiff::ColorType::RGB(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgb16(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGB(16) TIFF")),
                }
            }
            tiff::ColorType::RGBA(8) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U8(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgba8(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGBA(8) TIFF")),
                }
            }
            tiff::ColorType::RGBA(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgba16(img_buffer), false, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGBA(16) TIFF")),
                }
            }
            // Handle floating point formats that might not be supported by the image crate
            tiff::ColorType::Gray(32) => {
                info!("Loading 32-bit floating point grayscale TIFF");
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::F32(img_data) => {
                        // Find min/max values for proper normalization
                        let min_val = img_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = img_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        
                        info!("TIFF F32 range: {} to {}", min_val, max_val);
                        
                        // Convert f32 to u8 for display with proper normalization
                        let converted_data: Vec<u8> = if (max_val - min_val).abs() > f32::EPSILON {
                            img_data.iter()
                                .map(|&val| (((val - min_val) / (max_val - min_val)) * 255.0) as u8)
                                .collect()
                        } else {
                            // If all values are the same, use them directly or set to middle gray
                            vec![128u8; img_data.len()]
                        };
                        
                        let img_buffer = ImageBuffer::from_raw(width, height, converted_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageLuma8(img_buffer), true, Some((min_val, max_val))))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for Gray(32) TIFF")),
                }
            }
            tiff::ColorType::RGB(32) => {
                info!("Loading 32-bit floating point RGB TIFF");
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::F32(img_data) => {
                        // Find min/max values for proper normalization
                        let min_val = img_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = img_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        
                        info!("TIFF F32 range: {} to {}", min_val, max_val);
                        
                        // Convert f32 to u8 for display with proper normalization
                        let converted_data: Vec<u8> = if (max_val - min_val).abs() > f32::EPSILON {
                            img_data.iter()
                                .map(|&val| (((val - min_val) / (max_val - min_val)) * 255.0) as u8)
                                .collect()
                        } else {
                            // If all values are the same, use them directly or set to middle gray
                            vec![128u8; img_data.len()]
                        };
                        
                        let img_buffer = ImageBuffer::from_raw(width, height, converted_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgb8(img_buffer), true, Some((min_val, max_val))))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGB(32) TIFF")),
                }
            }
            tiff::ColorType::RGBA(32) => {
                info!("Loading 32-bit floating point RGBA TIFF");
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::F32(img_data) => {
                        // Find min/max values for proper normalization (excluding alpha channel)
                        let pixel_count = (width * height) as usize;
                        let rgb_data = &img_data[..pixel_count * 3]; // Only RGB channels for normalization
                        
                        let min_val = rgb_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                        let max_val = rgb_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                        
                        info!("TIFF F32 range: {} to {}", min_val, max_val);
                        
                        // Convert f32 to u8 for display with proper normalization
                        let converted_data: Vec<u8> = if (max_val - min_val).abs() > f32::EPSILON {
                            img_data.chunks(4)
                                .flat_map(|pixel| {
                                    let r = (((pixel[0] - min_val) / (max_val - min_val)) * 255.0) as u8;
                                    let g = (((pixel[1] - min_val) / (max_val - min_val)) * 255.0) as u8;
                                    let b = (((pixel[2] - min_val) / (max_val - min_val)) * 255.0) as u8;
                                    let a = (pixel[3].clamp(0.0, 1.0) * 255.0) as u8; // Alpha stays 0-1
                                    [r, g, b, a]
                                })
                                .collect()
                        } else {
                            // If all values are the same, use middle gray
                            img_data.chunks(4)
                                .flat_map(|pixel| {
                                    let a = (pixel[3].clamp(0.0, 1.0) * 255.0) as u8;
                                    [128u8, 128u8, 128u8, a]
                                })
                                .collect()
                        };
                        
                        let img_buffer = ImageBuffer::from_raw(width, height, converted_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgba8(img_buffer), true, Some((min_val, max_val))))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGBA(32) TIFF")),
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported TIFF color type: {:?}", colortype));
            }
        }
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
                let new_scale = (self.scale * zoom_delta).clamp(0.1, 20.0);
                
                if old_scale != new_scale {
                    zoom_info = Some((pointer_pos, old_scale, new_scale));
                }
            }
        }

        // Handle panning with left mouse button (only when pixel tool is off)
        if !self.show_pixel_tool {
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
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // First row: Open button, filename, and Scale
            ui.horizontal(|ui| {
                if ui.button("Open Image").clicked() {
                    // Create a file dialog with image filters
                    let file_dialog = rfd::FileDialog::new()
                        .add_filter("Images", &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif", "avif", "hdr", "exr", "farbfeld", "qoi", "dds", "tga", "pnm", "ff", "ico"]);
                    
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

                // Show filename of currently loaded image
                if let Some(path) = &self.image_path {
                    if let Some(filename) = path.file_name() {
                        ui.label(format!("File: {}", filename.to_string_lossy()));
                        ui.separator();
                    }
                }

                ui.label("Scale:");
                if ui.add(egui::Slider::new(&mut self.scale, 0.1..=20.0)).changed() {
                    self.texture_needs_update = true;
                }
            });
            
            // Second row: Normalization
            ui.horizontal(|ui| {
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
            });
            
            // Third row: Channel, Pixel Info, and image information
            ui.horizontal(|ui| {
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
                
                ui.separator();
                
                ui.checkbox(&mut self.show_pixel_tool, "Pixel Info");
                
                ui.separator();
                
                if let Some(img) = &self.image {
                    let (width, height) = img.dimensions();
                    ui.label(format!("Size: {}×{}", width, height));
                    
                    if self.is_floating_point_image {
                        ui.label("Type: Floating Point TIFF");
                        if let Some((min_val, max_val)) = self.original_data_range {
                            ui.label(format!("Range: {:.3} to {:.3}", min_val, max_val));
                        }
                    }
                }
                
                if let Some((x, y, r, g, b)) = self.pixel_info {
                    ui.separator();
                    ui.label(format!("Pixel: ({}, {}) RGB({}, {}, {})", x, y, r, g, b));
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
                    
                    // Handle pixel tool clicking
                    if self.show_pixel_tool {
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
                            if image_rect.contains(pointer_pos) && ui.input(|i| i.pointer.primary_clicked()) {
                                // Convert screen coordinates to image coordinates
                                let relative_pos = pointer_pos - image_rect.min;
                                let image_x = (relative_pos.x / final_scale) as u32;
                                let image_y = (relative_pos.y / final_scale) as u32;
                                
                                // Sample pixel from original image
                                if image_x < orig_width && image_y < orig_height {
                                    let pixel = img.get_pixel(image_x, image_y);
                                    let rgba = pixel.0;
                                    self.pixel_info = Some((image_x, image_y, rgba[0], rgba[1], rgba[2]));
                                    self.click_pos = Some(pointer_pos);
                                }
                            }
                        }
                    }
                    
                    // Only draw the image if it intersects with the visible area
                    if image_rect.intersects(available_rect) {
                        let image = egui::Image::new(texture)
                            .fit_to_exact_size(display_size);
                        ui.put(image_rect, image);
                    }
                    
                    // Display click information near cursor (after image to render on top)
                    if let (Some((x, y, r, g, b)), Some(click_pos)) = (self.pixel_info, self.click_pos) {
                        let text_pos = egui::pos2(click_pos.x + 2.0, click_pos.y - 20.0);
                        let text_content = format!("({}, {}) RGB({}, {}, {})", x, y, r, g, b);
                        
                        // Create a background for the text
                        let text_galley = ui.painter().layout_no_wrap(
                            text_content.clone(),
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
                        
                        let text_rect = egui::Rect::from_min_size(
                            text_pos,
                            text_galley.size() + egui::vec2(8.0, 4.0),
                        );
                        
                        // Draw background
                        ui.painter().rect_filled(
                            text_rect,
                            egui::Rounding::same(3),
                            egui::Color32::from_black_alpha(200),
                        );
                        
                        // Draw border
                        ui.painter().rect_stroke(
                            text_rect,
                            egui::Rounding::same(3),
                            egui::Stroke::new(1.0, egui::Color32::GRAY),
                            egui::StrokeKind::Outside,
                        );
                        
                        // Draw text
                        ui.painter().text(
                            text_pos + egui::vec2(4.0, 2.0),
                            egui::Align2::LEFT_TOP,
                            text_content,
                            egui::FontId::proportional(12.0),
                            egui::Color32::WHITE,
                        );
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
