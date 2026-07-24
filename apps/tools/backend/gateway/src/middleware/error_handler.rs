use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Unified JSON error response format.
#[derive(Debug)]
pub struct AppError {
    pub status_code: StatusCode,
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::BAD_REQUEST,
            code: "bad_request".to_string(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::NOT_FOUND,
            code: "not_found".to_string(),
            message: message.into(),
        }
    }

    pub fn too_large(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::PAYLOAD_TOO_LARGE,
            code: "file_too_large".to_string(),
            message: message.into(),
        }
    }

    pub fn rate_limited(retry_after: u64) -> Self {
        Self {
            status_code: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limit_exceeded".to_string(),
            message: format!("Rate limit exceeded. Retry after {} seconds", retry_after),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error".to_string(),
            message: message.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body = json!({
            "error": self.message,
            "code": self.code,
        });
        (self.status_code, Json(body)).into_response()
    }
}