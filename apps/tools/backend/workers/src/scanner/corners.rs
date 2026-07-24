use image::GrayImage;
use imageproc::contours::find_contours;

use tools_common::error::PipelineError;

/// Represents a detected corner point.
pub type CornerPoint = (f64, f64);

/// The fallback reason if corner detection fails.
pub enum FallbackReason {
    NoContours,
    NoRectangularContour,
    TooSmall,
}

/// Find the 4 corners of the document from an edge image.
pub fn detect_corners(edges: &GrayImage) -> Result<[CornerPoint; 4], FallbackReason> {
    let contours = find_contours::<u8>(edges);
    if contours.is_empty() {
        return Err(FallbackReason::NoContours);
    }

    // Convert contours to use i32 coordinates
    let contour_points: Vec<Vec<(i32, i32)>> = contours
        .iter()
        .map(|c| c.points.iter().map(|p| (p.x as i32, p.y as i32)).collect())
        .collect();

    // Sort by area descending
    let mut sorted: Vec<_> = contour_points.iter().collect();
    sorted.sort_by(|a, b| {
        contour_area_slice(b)
            .partial_cmp(&contour_area_slice(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for points in sorted.iter().take(5) {
        if let Some(corners) = approx_quadrilateral(points) {
            let ordered = order_corners(&corners);
            return Ok(ordered);
        }
    }

    // Fallback: use bounding rect of largest contour
    if let Some(largest) = sorted.first() {
        let rect = bounding_rect_slice(largest);
        let corners = vec![
            (rect.0 as f64, rect.1 as f64),
            (rect.2 as f64, rect.1 as f64),
            (rect.2 as f64, rect.3 as f64),
            (rect.0 as f64, rect.3 as f64),
        ];
        return Ok(order_corners(&corners));
    }

    Err(FallbackReason::NoContours)
}

/// Compute the area of a contour using the Shoelace formula.
fn contour_area_slice(points: &[(i32, i32)]) -> f64 {
    let n = points.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += points[i].0 as f64 * points[j].1 as f64;
        area -= points[j].0 as f64 * points[i].1 as f64;
    }
    area.abs() / 2.0
}

/// Approximate a contour to a quadrilateral.
fn approx_quadrilateral(points: &[(i32, i32)]) -> Option<Vec<CornerPoint>> {
    let n = points.len();
    if n < 4 {
        return None;
    }

    let top = points.iter().min_by(|a, b| a.1.cmp(&b.1))?;
    let bottom = points.iter().max_by(|a, b| a.1.cmp(&b.1))?;
    let left = points.iter().min_by(|a, b| a.0.cmp(&b.0))?;
    let right = points.iter().max_by(|a, b| a.0.cmp(&b.0))?;

    Some(vec![
        (left.0 as f64, left.1 as f64),
        (right.0 as f64, top.1 as f64),
        (right.0 as f64, bottom.1 as f64),
        (left.0 as f64, bottom.1 as f64),
    ])
}

/// Order 4 corners: top-left, top-right, bottom-right, bottom-left.
fn order_corners(points: &[CornerPoint]) -> [CornerPoint; 4] {
    let mut pts: Vec<CornerPoint> = points.to_vec();
    let mut ordered = [(0.0, 0.0); 4];

    if pts.len() >= 4 {
        // Sort by position
        // TL = min(x+y), BR = max(x+y)
        pts.sort_by(|a, b| {
            (a.0 + a.1)
                .partial_cmp(&(b.0 + b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ordered[0] = pts[0]; // TL
        ordered[2] = pts[3]; // BR

        // TR = max(x - y), BL = min(x - y)
        pts.sort_by(|a, b| {
            (a.0 - a.1)
                .partial_cmp(&(b.0 - b.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        ordered[1] = pts[3]; // TR
        ordered[3] = pts[0]; // BL
    }

    ordered
}

/// Compute bounding rectangle: (left, top, right, bottom).
fn bounding_rect_slice(points: &[(i32, i32)]) -> (i32, i32, i32, i32) {
    let left = points.iter().map(|p| p.0).min().unwrap_or(0);
    let top = points.iter().map(|p| p.1).min().unwrap_or(0);
    let right = points.iter().map(|p| p.0).max().unwrap_or(0);
    let bottom = points.iter().map(|p| p.1).max().unwrap_or(0);
    (left, top, right, bottom)
}

/// Detect corners with fallback: full resolution, then half, then error.
pub fn detect_corners_with_fallback(
    edges: &GrayImage,
) -> Result<[CornerPoint; 4], PipelineError> {
    // Attempt 1: Full resolution
    if let Ok(corners) = detect_corners(edges) {
        return Ok(corners);
    }

    // Attempt 2: Half resolution
    let (w, h) = (edges.width() / 2, edges.height() / 2);
    if w > 10 && h > 10 {
        let half = image::imageops::resize(
            edges,
            w,
            h,
            image::imageops::FilterType::Lanczos3,
        );
        if let Ok(corners) = detect_corners(&half) {
            return Ok(corners.map(|(x, y)| (x * 2.0, y * 2.0)));
        }
    }

    Err(PipelineError::CornerDetection(
        "Could not detect document corners automatically".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contour_area_slice() {
        let points = vec![(0, 0), (100, 0), (100, 100), (0, 100)];
        let area = contour_area_slice(&points);
        assert!((area - 10000.0).abs() < 1.0);
    }

    #[test]
    fn test_bounding_rect_slice() {
        let points = vec![(10, 20), (100, 30), (90, 150), (5, 140)];
        let rect = bounding_rect_slice(&points);
        assert_eq!(rect, (5, 20, 100, 150));
    }
}