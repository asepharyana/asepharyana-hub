use crate::config::WorkerConfig;
use tools_common::types::Job;

/// Process a PDF tool job.
pub async fn process_job(
    job: Job,
    _redis: &redis::Client,
    _config: &WorkerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!(job_id = %job.id, tool = %job.tool.as_str(), "Processing PDF job (stub)");
    // TODO: Phase 3 - implement actual PDF processing
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    tracing::info!(job_id = %job.id, "PDF job completed");
    Ok(())
}