use std::path::Path;
use std::time::Instant;

use image::DynamicImage;
use tools_common::error::PipelineError;
use tools_common::types::Job;

use crate::config::WorkerConfig;
use crate::nats::progress::ProgressReporter;

use super::binarize::sauvola_threshold;
use super::corners::detect_corners_with_fallback;
use super::deskew::deskew;
use super::edge::detect_edges;
use super::enhance::enhance_final;
use super::preprocess::preprocess;
use super::shadow::remove_shadow;
use super::warp::warp_perspective;

/// Result of the scanning pipeline.
pub struct ScanResult {
    pub output_path: String,
    pub page_count: u32,
    pub file_size: u64,
    pub ocr_text: Option<String>,
    pub processing_time_ms: u64,
}

/// Run the full scanner pipeline with all stages.
pub async fn process(
    job: &Job,
    config: &WorkerConfig,
    progress: &ProgressReporter,
) -> Result<ScanResult, Box<dyn std::error::Error + Send + Sync>> {
    let start = Instant::now();
    let input_path = Path::new(&job.file_path);

    // Create output directory
    let output_dir = config.storage_path.join("output");
    tokio::fs::create_dir_all(&output_dir).await?;

    // Stage 1: Load & Preprocess (0-15%)
    report(progress, "preprocess", 5, "Memuat dan meresize gambar...").await;
    let gray = preprocess(input_path)
        .map_err(|e| format!("Preprocess failed: {}", e))?;

    // Stage 2: Edge Detection (15-30%)
    report(progress, "edge_detection", 20, "Mendeteksi tepi dokumen...").await;
    let edges = detect_edges(&gray).map_err(|e| format!("Edge detection failed: {}", e))?;

    // Stage 3: Corner Detection (30-40%)
    report(progress, "corner_detection", 35, "Mencari sudut dokumen...").await;
    let corners = detect_corners_with_fallback(&edges)?;

    // Stage 4: Perspective Warp (40-55%)
    report(progress, "warp", 45, "Meluruskan perspektif dokumen...").await;
    let image = image::open(input_path)
        .map_err(|e| PipelineError::ImageLoad(e.to_string()))?;
    let warped = warp_perspective(&image, corners)?;

    // Stage 5: Shadow Removal (55-70%)
    report(progress, "shadow_removal", 60, "Menghilangkan bayangan...").await;
    let warped_gray = warped.to_luma8();
    let clean = remove_shadow(&warped_gray);

    // Stage 6: Binarization (70-80%)
    report(progress, "binarization", 75, "Mengubah ke hitam-putih...").await;
    let binary = sauvola_threshold(&clean, 30, 0.2);

    // Stage 7: Deskew (80-87%)
    report(progress, "deskew", 82, "Meluruskan teks...").await;
    let final_img = deskew(&binary);

    // Stage 8: Enhance (87-93%)
    report(progress, "enhance", 90, "Mengoptimalkan kualitas...").await;
    let final_img = enhance_final(&final_img);

    // Stage 9: Save output (93-100%)
    report(progress, "save", 95, "Menyimpan hasil...").await;

    let output_filename = format!("{}.png", progress.job_id());
    let output_path = output_dir.join(&output_filename);

    final_img.save(&output_path)?;

    let elapsed = start.elapsed().as_millis() as u64;

    tracing::info!(
        job_id = %progress.job_id(),
        duration_ms = elapsed,
        "Pipeline complete"
    );

    Ok(ScanResult {
        output_path: output_path.to_string_lossy().to_string(),
        page_count: 1,
        file_size: tokio::fs::metadata(&output_path).await.map(|m| m.len()).unwrap_or(0),
        ocr_text: None,
        processing_time_ms: elapsed,
    })
}

/// Helper to report progress.
async fn report(progress: &ProgressReporter, stage: &str, pct: u8, msg: &str) {
    let _ = progress
        .report(
            tools_common::types::JobStatus::Processing {
                stage: stage.to_string(),
                progress: pct,
            },
            stage,
            pct,
            msg,
        )
        .await;
}