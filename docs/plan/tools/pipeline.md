# Document Scanner — Processing Pipeline

> **LEGACY (2026-08-02):** Dokumen plan ini ditulis saat infra masih Docker/Traefik. Produksi sekarang Caddy + Nix/systemd dengan port 4000-an. Gunakan hanya sebagai referensi historis.

Ini adalah inti dari project. Pipeline mengubah foto dokumen HP jadi dokumen scan yang proper. Setiap tahap dibahas detail teknisnya.

## Pipeline Overview

```
Input: Foto HP (JPEG/PNG/HEIC, 2-12MP)
                │
                ▼
┌──────────────────────────────────┐
│ 1. Preprocess  ──▶ resize +      │
│    konversi grayscale            │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 2. Edge Detection  ──▶ cari      │
│    kontur dokumen                │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 3. Corner Detection  ──▶ 4 titik │
│    sudut dokumen                 │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 4. Perspective Warp  ──▶ lurusin│
│    (homography)                  │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 5. Shadow Removal  ──▶ iluminasi │
│    merata                        │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 6. Binarization  ──▶ hitam-putih │
│    bersih                        │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 7. Deskew  ──▶ lurusin teks      │
│    (kalau masih miring)          │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 8. OCR  ──▶ extract teks         │
└────────────────┬─────────────────┘
                 │
                 ▼
┌──────────────────────────────────┐
│ 9. Generate PDF  ──▶ output      │
│    PDF + hidden text layer       │
└────────────────┬─────────────────┘
                 │
                 ▼
Output: searchable PDF + teks OCR
```

---

## Stage 1: Preprocess

### Input
- Raw image dari HP (bisa 4000×3000 = 12MP, ~3-5MB JPEG)
- Format: JPEG, PNG, HEIC (via `image` crate, HEIC butuh feature)

### Proses
```rust
use image::{DynamicImage, imageops};

fn preprocess(img: &DynamicImage) -> DynamicImage {
    // 1. Resize kalau terlalu besar → max 2000px di sisi terpanjang
    //    Ini penting: edge detection di resolusi tinggi lambat
    //    dan ga nambah akurasi secara signifikan
    let max_dim = 2000.0;
    let (w, h) = (img.width() as f64, img.height() as f64);
    let img = if w.max(h) > max_dim {
        let scale = max_dim / w.max(h);
        let new_w = (w * scale) as u32;
        let new_h = (h * scale) as u32;
        img.resize_exact(new_w, new_h, imageops::FilterType::Lanczos3)
    } else {
        img.clone()
    };

    // 2. Grayscale → untuk edge detection
    img.grayscale()
}
```

### Edge Cases
| Kasus | Penanganan |
|-------|-----------|
| Foto resolusi rendah (<800px) | Skip resize, langsung proses |
| HEIC format | Butuh feature `heic` di `image` crate |
| Grayscale input | `img.grayscale()` no-op |
| Foto malam/noise tinggi | Gaussian blur sebelum edge detection |

---

## Stage 2: Edge Detection

### Tujuan
Cari tepi dokumen dalam foto. Ini hardest part karena background bisa kacau.

### Algoritma: Canny Edge Detection + Adaptive Threshold

```rust
use image::GrayImage;
use imageproc::edges::canny;

fn detect_edges(img: &GrayImage) -> GrayImage {
    // Canny dengan dual threshold
    // low: 50, high: 150 — parameter ini harus di-tune
    // buat kondisi pencahayaan yang berbeda
    canny(img, 50.0, 150.0)
}
```

### Masalah & Solusi

| Masalah | Penyebab | Solusi |
|---------|----------|--------|
| **Tepi dokumen putus** | Kontras rendah, bayangan | Morphological close (dilate → erode) untuk sambungin tepi |
| **Tepi palsu** | Background ramai (meja motif, lantai) | Cari contour terbesar + area terluas = dokumen |
| **Tidak ada tepi** | Background putih, dokumen putih (kertas di meja putih) | Adaptive threshold dulu sebelum Canny, atau fallback ke manual crop |
| **Noise garis** | Texture background | Gaussian blur (kernel 5x5) sebelum Canny |

### Implementation Detail

