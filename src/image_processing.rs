use image::{DynamicImage, ImageBuffer, Rgba, Luma};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

// Normalize floating-point data directly using log transform
pub fn log_min_max_normalize_fp(
    fp_data: &[f32],
    width: u32,
    height: u32,
    channels: u32,
) -> DynamicImage {
    let total_pixels = (width * height) as usize;

    // Find min and max values of log-transformed data for each channel
    let mut min_val = vec![f32::MAX; channels as usize];
    let mut max_val = vec![f32::MIN; channels as usize];

    for i in 0..total_pixels {
        for c in 0..channels as usize {
            let val = fp_data[i * channels as usize + c];
            if val > 0.0 {  // Only consider positive values for log
                let log_val = val.ln();
                min_val[c] = min_val[c].min(log_val);
                max_val[c] = max_val[c].max(log_val);
            }
        }
    }

    // Create normalized image
    match channels {
        1 => {
            // Grayscale
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;
                    let val = fp_data[idx];

                    let normalized = if val > 0.0 && max_val[0] > min_val[0] {
                        let log_val = val.ln();
                        (((log_val - min_val[0]) / (max_val[0] - min_val[0])) * 255.0).clamp(0.0, 255.0) as u8
                    } else {
                        0
                    };

                    output.put_pixel(x, y, Luma([normalized]));
                }
            }
            DynamicImage::ImageLuma8(output)
        }
        3 | 4 => {
            // RGB or RGBA
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * channels as usize;
                    let mut normalized = [0u8; 4];

                    for c in 0..3.min(channels as usize) {
                        let val = fp_data[idx + c];
                        normalized[c] = if val > 0.0 && max_val[c] > min_val[c] {
                            let log_val = val.ln();
                            (((log_val - min_val[c]) / (max_val[c] - min_val[c])) * 255.0).clamp(0.0, 255.0) as u8
                        } else {
                            0
                        };
                    }
                    normalized[3] = 255; // Alpha

                    output.put_pixel(x, y, Rgba(normalized));
                }
            }
            DynamicImage::ImageRgba8(output)
        }
        _ => {
            // Fallback to black image
            DynamicImage::ImageRgba8(ImageBuffer::new(width, height))
        }
    }
}

// Min-max normalize floating-point data directly
pub fn min_max_normalize_fp(
    fp_data: &[f32],
    width: u32,
    height: u32,
    channels: u32,
) -> DynamicImage {
    let total_pixels = (width * height) as usize;

    // Find min and max values for each channel
    let mut min_val = vec![f32::MAX; channels as usize];
    let mut max_val = vec![f32::MIN; channels as usize];

    for i in 0..total_pixels {
        for c in 0..channels as usize {
            let val = fp_data[i * channels as usize + c];
            min_val[c] = min_val[c].min(val);
            max_val[c] = max_val[c].max(val);
        }
    }

    // Create normalized image
    match channels {
        1 => {
            // Grayscale
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;
                    let val = fp_data[idx];

                    let normalized = if max_val[0] > min_val[0] {
                        (((val - min_val[0]) / (max_val[0] - min_val[0])) * 255.0).clamp(0.0, 255.0) as u8
                    } else {
                        128
                    };

                    output.put_pixel(x, y, Luma([normalized]));
                }
            }
            DynamicImage::ImageLuma8(output)
        }
        3 | 4 => {
            // RGB or RGBA
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * channels as usize;
                    let mut normalized = [0u8; 4];

                    for c in 0..3.min(channels as usize) {
                        let val = fp_data[idx + c];
                        normalized[c] = if max_val[c] > min_val[c] {
                            (((val - min_val[c]) / (max_val[c] - min_val[c])) * 255.0).clamp(0.0, 255.0) as u8
                        } else {
                            128
                        };
                    }
                    normalized[3] = 255; // Alpha

                    output.put_pixel(x, y, Rgba(normalized));
                }
            }
            DynamicImage::ImageRgba8(output)
        }
        _ => {
            // Fallback to gray image
            DynamicImage::ImageRgba8(ImageBuffer::new(width, height))
        }
    }
}

