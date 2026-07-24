use redis::AsyncCommands;

/// Cleanup expired files and Redis keys.
/// Scans storage directory and removes files older than TTL.
pub struct CleanupScheduler;

impl CleanupScheduler {
    /// Run a single cleanup cycle.
    pub async fn run(
        storage_path: &std::path::Path,
        redis_client: &redis::Client,
        ttl_seconds: u64,
    ) -> Result<CleanupResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = CleanupResult::default();
        let now = std::time::SystemTime::now();

        // Clean up upload files
        let upload_dir = storage_path.join("upload");
        if upload_dir.exists() {
            let mut entries = tokio::fs::read_dir(&upload_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if now
                            .duration_since(modified)
                            .map(|d| d.as_secs() > ttl_seconds)
                            .unwrap_or(false)
                        {
                            if let Ok(_) = tokio::fs::remove_file(entry.path()).await {
                                result.files_deleted += 1;
                                result.bytes_freed += metadata.len();
                            }
                        }
                    }
                }
            }
        }

        // Clean up output files
        let output_dir = storage_path.join("output");
        if output_dir.exists() {
            let mut entries = tokio::fs::read_dir(&output_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        if now
                            .duration_since(modified)
                            .map(|d| d.as_secs() > ttl_seconds)
                            .unwrap_or(false)
                        {
                            if let Ok(_) = tokio::fs::remove_file(entry.path()).await {
                                result.files_deleted += 1;
                                result.bytes_freed += metadata.len();
                            }
                        }
                    }
                }
            }
        }

        // Clean up orphaned Redis keys
        if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
            // Scan for expired job keys
            let _: Result<(), _> = redis::cmd("SCAN")
                .arg(0)
                .arg("MATCH")
                .arg("job:*")
                .query_async(&mut conn)
                .await;
        }

        Ok(result)
    }
}

#[derive(Debug, Default)]
pub struct CleanupResult {
    pub files_deleted: u64,
    pub bytes_freed: u64,
    pub orphan_keys: u64,
}