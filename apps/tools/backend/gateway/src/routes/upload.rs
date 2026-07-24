use std::sync::Arc;

use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use tokio::fs;
use uuid::Uuid;

use crate::routes::health::AppState;
use tools_common::error::UploadError;
use tools_common::types::*;

/// Handle POST /api/upload
pub async fn upload_handler(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, Json<serde_json::Value>)> {
    let config = &state.config;
    let max_size = config.max_file_size_bytes();

    // Extract fields from multipart
    let mut file_data: Option<(String, Vec<u8>)> = None;
    let mut tool_str: Option<String> = None;
    let mut options: serde_json::Value = serde_json::Value::Null;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let filename = field.file_name().unwrap_or("unknown").to_string();
                let data = field.bytes().await.map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({
                            "error": "Failed to read file",
                            "detail": e.to_string()
                        })),
                    )
                })?;
                file_data = Some((filename, data.to_vec()));
            }
            "tool" => {
                tool_str = Some(field.text().await.unwrap_or_default());
            }
            "options" => {
                let text = field.text().await.unwrap_or_default();
                if !text.is_empty() {
                    options = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
                }
            }
            _ => {}
        }
    }

    // Validate fields
    let (filename, data) = file_data.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No file provided" })),
        )
    })?;

    let tool_str = tool_str.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No tool specified" })),
        )
    })?;

    let tool = Tool::from_str(&tool_str).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Unknown tool: {}", tool_str) })),
        )
    })?;

    // Validate file size
    let file_size = data.len() as u64;
    if file_size > max_size {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "error": format!("File too large: {} bytes (max {} bytes)", file_size, max_size)
            })),
        ));
    }

    // Validate MIME type based on tool
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    validate_mime(&tool, &ext).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        )
    })?;

    // Verify magic bytes
    if !verify_magic_bytes(&data, &ext) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "File content does not match extension" })),
        ));
    }

    // Create directories
    let upload_dir = config.storage_path.join("upload");
    fs::create_dir_all(&upload_dir).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Storage error: {}", e) })),
        )
    })?;

    // Generate job ID and save file
    let job_id = Uuid::new_v4();
    let storage_filename = format!("{}.{}", job_id, ext);
    let file_path = upload_dir.join(&storage_filename);
    fs::write(&file_path, &data).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Failed to save file: {}", e) })),
        )
    })?;

    // Create job record
    let job = Job {
        id: job_id,
        tool: tool.clone(),
        status: JobStatus::Queued,
        file_path: file_path.to_string_lossy().to_string(),
        result_path: None,
        file_size,
        options: options.clone(),
        created_at: Utc::now(),
        ttl_seconds: config.job_ttl_seconds,
    };

    // Save to Redis
    {
        let mut conn = state.redis.get_multiplexed_async_connection().await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Redis error: {}", e) })),
            )
        })?;
        crate::redis::job::JobRepository::create(&mut conn, &job)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": format!("Failed to create job: {}", e) })),
                )
            })?;
    }

    // Publish to NATS
    crate::nats::publisher::NatsPublisher::publish_job(&state.nats, &tool, &job)
        .await
        .map_err(|e| {
            tracing::error!("Failed to publish job to NATS: {}", e);
        });

    // Update metrics
    state.metrics.increment_jobs_total(tool.as_str(), "queued");

    // Return response
    Ok(Json(UploadResponse {
        job_id,
        status: "queued".to_string(),
        tool: tool_str,
        ws_url: format!("/api/job/{}/ws", job_id),
        created_at: job.created_at,
        estimated_seconds: match tool {
            Tool::Scan => 5,
            _ => 3,
        },
    }))
}

fn validate_mime(tool: &Tool, ext: &str) -> Result<(), UploadError> {
    let image_exts = ["jpg", "jpeg", "png", "webp", "heic", "bmp", "tiff", "tif"];
    let pdf_exts = ["pdf"];
    let video_exts = ["mp4", "webm", "avi", "mov", "mkv"];
    let audio_exts = ["mp3", "wav", "flac", "aac", "ogg", "m4a"];

    match tool {
        Tool::Scan
        | Tool::ImageCompress
        | Tool::ImageResize
        | Tool::ImageConvert
        | Tool::RemoveBg => {
            if !image_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected image file, got .{}",
                    ext
                )));
            }
        }
        Tool::PdfMerge | Tool::PdfSplit | Tool::PdfCompress | Tool::PdfToImages => {
            if !pdf_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected PDF file, got .{}",
                    ext
                )));
            }
        }
        Tool::ImagesToPdf => {
            if !image_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected image file, got .{}",
                    ext
                )));
            }
        }
        Tool::VideoCompress | Tool::VideoTrim | Tool::GifMaker => {
            if !video_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected video file, got .{}",
                    ext
                )));
            }
        }
        Tool::AudioExtract => {
            if !video_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected video file, got .{}",
                    ext
                )));
            }
        }
        Tool::AudioConvert => {
            if !audio_exts.contains(&ext) {
                return Err(UploadError::InvalidMime(format!(
                    "Expected audio file, got .{}",
                    ext
                )));
            }
        }
    }
    Ok(())
}

fn verify_magic_bytes(data: &[u8], ext: &str) -> bool {
    if data.is_empty() {
        return false;
    }
    match ext {
        "jpg" | "jpeg" => data.starts_with(&[0xFF, 0xD8, 0xFF]),
        "png" => data.starts_with(&[0x89, 0x50, 0x4E, 0x47]),
        "webp" => data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP",
        "gif" => data.starts_with(b"GIF8"),
        "bmp" => data.starts_with(b"BM"),
        "pdf" => data.starts_with(b"%PDF"),
        "mp4" => data.len() > 8 && (&data[4..8] == b"ftyp" || &data[4..8] == b"ftyp"),
        "heic" => data.len() > 12 && &data[4..12] == b"ftypheic",
        _ => true,
    }
}