// Standardize floating-point data directly
pub fn standardize_fp(
    fp_data: &[f32],
    width: u32,
    height: u32,
    channels: u32,
) -> DynamicImage {
    let total_pixels = (width * height) as usize;

    // Calculate mean and standard deviation for each channel
    let mut sum = vec![0f32; channels as usize];
    let mut sum_sq = vec![0f32; channels as usize];

    for i in 0..total_pixels {
        for c in 0..channels as usize {
            let val = fp_data[i * channels as usize + c];
            sum[c] += val;
            sum_sq[c] += val * val;
        }
    }

    let mut mean = vec![0f32; channels as usize];
    let mut std = vec![0f32; channels as usize];

    for c in 0..channels as usize {
        mean[c] = sum[c] / total_pixels as f32;
        let variance = (sum_sq[c] / total_pixels as f32) - (mean[c] * mean[c]);
        std[c] = variance.sqrt();
    }

    // Create standardized image
    match channels {
        1 => {
            // Grayscale
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize;
                    let val = fp_data[idx];

                    let standardized = if std[0] > 0.0 {
                        (((val - mean[0]) / std[0]) * 50.0 + 127.0).clamp(0.0, 255.0) as u8
                    } else {
                        128
                    };

                    output.put_pixel(x, y, Luma([standardized]));
                }
            }
            DynamicImage::ImageLuma8(output)
        }
        3 | 4 => {
            // RGB or RGBA
            let mut output = ImageBuffer::new(width, height);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * width + x) as usize * channels as usize;
                    let mut standardized = [0u8; 4];

                    for c in 0..3.min(channels as usize) {
                        let val = fp_data[idx + c];
                        standardized[c] = if std[c] > 0.0 {
                            (((val - mean[c]) / std[c]) * 50.0 + 127.0).clamp(0.0, 255.0) as u8
                        } else {
                            128
                        };
                    }
                    standardized[3] = 255; // Alpha

                    output.put_pixel(x, y, Rgba(standardized));
                }
            }
            DynamicImage::ImageRgba8(output)
        }
        _ => {
            // Fallback
            DynamicImage::ImageRgba8(ImageBuffer::new(width, height))
        }
    }
}

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

pub fn fft(img: &DynamicImage) -> DynamicImage {
    let grayscale = img.to_luma8();
    let (width, height) = grayscale.dimensions();
    

    let mut input: Vec<Vec<Complex<f32>>> = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| {
                    let pixel = grayscale.get_pixel(x, y)[0] as f32;
                    // Aplikujeme váhovací funkci (windowing function) - Hamming window
                    let window = 0.54 - 0.46 * (2.0 * PI * x as f32 / (width as f32 - 1.0)).cos();
                    Complex::new(pixel * window, 0.0)
                })
                .collect()
        })
        .collect();
    
    let mut planner = FftPlanner::new();
    
    for row in input.iter_mut() {
        let fft = planner.plan_fft_forward(width as usize);
        fft.process(row);
    }
    
    let mut transposed = vec![vec![Complex::new(0.0, 0.0); height as usize]; width as usize];
    for y in 0..height as usize {
        for x in 0..width as usize {
            transposed[x][y] = input[y][x];
        }
    }
    
    for row in transposed.iter_mut() {
        let fft = planner.plan_fft_forward(height as usize);
        fft.process(row);
    }
    
    for y in 0..height as usize {
        for x in 0..width as usize {
            input[y][x] = transposed[x][y];
        }
    }

    let mut max_magnitude = 0.0f32;
    for y in 0..height as usize {
        for x in 0..width as usize {
            let magnitude = (input[y][x].norm() + 1.0).log10(); // Logaritmická škála pro lepší vizualizaci
            max_magnitude = max_magnitude.max(magnitude);
        }
    }
    
    let mut fft_image = ImageBuffer::new(width, height);
    
    for y in 0..height {
        for x in 0..width {
            let nx = (x + width / 2) % width;
            let ny = (y + height / 2) % height;
            
            let magnitude = (input[y as usize][x as usize].norm() + 1.0).log10();
            let normalized = (magnitude / max_magnitude * 255.0) as u8;
            
            fft_image.put_pixel(nx, ny, Luma([normalized]));
        }
    }
    
    DynamicImage::ImageLuma8(fft_image)
}