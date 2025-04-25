use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};

pub fn min_max_normalize(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    // Find min and max values
    let mut min_val = [u8::MAX; 4];
    let mut max_val = [u8::MIN; 4];
    
    for pixel in rgba.pixels() {
        for i in 0..4 {
            min_val[i] = min_val[i].min(pixel[i]);
            max_val[i] = max_val[i].max(pixel[i]);
        }
    }
    
    // Create normalized image
    let mut output = ImageBuffer::new(width, height);
    
    for (x, y, pixel) in output.enumerate_pixels_mut() {
        let input_pixel = rgba.get_pixel(x, y);
        let mut normalized = [0u8; 4];
        
        for i in 0..4 {
            if max_val[i] > min_val[i] {
                normalized[i] = (((input_pixel[i] as f32 - min_val[i] as f32) / 
                    (max_val[i] as f32 - min_val[i] as f32)) * 255.0) as u8;
            } else {
                normalized[i] = input_pixel[i];
            }
        }
        
        *pixel = Rgba(normalized);
    }
    
    DynamicImage::ImageRgba8(output)
}

pub fn log_min_max_normalize(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    // Find min and max values of log-transformed data
    let mut min_val = [f32::MAX; 4];
    let mut max_val = [f32::MIN; 4];
    
    for pixel in rgba.pixels() {
        for i in 0..4 {
            let val = pixel[i] as f32;
            if val > 0.0 {  // Only consider non-zero values for log
                let log_val = val.ln();
                min_val[i] = min_val[i].min(log_val);
                max_val[i] = max_val[i].max(log_val);
            }
        }
    }
    
    // Create normalized image
    let mut output = ImageBuffer::new(width, height);
    
    for (x, y, pixel) in output.enumerate_pixels_mut() {
        let input_pixel = rgba.get_pixel(x, y);
        let mut normalized = [0u8; 4];
        
        for i in 0..4 {
            let val = input_pixel[i] as f32;
            if val > 0.0 && max_val[i] > min_val[i] {
                let log_val = val.ln();
                normalized[i] = (((log_val - min_val[i]) / (max_val[i] - min_val[i])) * 255.0) as u8;
            } else {
                normalized[i] = input_pixel[i];
            }
        }
        
        *pixel = Rgba(normalized);
    }
    
    DynamicImage::ImageRgba8(output)
}

pub fn standardize(img: &DynamicImage) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    
    // Calculate mean and standard deviation for each channel
    let mut sum = [0f32; 4];
    let mut sum_sq = [0f32; 4];
    let total_pixels = (width * height) as f32;
    
    for pixel in rgba.pixels() {
        for i in 0..4 {
            let val = pixel[i] as f32;
            sum[i] += val;
            sum_sq[i] += val * val;
        }
    }
    
    let mut mean = [0f32; 4];
    let mut std = [0f32; 4];
    
    for i in 0..4 {
        mean[i] = sum[i] / total_pixels;
        let variance = (sum_sq[i] / total_pixels) - (mean[i] * mean[i]);
        std[i] = variance.sqrt();
    }
    
    // Create standardized image
    let mut output = ImageBuffer::new(width, height);
    
    for (x, y, pixel) in output.enumerate_pixels_mut() {
        let input_pixel = rgba.get_pixel(x, y);
        let mut standardized = [0u8; 4];
        
        for i in 0..4 {
            if std[i] > 0.0 {
                let val = ((input_pixel[i] as f32 - mean[i]) / std[i]) * 50.0 + 127.0;
                standardized[i] = val.clamp(0.0, 255.0) as u8;
            } else {
                standardized[i] = input_pixel[i];
            }
        }
        
        *pixel = Rgba(standardized);
    }
    
    DynamicImage::ImageRgba8(output)
} 