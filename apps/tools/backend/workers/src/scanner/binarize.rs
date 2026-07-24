use image::{GrayImage, Luma};

/// Apply Sauvola local threshold for clean black-and-white output.
///
/// Sauvola: T(x,y) = m(x,y) * [1 + k * (s(x,y)/R - 1)]
/// where m = local mean, s = local std dev, R = 128, k = 0.2
pub fn sauvola_threshold(img: &GrayImage, window_size: u32, k: f64) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let half_win = (window_size / 2) as i32;
    let mut output = GrayImage::new(w, h);

    // Integral images for O(1) mean and variance computation
    let integral = compute_integral_image(img);
    let integral_sq = compute_integral_image_sq(img);

    for y in 0..h {
        for x in 0..w {
            let (mean, variance) = local_stats(
                &integral,
                &integral_sq,
                x as i32,
                y as i32,
                half_win,
                w as i32,
                h as i32,
            );
            let std_dev = variance.sqrt();
            let threshold = mean * (1.0 + k * (std_dev / 128.0 - 1.0));

            let pixel = img.get_pixel(x, y)[0] as f64;
            output.put_pixel(x, y, Luma([if pixel > threshold { 255 } else { 0 }]));
        }
    }

    output
}

/// Compute integral image for O(1) sum queries.
fn compute_integral_image(img: &GrayImage) -> Vec<u64> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut integral = vec![0u64; (w + 1) * (h + 1)];

    for y in 0..h {
        for x in 0..w {
            let idx = (y + 1) * (w + 1) + (x + 1);
            let pixel = img.get_pixel(x as u32, y as u32)[0] as u64;
            integral[idx] = pixel
                + integral[(y + 1) * (w + 1) + x]
                + integral[y * (w + 1) + (x + 1)]
                - integral[y * (w + 1) + x];
        }
    }

    integral
}

/// Compute squared integral image for O(1) variance queries.
fn compute_integral_image_sq(img: &GrayImage) -> Vec<u64> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut integral = vec![0u64; (w + 1) * (h + 1)];

    for y in 0..h {
        for x in 0..w {
            let idx = (y + 1) * (w + 1) + (x + 1);
            let pixel = img.get_pixel(x as u32, y as u32)[0] as u64;
            let pixel_sq = pixel * pixel;
            integral[idx] = pixel_sq
                + integral[(y + 1) * (w + 1) + x]
                + integral[y * (w + 1) + (x + 1)]
                - integral[y * (w + 1) + x];
        }
    }

    integral
}

/// Compute local mean and variance for a window around (x, y) using integral images.
fn local_stats(
    integral: &[u64],
    integral_sq: &[u64],
    x: i32,
    y: i32,
    half_win: i32,
    w: i32,
    h: i32,
) -> (f64, f64) {
    let x1 = (x - half_win).max(0);
    let y1 = (y - half_win).max(0);
    let x2 = (x + half_win).min(w - 1);
    let y2 = (y + half_win).min(h - 1);

    let width = (w + 1) as usize;
    let area = ((x2 - x1 + 1) * (y2 - y1 + 1)) as f64;

    if area <= 0.0 {
        return (0.0, 0.0);
    }

    // Sum from integral image
    let idx_tl = (y1) as usize * width + (x1) as usize;
    let idx_tr = (y1) as usize * width + (x2 + 1) as usize;
    let idx_bl = (y2 + 1) as usize * width + (x1) as usize;
    let idx_br = (y2 + 1) as usize * width + (x2 + 1) as usize;

    let sum = integral[idx_br]
        .wrapping_sub(integral[idx_tr])
        .wrapping_sub(integral[idx_bl])
        .wrapping_add(integral[idx_tl]);

    // Sum of squares
    let sum_sq = integral_sq[idx_br]
        .wrapping_sub(integral_sq[idx_tr])
        .wrapping_sub(integral_sq[idx_bl])
        .wrapping_add(integral_sq[idx_tl]);

    let mean = sum as f64 / area;
    let variance = (sum_sq as f64 / area) - mean * mean;

    (mean, variance.max(0.0))
}

