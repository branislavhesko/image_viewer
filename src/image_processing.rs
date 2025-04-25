use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, Luma};
use rustfft::{FftPlanner, num_complex::Complex};
use std::f32::consts::PI;

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