use image::GrayImage;

use tools_common::error::PipelineError;

/// OCR result with text and word-level bounding boxes.
pub struct OcrResult {
    pub full_text: String,
    pub words: Vec<OcrWord>,
    pub confidence: f32,
}

/// A single word detected by OCR with its bounding box.
#[derive(Debug, Clone)]
pub struct OcrWord {
    pub text: String,
    pub bbox: Bbox,
    pub confidence: i32,
}

/// Bounding box coordinates.
#[derive(Debug, Clone)]
pub struct Bbox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Initialize Tesseract OCR engine.
/// Uses leptess crate which binds to libtesseract.
/// Falls back gracefully if Tesseract is not installed.
#[cfg(feature = "tesseract")]
fn init_tesseract(lang: &str) -> Result<leptess::LepTess, PipelineError> {
    let tessdata_prefix = std::env::var("TESSDATA_PREFIX")
        .unwrap_or_else(|_| "/usr/share/tesseract-ocr/5/tessdata".to_string());

    let mut tess = leptess::LepTess::new(Some(&tessdata_prefix), lang)
        .map_err(|e| PipelineError::Ocr(format!("Failed to init Tesseract: {}", e)))?;

    Ok(tess)
}

/// Run OCR on a grayscale image and return extracted text.
/// Uses Tesseract via leptess crate when the "tesseract" feature is enabled.
/// Falls back to a placeholder when Tesseract is unavailable.
pub fn ocr_text(img: &GrayImage, lang: &str) -> Result<OcrResult, PipelineError> {
    #[cfg(feature = "tesseract")]
    {
        let mut tess = init_tesseract(lang)?;

        let width = img.width() as i32;
        let height = img.height() as i32;

        // Set image from memory
        tess.set_image_from_mem(&img.to_vec(), width, height, 1, width)
            .map_err(|e| PipelineError::Ocr(format!("Failed to set image: {}", e)))?;

        tess.set_source_resolution(300);

        // Set PSM to automatic
        tess.set_page_seg_mode(3);

        let text = tess.get_utf8_text()
            .map_err(|e| PipelineError::Ocr(format!("OCR failed: {}", e)))?;

        let words = tess.get_words()
            .iter()
            .map(|w| OcrWord {
                text: w.text.clone(),
                bbox: Bbox {
                    x: w.x,
                    y: w.y,
                    width: w.w,
                    height: w.h,
                },
                confidence: w.confidence,
            })
            .collect();

        let confidence = if words.is_empty() {
            0.0
        } else {
            words.iter().map(|w| w.confidence as f32).sum::<f32>() / words.len() as f32
        };

        Ok(OcrResult {
            full_text: text,
            words,
            confidence,
        })
    }

    #[cfg(not(feature = "tesseract"))]
    {
        tracing::warn!("Tesseract feature not enabled, OCR returning placeholder");
        Ok(OcrResult {
            full_text: String::new(),
            words: Vec::new(),
            confidence: 0.0,
        })
    }
}

/// Run OCR on a grayscale image, returning only the text.
pub fn ocr_text_only(img: &GrayImage, lang: &str) -> Result<String, PipelineError> {
    ocr_text(img, lang).map(|r| r.full_text)
}

/// Run OCR with word-level bounding boxes.
pub fn ocr_words(img: &GrayImage, lang: &str) -> Result<Vec<OcrWord>, PipelineError> {
    ocr_text(img, lang).map(|r| r.words)
}