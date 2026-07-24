use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

use crate::routes::health::AppState;
use tools_common::types::JobStatus;

/// Handle GET /api/download/{id}
pub async fn download_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Response, (StatusCode, JsonResponse)> {
    let mut conn = state.redis.get_multiplexed_async_connection().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({ "error": "Redis connection failed" })),
        )
    })?;

    let job = crate::redis::job::JobRepository::get(&mut conn, id)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                JsonResponse(serde_json::json!({ "error": "Job not found or expired" })),
            )
        })?;

    // Verify job is completed
    if job.status != JobStatus::Completed {
        return Err((
            StatusCode::BAD_REQUEST,
            JsonResponse(serde_json::json!({
                "error": "Job is not completed yet",
                "status": match job.status {
                    JobStatus::Queued => "queued",
                    JobStatus::Processing { .. } => "processing",
                    JobStatus::NeedsManualCrop => "needs_manual_crop",
                    JobStatus::Failed(_) => "failed",
                    _ => "unknown",
                }
            })),
        ));
    }

    let result_path = job.result_path.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            JsonResponse(serde_json::json!({ "error": "Result file path not found" })),
        )
    })?;

    // Open file
    let file = tokio::fs::File::open(&result_path).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            JsonResponse(serde_json::json!({ "error": format!("File not found: {}", e) })),
        )
    })?;

    let metadata = file.metadata().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            JsonResponse(serde_json::json!({
                "error": format!("Failed to read metadata: {}", e)
            })),
        )
    })?;

    // Determine content type
    let ext = result_path.rsplit('.').next().unwrap_or("bin").to_string();
    let content_type = match ext.as_str() {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "mp4" => "video/mp4",
        "mp3" => "audio/mpeg",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    };

    // Generate filename for download
    let file_name = format!(
        "{}_{}.{}",
        job.tool.as_str(),
        job.id.to_string().split('-').next().unwrap_or("result"),
        ext
    );

    // Stream the file
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let response = Response::builder()
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file_name),
        )
        .header(header::CONTENT_LENGTH, metadata.len().to_string())
        .body(body)
        .unwrap();

    Ok(response)
}

/// Wrapper for JSON error responses.
pub struct JsonResponse(pub serde_json::Value);

impl IntoResponse for JsonResponse {
    fn into_response(self) -> Response {
        (StatusCode::OK, axum::Json(self.0)).into_response()
    }
}