```rust
/// Edge detection yang robust terhadap berbagai kondisi
fn robust_edge_detection(img: &GrayImage) -> GrayImage {
    // 1. Gaussian blur untuk noise reduction
    let blurred = imageproc::filter::gaussian_blur_f32(img, 3.0);

    // 2. Coba Canny standard
    let edges = canny(&blurred, 50.0, 150.0);

    // 3. Morphological close untuk sambung tepi yang putus
    let kernel = imageproc::morphology::dilate_square(5);
    let closed = imageproc::morphology::close(&edges, &kernel);

    // 4. Kalau jumlah tepi terlalu sedikit (<1% pixels),
    //    ulang dengan threshold lebih rendah
    let edge_count = count_non_zero(&closed);
    let total_pixels = (closed.width() * closed.height()) as u32;
    if edge_count < total_pixels / 100 {
        let edges2 = canny(&blurred, 20.0, 80.0);
        return imageproc::morphology::close(&edges2, &kernel);
    }

    closed
}
```

---

## Stage 3: Corner Detection

### Tujuan
Dari edge image, cari 4 sudut dokumen.

### Algoritma: Contour Detection → Largest Rectangle

```rust
use imageproc::contours::{find_contours, Contour};

fn find_document_corners(edges: &GrayImage) -> Option<[(f64, f64); 4]> {
    // 1. Cari semua contours
    let contours = find_contours(edges);

    // 2. Filter: cuma contour dengan area > 20% dari total image
    //    (dokumen biasanya mengisi sebagian besar frame)
    let total_area = edges.width() as f64 * edges.height() as f64;
    let docs: Vec<&Contour> = contours
        .iter()
        .filter(|c| area_perimeter_ratio(c) > 0.3)
        .collect();

    // 3. Approximate polygon → cari yang 4 sisi
    for contour in docs {
        // Approximate contour ke polygon
        let polygon = approximate_polygon(&contour.points, 4);
        if let Some(vertices) = polygon {
            // Urutkan: top-left, top-right, bottom-right, bottom-left
            let corners = order_corners(vertices);
            return Some(corners);
        }
    }

    // 4. Fallback: contour terbesar → bounding rect
    contours.iter()
        .max_by_key(|c| c.points.len())
        .map(|c| {
            let rect = bounding_rect(&c.points);
            order_corners(vec![
                (rect.left as f64, rect.top as f64),
                (rect.right as f64, rect.top as f64),
                (rect.right as f64, rect.bottom as f64),
                (rect.left as f64, rect.bottom as f64),
            ])
        })
}
```

### Corner Ordering Convention

```
(0,0)  top-left ────────── top-right (w,0)
           │                    │
           │    DOKUMEN         │
           │                    │
(0,h) bottom-left ────── bottom-right (w,h)
```

### Fallback Strategy

Kalau auto-detect gagal total (contour tidak ketemu, confidence rendah):
1. **Fallback 1**: Coba di resolusi lebih rendah (noise berkurang)
2. **Fallback 2**: Coba adaptive threshold + Canny ulang
3. **Fallback 3**: Minta user crop manual — 4 draggable corners di canvas

```rust
fn detect_corners_with_fallback(img: &GrayImage) -> Result<[(f64, f64); 4], CropMode> {
    // Attempt 1: Resolusi penuh
    if let Some(corners) = find_document_corners(img) {
        return Ok(corners);
    }

    // Attempt 2: Half resolution (noise reduction)
    let half = image::imageops::resize(img, img.width() / 2, img.height() / 2,
                                        imageops::FilterType::Lanczos3);
    if let Some(corners) = find_document_corners(&half) {
        return Ok(corners.map(|(x, y)| (x * 2.0, y * 2.0)));
    }

    // Fallback: user manual
    Err(CropMode::Manual)
}
```

---

## Stage 4: Perspective Warp

### Tujuan
Transform 4 titik sudut ke persegi panjang (rectangular). Koreksi perspektif dari foto miring.

### Algoritma: Homography

