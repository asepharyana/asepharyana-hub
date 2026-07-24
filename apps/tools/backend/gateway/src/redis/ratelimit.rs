use redis::AsyncCommands;

/// Sliding window rate limiter using Redis sorted sets.
pub struct RateLimiter;

impl RateLimiter {
    /// Check if a request is within the rate limit.
    pub async fn check(
        conn: &mut impl AsyncCommands,
        ip: &str,
        tool: &str,
        max_per_minute: u32,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let key = format!("ratelimit:{}:{}", ip, tool);
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - 60_000;

        // Remove entries outside the window
        let _: usize = conn.zrembyscore(&key, 0, window_start).await?;

        // Add current entry
        let _: usize = conn
            .zadd(&key, format!("{}:{}", ip, now), now as f64)
            .await?;

        // Set TTL on the key (cleanup)
        let _: usize = conn.expire(&key, 120).await?;

        // Count entries in window
        let count: u32 = conn.zcount(&key, window_start, now).await?;

        Ok(count <= max_per_minute)
    }

    /// Get remaining requests within the current window.
    pub async fn remaining(
        conn: &mut impl AsyncCommands,
        ip: &str,
        tool: &str,
        max_per_minute: u32,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        let key = format!("ratelimit:{}:{}", ip, tool);
        let now = chrono::Utc::now().timestamp_millis();
        let window_start = now - 60_000;

        let count: u32 = conn.zcount(&key, window_start, now).await?;

        Ok(max_per_minute.saturating_sub(count))
    }
}