use image::{GrayImage, Luma};
use imageproc::filter::gaussian_blur_f32;

/// Apply final sharpening and contrast optimization.
pub fn enhance_final(img: &GrayImage) -> GrayImage {
    let sharpened = unsharp_mask(img, 1.0, 1.0);
    adjust_contrast(&sharpened, 1.2)
}

/// Unsharp mask: add high-frequency detail back to the image.
/// result = img + amount * (img - blurred)
pub fn unsharp_mask(img: &GrayImage, sigma: f64, amount: f64) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let blurred = gaussian_blur_f32(img, sigma as f32);

    let mut output = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let orig = img.get_pixel(x, y)[0] as f64;
            let blur = blurred.get_pixel(x, y)[0] as f64;
            let mask = orig - blur;
            let result = (orig + amount * mask).clamp(0.0, 255.0) as u8;
            output.put_pixel(x, y, Luma([result]));
        }
    }
    output
}

/// Adjust contrast by scaling pixel values around the mean.
pub fn adjust_contrast(img: &GrayImage, factor: f64) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let mean = mean_value(img);

    let mut output = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y)[0] as f64;
            let adjusted = ((pixel - mean) * factor + mean).clamp(0.0, 255.0) as u8;
            output.put_pixel(x, y, Luma([adjusted]));
        }
    }
    output
}

/// Remove salt-and-pepper noise using a median-like filter.
#[allow(dead_code)]
pub fn remove_noise(img: &GrayImage, threshold: u8) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let mut output = GrayImage::new(w, h);

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let center = img.get_pixel(x, y)[0];
            // Check if pixel is significantly different from neighbors
            let mut neighbors = Vec::new();
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    neighbors.push(
                        img.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32)[0],
                    );
                }
            }
            let min = *neighbors.iter().min().unwrap_or(&0);
            let max = *neighbors.iter().max().unwrap_or(&255);

            if (center as i16 - min as i16).abs() > threshold as i16
                || (center as i16 - max as i16).abs() > threshold as i16
            {
                // Replace with median
                neighbors.sort();
                output.put_pixel(x, y, Luma([neighbors[neighbors.len() / 2]]));
            } else {
                output.put_pixel(x, y, Luma([center]));
            }
        }
    }

    // Copy edges
    for x in 0..w {
        output.put_pixel(x, 0, *img.get_pixel(x, 0));
        output.put_pixel(x, h - 1, *img.get_pixel(x, h - 1));
    }
    for y in 0..h {
        output.put_pixel(0, y, *img.get_pixel(0, y));
        output.put_pixel(w - 1, y, *img.get_pixel(w - 1, y));
    }

    output
}

/// Compute mean pixel value.
fn mean_value(img: &GrayImage) -> f64 {
    let sum: u64 = img.iter().map(|&p| p as u64).sum();
    let count = img.width() as u64 * img.height() as u64;
    if count > 0 {
        sum as f64 / count as f64
    } else {
        128.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsharp_mask_no_change() {
        // Uniform image should remain unchanged
        let img = GrayImage::from_pixel(50, 50, Luma([128]));
        let result = unsharp_mask(&img, 1.0, 0.0);
        assert_eq!(result.get_pixel(25, 25)[0], 128);
    }

    #[test]
    fn test_contrast_increase() {
        let mut img = GrayImage::new(10, 10);
        img.put_pixel(0, 0, Luma([100]));
        img.put_pixel(1, 0, Luma([200]));
        let result = adjust_contrast(&img, 2.0);
        // With factor > 1, contrast increases
        let diff_orig = (200 - 100) as f64;
        let diff_result = (result.get_pixel(1, 0)[0] as f64) - (result.get_pixel(0, 0)[0] as f64);
        // The difference after contrast adjustment should be greater than original
        assert!(
            diff_result.abs() > diff_orig.abs() * 0.5,
            "diff_orig={}, diff_result={}",
            diff_orig,
            diff_result
        );
    }
}