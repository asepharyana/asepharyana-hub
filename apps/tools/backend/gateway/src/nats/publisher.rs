use async_nats::Client;
use tools_common::error::NatsError;
use tools_common::nats;
use tools_common::types::{Job, JobProgress, Tool};

/// NATS publisher for job and progress messages.
pub struct NatsPublisher;

impl NatsPublisher {
    /// Connect to NATS server.
    pub async fn connect(url: &str) -> Result<Client, NatsError> {
        async_nats::connect(url)
            .await
            .map_err(|e| NatsError::Connection(e.to_string()))
    }

    /// Publish a job to the appropriate NATS subject.
    pub async fn publish_job(nats: &Client, tool: &Tool, job: &Job) -> Result<(), NatsError> {
        let prefix = tool.subject_prefix();
        let subject = nats::job_subject(prefix, &job.id.to_string());
        let payload = serde_json::to_vec(job)
            .map_err(|e| NatsError::Publish(e.to_string()))?;

        nats.publish(subject, payload.into())
            .await
            .map_err(|e| NatsError::Publish(e.to_string()))?;

        tracing::debug!(
            job_id = %job.id,
            tool = %tool.as_str(),
            "Published job to NATS"
        );

        Ok(())
    }

    /// Publish a progress update to the NATS progress subject.
    pub async fn publish_progress(
        nats: &Client,
        progress: &JobProgress,
    ) -> Result<(), NatsError> {
        let tool_prefix = ""; // We need the tool from somewhere — stored in progress
        let subject = format!("tools.*.progress.{}", progress.job_id);
        let payload = serde_json::to_vec(progress)
            .map_err(|e| NatsError::Publish(e.to_string()))?;

        nats.publish(subject, payload.into())
            .await
            .map_err(|e| NatsError::Publish(e.to_string()))?;

        Ok(())
    }

    /// Subscribe to NATS progress updates for a specific job.
    pub async fn subscribe_progress(
        nats: &Client,
        job_id: &str,
    ) -> Result<async_nats::Subscriber, NatsError> {
        let subject = format!("tools.*.progress.{}", job_id);
        nats.subscribe(subject)
            .await
            .map_err(|e| NatsError::Subscribe(e.to_string()))
    }
}