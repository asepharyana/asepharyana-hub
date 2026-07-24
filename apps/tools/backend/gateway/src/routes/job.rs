use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::routes::health::AppState;
use tools_common::types::*;

/// Handle GET /api/job/{id}
pub async fn job_status_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobStatusResponse>, (StatusCode, Json<serde_json::Value>)> {
    let mut conn = state.redis.get_multiplexed_async_connection().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Redis error: {}", e) })),
        )
    })?;

    let job = crate::redis::job::JobRepository::get(&mut conn, id)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Job not found or expired" })),
            )
        })?;

    let status_str = match &job.status {
        JobStatus::Queued => "queued",
        JobStatus::Processing { .. } => "processing",
        JobStatus::Completed => "completed",
        JobStatus::NeedsManualCrop => "needs_manual_crop",
        JobStatus::Failed(_) => "failed",
    };

    let (progress, stage, message) = match &job.status {
        JobStatus::Processing { stage, progress } => (*progress, stage.clone(), String::new()),
        JobStatus::Failed(msg) => (0, String::new(), msg.clone()),
        JobStatus::Completed => (100, "complete".to_string(), "Processing complete".to_string()),
        JobStatus::Queued => (0, "queued".to_string(), "Waiting in queue".to_string()),
        JobStatus::NeedsManualCrop => {
            (0, "manual_crop".to_string(), "Manual crop needed".to_string())
        }
    };

    let result = if job.status == JobStatus::Completed {
        let file_name = job
            .result_path
            .as_ref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("result")
            .to_string();
        Some(ResultInfo {
            download_url: format!("/api/download/{}", job.id),
            file_size: job.file_size,
            file_name,
            preview_url: Some(format!("/api/job/{}/preview", job.id)),
        })
    } else {
        None
    };

    let error = match &job.status {
        JobStatus::Failed(msg) => Some(msg.clone()),
        _ => None,
    };

    Ok(Json(JobStatusResponse {
        job_id: job.id,
        status: status_str.to_string(),
        tool: job.tool.as_str().to_string(),
        progress,
        stage,
        message,
        result,
        created_at: job.created_at,
        error,
    }))
}

/// Handle GET /api/job/{id}/preview
pub async fn job_preview_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, [(String, String); 2], Vec<u8>), (StatusCode, Json<serde_json::Value>)> {
    let mut conn = state.redis.get_multiplexed_async_connection().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Redis error: {}", e) })),
        )
    })?;

    let job = crate::redis::job::JobRepository::get(&mut conn, id)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": "Job not found or expired" })),
            )
        })?;

    let result_path = job.result_path.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No result available yet" })),
        )
    })?;

    let data = tokio::fs::read(&result_path).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("File not found: {}", e) })),
        )
    })?;

    let ext = result_path.rsplit('.').next().unwrap_or("bin").to_string();
    let content_type = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    };

    Ok((
        StatusCode::OK,
        [
            ("Content-Type".to_string(), content_type.to_string()),
            (
                "Cache-Control".to_string(),
                "private, max-age=300".to_string(),
            ),
        ],
        data,
    ))
}