use std::sync::Arc;

use axum::{extract::State, Json};
use redis::AsyncCommands;
use serde::Serialize;

use crate::metrics::Metrics;

/// Shared application state accessible from all handlers.
pub struct AppState {
    pub redis: redis::Client,
    pub nats: async_nats::Client,
    pub config: crate::config::AppConfig,
    pub metrics: Metrics,
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub redis: String,
    pub nats: String,
    pub uptime_seconds: u64,
}

/// Handle GET /health
pub async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let redis_status = {
        match state.redis.get_multiplexed_async_connection().await {
            Ok(mut conn) => match redis::cmd("PING").query_async::<String>(&mut conn).await {
                Ok(_) => "connected".to_string(),
                Err(_) => "error".to_string(),
            },
            Err(_) => "disconnected".to_string(),
        }
    };

    let nats_status = if state
        .nats
        .publish("tools.health.check", b"ping".to_vec().into())
        .await
        .is_ok()
    {
        "connected".to_string()
    } else {
        "disconnected".to_string()
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        redis: redis_status,
        nats: nats_status,
        uptime_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    })
}

/// Handle GET /metrics
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> Result<String, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    Ok(state.metrics.format())
}