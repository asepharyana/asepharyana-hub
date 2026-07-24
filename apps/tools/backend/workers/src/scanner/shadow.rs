use image::{GrayImage, Luma};
use imageproc::filter::gaussian_blur_f32;

/// Remove uneven lighting and shadows from a grayscale document image.
///
/// Algorithm:
/// 1. Large Gaussian blur to estimate background illumination
/// 2. Subtract background from original
/// 3. Apply CLAHE for local contrast normalization
pub fn remove_shadow(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());

    // 1. Large Gaussian blur for illumination estimate
    let blur_radius = (w.min(h) as f64 / 50.0).max(15.0);
    let background = gaussian_blur_f32(img, blur_radius as f32);

    // 2. Subtract background
    let bg_mean = mean_pixel(&background);
    let mut corrected = GrayImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let orig = img.get_pixel(x, y)[0] as f32;
            let bg = background.get_pixel(x, y)[0] as f32;
            let corrected_val = (orig - bg + bg_mean).clamp(0.0, 255.0) as u8;
            corrected.put_pixel(x, y, Luma([corrected_val]));
        }
    }

    // 3. Apply CLAHE
    apply_clahe(&corrected, 8, 4)
}

/// Compute mean pixel value of a grayscale image.
fn mean_pixel(img: &GrayImage) -> f32 {
    let sum: u32 = img.iter().map(|&p| p as u32).sum();
    let count = img.width() * img.height();
    if count > 0 {
        sum as f32 / count as f32
    } else {
        0.0
    }
}

/// Contrast Limited Adaptive Histogram Equalization.
/// Divides the image into tiles and applies histogram equalization to each.
fn apply_clahe(img: &GrayImage, tile_size: u32, clip_limit: u8) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let tiles_x = (w + tile_size - 1) / tile_size;
    let tiles_y = (h + tile_size - 1) / tile_size;

    let mut output = GrayImage::new(w, h);

    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let start_x = tx * tile_size;
            let start_y = ty * tile_size;
            let end_x = (start_x + tile_size).min(w);
            let end_y = (start_y + tile_size).min(h);

            // Compute histogram for this tile
            let mut hist = [0u32; 256];
            for y in start_y..end_y {
                for x in start_x..end_x {
                    hist[img.get_pixel(x, y)[0] as usize] += 1;
                }
            }

            // Clip histogram
            let tile_pixels = (end_x - start_x) * (end_y - start_y);
            let clip_limit_count = tile_pixels as u32 * clip_limit as u32 / 255 / 10;
            let mut excess = 0u32;
            for count in hist.iter_mut() {
                if *count > clip_limit_count {
                    excess += *count - clip_limit_count;
                    *count = clip_limit_count;
                }
            }
            // Redistribute excess
            let add_per_bin = excess / 256;
            for count in hist.iter_mut() {
                *count += add_per_bin;
            }

            // Build CDF
            let mut cdf = [0u32; 256];
            cdf[0] = hist[0];
            for i in 1..256 {
                cdf[i] = cdf[i - 1] + hist[i];
            }
            let cdf_min = cdf.iter().find(|&&v| v > 0).copied().unwrap_or(0);

            // Apply equalization to this tile
            for y in start_y..end_y {
                for x in start_x..end_x {
                    let pixel = img.get_pixel(x, y)[0] as usize;
                    let equalized = if cdf_max(cdf) > cdf_min {
                        ((cdf[pixel].saturating_sub(cdf_min)) as f64
                            / (cdf_max(cdf).saturating_sub(cdf_min)) as f64
                            * 255.0) as u8
                    } else {
                        pixel as u8
                    };
                    output.put_pixel(x, y, Luma([equalized]));
                }
            }
        }
    }

    output
}

/// Get the maximum value in the CDF array.
fn cdf_max(cdf: [u32; 256]) -> u32 {
    *cdf.iter().max().unwrap_or(&0)
}

/// Retinex-based shadow removal (alternative algorithm).
#[allow(dead_code)]
fn retinex_shadow_removal(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let blurred = gaussian_blur_f32(img, 30.0);

    let mut output = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let orig = img.get_pixel(x, y)[0] as f32;
            let bg = blurred.get_pixel(x, y)[0] as f32;
            if bg > 0.0 {
                let retinex = (orig / bg).ln() * 255.0;
                output.put_pixel(x, y, Luma([retinex.clamp(0.0, 255.0) as u8]));
            } else {
                output.put_pixel(x, y, Luma([0]));
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_removal_uniform() {
        // Uniform image should remain uniform
        let img = GrayImage::from_pixel(100, 100, Luma([128]));
        let result = remove_shadow(&img);
        assert_eq!(result.width(), 100);
        assert_eq!(result.height(), 100);
        // The result should have fewer dark pixels than a shadowed version
        let dark_count = result.iter().filter(|&&p| p < 50).count();
        assert!(dark_count < 100); // Very few dark pixels
    }

    #[test]
    fn test_mean_pixel() {
        let img = GrayImage::from_pixel(10, 10, Luma([100]));
        assert!((mean_pixel(&img) - 100.0).abs() < 1.0);
    }
}