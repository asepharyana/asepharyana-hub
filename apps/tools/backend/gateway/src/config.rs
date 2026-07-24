use std::path::PathBuf;

/// Application configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub port: u16,
    pub nats_url: String,
    pub redis_url: String,
    pub storage_path: PathBuf,
    pub max_file_size_mb: u64,
    pub job_ttl_seconds: u64,
    pub rate_limit_per_minute: u32,
    pub rust_log: String,
}

impl AppConfig {
    /// Load configuration from environment variables with sensible defaults.
    pub fn from_env() -> Self {
        Self {
            port: env_or_default("GATEWAY_PORT", "3001")
                .parse()
                .unwrap_or(3001),
            nats_url: env_or_default("NATS_URL", "nats://localhost:4222"),
            redis_url: env_or_default("REDIS_URL", "redis://localhost:6379"),
            storage_path: PathBuf::from(env_or_default("STORAGE_PATH", "/data/tools")),
            max_file_size_mb: env_or_default("MAX_FILE_SIZE_MB", "50")
                .parse()
                .unwrap_or(50),
            job_ttl_seconds: env_or_default("JOB_TTL_SECONDS", "3600")
                .parse()
                .unwrap_or(3600),
            rate_limit_per_minute: env_or_default("RATE_LIMIT_PER_MINUTE", "30")
                .parse()
                .unwrap_or(30),
            rust_log: env_or_default("RUST_LOG", "info"),
        }
    }

    pub fn max_file_size_bytes(&self) -> u64 {
        self.max_file_size_mb * 1024 * 1024
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::from_env();
        assert_eq!(config.port, 3001);
        assert_eq!(config.nats_url, "nats://localhost:4222");
        assert_eq!(config.redis_url, "redis://localhost:6379");
        assert_eq!(config.max_file_size_mb, 50);
        assert_eq!(config.job_ttl_seconds, 3600);
        assert_eq!(config.rate_limit_per_minute, 30);
    }

    #[test]
    fn test_file_size_bytes() {
        let config = AppConfig::from_env();
        assert_eq!(config.max_file_size_bytes(), 50 * 1024 * 1024);
    }

    #[test]
    fn test_env_override() {
        std::env::set_var("GATEWAY_PORT", "9999");
        let config = AppConfig::from_env();
        assert_eq!(config.port, 9999);
        std::env::remove_var("GATEWAY_PORT");
    }
}