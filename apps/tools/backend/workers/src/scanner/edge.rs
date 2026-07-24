use image::{GrayImage};
use imageproc::edges::canny;
use imageproc::filter::gaussian_blur_f32;
use imageproc::distance_transform::Norm;
use imageproc::morphology::close;

use tools_common::error::PipelineError;

/// Detect edges using Canny algorithm with adaptive threshold.
pub fn detect_edges(img: &GrayImage) -> Result<GrayImage, PipelineError> {
    // 1. Gaussian blur for noise reduction
    let blurred = gaussian_blur_f32(img, 3.0);

    // 2. First attempt: Canny with standard thresholds
    let edges = canny(&blurred, 50.0, 150.0);

    // 3. Morphological close to connect broken edges
    let closed = close(&edges, Norm::L1, 5);

    // 4. Check edge coverage
    let edge_count = count_non_zero(&closed);
    let total_pixels = (closed.width() * closed.height()) as u32;

    // If too few edges (<1%), retry with lower thresholds
    if edge_count < total_pixels / 100 {
        let edges2 = canny(&blurred, 20.0, 80.0);
        let closed2 = close(&edges2, Norm::L1, 5);
        let edge_count2 = count_non_zero(&closed2);

        if edge_count2 < total_pixels / 200 {
            return Err(PipelineError::EdgeDetection(
                "Too few edges detected even with low threshold".to_string(),
            ));
        }
        return Ok(closed2);
    }

    Ok(closed)
}

/// Count non-zero (white) pixels in a binary image.
fn count_non_zero(img: &GrayImage) -> u32 {
    let mut count = 0u32;
    for pixel in img.iter() {
        if *pixel > 0 {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn test_edge_detection_on_simple_image() {
        let mut img = GrayImage::new(200, 200);
        for y in 30..170 {
            for x in 30..170 {
                img.put_pixel(x, y, Luma([255]));
            }
        }
        let result = detect_edges(&img);
        assert!(result.is_ok());
        let edges = result.unwrap();
        assert!(count_non_zero(&edges) > 0);
    }

    #[test]
    fn test_empty_image_returns_error() {
        let img = GrayImage::new(100, 100);
        let result = detect_edges(&img);
        assert!(result.is_err());
    }
}