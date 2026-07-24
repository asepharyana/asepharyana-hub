pub mod binarize;
pub mod corners;
pub mod deskew;
pub mod edge;
pub mod enhance;
pub mod ocr;
pub mod pdf;
pub mod pipeline;
pub mod preprocess;
pub mod shadow;
pub mod warp;

use crate::config::WorkerConfig;
use crate::nats::progress::ProgressReporter;
use tools_common::types::{Job, JobStatus, Tool};

/// Process a scan job through the full pipeline.
pub async fn process_job(
    job: Job,
    redis: &redis::Client,
    config: &WorkerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(job_id = %job.id, "Processing scan job");

    let nats = async_nats::connect(&config.nats_url).await?;
    let progress = ProgressReporter::new(redis.clone(), nats, job.id, Tool::Scan);

    progress
        .report(
            JobStatus::Processing {
                stage: "preprocess".to_string(),
                progress: 5,
            },
            "preprocess",
            5,
            "Memproses gambar...",
        )
        .await?;

    let result = pipeline::process(&job, config, &progress).await;

    match result {
        Ok(scan_result) => {
            progress
                .report(JobStatus::Completed, "complete", 100, "Scan selesai")
                .await?;

            let mut conn = redis.get_multiplexed_async_connection().await?;
            crate::nats::consumer::JobConsumer::update_job_result(
                &mut conn,
                job.id,
                &scan_result.output_path,
                job.ttl_seconds,
            )
            .await?;

            tracing::info!(
                job_id = %job.id,
                output = %scan_result.output_path,
                duration_ms = %scan_result.processing_time_ms,
                "Scan job completed"
            );

            Ok(())
        }
        Err(e) => {
            progress
                .report(
                    JobStatus::Failed(e.to_string()),
                    "error",
                    0,
                    &format!("Gagal: {}", e),
                )
                .await?;

            tracing::error!(job_id = %job.id, error = %e, "Scan job failed");
            Err(e)
        }
    }
}