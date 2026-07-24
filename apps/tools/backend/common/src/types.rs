use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a processing job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Queued,
    Processing {
        stage: String,
        progress: u8,
    },
    Completed,
    NeedsManualCrop,
    Failed(String),
}

/// Available tool types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Tool {
    Scan,
    ImageCompress,
    ImageResize,
    ImageConvert,
    RemoveBg,
    PdfMerge,
    PdfSplit,
    ImagesToPdf,
    PdfCompress,
    PdfToImages,
    VideoCompress,
    AudioExtract,
    VideoTrim,
    GifMaker,
    AudioConvert,
}

impl Tool {
    /// Returns the NATS subject prefix for this tool.
    pub fn subject_prefix(&self) -> &'static str {
        match self {
            Tool::Scan => "tools.scan",
            Tool::ImageCompress
            | Tool::ImageResize
            | Tool::ImageConvert
            | Tool::RemoveBg => "tools.image",
            Tool::PdfMerge
            | Tool::PdfSplit
            | Tool::ImagesToPdf
            | Tool::PdfCompress
            | Tool::PdfToImages => "tools.pdf",
            Tool::VideoCompress | Tool::VideoTrim | Tool::GifMaker => "tools.video",
            Tool::AudioExtract | Tool::AudioConvert => "tools.audio",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Tool::Scan => "scan",
            Tool::ImageCompress => "image-compress",
            Tool::ImageResize => "image-resize",
            Tool::ImageConvert => "image-convert",
            Tool::RemoveBg => "remove-bg",
            Tool::PdfMerge => "pdf-merge",
            Tool::PdfSplit => "pdf-split",
            Tool::ImagesToPdf => "images-to-pdf",
            Tool::PdfCompress => "pdf-compress",
            Tool::PdfToImages => "pdf-to-images",
            Tool::VideoCompress => "video-compress",
            Tool::AudioExtract => "audio-extract",
            Tool::VideoTrim => "video-trim",
            Tool::GifMaker => "gif-maker",
            Tool::AudioConvert => "audio-convert",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scan" => Some(Tool::Scan),
            "image-compress" => Some(Tool::ImageCompress),
            "image-resize" => Some(Tool::ImageResize),
            "image-convert" => Some(Tool::ImageConvert),
            "remove-bg" => Some(Tool::RemoveBg),
            "pdf-merge" => Some(Tool::PdfMerge),
            "pdf-split" => Some(Tool::PdfSplit),
            "images-to-pdf" => Some(Tool::ImagesToPdf),
            "pdf-compress" => Some(Tool::PdfCompress),
            "pdf-to-images" => Some(Tool::PdfToImages),
            "video-compress" => Some(Tool::VideoCompress),
            "audio-extract" => Some(Tool::AudioExtract),
            "video-trim" => Some(Tool::VideoTrim),
            "gif-maker" => Some(Tool::GifMaker),
            "audio-convert" => Some(Tool::AudioConvert),
            _ => None,
        }
    }
}

/// Options for document scanning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanOptions {
    pub ocr: bool,
    pub enhance: bool,
    pub output_format: OutputFormat,
    pub dpi: u32,
    pub quality: u8,
    pub language: String,
    pub color_mode: ColorMode,
    pub page_size: PageSize,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            ocr: true,
            enhance: true,
            output_format: OutputFormat::Pdf,
            dpi: 300,
            quality: 90,
            language: "eng+ind".to_string(),
            color_mode: ColorMode::BlackAndWhite,
            page_size: PageSize::A4,
        }
    }
}

/// Options for image tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageOptions {
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: Option<String>,
    pub fit: Option<String>,
    pub bg_color: Option<[u8; 3]>,
}

/// Options for PDF tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfOptions {
    pub quality: Option<u8>,
    pub pages: Option<String>,
    pub dpi: Option<u32>,
    pub page_size: Option<PageSize>,
    pub margin_mm: Option<u32>,
}

/// A complete job record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub tool: Tool,
    pub status: JobStatus,
    pub file_path: String,
    pub result_path: Option<String>,
    pub file_size: u64,
    pub options: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub ttl_seconds: u64,
}

/// Progress update sent via NATS and forwarded via WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    pub job_id: Uuid,
    pub status: JobStatus,
    pub stage: String,
    pub progress: u8,
    pub message: String,
}

/// Response returned after successful upload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub job_id: Uuid,
    pub status: String,
    pub tool: String,
    pub ws_url: String,
    pub created_at: DateTime<Utc>,
    pub estimated_seconds: u8,
}

/// Job status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub status: String,
    pub tool: String,
    pub progress: u8,
    pub stage: String,
    pub message: String,
    pub result: Option<ResultInfo>,
    pub created_at: DateTime<Utc>,
    pub error: Option<String>,
}

/// Result metadata included in status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultInfo {
    pub download_url: String,
    pub file_size: u64,
    pub file_name: String,
    pub preview_url: Option<String>,
}

/// Output format for scan results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    Pdf,
    Jpeg,
    Png,
}

/// Color mode for processed output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorMode {
    BlackAndWhite,
    Grayscale,
    Color,
}

/// Page size for PDF output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageSize {
    A4,
    Letter,
    Auto,
}