use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod config;
mod metrics;
mod middleware;
mod nats;
mod redis;
mod routes;

use config::AppConfig;
use metrics::Metrics;
use routes::health::AppState;

#[tokio::main]
async fn main() {
    // Load config
    let config = AppConfig::from_env();

    // Init logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.rust_log))
        .init();

    tracing::info!("Starting tools-gateway...");

    // Init Redis client
    let redis_client = redis::create_client(&config.redis_url)
        .expect("Failed to create Redis client");
    tracing::info!("Redis client created for {}", config.redis_url);

    // Init NATS connection
    let nats = nats::publisher::NatsPublisher::connect(&config.nats_url)
        .await
        .expect("Failed to connect to NATS");
    tracing::info!("Connected to NATS at {}", config.nats_url);

    // Ensure NATS streams exist
    if let Err(e) = ensure_nats_streams(&nats).await {
        tracing::warn!("Failed to create NATS streams: {}", e);
    }

    // Init metrics
    let metrics = Metrics::new();

    // Shared state
    let state = Arc::new(AppState {
        redis: redis_client,
        nats,
        config: config.clone(),
        metrics,
    });

    // Build router
    let app = Router::new()
        .route("/api/upload", post(routes::upload::upload_handler))
        .route("/api/job/{id}", get(routes::job::job_status_handler))
        .route(
            "/api/job/{id}/preview",
            get(routes::job::job_preview_handler),
        )
        .route("/api/job/{id}/ws", get(routes::ws::ws_handler))
        .route("/api/download/{id}", get(routes::download::download_handler))
        .route("/health", get(routes::health::health_handler))
        .route("/metrics", get(routes::health::metrics_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(RequestBodyLimitLayer::new(
            ((config.max_file_size_mb + 1) * 1024 * 1024) as usize,
        ))
        .with_state(state);

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

/// Ensure required NATS JetStream streams exist.
async fn ensure_nats_streams(
    nats: &async_nats::Client,
) -> Result<(), Box<dyn std::error::Error>> {
    let js = async_nats::jetstream::new(nats.clone());

    match js
        .get_or_create_stream(tools_common::nats::jobs_stream_config())
        .await
    {
        Ok(_) => tracing::info!("NATS stream 'tools-jobs' ready"),
        Err(e) => tracing::warn!("Failed to create tools-jobs stream: {}", e),
    }

    match js
        .get_or_create_stream(tools_common::nats::progress_stream_config())
        .await
    {
        Ok(_) => tracing::info!("NATS stream 'tools-progress' ready"),
        Err(e) => tracing::warn!("Failed to create tools-progress stream: {}", e),
    }

    Ok(())
}

/// Handle graceful shutdown on SIGINT/SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutting down gateway...");
}