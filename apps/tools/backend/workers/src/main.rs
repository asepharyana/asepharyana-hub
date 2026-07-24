mod config;
mod image;
mod nats;
mod pdf;
mod scanner;
mod scheduler;
mod video;
mod audio;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    let config = config::WorkerConfig::from_env();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.rust_log))
        .init();

    tracing::info!("Starting tools-workers...");

    // Connect to NATS
    let nats = nats::consumer::JobConsumer::connect(&config.nats_url)
        .await
        .expect("Failed to connect to NATS");
    tracing::info!("Connected to NATS at {}", config.nats_url);

    // Connect to Redis
    let redis = nats::consumer::JobConsumer::connect_redis(&config.redis_url)
        .await
        .expect("Failed to connect to Redis");
    tracing::info!("Connected to Redis at {}", config.redis_url);

    // Start NATS consumers (blocks forever)
    tracing::info!("Starting job consumers...");
    if let Err(e) = nats::consumer::JobConsumer::start(&nats, &redis, &config).await {
        tracing::error!("Consumer error: {}", e);
    }
}