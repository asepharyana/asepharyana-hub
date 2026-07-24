use redis::{AsyncCommands, RedisError};
use uuid::Uuid;

use tools_common::types::Job;

/// Repository for job CRUD operations on Redis.
pub struct JobRepository;

impl JobRepository {
    /// Create a new job record in Redis with TTL.
    pub async fn create(
        conn: &mut impl AsyncCommands,
        job: &Job,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("job:{}", job.id);
        let json = serde_json::to_string(job)?;
        let _: () = conn
            .set_ex(key, json, job.ttl_seconds)
            .await
            .map_err(|e: RedisError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }

    /// Get a job by ID from Redis.
    pub async fn get(
        conn: &mut impl AsyncCommands,
        job_id: Uuid,
    ) -> Result<Job, Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("job:{}", job_id);
        let json: String = conn.get(&key).await.map_err(|_| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Job {} not found", job_id),
            )) as Box<dyn std::error::Error + Send + Sync>
        })?;
        let job: Job = serde_json::from_str(&json)?;
        Ok(job)
    }

    /// Update the status of a job in Redis and refresh TTL.
    pub async fn update_status(
        conn: &mut impl AsyncCommands,
        job_id: Uuid,
        status: &tools_common::types::JobStatus,
        result_path: Option<String>,
        ttl_seconds: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("job:{}", job_id);
        let json: String = conn
            .get(&key)
            .await
            .map_err(|e: RedisError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        let mut job: Job = serde_json::from_str(&json)?;
        job.status = status.clone();
        if let Some(path) = result_path {
            job.result_path = Some(path);
        }
        let json = serde_json::to_string(&job)?;
        let _: () = conn
            .set_ex(key, json, ttl_seconds)
            .await
            .map_err(|e: RedisError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }

    /// Delete a job from Redis.
    pub async fn delete(
        conn: &mut impl AsyncCommands,
        job_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let key = format!("job:{}", job_id);
        let _: usize = conn
            .del(key)
            .await
            .map_err(|e: RedisError| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }
}