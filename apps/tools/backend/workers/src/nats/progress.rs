use redis::AsyncCommands;
use uuid::Uuid;

use tools_common::types::{JobStatus, Tool};

/// Reports progress from worker to NATS and Redis.
pub struct ProgressReporter {
    redis: redis::Client,
    nats: async_nats::Client,
    job_id: Uuid,
    tool: Tool,
}

impl ProgressReporter {
    pub fn new(redis: redis::Client, nats: async_nats::Client, job_id: Uuid, tool: Tool) -> Self {
        Self {
            redis,
            nats,
            job_id,
            tool,
        }
    }

    /// Report progress: updates Redis and publishes to NATS.
    pub async fn report(
        &self,
        status: JobStatus,
        stage: &str,
        progress: u8,
        message: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Update Redis
        if let Ok(mut conn) = self.redis.get_multiplexed_async_connection().await {
            let key = format!("job:{}", self.job_id);
            if let Ok(json) = conn.get::<_, String>(&key).await {
                if let Ok(mut job) = serde_json::from_str::<tools_common::types::Job>(&json) {
                    job.status = status.clone();
                    let updated = serde_json::to_string(&job).unwrap_or(json);
                    let _: Result<(), _> = conn.set_ex(key, updated, job.ttl_seconds).await;
                }
            }
        }

        // Publish to NATS
        let progress_msg = tools_common::types::JobProgress {
            job_id: self.job_id,
            status,
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
        };

        let subject = format!("tools.{}.progress.{}", self.tool.subject_prefix(), self.job_id);
        if let Ok(payload) = serde_json::to_vec(&progress_msg) {
            let _ = self.nats.publish(subject, payload.into()).await;
        }

        tracing::debug!(
            job_id = %self.job_id,
            stage = %stage,
            progress = %progress,
            "Progress update"
        );

        Ok(())
    }

    pub fn job_id(&self) -> Uuid {
        self.job_id
    }
}

impl Clone for ProgressReporter {
    fn clone(&self) -> Self {
        Self {
            redis: self.redis.clone(),
            nats: self.nats.clone(),
            job_id: self.job_id,
            tool: self.tool.clone(),
        }
    }
}