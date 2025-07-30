#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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
use std::sync::{Arc, Mutex};
use std::fs;

const ICON: &[u8] = include_bytes!("../assets/icon.png");

#[derive(Default, Clone)]
struct HistogramData {
    histograms: Option<Vec<Vec<u32>>>,
    hover_info: Option<(u32, u32, f32)>,
    hover_pos: Option<egui::Pos2>,
    close_requested: bool,
}

struct ImageViewerApp {
    image: Option<DynamicImage>,
    image_path: Option<PathBuf>,
    last_opened_folder: Option<PathBuf>,
    scale: f32,
    base_scale: f32, // Scale to fit image in window
    normalization: NormalizationType,
    channel: ChannelType,
    texture: Option<egui::TextureHandle>,
    offset: egui::Vec2,
    dragging: bool,
    texture_needs_update: bool,
    last_texture_scale: f32,
    last_normalization: NormalizationType,
    last_channel: ChannelType,
    pixel_info: Option<(u32, u32, u8, u8, u8)>, // (x, y, r, g, b)
    pixel_info_fp: Option<(u32, u32, f32, f32, f32)>, // (x, y, r, g, b) for floating point images
    pixel_info_channels: Option<u32>, // Number of channels for current pixel info
    show_pixel_tool: bool,
    hover_pos: Option<egui::Pos2>,
    is_floating_point_image: bool,
    original_data_range: Option<(f32, f32)>, // (min, max) of original floating point data
    original_fp_data: Option<Vec<f32>>, // Store original floating point pixel data
    original_fp_dimensions: Option<(u32, u32)>, // Width, height of original FP data
    original_fp_channels: Option<u32>, // Number of channels (1 for Gray, 3 for RGB)
    show_histogram: bool, // Whether histogram window is open
    histogram_data: Option<Vec<Vec<u32>>>, // Histogram data for each channel (RGB)
    histogram_needs_update: bool, // Whether histogram needs recalculation
    histogram_shared_data: Arc<Mutex<HistogramData>>, // Shared data for histogram window
    histogram_window_id: Option<egui::ViewportId>, // ID of the histogram window
    folder_images: Vec<PathBuf>, // List of images in current folder
    current_image_index: Option<usize>, // Index of current image in folder_images
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
            last_opened_folder: None,
            scale: 1.0,
            base_scale: 1.0,
            normalization: NormalizationType::None,
            channel: ChannelType::RGB,
            texture: None,
            offset: egui::Vec2::ZERO,
            dragging: false,
            texture_needs_update: false,
            last_texture_scale: 1.0,
            last_normalization: NormalizationType::None,
            last_channel: ChannelType::RGB,
            pixel_info: None,
            pixel_info_fp: None,
            pixel_info_channels: None,
            show_pixel_tool: false,
            hover_pos: None,
            is_floating_point_image: false,
            original_data_range: None,
            original_fp_data: None,
            original_fp_dimensions: None,
            original_fp_channels: None,
            show_histogram: false,
            histogram_data: None,
            histogram_needs_update: false,
            histogram_shared_data: Arc::new(Mutex::new(HistogramData::default())),
            histogram_window_id: None,
            folder_images: Vec::new(),
            current_image_index: None,
        }
    }
}

impl ImageViewerApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    fn scan_folder_images(&mut self, current_path: &PathBuf) {
        self.folder_images.clear();
        self.current_image_index = None;
        
        if let Some(parent_dir) = current_path.parent() {
            if let Ok(entries) = fs::read_dir(parent_dir) {
                let supported_extensions = [
                    "png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "gif", 
                    "avif", "hdr", "exr", "farbfeld", "qoi", "dds", "tga", 
                    "pnm", "ff", "ico"
                ];
                
                let mut image_files: Vec<PathBuf> = entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.file_type().ok().map_or(false, |ft| ft.is_file()))
                    .map(|entry| entry.path())
                    .filter(|path| {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            supported_extensions.contains(&ext_str.as_str())
                        } else {
                            false
                        }
                    })
                    .collect();
                
                // Sort alphabetically
                image_files.sort();
                
                // Find current image index
                if let Some(current_index) = image_files.iter().position(|p| p == current_path) {
                    self.current_image_index = Some(current_index);
                }
                
