use image::{DynamicImage, GrayImage, Luma};
use image::imageops::FilterType;

use tools_common::error::PipelineError;

/// Maximum dimension for processing (edge detection works fine at this resolution).
const MAX_DIMENSION: u32 = 2000;

/// Load image from file path.
pub fn load_image(path: &std::path::Path) -> Result<DynamicImage, PipelineError> {
    image::open(path).map_err(|e| PipelineError::ImageLoad(e.to_string()))
}

/// Resize image if it exceeds the maximum dimension, preserving aspect ratio.
/// Uses Lanczos3 filter for sharpest downscale.
pub fn safe_resize(img: &DynamicImage) -> DynamicImage {
    let (w, h) = (img.width(), img.height());
    let max_dim = w.max(h) as f64;

    if max_dim > MAX_DIMENSION as f64 {
        let scale = MAX_DIMENSION as f64 / max_dim;
        let new_w = (w as f64 * scale) as u32;
        let new_h = (h as f64 * scale) as u32;
        img.resize_exact(new_w.max(1), new_h.max(1), FilterType::Lanczos3)
    } else {
        img.clone()
    }
}

/// Convert to grayscale (Luma8).
pub fn to_grayscale(img: &DynamicImage) -> GrayImage {
    img.to_luma8()
}

/// Full preprocess pipeline: load → resize → grayscale.
pub fn preprocess(path: &std::path::Path) -> Result<GrayImage, PipelineError> {
    let img = load_image(path)?;
    let resized = safe_resize(&img);
    Ok(to_grayscale(&resized))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_resize_no_resize() {
        // Image smaller than MAX_DIMENSION should not be resized
        let img = DynamicImage::new_luma8(800, 600);
        let result = safe_resize(&img);
        assert_eq!(result.width(), 800);
        assert_eq!(result.height(), 600);
    }

    #[test]
    fn test_safe_resize_downscale() {
        // 12MP image (4000x3000) should be resized to ≤2000px
        let img = DynamicImage::new_luma8(4000, 3000);
        let result = safe_resize(&img);
        assert!(result.width() <= 2000);
        assert!(result.height() <= 2000);
        // Aspect ratio preserved: 4000/3000 = 1.333
        let ratio = result.width() as f64 / result.height() as f64;
        assert!((ratio - 4.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn test_to_grayscale() {
        let img = DynamicImage::new_rgba8(100, 100);
        let gray = to_grayscale(&img);
        assert_eq!(gray.width(), 100);
        assert_eq!(gray.height(), 100);
    }
}