```rust
use image::{DynamicImage, GrayImage};
use std::f64::consts::PI;

fn perspective_warp(img: &DynamicImage, corners: [(f64, f64); 4]) -> DynamicImage {
    // Target: persegi panjang dengan aspect ratio dokumen
    // Hitung lebar dan tinggi target dari 4 corner
    let [tl, tr, br, bl] = corners;

    let width_top = distance(tl, tr);
    let width_bot = distance(bl, br);
    let width = width_top.max(width_bot).ceil() as u32;

    let height_left = distance(tl, bl);
    let height_right = distance(tr, br);
    let height = height_left.max(height_right).ceil() as u32;

    // Source points (4 corners dari detection)
    let src = [
        tl,  // top-left
        tr,  // top-right
        br,  // bottom-right
        bl,  // bottom-left
    ];

    // Destination points (rectangle)
    let dst = [
        (0.0, 0.0),           // top-left
        (width as f64, 0.0),  // top-right
        (width as f64, height as f64), // bottom-right
        (0.0, height as f64), // bottom-left
    ];

    // Hitung homography matrix
    let h = compute_homography(&src, &dst);

    // Apply warp (backward mapping + bilinear interpolation)
    warp_image(img, &h, width, height)
}
```

### Homography Matrix

```
H = [h11 h12 h13]    x' = (h11*x + h12*y + h13) / (h31*x + h32*y + 1)
    [h21 h22 h23]    y' = (h21*x + h22*y + h23) / (h31*x + h32*y + 1)
    [h31 h32  1 ]
```

Komputasi manual (tanpa OpenCV):
```rust
/// Compute homography from 4 point correspondences using DLT algorithm
fn compute_homography(src: &[(f64, f64); 4], dst: &[(f64, f64); 4]) -> [[f64; 3]; 3] {
    // Direct Linear Transform
    // Bangun matrix A (8x9) dari 4 titik
    // Solve Ah = 0 via SVD → h = last column of V
    // Reshape ke 3x3
    //
    // Detail implementasi:
    // Setiap titik correspondence (x,y) → (x',y') menghasilkan 2 baris:
    // [-x, -y, -1,  0,  0,  0, x*x', y*x', x'] = 0
    // [ 0,  0,  0, -x, -y, -1, x*y', y*y', y'] = 0
    //
    // 4 titik → 8 baris → SVD → H matrix

    // Implementasi SVD atau pakai crate `nalgebra` atau `splines`
    todo!("Implement DLT + SVD")
}
```

### Image Warp (Backward Mapping)

```rust
fn warp_image(img: &DynamicImage, h: &[[f64; 3]; 3], width: u32, height: u32) -> DynamicImage {
    let gray = img.grayscale().into_luma8();
    let mut output = GrayImage::new(width, height);

    // Inverse homography (backward mapping)
    // tiap pixel output = sample dari input
    let h_inv = invert_homography(h);

    for y in 0..height {
        for x in 0..width {
            // Map (x,y) → source image coordinates
            let (sx, sy) = apply_homography(&h_inv, x as f64, y as f64);

            // Bilinear interpolation
            let pixel = bilinear_interpolate(&gray, sx, sy);
            output.put_pixel(x, y, pixel);
        }
    }

    DynamicImage::ImageLuma8(output)
}
```

### Edge Cases

| Masalah | Solusi |
|---------|--------|
| Dokuen sangat miring (>60°) | Warping mungkin hasilnya gepeng. Deteksi dan skip kalau sudut terlalu ekstrim |
| Output sangat besar | Clamp width/height ke max 3000px |
| Pixel jaggy (aliasing) | Bilinear interpolation (bukan nearest neighbor) |
| Koordinat negative | Clamp ke 0 |
| Warp membuat rasio aneh | Lock aspect ratio ke common (A4=1.414, Letter=1.294) |

---

## Stage 5: Shadow Removal

### Tujuan
Hilangkan bayangan (dari lampu, jari, atau sudut ruangan).

### Algoritma: Adaptive Illumination Correction

Shadow adalah low-frequency variation. Teks adalah high-frequency. Pisahkan pake low-pass filter.

```rust
fn remove_shadow(img: &GrayImage) -> GrayImage {
    let (w, h) = (img.width(), img.height());

    // 1. Large Gaussian blur untuk estimasi iluminasi background
    //    Kernel besar (≥sx/50) → cuma dapet variasi iluminasi, bukan teks
    let blur_radius = (w.min(h) as f64 / 50.0).max(15.0);
    let background = imageproc::filter::gaussian_blur_f32(img, blur_radius);

    // 2. Subtract background dari original
    //    pixel = max(0, original - background + mean(background))
    let bg_mean = mean_pixel(&background);
    let mut corrected = GrayImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let orig = img.get_pixel(x, y)[0] as f32;
            let bg = background.get_pixel(x, y)[0] as f32;
            let corrected_val = (orig - bg + bg_mean) as u8;
            corrected.put_pixel(x, y, Luma([corrected_val]));
        }
    }

    // 3. CLAHE (Contrast Limited Adaptive Histogram Equalization)
    //    untuk normalisasi kontras lokal
    apply_clahe(&corrected, 8, 4)  // 8x8 tiles, clip limit 4
}
```