                self.folder_images = image_files;
                info!("Found {} images in folder, current index: {:?}", 
                      self.folder_images.len(), self.current_image_index);
            }
        }
    }

    fn navigate_to_adjacent_image(&mut self, direction: i32) -> anyhow::Result<()> {
        if self.folder_images.is_empty() {
            return Ok(());
        }
        
        let current_index = self.current_image_index.unwrap_or(0);
        let new_index = if direction < 0 {
            // Previous image (left arrow)
            if current_index == 0 {
                self.folder_images.len() - 1 // Wrap to last image
            } else {
                current_index - 1
            }
        } else {
            // Next image (right arrow)
            if current_index >= self.folder_images.len() - 1 {
                0 // Wrap to first image
            } else {
                current_index + 1
            }
        };
        
        if new_index < self.folder_images.len() {
            let new_path = self.folder_images[new_index].clone();
            info!("Navigating to image {}/{}: {:?}", 
                  new_index + 1, self.folder_images.len(), new_path);
            self.load_image(new_path)?;
        }
        
        Ok(())
    }

    fn load_image(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let (img, is_fp, data_range, fp_data, fp_dims, fp_channels) = self.load_image_with_fallback(&path)?;
        
        // Calculate base scale to fit image in window
        let (img_width, img_height) = img.dimensions();
        let max_display_size = 1024.0 - 100.0; // Account for UI
        let scale_w = max_display_size / img_width as f32;
        let scale_h = max_display_size / img_height as f32;
        self.base_scale = scale_w.min(scale_h).min(1.0);
        
        // Store original image without resizing
        self.image = Some(img);
        self.image_path = Some(path.clone());
        // Store the folder path for future file dialogs
        if let Some(parent) = path.parent() {
            self.last_opened_folder = Some(parent.to_path_buf());
        }
        self.is_floating_point_image = is_fp;
        self.original_data_range = data_range;
        // Store floating point data if available
        self.original_fp_data = fp_data;
        self.original_fp_dimensions = fp_dims;
        self.original_fp_channels = fp_channels;
        self.offset = egui::Vec2::ZERO;
        self.scale = 1.0; // Reset user scale
        self.texture = None;
        self.texture_needs_update = true;
        // Reset cached values
        self.last_texture_scale = 1.0;
        self.last_normalization = self.normalization;
        self.last_channel = self.channel;
        // Mark histogram for update
        self.histogram_needs_update = true;
        self.histogram_data = None;
        
        // Scan folder for adjacent images
        self.scan_folder_images(&path);
        
        Ok(())
    }
    
    fn load_image_with_fallback(&self, path: &PathBuf) -> anyhow::Result<(DynamicImage, bool, Option<(f32, f32)>, Option<Vec<f32>>, Option<(u32, u32)>, Option<u32>)> {
        // Try the standard image crate first
        match image::open(path) {
            Ok(img) => {
                info!("Successfully loaded image using standard image crate");
                return Ok((img, false, None, None, None, None));
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
    
    fn load_tiff_direct(&self, path: &PathBuf) -> anyhow::Result<(DynamicImage, bool, Option<(f32, f32)>, Option<Vec<f32>>, Option<(u32, u32)>, Option<u32>)> {
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
                        Ok((DynamicImage::ImageLuma8(img_buffer), false, None, None, None, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for Gray(8) TIFF")),
                }
            }
            tiff::ColorType::Gray(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageLuma16(img_buffer), false, None, None, None, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for Gray(16) TIFF")),
                }
            }
            tiff::ColorType::RGB(8) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U8(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgb8(img_buffer), false, None, None, None, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGB(8) TIFF")),
                }
            }
            tiff::ColorType::RGB(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgb16(img_buffer), false, None, None, None, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGB(16) TIFF")),
                }
            }
            tiff::ColorType::RGBA(8) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U8(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgba8(img_buffer), false, None, None, None, None))
                    }
                    _ => Err(anyhow::anyhow!("Unexpected data type for RGBA(8) TIFF")),
                }
            }
            tiff::ColorType::RGBA(16) => {
                match decoder.read_image()? {
                    tiff::decoder::DecodingResult::U16(img_data) => {
                        let img_buffer = ImageBuffer::from_raw(width, height, img_data)
                            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer from TIFF data"))?;
                        Ok((DynamicImage::ImageRgba16(img_buffer), false, None, None, None, None))
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
                        Ok((DynamicImage::ImageLuma8(img_buffer), true, Some((min_val, max_val)), Some(img_data), Some((width, height)), Some(1)))
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
                        Ok((DynamicImage::ImageRgb8(img_buffer), true, Some((min_val, max_val)), Some(img_data), Some((width, height)), Some(3)))
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
                        Ok((DynamicImage::ImageRgba8(img_buffer), true, Some((min_val, max_val)), Some(img_data), Some((width, height)), Some(4)))
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
    
    fn render_histogram_in_viewport(
        ui: &mut egui::Ui, 
        histograms: &[Vec<u32>], 
        histogram_hover_info: &mut Option<(u32, u32, f32)>,
        histogram_hover_pos: &mut Option<egui::Pos2>
    ) {
        let available_size = ui.available_size();
        let plot_size = egui::vec2(available_size.x, available_size.y - 40.0);
        
        ui.allocate_ui(plot_size, |ui| {
            let rect = ui.available_rect_before_wrap();
            
            // Handle mouse hover for histogram info
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(hover_pos) {
                    // Calculate which bin we're hovering over
                    let relative_x = hover_pos.x - rect.min.x;
                    let bin = ((relative_x / rect.width()) * 256.0) as usize;
                    
                    if bin < 256 {
                        // Get counts for all channels
                        let red_count = histograms[0][bin];
                        let green_count = histograms[1][bin];
                        let blue_count = histograms[2][bin];
                        
                        // For grayscale images (where R=G=B), just use one count
                        let display_count = if red_count == green_count && green_count == blue_count {
                            red_count
                        } else {
                            red_count.max(green_count).max(blue_count)
                        };
                        
                        // Calculate total pixels for percentage
                        let total_pixels: u32 = histograms[0].iter().sum();
                        let percentage = if total_pixels > 0 {
                            (display_count as f32 / total_pixels as f32) * 100.0
                        } else {
                            0.0
                        };
                        
                        *histogram_hover_info = Some((bin as u32, display_count, percentage));
                        *histogram_hover_pos = Some(hover_pos);
                    }
                } else {
                    *histogram_hover_info = None;
                    *histogram_hover_pos = None;
                }
            } else {
                *histogram_hover_info = None;
                *histogram_hover_pos = None;
            }
            
            // Find max value for scaling
            let max_value = histograms.iter()
                .flat_map(|h| h.iter())
                .cloned()
                .max()
                .unwrap_or(1) as f32;
            
            // Draw histogram bars
            let bar_width = rect.width() / 256.0;
            let colors = [
                egui::Color32::from_rgb(255, 80, 80),   // Red
                egui::Color32::from_rgb(80, 255, 80),   // Green
                egui::Color32::from_rgb(80, 80, 255),   // Blue
            ];
            
            // Draw background
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(2),
                egui::Color32::from_gray(15),
            );
            
            // Draw grid lines
            let grid_color = egui::Color32::from_gray(40);
            // Vertical grid lines (every 32 values)
            for i in (0..=256).step_by(32) {
                let x = rect.min.x + (i as f32 / 256.0) * rect.width();
                ui.painter().line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(1.0, grid_color),
                );
            }
            // Horizontal grid lines
            for i in 0..5 {
                let y = rect.min.y + (i as f32 / 4.0) * rect.height();
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0, grid_color),
                );
            }
            
            // Draw histogram for each channel
            for (channel, histogram) in histograms.iter().enumerate() {
                let color = colors[channel];
                
                for (bin, &count) in histogram.iter().enumerate() {
                    if count > 0 {
                        let height = (count as f32 / max_value) * rect.height();
                        let x = rect.min.x + bin as f32 * bar_width;
                        let y = rect.max.y - height;
                        
                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(bar_width.max(1.0), height),
                        );
                        
                        ui.painter().rect_filled(
                            bar_rect,
                            egui::CornerRadius::ZERO,
                            egui::Color32::from_rgba_unmultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                150, // More opaque
                            ),
                        );
                    }
                }
            }
            
            // Draw border
            ui.painter().rect_stroke(
                rect,
                egui::CornerRadius::same(2),
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                egui::StrokeKind::Outside,
            );
            
            // Draw axis labels
            ui.painter().text(
                rect.min + egui::vec2(5.0, 5.0),
                egui::Align2::LEFT_TOP,
                format!("Histogram (Max: {})", max_value as u32),
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
            
            // X-axis labels (pixel values)
            for i in (0..=256).step_by(32) {
                let x = rect.min.x + (i as f32 / 256.0) * rect.width();
                ui.painter().text(
                    egui::pos2(x, rect.max.y + 5.0),
                    egui::Align2::CENTER_TOP,
                    i.to_string(),
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            
            // Y-axis labels (count values)
            for i in 0..5 {
                let y = rect.max.y - (i as f32 / 4.0) * rect.height();
                let count = (max_value * i as f32 / 4.0) as u32;
                ui.painter().text(
                    egui::pos2(rect.min.x - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            
            // Display hover information similar to pixel info
            if let (Some((bin, count, percentage)), Some(hover_pos)) = (*histogram_hover_info, *histogram_hover_pos) {
                let text_pos = egui::pos2(hover_pos.x + 15.0, hover_pos.y - 50.0);
                
                // Show detailed information for each channel
                let red_count = histograms[0][bin as usize];
                let green_count = histograms[1][bin as usize];
                let blue_count = histograms[2][bin as usize];
                
                let text_content = if red_count == green_count && green_count == blue_count {
                    // Grayscale image
                    format!("Value: {}\nCount: {} ({:.2}%)", bin, count, percentage)
                } else {
                    // Color image - show all channels
                    format!("Value: {}\nRed: {}\nGreen: {}\nBlue: {}\nTotal: {:.2}%", 
                           bin, red_count, green_count, blue_count, percentage)
                };
                
                // Create a background for the text
                let text_galley = ui.painter().layout(
                    text_content.clone(),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                    200.0, // Max width for text wrapping
                );
                
                let text_rect = egui::Rect::from_min_size(
                    text_pos,
                    text_galley.size() + egui::vec2(12.0, 8.0),
                );
                
                // Draw background
                ui.painter().rect_filled(
                    text_rect,
                    egui::CornerRadius::same(4),
                    egui::Color32::from_black_alpha(220),
                );
                
                // Draw border
                ui.painter().rect_stroke(
                    text_rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.5, egui::Color32::LIGHT_GRAY),
                    egui::StrokeKind::Outside,
                );
                
                // Draw text
                ui.painter().galley(
                    text_pos + egui::vec2(6.0, 4.0),
                    text_galley,
                    egui::Color32::WHITE,
                );
            }
        });
        
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Channels: ");
            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "■ Red");
            ui.colored_label(egui::Color32::from_rgb(80, 255, 80), "■ Green");
            ui.colored_label(egui::Color32::from_rgb(80, 80, 255), "■ Blue");
            ui.separator();
            ui.label("Hover over histogram to see detailed values");
        });
    }

    #[allow(dead_code)]
    fn render_histogram_static(
        ui: &mut egui::Ui, 
        histograms: &[Vec<u32>], 
        histogram_hover_info: &mut Option<(u32, u32, f32)>,
        histogram_hover_pos: &mut Option<egui::Pos2>
    ) {
        let available_size = ui.available_size();
        let plot_size = egui::vec2(available_size.x, available_size.y - 40.0);
        
        ui.allocate_ui(plot_size, |ui| {
            let rect = ui.available_rect_before_wrap();
            
            // Handle mouse hover for histogram info
            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if rect.contains(hover_pos) {
                    // Calculate which bin we're hovering over
                    let relative_x = hover_pos.x - rect.min.x;
                    let bin = ((relative_x / rect.width()) * 256.0) as usize;
                    
                    if bin < 256 {
                        // Get counts for all channels
                        let red_count = histograms[0][bin];
                        let green_count = histograms[1][bin];
                        let blue_count = histograms[2][bin];
                        
                        // For grayscale images (where R=G=B), just use one count
                        let display_count = if red_count == green_count && green_count == blue_count {
                            red_count
                        } else {
                            red_count.max(green_count).max(blue_count)
                        };
                        
                        // Calculate total pixels for percentage
                        let total_pixels: u32 = histograms[0].iter().sum();
                        let percentage = if total_pixels > 0 {
                            (display_count as f32 / total_pixels as f32) * 100.0
                        } else {
                            0.0
                        };
                        
                        *histogram_hover_info = Some((bin as u32, display_count, percentage));
                        *histogram_hover_pos = Some(hover_pos);
                    }
                } else {
                    *histogram_hover_info = None;
                    *histogram_hover_pos = None;
                }
            } else {
                *histogram_hover_info = None;
                *histogram_hover_pos = None;
            }
            
            // Find max value for scaling
            let max_value = histograms.iter()
                .flat_map(|h| h.iter())
                .cloned()
                .max()
                .unwrap_or(1) as f32;
            
            // Draw histogram bars
            let bar_width = rect.width() / 256.0;
            let colors = [
                egui::Color32::from_rgb(255, 80, 80),   // Red
                egui::Color32::from_rgb(80, 255, 80),   // Green
                egui::Color32::from_rgb(80, 80, 255),   // Blue
            ];
            
            // Draw background
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(2),
                egui::Color32::from_gray(15),
            );
            
            // Draw grid lines
            let grid_color = egui::Color32::from_gray(40);
            // Vertical grid lines (every 32 values)
            for i in (0..=256).step_by(32) {
                let x = rect.min.x + (i as f32 / 256.0) * rect.width();
                ui.painter().line_segment(
                    [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
                    egui::Stroke::new(1.0, grid_color),
                );
            }
            // Horizontal grid lines
            for i in 0..5 {
                let y = rect.min.y + (i as f32 / 4.0) * rect.height();
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                    egui::Stroke::new(1.0, grid_color),
                );
            }
            
            // Draw histogram for each channel
            for (channel, histogram) in histograms.iter().enumerate() {
                let color = colors[channel];
                
                for (bin, &count) in histogram.iter().enumerate() {
                    if count > 0 {
                        let height = (count as f32 / max_value) * rect.height();
                        let x = rect.min.x + bin as f32 * bar_width;
                        let y = rect.max.y - height;
                        
                        let bar_rect = egui::Rect::from_min_size(
                            egui::pos2(x, y),
                            egui::vec2(bar_width.max(1.0), height),
                        );
                        
                        ui.painter().rect_filled(
                            bar_rect,
                            egui::CornerRadius::ZERO,
                            egui::Color32::from_rgba_unmultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                150, // More opaque
                            ),
                        );
                    }
                }
            }
            
            // Draw border
            ui.painter().rect_stroke(
                rect,
                egui::CornerRadius::same(2),
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                egui::StrokeKind::Outside,
            );
            
            // Draw axis labels
            ui.painter().text(
                rect.min + egui::vec2(5.0, 5.0),
                egui::Align2::LEFT_TOP,
                format!("Histogram (Max: {})", max_value as u32),
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
            
            // X-axis labels (pixel values)
            for i in (0..=256).step_by(32) {
                let x = rect.min.x + (i as f32 / 256.0) * rect.width();
                ui.painter().text(
                    egui::pos2(x, rect.max.y + 5.0),
                    egui::Align2::CENTER_TOP,
                    i.to_string(),
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            
            // Y-axis labels (count values)
            for i in 0..5 {
                let y = rect.max.y - (i as f32 / 4.0) * rect.height();
                let count = (max_value * i as f32 / 4.0) as u32;
                ui.painter().text(
                    egui::pos2(rect.min.x - 5.0, y),
                    egui::Align2::RIGHT_CENTER,
                    count.to_string(),
                    egui::FontId::proportional(10.0),
                    egui::Color32::LIGHT_GRAY,
                );
            }
            
            // Display hover information similar to pixel info
            if let (Some((bin, count, percentage)), Some(hover_pos)) = (*histogram_hover_info, *histogram_hover_pos) {
                let text_pos = egui::pos2(hover_pos.x + 15.0, hover_pos.y - 50.0);
                
                // Show detailed information for each channel
                let red_count = histograms[0][bin as usize];
                let green_count = histograms[1][bin as usize];
                let blue_count = histograms[2][bin as usize];
                
                let text_content = if red_count == green_count && green_count == blue_count {
                    // Grayscale image
                    format!("Value: {}\nCount: {} ({:.2}%)", bin, count, percentage)
                } else {
                    // Color image - show all channels
                    format!("Value: {}\nRed: {}\nGreen: {}\nBlue: {}\nTotal: {:.2}%", 
                           bin, red_count, green_count, blue_count, percentage)
                };
                
                // Create a background for the text
                let text_galley = ui.painter().layout(
                    text_content.clone(),
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                    200.0, // Max width for text wrapping
                );
                
                let text_rect = egui::Rect::from_min_size(
                    text_pos,
                    text_galley.size() + egui::vec2(12.0, 8.0),
                );
                
                // Draw background
                ui.painter().rect_filled(
                    text_rect,
                    egui::CornerRadius::same(4),
                    egui::Color32::from_black_alpha(220),
                );
                
                // Draw border
                ui.painter().rect_stroke(
                    text_rect,
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(1.5, egui::Color32::LIGHT_GRAY),
                    egui::StrokeKind::Outside,
                );
                
                // Draw text
                ui.painter().galley(
                    text_pos + egui::vec2(6.0, 4.0),
                    text_galley,
                    egui::Color32::WHITE,
                );
            }
        });
        
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Channels: ");
            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), "■ Red");
            ui.colored_label(egui::Color32::from_rgb(80, 255, 80), "■ Green");
            ui.colored_label(egui::Color32::from_rgb(80, 80, 255), "■ Blue");
            ui.separator();
            ui.label("Hover over histogram to see detailed values");
        });
    }


    fn calculate_histogram(&mut self) {
        if let Some(image) = &self.image {
            let (width, height) = image.dimensions();
            let mut histograms = vec![vec![0u32; 256]; 3]; // RGB channels
            
            // Check if we have original floating point data
            if let (Some(fp_data), Some(fp_channels)) = (&self.original_fp_data, self.original_fp_channels) {
                // Get the data range for proper normalization
                let (min_val, max_val) = if let Some((min, max)) = self.original_data_range {
                    (min, max)
                } else {
                    // Calculate min/max on the fly
                    let min = fp_data.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max = fp_data.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                    (min, max)
                };
                
                let range = max_val - min_val;
                
                // Calculate histogram from original floating point data
                match fp_channels {
                    1 => {
                        // Grayscale floating point
                        for &value in fp_data {
                            let normalized = if range > f32::EPSILON {
                                ((value - min_val) / range).clamp(0.0, 1.0)
                            } else {
                                0.5
                            };
                            let bin = (normalized * 255.0) as usize;
                            histograms[0][bin] += 1;
                            histograms[1][bin] += 1; // Copy to G and B for display
                            histograms[2][bin] += 1;
                        }
                    }
                    3 => {
                        // RGB floating point
                        for chunk in fp_data.chunks(3) {
                            if chunk.len() == 3 {
                                for (channel, &value) in chunk.iter().enumerate() {
                                    let normalized = if range > f32::EPSILON {
                                        ((value - min_val) / range).clamp(0.0, 1.0)
                                    } else {
                                        0.5
                                    };
                                    let bin = (normalized * 255.0) as usize;
                                    histograms[channel][bin] += 1;
                                }
                            }
                        }
                    }
                    4 => {
                        // RGBA floating point - use only RGB
                        for chunk in fp_data.chunks(4) {
                            if chunk.len() == 4 {
                                for (channel, &value) in chunk.iter().take(3).enumerate() {
                                    let normalized = if range > f32::EPSILON {
                                        ((value - min_val) / range).clamp(0.0, 1.0)
                                    } else {
                                        0.5
                                    };
                                    let bin = (normalized * 255.0) as usize;
                                    histograms[channel][bin] += 1;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                // Calculate histogram from regular image data
                for y in 0..height {
                    for x in 0..width {
                        let pixel = image.get_pixel(x, y);
                        let rgba = pixel.0;
                        
                        // Handle different image types
                        match image {
                            image::DynamicImage::ImageLuma8(_) | image::DynamicImage::ImageLuma16(_) => {
                                // Grayscale - use first channel for all RGB
                                let bin = rgba[0] as usize;
                                histograms[0][bin] += 1;
                                histograms[1][bin] += 1;
                                histograms[2][bin] += 1;
                            }
                            _ => {
                                // RGB/RGBA - use separate channels
                                histograms[0][rgba[0] as usize] += 1; // Red
                                histograms[1][rgba[1] as usize] += 1; // Green
                                histograms[2][rgba[2] as usize] += 1; // Blue
                            }
                        }
                    }
                }
            }
            
            self.histogram_data = Some(histograms.clone());
            
            // Update shared data for the separate window
            if let Ok(mut shared) = self.histogram_shared_data.lock() {
                shared.histograms = Some(histograms);
            }
            
            self.histogram_needs_update = false;
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

        // Handle keyboard navigation
        ctx.input(|i| {
            if i.key_pressed(egui::Key::ArrowLeft) {
                if let Err(e) = self.navigate_to_adjacent_image(-1) {
                    error!("Failed to navigate to previous image: {}", e);
                }
            }
            if i.key_pressed(egui::Key::ArrowRight) {
                if let Err(e) = self.navigate_to_adjacent_image(1) {
                    error!("Failed to navigate to next image: {}", e);
                }
            }
        });

        // Store zoom info for use in central panel
        let mut zoom_info: Option<(egui::Pos2, f32, f32)> = None;
        if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
            let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
            
            if scroll_delta.y != 0.0 {
                let old_scale = self.scale;
                // Convert scroll to zoom_delta format (scroll up = zoom in)
                let zoom_delta = if scroll_delta.y > 0.0 { 1.1 } else { 1.0 / 1.1 };
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
                    let file_dialog = if let Some(last_folder) = &self.last_opened_folder {
                        if last_folder.exists() {
                            file_dialog.set_directory(last_folder)
                        } else {
                            // Fallback to Pictures or current directory if last folder doesn't exist
                            if let Ok(home_dir) = env::var("HOME") {
                                let pictures_dir = PathBuf::from(home_dir).join("Pictures");
                                if pictures_dir.exists() {
                                    file_dialog.set_directory(pictures_dir)
                                } else {
                                    file_dialog.set_directory(env::current_dir().unwrap_or_default())
                                }
                            } else {
                                file_dialog.set_directory(env::current_dir().unwrap_or_default())
                            }
                        }
                    } else {
                        // No last folder, use Pictures or current directory
                        if let Ok(home_dir) = env::var("HOME") {
                            let pictures_dir = PathBuf::from(home_dir).join("Pictures");
                            if pictures_dir.exists() {
                                file_dialog.set_directory(pictures_dir)
                            } else {
                                file_dialog.set_directory(env::current_dir().unwrap_or_default())
                            }
                        } else {
                            file_dialog.set_directory(env::current_dir().unwrap_or_default())
                        }
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
                        let file_info = if let Some(index) = self.current_image_index {
                            format!("File: {} ({}/{})", 
                                   filename.to_string_lossy(), 
                                   index + 1, 
                                   self.folder_images.len())
                        } else {
                            format!("File: {}", filename.to_string_lossy())
                        };
                        ui.label(file_info);
                        ui.separator();
                    }
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
                    self.histogram_needs_update = true;
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
                    self.histogram_needs_update = true;
                }
                
                ui.separator();
                
                ui.checkbox(&mut self.show_pixel_tool, "Pixel Info");
                
                ui.separator();
                
                if ui.button("Histogram").clicked() {
                    if self.show_histogram {
                        // Close the histogram window
                        self.show_histogram = false;
                        self.histogram_window_id = None;
                    } else {
                        // Open the histogram window
                        self.show_histogram = true;
                        if self.histogram_needs_update {
                            self.calculate_histogram();
                        }
                        
                        // Create a new viewport for the histogram window
                        let histogram_id = egui::ViewportId::from_hash_of("histogram_window");
                        self.histogram_window_id = Some(histogram_id);
                    }
                }
                
                ui.separator();
                
                // Show navigation hint if we have multiple images in folder
                if self.folder_images.len() > 1 {
                    ui.label("Navigate: ← → arrows");
                    ui.separator();
                }
                
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
                    
                    // Handle pixel tool hovering
                    if self.show_pixel_tool {
                        if let Some(pointer_pos) = ui.input(|i| i.pointer.interact_pos()) {
                            if image_rect.contains(pointer_pos) {
                                // Convert screen coordinates to image coordinates
                                let relative_pos = pointer_pos - image_rect.min;
                                let image_x = (relative_pos.x / final_scale) as u32;
                                let image_y = (relative_pos.y / final_scale) as u32;
                                
                                // Sample pixel from original image
                                if image_x < orig_width && image_y < orig_height {
                                    // Check if we have original floating point data
                                    if let (Some(fp_data), Some((fp_width, _fp_height)), Some(fp_channels)) = (
                                        &self.original_fp_data,
                                        self.original_fp_dimensions,
                                        self.original_fp_channels
                                    ) {
                                        // Sample from original floating point data
                                        let pixel_idx = (image_y * fp_width + image_x) as usize;
                                        match fp_channels {
                                            1 => {
                                                // Grayscale
                                                if pixel_idx < fp_data.len() {
                                                    let gray = fp_data[pixel_idx];
                                                    self.pixel_info_fp = Some((image_x, image_y, gray, gray, gray));
                                                    self.pixel_info_channels = Some(1);
                                                }
                                            }
                                            3 => {
                                                // RGB
                                                let base_idx = pixel_idx * 3;
                                                if base_idx + 2 < fp_data.len() {
                                                    let r = fp_data[base_idx];
                                                    let g = fp_data[base_idx + 1];
                                                    let b = fp_data[base_idx + 2];
                                                    self.pixel_info_fp = Some((image_x, image_y, r, g, b));
                                                    self.pixel_info_channels = Some(3);
                                                }
                                            }
                                            4 => {
                                                // RGBA - use RGB channels
                                                let base_idx = pixel_idx * 4;
                                                if base_idx + 2 < fp_data.len() {
                                                    let r = fp_data[base_idx];
                                                    let g = fp_data[base_idx + 1];
                                                    let b = fp_data[base_idx + 2];
                                                    self.pixel_info_fp = Some((image_x, image_y, r, g, b));
                                                    self.pixel_info_channels = Some(4);
                                                }
                                            }
                                            _ => {
                                                // Fallback to normalized values
                                                let pixel = img.get_pixel(image_x, image_y);
                                                let rgba = pixel.0;
                                                self.pixel_info = Some((image_x, image_y, rgba[0], rgba[1], rgba[2]));
                                                self.pixel_info_fp = None;
                                                self.pixel_info_channels = None;
                                            }
                                        }
                                    } else {
                                        // Use normalized values for non-floating point images
                                        let pixel = img.get_pixel(image_x, image_y);
                                        let rgba = pixel.0;
                                        self.pixel_info = Some((image_x, image_y, rgba[0], rgba[1], rgba[2]));
                                        self.pixel_info_fp = None;
                                        
                                        // Determine channel count based on image type
                                        use image::DynamicImage;
                                        self.pixel_info_channels = Some(match img {
                                            DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_) => 1,
                                            DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgb16(_) => 3,
                                            DynamicImage::ImageRgba8(_) | DynamicImage::ImageRgba16(_) => 4,
                                            _ => 3, // Default to RGB for other types
                                        });
                                    }
                                    self.hover_pos = Some(pointer_pos);
                                }
                            } else {
                                // Clear pixel info when not hovering over image
                                self.pixel_info = None;
                                self.pixel_info_fp = None;
                                self.pixel_info_channels = None;
                                self.hover_pos = None;
                            }
                        } else {
                            // Clear pixel info when no pointer interaction
                            self.pixel_info = None;
                            self.pixel_info_fp = None;
                            self.pixel_info_channels = None;
                            self.hover_pos = None;
                        }
                    }
                    
                    // Only draw the image if it intersects with the visible area
                    if image_rect.intersects(available_rect) {
                        let image = egui::Image::new(texture)
                            .fit_to_exact_size(display_size);
                        ui.put(image_rect, image);
                    }
                    
                    // Display hover information near cursor (after image to render on top)
                    if let Some(hover_pos) = self.hover_pos {
                        let text_pos = egui::pos2(hover_pos.x + 2.0, hover_pos.y - 20.0);
                        let text_content = if let Some((x, y, r, g, b)) = self.pixel_info_fp {
                            // Show original floating point values
                            match self.pixel_info_channels {
                                Some(1) => format!("({}, {}) Gray({:.4})", x, y, r),
                                _ => format!("({}, {}) RGB({:.4}, {:.4}, {:.4})", x, y, r, g, b),
                            }
                        } else if let Some((x, y, r, g, b)) = self.pixel_info {
                            // Show normalized u8 values
                            match self.pixel_info_channels {
                                Some(1) => format!("({}, {}) Gray({})", x, y, r),
                                _ => format!("({}, {}) RGB({}, {}, {})", x, y, r, g, b),
                            }
                        } else {
                            String::new()
                        };
                        
                        if !text_content.is_empty() {
                        
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
                            egui::CornerRadius::same(3),
                            egui::Color32::from_black_alpha(200),
                        );
                        
                        // Draw border
                        ui.painter().rect_stroke(
                            text_rect,
                            egui::CornerRadius::same(3),
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
        
        // Add scale slider in bottom right corner (fixed position)
        if self.image.is_some() {
            egui::Area::new(egui::Id::new("scale_bar"))
                .fixed_pos(egui::pos2(
                    ctx.screen_rect().max.x - 220.0,
                    ctx.screen_rect().max.y - 40.0
                ))
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(150))
                        .corner_radius(egui::CornerRadius::same(5))
                        .inner_margin(egui::Margin::same(5))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                if ui.add(egui::Slider::new(&mut self.scale, 0.1..=20.0).show_value(true)).changed() {
                                    self.texture_needs_update = true;
                                }
                            });
                        });
                });
        }
        
        // Show histogram in a separate OS window if enabled
        if self.show_histogram && self.image.is_some() {
            if let Some(histogram_id) = self.histogram_window_id {
                // Calculate histogram if needed
                if self.histogram_needs_update {
                    self.calculate_histogram();
                }
                
                // Clone the shared data for the viewport closure
                let shared_data = Arc::clone(&self.histogram_shared_data);
                
                // Create the actual separate window using viewports
                ctx.show_viewport_deferred(
                    histogram_id,
                    egui::ViewportBuilder::default()
                        .with_title("Histogram")
                        .with_inner_size([800.0, 500.0])
                        .with_min_inner_size([600.0, 400.0])
                        .with_resizable(true),
                    move |ctx, _class| {
                        // Check if the window should be closed
                        if ctx.input(|i| i.viewport().close_requested()) {
                            // Set the close flag in shared data
                            if let Ok(mut data) = shared_data.lock() {
                                data.close_requested = true;
                            }
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        
                        egui::CentralPanel::default().show(ctx, |ui| {
                            // Access shared data from the separate window
                            if let Ok(mut data) = shared_data.lock() {
                                if let Some(histograms) = data.histograms.clone() {
                                    // Handle the rendering with separate scope for mutable borrows
                                    let mut hover_info = data.hover_info;
                                    let mut hover_pos = data.hover_pos;
                                    
                                    Self::render_histogram_in_viewport(ui, &histograms, &mut hover_info, &mut hover_pos);
                                    
                                    // Update the shared data
                                    data.hover_info = hover_info;
                                    data.hover_pos = hover_pos;
                                }
                            }
                        });
                    },
                );
            }
        } else {
            // Clear the histogram window ID if histogram is not shown
            self.histogram_window_id = None;
        }
        
        // Check if histogram window was closed externally
        if let Ok(mut data) = self.histogram_shared_data.lock() {
            if data.close_requested {
                self.show_histogram = false;
                self.histogram_window_id = None;
                data.close_requested = false; // Reset the flag
            }
        }
    }
}
//TODO: Add a way to save the image
fn main() -> Result<(), eframe::Error> {
    let icon_data = from_png_bytes(ICON).unwrap();
    env_logger::init();
    info!("Starting Image Viewer application");

    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW constant is defined above and integrated via:
        // 1. /SUBSYSTEM:WINDOWS linker flag in build.rs (prevents console window)
        // 2. Windows-specific native options below
        info!("Running on Windows with CREATE_NO_WINDOW equivalent configuration");
    }

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
        // Windows-specific configuration is handled in build.rs with /SUBSYSTEM:WINDOWS
        // This prevents console window from opening (equivalent to CREATE_NO_WINDOW)
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
