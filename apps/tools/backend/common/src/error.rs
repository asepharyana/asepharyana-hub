use thiserror::Error;

/// Errors that can occur during file upload.
#[derive(Debug, Error)]
pub enum UploadError {
    #[error("Invalid MIME type: {0}")]
    InvalidMime(String),

    #[error("File too large: {0} bytes exceeds maximum of {1} bytes")]
    FileTooLarge(u64, u64),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Virus or suspicious content detected")]
    VirusDetected,

    #[error("Invalid tool: {0}")]
    InvalidTool(String),

    #[error("Missing file in upload")]
    MissingFile,

    #[error("Missing tool parameter")]
    MissingTool,

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Errors during processing pipeline execution.
#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("Failed to load image: {0}")]
    ImageLoad(String),

    #[error("Edge detection failed: {0}")]
    EdgeDetection(String),

    #[error("Corner detection failed: {0}")]
    CornerDetection(String),

    #[error("Perspective warp failed: {0}")]
    Warp(String),

    #[error("Shadow removal failed: {0}")]
    ShadowRemoval(String),

    #[error("Binarization failed: {0}")]
    Binarization(String),

    #[error("OCR processing failed: {0}")]
    Ocr(String),

    #[error("PDF generation failed: {0}")]
    PdfGeneration(String),

    #[error("Pipeline timed out")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Errors related to NATS messaging.
#[derive(Debug, Error)]
pub enum NatsError {
    #[error("Failed to publish message: {0}")]
    Publish(String),

    #[error("Failed to subscribe: {0}")]
    Subscribe(String),

    #[error("JetStream error: {0}")]
    JetStream(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("NATS connection error: {0}")]
    Connection(String),
}

/// Errors related to Redis operations.
#[derive(Debug, Error)]
pub enum RedisError {
    #[error("Redis connection failed: {0}")]
    Connection(String),

    #[error("Redis query failed: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Key not found: {0}")]
    NotFound(String),
}

impl From<redis::RedisError> for RedisError {
    fn from(e: redis::RedisError) -> Self {
        RedisError::Query(e.to_string())
    }
}