### Alternatif: Retinex Theory

```rust
/// Retinex-based illumination correction
/// I(x,y) = R(x,y) × L(x,y)
/// I = observed image, R = reflectance (teks), L = illumination (shadow)
fn retinex_shadow_removal(img: &GrayImage) -> GrayImage {
    // Single-scale Retinex
    // log(R) = log(I) - log(G * I)
    // dimana G = Gaussian kernel

    let float_img = convert_to_float(img);
    let blurred = gaussian_blur_float(&float_img, 30.0);
    let retinex = element_wise(|p| (p.0.ln() - p.1.ln()), &float_img, &blurred);

    // Normalize ke [0, 255]
    normalize_to_u8(&retinex)
}
```

---

## Stage 6: Binarization

### Tujuan
Ubah ke hitam-putih bersih — teks hitam, background putih.

### Algoritma: Sauvola Local Threshold

Global threshold (Otsu) gagal kalau iluminasi ga merata. Sauvola adaptif per region.

```rust
fn sauvola_threshold(img: &GrayImage, window_size: u32, k: f32) -> GrayImage {
    // Sauvola: T(x,y) = m(x,y) * [1 + k * (s(x,y)/R - 1)]
    // m = local mean, s = local std dev, R = max std dev (128), k = parameter (~0.2)

    let (w, h) = (img.width(), img.height());
    let half_win = (window_size / 2) as i32;
    let mut output = GrayImage::new(w, h);

    // Integral image for O(1) mean and variance computation
    let integral = compute_integral_image(img);
    let integral_sq = compute_integral_image_sq(img);

    for y in 0..h {
        for x in 0..w {
            let (mean, variance) = local_stats(&integral, &integral_sq,
                                                x as i32, y as i32,
                                                half_win, w as i32, h as i32);
            let std_dev = variance.sqrt();
            let threshold = mean * (1.0 + k * (std_dev / 128.0 - 1.0));

            let pixel = img.get_pixel(x, y)[0] as f32;
            output.put_pixel(x, y, Luma([if pixel > threshold { 255 } else { 0 }]));
        }
    }

    output
}
```

### Parameter Default

| Parameter | Value | Notes |
|-----------|-------|-------|
| Window size | max(w,h)/30 | Minimum 15, maksimum 100 |
| k | 0.2 | Lower → lebih sensitif, higher → lebih toleran |

### Edge Cases

| Masalah | Solusi |
|---------|--------|
| Dokumen berwarna (bukan putih) | Deteksi warna dominan background, invert logic |
| Background gradasi | Sauvola handle ini lebih baik dari Otsu |
| Foto terlalu gelap | CLAHE dulu sebelum binarization |
| Text tipis/kabur | Morphological erode tipis sesudah binarization |

---

## Stage 7: Deskew

### Tujuan
Koreksi rotasi sisa (kalau dokumen masih miring sedikit — biasanya <5°).

### Algoritma: Hough Transform

```rust
fn deskew(img: &GrayImage) -> GrayImage {
    // 1. Cari garis teks via Hough transform
    //    Probabilistic Hough lebih cepat
    let lines = probabilistic_hough_lines(img, 10, PI / 180.0, 50, 50.0, 10.0);

    if lines.is_empty() {
        return img.clone();
    }

    // 2. Hitung sudut rata-rata semua garis
    let angles: Vec<f64> = lines.iter()
        .map(|line| line.angle().to_degrees())
        .filter(|a| a.abs() < 45.0)  // skip garis vertikal
        .collect();

    if angles.is_empty() {
        return img.clone();
    }

    let median_angle = median(&angles);

    // Skip kalau sudutnya <0.5 derajat (ga perlu koreksi)
    if median_angle.abs() < 0.5 {
        return img.clone();
    }

    // 3. Rotate image
    rotate(img, median_angle, imageops::FilterType::Lanczos3)
}
```

