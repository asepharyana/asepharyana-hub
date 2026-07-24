use crate::config::WorkerConfig;
use tools_common::types::Job;

/// Process an image tool job.
pub async fn process_job(
    job: Job,
    _redis: &redis::Client,
    _config: &WorkerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(job_id = %job.id, tool = %job.tool.as_str(), "Processing image job (stub)");
    // TODO: Phase 2.2 - implement actual image processing
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    tracing::info!(job_id = %job.id, "Image job completed");
    Ok(())
}