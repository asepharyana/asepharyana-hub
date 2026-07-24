use async_nats::Client;
use futures::StreamExt;
use redis::AsyncCommands;
use uuid::Uuid;

use tools_common::types::{Job, JobStatus};

use crate::config::WorkerConfig;

/// NATS consumer setup and management.
pub struct JobConsumer;

impl JobConsumer {
    /// Connect to NATS.
    pub async fn connect(url: &str) -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
        Ok(async_nats::connect(url).await?)
    }

    /// Connect to Redis.
    pub async fn connect_redis(
        url: &str,
    ) -> Result<redis::Client, Box<dyn std::error::Error + Send + Sync>> {
        Ok(redis::Client::open(url)?)
    }

    /// Start consuming job messages from NATS for all tool groups.
    pub async fn start(
        nats: &Client,
        redis: &redis::Client,
        config: &WorkerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Subscribe to scan jobs
        let scan_sub = nats
            .queue_subscribe("tools.scan.jobs.>", "scan-workers".to_string())
            .await?;
        tracing::info!("Subscribed to tools.scan.jobs.>");

        // Subscribe to image jobs
        let image_sub = nats
            .queue_subscribe("tools.image.jobs.>", "image-workers".to_string())
            .await?;
        tracing::info!("Subscribed to tools.image.jobs.>");

        // Subscribe to pdf jobs
        let pdf_sub = nats
            .queue_subscribe("tools.pdf.jobs.>", "pdf-workers".to_string())
            .await?;
        tracing::info!("Subscribed to tools.pdf.jobs.>");

        // Subscribe to cleanup scheduler
        let cleanup_sub = nats
            .subscribe("tools.scheduler.cleanup".to_string())
            .await?;
        tracing::info!("Subscribed to tools.scheduler.cleanup");

        let redis_clone = redis.clone();
        let config_clone = config.clone();

        // Process messages concurrently
        tokio::select! {
            _ = Self::process_subscription(scan_sub, redis.clone(), config.clone()) => {},
            _ = Self::process_subscription(image_sub, redis.clone(), config.clone()) => {},
            _ = Self::process_subscription(pdf_sub, redis.clone(), config.clone()) => {},
            _ = Self::process_cleanup(cleanup_sub, config_clone) => {},
        }

        Ok(())
    }

    /// Process messages from a NATS subscription.
    async fn process_subscription(
        mut sub: async_nats::Subscriber,
        redis: redis::Client,
        config: WorkerConfig,
    ) {
        while let Some(msg) = sub.next().await {
            if let Ok(job) = serde_json::from_slice::<Job>(&msg.payload) {
                let redis = redis.clone();
                let config = config.clone();

                tokio::spawn(async move {
                    let tool = job.tool.clone();
                    tracing::info!(
                        job_id = %job.id,
                        tool = %tool.as_str(),
                        "Received job"
                    );

                    match Self::dispatch_job(tool, job, &redis, &config).await {
                        Ok(()) => tracing::info!("Job completed successfully"),
                        Err(e) => tracing::error!("Job failed: {}", e),
                    }
                });
            }
        }
    }

    /// Process cleanup scheduler messages.
    async fn process_cleanup(mut sub: async_nats::Subscriber, config: WorkerConfig) {
        while let Some(msg) = sub.next().await {
            tracing::info!("Running cleanup cycle");
            let redis_url = config.redis_url.clone();
            match redis::Client::open(redis_url.as_str()) {
                Ok(client) => {
                    match crate::scheduler::cleanup::CleanupScheduler::run(
                        &config.storage_path,
                        &client,
                        config.job_ttl_seconds,
                    )
                    .await
                    {
                        Ok(result) => {
                            tracing::info!(
                                "Cleanup: {} files deleted, {} bytes freed",
                                result.files_deleted,
                                result.bytes_freed
                            );
                        }
                        Err(e) => {
                            tracing::error!("Cleanup failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create Redis client for cleanup: {}", e);
                }
            }
            // Consume the message (no ack for core NATS)
            let _ = msg;
        }
    }

    /// Dispatch a job to the appropriate handler based on tool type.
    async fn dispatch_job(
        tool: tools_common::types::Tool,
        job: Job,
        redis: &redis::Client,
        config: &WorkerConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match tool {
            tools_common::types::Tool::Scan => {
                crate::scanner::process_job(job, redis, config).await
            }
            tools_common::types::Tool::ImageCompress
            | tools_common::types::Tool::ImageResize
            | tools_common::types::Tool::ImageConvert
            | tools_common::types::Tool::RemoveBg => {
                crate::image::process_job(job, redis, config).await
            }
            tools_common::types::Tool::PdfMerge
            | tools_common::types::Tool::PdfSplit
            | tools_common::types::Tool::ImagesToPdf
            | tools_common::types::Tool::PdfCompress
            | tools_common::types::Tool::PdfToImages => {
                crate::pdf::process_job(job, redis, config).await
            }
            _ => {
                tracing::warn!(tool = %tool.as_str(), "Tool handler not yet implemented");
                Ok(())
            }
        }
    }

    /// Update job result in Redis after processing.
    pub async fn update_job_result(
        conn: &mut impl AsyncCommands,
        job_id: Uuid,
        result_path: &str,
        ttl_seconds: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("job:{}", job_id);
        let json: String = conn
            .get(&key)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let mut job: Job = serde_json::from_str(&json)?;
        job.status = JobStatus::Completed;
        job.result_path = Some(result_path.to_string());
        let updated = serde_json::to_string(&job)?;
        let _: () = conn
            .set_ex(key, updated, ttl_seconds)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }
}