/// Otsu global threshold (fallback for when Sauvola is too slow).
#[allow(dead_code)]
pub fn otsu_threshold(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());
    let total_pixels = w * h;

    // Compute histogram
    let mut hist = [0u32; 256];
    for pixel in img.iter() {
        hist[*pixel as usize] += 1;
    }

    // Normalize to probabilities
    let mut prob = [0.0f64; 256];
    for i in 0..256 {
        prob[i] = hist[i] as f64 / total_pixels as f64;
    }

    // Find threshold that maximizes between-class variance
    let mut best_threshold = 128u8;
    let mut best_variance = 0.0f64;

    for t in 1..255 {
        let w0: f64 = prob[..t].iter().sum();
        let w1: f64 = prob[t..].iter().sum();

        if w0 < 1e-6 || w1 < 1e-6 {
            continue;
        }

        let mut mean0 = 0.0f64;
        let mut mean1 = 0.0f64;

        for i in 0..t {
            mean0 += i as f64 * prob[i] / w0;
        }
        for i in t..256 {
            mean1 += i as f64 * prob[i] / w1;
        }

        let variance = w0 * w1 * (mean0 - mean1).powi(2);
        if variance > best_variance {
            best_variance = variance;
            best_threshold = t as u8;
        }
    }

    // Apply threshold
    let mut output = GrayImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y)[0];
            output.put_pixel(x, y, Luma([if pixel > best_threshold { 255 } else { 0 }]));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sauvola_on_simple_image() {
        // Create document-like image: white background with dark text lines
        let mut img = GrayImage::new(100, 100);
        // White background
        for y in 0..100 {
            for x in 0..100 {
                img.put_pixel(x, y, Luma([220]));
            }
        }
        // Dark text lines (simulated with thin dark rectangles)
        for y in 0..100 {
            for x in 0..100 {
                // Alternate thin dark "text" lines
                if y % 10 < 3 && x > 10 && x < 90 {
                    img.put_pixel(x, y, Luma([30]));
                }
            }
        }

        let result = sauvola_threshold(&img, 25, 0.2);
        // Text line at y=1 should be black (0)
        let text_pixel1 = result.get_pixel(50, 1)[0];
        let text_pixel2 = result.get_pixel(50, 2)[0];
        assert_eq!(text_pixel1, 0, "Text line at y=1 should be black (0), got {}", text_pixel1);
        assert_eq!(text_pixel2, 0, "Text line at y=2 should be black (0), got {}", text_pixel2);
        // Background at y=5 should be white (255)
        let bg_pixel = result.get_pixel(50, 5)[0];
        assert_eq!(bg_pixel, 255, "Background at y=5 should be white (255), got {}", bg_pixel);
    }

    #[test]
    fn test_integral_image() {
        let mut img = GrayImage::new(4, 4);
        img.put_pixel(0, 0, Luma([1]));
        img.put_pixel(1, 0, Luma([2]));
        img.put_pixel(0, 1, Luma([3]));
        img.put_pixel(1, 1, Luma([4]));

        let integral = compute_integral_image(&img);
        let width = 5; // (w+1)
        // Sum of all 4 pixels at (2,2)
        let sum = integral[2 * width + 2];
        assert_eq!(sum, 1 + 2 + 3 + 4); // 10
    }

    #[test]
    fn test_otsu_on_bimodal() {
        // Create a bimodal image: half black, half white
        let mut img = GrayImage::new(50, 50);
        for y in 0..50 {
            for x in 0..50 {
                let val = if x < 25 { 30 } else { 200 };
                img.put_pixel(x, y, Luma([val]));
            }
        }
        let result = otsu_threshold(&img);
        // Should threshold correctly at ~115
        assert_eq!(result.get_pixel(10, 25)[0], 0); // dark side
        assert_eq!(result.get_pixel(35, 25)[0], 255); // light side
    }
}