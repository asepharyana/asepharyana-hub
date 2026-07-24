use image::{DynamicImage, GrayImage, Luma};
use nalgebra::{Matrix3, SVD};

use tools_common::error::PipelineError;

use crate::scanner::corners::CornerPoint;

/// Compute homography matrix from 4 point correspondences using DLT algorithm.
pub fn compute_homography(
    src: &[CornerPoint; 4],
    dst: &[CornerPoint; 4],
) -> Result<[[f64; 3]; 3], PipelineError> {
    // Build 8x9 matrix A from 4 point correspondences
    // Each correspondence (x,y) -> (x',y') gives 2 rows:
    // [-x, -y, -1,  0,  0,  0, x*x', y*x', x']
    // [ 0,  0,  0, -x, -y, -1, x*y', y*y', y']
    let mut a = nalgebra::DMatrix::<f64>::zeros(8, 9);

    for i in 0..4 {
        let x = src[i].0;
        let y = src[i].1;
        let xp = dst[i].0;
        let yp = dst[i].1;

        // First row
        a[(i * 2, 0)] = -x;
        a[(i * 2, 1)] = -y;
        a[(i * 2, 2)] = -1.0;
        a[(i * 2, 3)] = 0.0;
        a[(i * 2, 4)] = 0.0;
        a[(i * 2, 5)] = 0.0;
        a[(i * 2, 6)] = x * xp;
        a[(i * 2, 7)] = y * xp;
        a[(i * 2, 8)] = xp;

        // Second row
        a[(i * 2 + 1, 0)] = 0.0;
        a[(i * 2 + 1, 1)] = 0.0;
        a[(i * 2 + 1, 2)] = 0.0;
        a[(i * 2 + 1, 3)] = -x;
        a[(i * 2 + 1, 4)] = -y;
        a[(i * 2 + 1, 5)] = -1.0;
        a[(i * 2 + 1, 6)] = x * yp;
        a[(i * 2 + 1, 7)] = y * yp;
        a[(i * 2 + 1, 8)] = yp;
    }

    // Solve Ah = 0 via SVD: h = last column of V
    let svd = SVD::new(a, true, true);
    if let Some(v_t) = &svd.v_t {
        let nrows = v_t.nrows();
        if nrows > 0 {
            let h_vec: Vec<f64> = v_t.row(nrows - 1).iter().copied().collect();
            if h_vec.len() >= 9 {
                let h = [
                    [h_vec[0], h_vec[1], h_vec[2]],
                    [h_vec[3], h_vec[4], h_vec[5]],
                    [h_vec[6], h_vec[7], h_vec[8]],
                ];
                return Ok(h);
            }
        }
    }

    Err(PipelineError::Warp("SVD decomposition failed".to_string()))
}

/// Invert a 3x3 homography matrix.
pub fn invert_homography(h: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let m = Matrix3::new(h[0][0], h[0][1], h[0][2], h[1][0], h[1][1], h[1][2], h[2][0], h[2][1], h[2][2]);
    let inv = m
        .try_inverse()
        .unwrap_or(Matrix3::identity());
    [
        [inv[(0, 0)], inv[(0, 1)], inv[(0, 2)]],
        [inv[(1, 0)], inv[(1, 1)], inv[(1, 2)]],
        [inv[(2, 0)], inv[(2, 1)], inv[(2, 2)]],
    ]
}

/// Apply homography to a point (forward mapping).
pub fn apply_homography(h: &[[f64; 3]; 3], x: f64, y: f64) -> (f64, f64) {
    let z = h[2][0] * x + h[2][1] * y + h[2][2];
    if z.abs() < 1e-10 {
        return (x, y);
    }
    let xp = (h[0][0] * x + h[0][1] * y + h[0][2]) / z;
    let yp = (h[1][0] * x + h[1][1] * y + h[1][2]) / z;
    (xp, yp)
}

/// Bilinear interpolation at sub-pixel coordinates.
fn bilinear_interpolate(img: &GrayImage, x: f64, y: f64) -> Luma<u8> {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let w = img.width() as i32;
    let h = img.height() as i32;

    // Clamp coordinates
    let x0 = x0.clamp(0, w - 1);
    let x1 = x1.clamp(0, w - 1);
    let y0 = y0.clamp(0, h - 1);
    let y1 = y1.clamp(0, h - 1);

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let p00 = img.get_pixel(x0 as u32, y0 as u32)[0] as f64;
    let p10 = img.get_pixel(x1 as u32, y0 as u32)[0] as f64;
    let p01 = img.get_pixel(x0 as u32, y1 as u32)[0] as f64;
    let p11 = img.get_pixel(x1 as u32, y1 as u32)[0] as f64;

    let val = p00 * (1.0 - fx) * (1.0 - fy)
        + p10 * fx * (1.0 - fy)
        + p01 * (1.0 - fx) * fy
        + p11 * fx * fy;

    Luma([val.round().clamp(0.0, 255.0) as u8])
}

/// Apply perspective warp to correct the document perspective.
/// Takes the original color image and 4 corners, returns warped image.
pub fn warp_perspective(
    img: &DynamicImage,
    corners: [CornerPoint; 4],
) -> Result<DynamicImage, PipelineError> {
    let [tl, tr, br, bl] = corners;

    // Compute target width and height (preserve aspect ratio)
    let width_top = distance(tl, tr);
    let width_bot = distance(bl, br);
    let width = width_top.max(width_bot).ceil() as u32;

    let height_left = distance(tl, bl);
    let height_right = distance(tr, br);
    let height = height_left.max(height_right).ceil() as u32;

    // Clamp output dimensions
    let width = width.min(3000).max(1);
    let height = height.min(3000).max(1);

    let src = [tl, tr, br, bl];
    let dst = [
        (0.0, 0.0),
        (width as f64, 0.0),
        (width as f64, height as f64),
        (0.0, height as f64),
    ];

    let h = compute_homography(&src, &dst)?;
    let h_inv = invert_homography(&h);

    let gray = img.to_luma8();
    let mut output = GrayImage::new(width, height);

    // Backward mapping: for each output pixel, find source pixel
    for y in 0..height {
        for x in 0..width {
            let (sx, sy) = apply_homography(&h_inv, x as f64, y as f64);
            let pixel = bilinear_interpolate(&gray, sx, sy);
            output.put_pixel(x, y, pixel);
        }
    }

    Ok(DynamicImage::ImageLuma8(output))
}

/// Euclidean distance between two points.
fn distance(a: CornerPoint, b: CornerPoint) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_homography_identity() {
        // Identity mapping should produce identity matrix
        let src = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let dst = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let h = compute_homography(&src, &dst).unwrap();
        let (xp, yp) = apply_homography(&h, 50.0, 50.0);
        assert!((xp - 50.0).abs() < 1.0);
        assert!((yp - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_invert_homography() {
        let h = [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
        let inv = invert_homography(&h);
        let (xp, yp) = apply_homography(&inv, 100.0, 100.0);
        assert!((xp - 50.0).abs() < 0.001);
        assert!((yp - 50.0).abs() < 0.001);
    }

    #[test]
    fn test_bilinear_interpolate() {
        let mut img = GrayImage::new(3, 3);
        img.put_pixel(0, 0, Luma([100]));
        img.put_pixel(1, 0, Luma([200]));
        let pixel = bilinear_interpolate(&img, 0.5, 0.0);
        assert_eq!(pixel[0], 150); // Midpoint between 100 and 200
    }
}