---

## Stage 8: OCR

### Tujuan
Extract teks dari gambar biar PDF-nya searchable dan teks bisa di-copy.

### Implementation

```rust
use leptess::LepTess;

fn ocr(img: &GrayImage, lang: &str) -> Result<String, OcrError> {
    // 1. Init Tesseract
    let mut tess = LepTess::new(Some("/usr/share/tesseract/tessdata"), lang)?;

    // 2. Set image
    tess.set_image_from_mem(&img.to_bytes())?;
    // 3. Set PSM (Page Segmentation Mode)
    //    PSM 3 = Fully automatic, default
    //    PSM 6 = Assume single uniform block of text
    //    PSM 4 = Assume single column of text
    tess.set_source_resolution(300);
    
    // 4. Recognize
    let text = tess.get_utf8_text()?;

    Ok(text)
}

/// Dapatkan word-level bounding boxes untuk positioning di PDF
fn ocr_words(img: &GrayImage, lang: &str) -> Result<Vec<Word>, OcrError> {
    let mut tess = LepTess::new(Some("/usr/share/tesseract/tessdata"), lang)?;
    tess.set_image_from_mem(&img.to_bytes())?;

    let words = tess.get_words()
        .iter()
        .map(|w| Word {
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

    Ok(words)
}
```

### Output Format

```rust
struct Word {
    text: String,
    bbox: Bbox,
    confidence: i32,  // 0-100
}
```

---

## Stage 9: PDF Generation

### Tujuan
Generate PDF yang:
1. Berisi gambar hasil scan (JPEG compressed)
2. Hidden text layer dari OCR (biar searchable, selectable)

### Implementation

```rust
use lopdf::{Document, Object, Stream};
use std::io::Write;

fn generate_searchable_pdf(
    image_data: &[u8],      // JPEG-compressed scan image
    ocr_text: &str,         // Full OCR text
    words: &[Word],         // Word positions
    page_width: f64,        // PDF page width in points
    page_height: f64,       // PDF page height in points
) -> Result<Vec<u8>, PdfError> {
    let mut doc = Document::new();

    // 1. Create image XObject
    let image_stream = Stream::new(
        dictionary! {
            "Type" => "XObject",
            "Subtype" => "Image",
            "Width" => page_width as u32,
            "Height" => page_height as u32,
            "ColorSpace" => "DeviceGray",
            "BitsPerComponent" => 8,
            "Filter" => "DCTDecode", // JPEG compression
        },
        image_data,
    );
    let image_id = doc.add_object(image_stream);

    // 2. Create content stream: place image, then invisible text
    //    Text layer is invisible (rendering mode 3 = neither fill nor stroke)
    let mut content = Vec::new();
    writeln!(content, "q")?;                              // save state
    writeln!(content, "{} 0 0 {} 0 0 cm", page_width, page_height)?; // scale to page
    writeln!(content, "/Im0 Do")?;                        // place image
    writeln!(content, "Q")?;                              // restore state

    // 3. Add invisible text layer (searchable)
    for word in words {
        let x = word.bbox.x as f64 / DPI * 72.0;         // convert pixels → points
        let y = (page_height - word.bbox.y as f64 / DPI * 72.0);
        writeln!(content, "BT")?;
        writeln!(content, "3 Tr")?;                       // rendering mode: invisible
        writeln!(content, "1 Tw")?;                       // word spacing
        writeln!(content, "{} {} Td", x, y)?;             // position
        writeln!(content, "({}) Tj", escape_pdf_string(&word.text))?;
        writeln!(content, "ET")?;
    }

    let content_stream = Stream::new(
        dictionary! {},
        content,
    );
    let content_id = doc.add_object(content_stream);

    // 4. Create page
    let page_id = doc.new_object_id();
    let pages_id = doc.new_object_id();

    doc.objects.insert(page_id, Object::Dictionary(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.0, 0.0, page_width, page_height],
        "Contents" => content_id,
        "Resources" => dictionary! {
            "XObject" => dictionary! {
                "Im0" => image_id,
            },
        },
    }));

    // 5. Close and return bytes
    let bytes = doc.save_to_bytes()?;
    Ok(bytes)
}
```

### PDF Coordinate System

