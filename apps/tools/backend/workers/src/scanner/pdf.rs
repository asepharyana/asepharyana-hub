use image::GrayImage;
use lopdf::{Document, Object, Stream, Dictionary};

use tools_common::error::PipelineError;

/// A4 page dimensions in points (1 pt = 1/72 inch).
pub const A4_WIDTH_PT: f64 = 595.28;
pub const A4_HEIGHT_PT: f64 = 841.89;

/// Generate a searchable PDF with JPEG image + invisible OCR text layer.
pub fn generate_searchable_pdf(
    image_data: &[u8],
    _ocr_text: &str,
    words: &[super::ocr::OcrWord],
    page_width: f64,
    page_height: f64,
) -> Result<Vec<u8>, PipelineError> {
    let mut doc = Document::new();

    // ── Pages object ──
    let pages_id = doc.new_object_id();
    let mut pages = Dictionary::new();
    pages.set("Type", Object::Name("Pages".as_bytes().to_vec()));
    pages.set("Kids", Object::Array(vec![]));
    pages.set("Count", Object::Integer(0));
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    // ── Image XObject ──
    let mut img_dict = Dictionary::new();
    img_dict.set("Type", Object::Name("XObject".as_bytes().to_vec()));
    img_dict.set("Subtype", Object::Name("Image".as_bytes().to_vec()));
    img_dict.set("Width", Object::Integer(page_width as i64));
    img_dict.set("Height", Object::Integer(page_height as i64));
    img_dict.set("ColorSpace", Object::Name("DeviceGray".as_bytes().to_vec()));
    img_dict.set("BitsPerComponent", Object::Integer(8));
    img_dict.set("Filter", Object::Name("DCTDecode".as_bytes().to_vec()));

    let image_stream = Stream::new(img_dict, image_data.to_vec());
    let image_id = doc.add_object(Object::Stream(image_stream));

    // ── Content stream: place image + invisible text ──
    let mut content = Vec::new();

    // Place image at full page
    content.extend_from_slice(b"q\n");
    content.extend_from_slice(
        format!("{} 0 0 {} 0 0 cm\n", page_width, page_height).as_bytes(),
    );
    content.extend_from_slice(b"/Im0 Do\n");
    content.extend_from_slice(b"Q\n");

    // Add invisible text layer (searchable)
    for word in words {
        let x = word.bbox.x as f64 / 300.0 * 72.0;
        let y = page_height - (word.bbox.y as f64 / 300.0 * 72.0);
        let font_size = (word.bbox.height as f64 / 300.0 * 72.0 * 0.8).max(4.0);

        content.extend_from_slice(b"BT\n");
        content.extend_from_slice(b"3 Tr\n"); // Rendering mode: invisible (neither fill nor stroke)
        content.extend_from_slice(
            format!("/F1 {} Tf\n{} {} Td\n", font_size, x, y - font_size).as_bytes(),
        );
        content.extend_from_slice(
            format!("({}) Tj\n", escape_pdf_string(&word.text)).as_bytes(),
        );
        content.extend_from_slice(b"ET\n");
    }

    let content_stream = Stream::new(Dictionary::new(), content);
    let content_id = doc.add_object(Object::Stream(content_stream));

    // ── Font dictionary ──
    let mut font_dict = Dictionary::new();
    let mut f1 = Dictionary::new();
    f1.set("Type", Object::Name("Font".as_bytes().to_vec()));
    f1.set("Subtype", Object::Name("Type1".as_bytes().to_vec()));
    f1.set("BaseFont", Object::Name("Helvetica".as_bytes().to_vec()));
    font_dict.set("F1", Object::Dictionary(f1));

    // ── Resources dictionary ──
    let mut xobject_dict = Dictionary::new();
    xobject_dict.set("Im0", Object::Reference(image_id));

    let mut resources = Dictionary::new();
    resources.set("XObject", Object::Dictionary(xobject_dict));
    resources.set("Font", Object::Dictionary(font_dict));

    // ── Page object ──
    let page_id = doc.new_object_id();
    let mut page = Dictionary::new();
    page.set("Type", Object::Name("Page".as_bytes().to_vec()));
    page.set("Parent", Object::Reference(pages_id));
    page.set(
        "MediaBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(page_width as f32),
            Object::Real(page_height as f32),
        ]),
    );
    page.set("Contents", Object::Reference(content_id));
    page.set("Resources", Object::Dictionary(resources));

    doc.objects.insert(page_id, Object::Dictionary(page));

    // ── Update pages object ──
    if let Some(Object::Dictionary(ref mut pages_dict)) = doc.objects.get_mut(&pages_id) {
        pages_dict.set("Count", Object::Integer(1));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
    }

    // ── Save ──
    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PipelineError::PdfGeneration(e.to_string()))?;

    Ok(output)
}

/// Escape special characters for PDF string literals.
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result
}

/// Compress grayscale image as JPEG bytes.
pub fn compress_image_jpeg(img: &GrayImage, quality: u8) -> Result<Vec<u8>, PipelineError> {
    let mut bytes = Vec::new();
    let rgb = image::DynamicImage::ImageLuma8(img.clone()).into_rgb8();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality);
    encoder
        .encode(
            rgb.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|e| PipelineError::PdfGeneration(format!("JPEG compression failed: {}", e)))?;
    Ok(bytes)
}

/// Compress RGB image data as JPEG bytes.
pub fn compress_rgb_image_jpeg(
    data: &[u8],
    width: u32,
    height: u32,
    quality: u8,
) -> Result<Vec<u8>, PipelineError> {
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality);
    encoder
        .encode(data, width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| PipelineError::PdfGeneration(format!("JPEG compression failed: {}", e)))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;

    #[test]
    fn test_escape_pdf_string() {
        assert_eq!(escape_pdf_string("hello"), "hello");
        assert_eq!(escape_pdf_string("(parens)"), "\\(parens\\)");
        assert_eq!(escape_pdf_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_jpeg_compression() {
        let img = GrayImage::from_pixel(100, 100, Luma([128]));
        let result = compress_image_jpeg(&img, 90);
        assert!(result.is_ok(), "JPEG compression failed: {:?}", result.err());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
        assert_eq!(&bytes[0..2], &[0xFF, 0xD8]);
    }
}