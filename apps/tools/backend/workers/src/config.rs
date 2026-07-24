use std::path::PathBuf;

/// Worker configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub nats_url: String,
    pub redis_url: String,
    pub storage_path: PathBuf,
    pub concurrency: u32,
    pub job_ttl_seconds: u64,
    pub rust_log: String,
}

impl WorkerConfig {
    pub fn from_env() -> Self {
        Self {
            nats_url: env_or_default("NATS_URL", "nats://localhost:4222"),
            redis_url: env_or_default("REDIS_URL", "redis://localhost:6379"),
            storage_path: PathBuf::from(env_or_default("STORAGE_PATH", "/data/tools")),
            concurrency: env_or_default("TOOLS_WORKER_CONCURRENCY", "4")
                .parse()
                .unwrap_or(4),
            job_ttl_seconds: env_or_default("JOB_TTL_SECONDS", "3600")
                .parse()
                .unwrap_or(3600),
            rust_log: env_or_default("RUST_LOG", "info"),
        }
    }
}

fn env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}