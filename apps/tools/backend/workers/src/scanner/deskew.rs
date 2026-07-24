use image::{GrayImage, Luma};
use image::imageops;

/// Detect and correct small rotation (<5°) of text lines using Hough transform.
pub fn deskew(img: &GrayImage) -> GrayImage {
    let lines = hough_lines(img, 10, 50);

    if lines.is_empty() {
        return img.clone();
    }

    // Compute median angle of all detected lines
    let angles: Vec<f64> = lines
        .iter()
        .map(|line| line.angle_deg())
        .filter(|a| a.abs() < 45.0) // Skip vertical lines
        .collect();

    if angles.is_empty() {
        return img.clone();
    }

    let median_angle = median(&angles);

    // Skip if angle is very small (<0.5°)
    if median_angle.abs() < 0.5 {
        return img.clone();
    }

    // Rotate image
    rotate_image(img, median_angle)
}

/// Represents a line detected by Hough transform.
#[derive(Debug, Clone)]
struct HoughLine {
    rho: f64,
    theta: f64,
}

impl HoughLine {
    fn angle_deg(&self) -> f64 {
        self.theta.to_degrees() - 90.0
    }
}

/// Simple Hough line detection.
fn hough_lines(img: &GrayImage, threshold: u32, _max_lines: usize) -> Vec<HoughLine> {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let max_rho = ((w * w + h * h) as f64).sqrt().ceil() as i32;

    let theta_step = 1.0_f64.to_radians();
    let num_thetas = 180;

    // Accumulator
    let mut accumulator =
        vec![vec![0u32; (2 * max_rho + 1) as usize]; num_thetas];

    // Vote
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x as u32, y as u32)[0] > 128 {
                for t_idx in 0..num_thetas {
                    let theta = t_idx as f64 * theta_step;
                    let rho = (x as f64 * theta.cos() + y as f64 * theta.sin()).round() as i32;
                    let rho_idx = rho + max_rho;
                    if rho_idx >= 0 && (rho_idx as usize) < accumulator[t_idx].len() {
                        accumulator[t_idx][rho_idx as usize] += 1;
                    }
                }
            }
        }
    }

    // Find local maxima above threshold
    let mut lines = Vec::new();
    for t_idx in 0..num_thetas {
        let theta = t_idx as f64 * theta_step;
        for (r_idx, &count) in accumulator[t_idx].iter().enumerate() {
            if count > threshold {
                let rho = r_idx as i32 - max_rho;
                lines.push(HoughLine {
                    rho: rho as f64,
                    theta,
                });
            }
        }
    }

    // Sort by votes (descending) and take top N
    lines.sort_by(|a, b| {
        let a_idx = (a.theta / theta_step).round() as usize;
        let b_idx = (b.theta / theta_step).round() as usize;
        let a_rho_idx = (a.rho + max_rho as f64).round() as usize;
        let b_rho_idx = (b.rho + max_rho as f64).round() as usize;
        let a_count = accumulator[a_idx.min(num_thetas - 1)][a_rho_idx.min(accumulator[0].len() - 1)];
        let b_count = accumulator[b_idx.min(num_thetas - 1)][b_rho_idx.min(accumulator[0].len() - 1)];
        b_count.cmp(&a_count)
    });

    lines.truncate(100);
    lines
}

/// Compute median of a sorted slice of f64 values.
fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Rotate an image by the given angle in degrees.
fn rotate_image(img: &GrayImage, angle_deg: f64) -> GrayImage {
    let angle_rad = angle_deg.to_radians();
    let (w, h) = (img.width(), img.height());

    // Compute new image dimensions to fit the rotated content
    let cos = angle_rad.cos().abs();
    let sin = angle_rad.sin().abs();
    let new_w = (w as f64 * cos + h as f64 * sin).ceil() as u32;
    let new_h = (w as f64 * sin + h as f64 * cos).ceil() as u32;
    let new_w = new_w.max(1);
    let new_h = new_h.max(1);

    let mut output = GrayImage::new(new_w, new_h);
    let cx = w as f64 / 2.0;
    let cy = h as f64 / 2.0;
    let new_cx = new_w as f64 / 2.0;
    let new_cy = new_h as f64 / 2.0;

    // Backward mapping
    for out_y in 0..new_h {
        for out_x in 0..new_w {
            // Translate to origin, rotate, translate back
            let dx = out_x as f64 - new_cx;
            let dy = out_y as f64 - new_cy;
            let src_x = dx * cos + dy * sin + cx;
            let src_y = -dx * sin + dy * cos + cy;

            if src_x >= 0.0 && src_x < w as f64 - 1.0 && src_y >= 0.0 && src_y < h as f64 - 1.0 {
                // Bilinear interpolation
                let x0 = src_x.floor() as u32;
                let y0 = src_y.floor() as u32;
                let x1 = (x0 + 1).min(w - 1);
                let y1 = (y0 + 1).min(h - 1);
                let fx = src_x - x0 as f64;
                let fy = src_y - y0 as f64;

                let p00 = img.get_pixel(x0, y0)[0] as f64;
                let p10 = img.get_pixel(x1, y0)[0] as f64;
                let p01 = img.get_pixel(x0, y1)[0] as f64;
                let p11 = img.get_pixel(x1, y1)[0] as f64;

                let val = p00 * (1.0 - fx) * (1.0 - fy)
                    + p10 * fx * (1.0 - fy)
                    + p01 * (1.0 - fx) * fy
                    + p11 * fx * fy;

                output.put_pixel(out_x, out_y, Luma([val.round().clamp(0.0, 255.0) as u8]));
            } else {
                output.put_pixel(out_x, out_y, Luma([255])); // White padding
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_odd() {
        let v = vec![1.0, 3.0, 5.0];
        assert!((median(&v) - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_median_even() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        assert!((median(&v) - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_empty() {
        assert!((median(&[]) - 0.0).abs() < 0.001);
    }
}