```
PDF origin = bottom-left
Image origin = top-left

Perlu flip Y coordinate untuk text layer:
y_pdf = page_height - (y_image / dpi * 72)
```

---

## Complete Pipeline Assembly

```rust
pub struct ScanPipeline {
    config: PipelineConfig,
    metrics: MetricsRecorder,
}

impl ScanPipeline {
    pub async fn process(&self, input_path: &Path, options: ScanOptions)
        -> Result<ScanResult, PipelineError>
    {
        let timer = self.metrics.start_timer("scan.full");

        // 1. Load
        let img = image::open(input_path)
            .map_err(PipelineError::ImageLoad)?;
        self.metrics.stage_duration("load", timer.split());

        // 2. Preprocess
        let gray = preprocess(&img);
        self.metrics.stage_duration("preprocess", timer.split());

        // 3. Edge detection + corners (fallback chain)
        let corners = detect_corners_with_fallback(&gray)
            .map_err(PipelineError::CornerDetection)?;
        self.metrics.stage_duration("corner_detection", timer.split());

        // 4. Perspective warp
        let warped = perspective_warp(&img, corners);  // warp from COLOR original, not gray
        self.metrics.stage_duration("warp", timer.split());

        let warped_gray = warped.grayscale().into_luma8();

        // 5. Shadow removal
        let clean = remove_shadow(&warped_gray);
        self.metrics.stage_duration("shadow_removal", timer.split());

        // 6. Binarization
        let binary = sauvola_threshold(&clean, 50, 0.2);
        self.metrics.stage_duration("binarization", timer.split());

        // 7. Deskew
        let final_image = deskew(&binary);
        self.metrics.stage_duration("deskew", timer.split());

        // 8. Enhance final (sharpening)
        let final_image = sharpen(&final_image, 1.0);
        self.metrics.stage_duration("sharpen", timer.split());

        // 9. OCR
        let ocr_text = if options.ocr {
            Some(ocr(&final_image, "eng")?)
        } else {
            None
        };
        self.metrics.stage_duration("ocr", timer.split());

        // 10. Generate PDF
        let pdf_bytes = generate_searchable_pdf(
            &compress_jpeg(&final_image, 90)?,
            &ocr_text.unwrap_or_default(),
            &[],  // word positions (simplified)
            A4_WIDTH_PT,
            A4_HEIGHT_PT,
        )?;
        self.metrics.stage_duration("pdf_generation", timer.split());

        // 11. Save
        let output_path = PathBuf::from("/tmp/tools").join(format!("{}.pdf", uuid::Uuid::new_v4()));
        std::fs::write(&output_path, &pdf_bytes)?;

        timer.finish();
        
        Ok(ScanResult {
            output_path,
            page_count: 1,
            file_size: pdf_bytes.len() as u64,
            ocr_text,
        })
    }
}
```

## Performance Budget

| Stage | Target | Notes |
|-------|--------|-------|
| Load + Preprocess | <200ms | File I/O + resize |
| Edge + Corner Detection | <500ms | Canny + contour |
| Perspective Warp | <800ms | Per-pixel backward mapping |
| Shadow Removal | <300ms | FFT convolution atau integral image |
| Binarization | <200ms | Integral image |
| Deskew | <300ms | Hough transform |
| OCR | <1.5s | Tesseract, 300dpi |
| PDF Generation | <200ms | lopdf |
| **Total** | **<4s** | Per page |

> **Catatan**: Target di atas untuk image 12MP (4000×3000). Parallel via Rayon untuk batch processing.

## Edge Cases Matrix

| Skenario | Pipeline Behavior |
|----------|------------------|
| Kertas putih di meja putih | Edge detection gagal → fallback ke manual crop |
| Foto dari sudut 45° | Warp koreksi perspektif, output presisi |
| Dokumen terlipat | Edge detection dapet bentuk aneh → fallback manual |
| Bayangan jari | Shadow removal hilangkan |
| Teks pudar/pensil | Sauvola threshold adaptif, contrast enhance dulu |
| Tanda tangan & stempel | OCR bisa gagal di handwriting, tetap di-image |
| Multi-page (buku/kontrak) | Batch upload, masing-masing diproses, digabung 1 PDF |
| Foto malam | CLAHE + strong denoise sebelum edge detection |
| Latar belakang gradasi | Sauvola handle lebih baik dari Otsu |
