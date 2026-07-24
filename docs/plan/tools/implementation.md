# Implementation Plan

## Phase Breakdown

```
Phase 1 — Foundation + Document Scanner MVP
├── Milestone 1.1: Rust backend skeleton (Gateway + Worker + NATS + Redis)
├── Milestone 1.2: Scanner pipeline core (edge → warp → enhance → B&W)
├── Milestone 1.3: Next.js frontend + upload + download
└── Milestone 1.4: OCR + searchable PDF + WebSocket progress

Phase 2 — Scanner Complete + Image Tools
├── Milestone 2.1: Scanner fallback manual crop + PWA camera
├── Milestone 2.2: Image compress, resize, convert (WASM client-side)
├── Milestone 2.3: Background removal (ONNX)
└── Milestone 2.4: Batch processing

Phase 3 — PDF Tools + Video/Audio
├── Milestone 3.1: PDF merge, split, images-to-pdf
├── Milestone 3.2: Video compress, extract audio
└── Milestone 3.3: Final polish + load testing
```

---

## Phase 1: Foundation + Document Scanner MVP

### Milestone 1.1 — Rust Backend Skeleton

**Goal**: Gateway + Worker bisa connected ke NATS + Redis, upload flow work.

**Tasks**:

| # | Task | Files | Detail |
|---|------|-------|--------|
| 1.1.1 | Init Rust workspace | `apps/tools/backend/Cargo.toml` | Workspace dengan 3 crate: `gateway`, `workers`, `common` |
| 1.1.2 | Common types | `common/src/types.rs` | `JobStatus`, `Job`, `Tool`, `ScanOptions`, `UploadResponse` |
| 1.1.3 | Common error | `common/src/error.rs` | `PipelineError`, `UploadError`, `NatsError` |
| 1.1.4 | NATS subjects | `common/src/nats.rs` | Constants untuk semua subject/stream |
| 1.1.5 | Gateway: config | `gateway/src/config.rs` | Env-based config (Redis URL, NATS URL, storage path) |
| 1.1.6 | Gateway: upload route | `gateway/src/routes/upload.rs` | Multipart upload, validasi, save ke temp, publish NATS |
| 1.1.7 | Gateway: job status | `gateway/src/routes/job.rs` | GET job status dari Redis |
| 1.1.8 | Gateway: download | `gateway/src/routes/download.rs` | Stream file dari storage |
| 1.1.9 | Gateway: NATS publish | `gateway/src/nats/publisher.rs` | Publish job + progress |
| 1.1.10 | Gateway: Redis job | `gateway/src/redis/job.rs` | CRUD job status di Redis |
| 1.1.11 | Gateway: main | `gateway/src/main.rs` | Axum app bootstrap + routes |
| 1.1.12 | Workers: main loop | `workers/src/main.rs` | NATS consumer, dispatch ke tool handler |
| 1.1.13 | Workers: NATS consumer | `workers/src/nats/consumer.rs` | Subscribe jobs queue, ack/nack |
| 1.1.14 | Worker: scanner stub | `workers/src/scanner/mod.rs` | Cuma menerima job, update progress, complete |

**Acceptance**: `curl -X POST -F "file=@test.jpg" -F "tool=scan" http://localhost:3001/api/upload` → return job_id, setelah beberapa detik `GET /api/job/:id` return completed.

**Effort**: ~3-4 hari

---

### Milestone 1.2 — Scanner Pipeline Core

**Goal**: Image processing pipeline bisa ngubah foto miring jadi lurus + bersih + hitam-putih. Belum termasuk OCR dan PDF.

**Tasks**:

| # | Task | Detail | Referensi |
|---|------|--------|-----------|
| 1.2.1 | Edge detection | Canny + morphological close + adaptive fallback | `pipeline.md` Stage 2-3 |
| 1.2.2 | Corner detection | Contour detection, largest rectangle filter, polygon approximation | `pipeline.md` Stage 3 |
| 1.2.3 | Perspective warp | DLT homography + backward mapping + bilinear interpolation | `pipeline.md` Stage 4 |
| 1.2.4 | Shadow removal | Adaptive illumination correction + Retinex | `pipeline.md` Stage 5 |
| 1.2.5 | Binarization | Sauvola local threshold + integral image optimization | `pipeline.md` Stage 6 |
| 1.2.6 | Deskew | Hough transform line detection + rotation | `pipeline.md` Stage 7 |
| 1.2.7 | Image enhancement | CLAHE + sharpen + contrast | `pipeline.md` Stage 5 |
| 1.2.8 | Pipeline assembly | All stages connected, progress callback per stage | `pipeline.md` Complete assembly |

**Critical Algorithm**: Perspective warp via SVD untuk homography matrix. Butuh implementasi DLT algorithm atau pin `nalgebra` crate.

```rust
// Pseudo untuk testing before optimization
// imageproc contour → corner detection → warp
let edges = robust_edge_detection(&gray);
let corners = find_document_corners(&edges)?;
let warped = perspective_warp(&original, corners);
let cleaned = remove_shadow(&warped);
let binary = binarize(&cleaned);
let final_img = deskew(&binary);
```

**Acceptance**: Image foto miring test → keluar hasil lurus bersih hitam-putih.

**Effort**: ~5-7 hari (ini bagian paling susah)

---

### Milestone 1.3 — Next.js Frontend

**Goal**: User bisa upload foto, liat progress, download hasil.

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 1.3.1 | Init Next.js app | `bun create next-app` dengan Tailwind v4 + shadcn/ui |
| 1.3.2 | Landing page | Cards: Scan, Image, PDF — link ke masing-masing tool |
| 1.3.3 | Upload zone component | Drag & drop + file picker, validasi tipe/ukuran |
| 1.3.4 | Scanner page | `/scan` — upload area, tool options, progress bar |
| 1.3.5 | API upload route | Next.js API route → proxy ke Rust Gateway |
| 1.3.6 | Progress bar component | Animated bar + stage label dari WebSocket |
| 1.3.7 | Preview component | Before/after comparison slider |
| 1.3.8 | Result page | Preview + download button + file info |
| 1.3.9 | Error handling | Upload error, processing error, timeout |

**Acceptance**: User upload foto → liat progress bar → download PDF.

**Effort**: ~3-4 hari

---

### Milestone 1.4 — OCR + Searchable PDF

**Goal**: Output berupa PDF dengan hidden text layer — teks bisa di-copy, file bisa di-search.

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 1.4.1 | Install Tesseract data | Tambah `tessdata` di Docker image |
| 1.4.2 | OCR integration | `leptess` binding, set language, get text + word boxes |
| 1.4.3 | PDF generation | `lopdf` — page with image + invisible text layer |
| 1.4.4 | WebSocket progress | NATS consumer di Gateway → broadcast ke WS client |
| 1.4.5 | Auto-cleanup scheduler | NATS cron tiap 10 menit, hapus file expired >1 jam |
| 1.4.6 | Rate limiting | Redis sliding window per IP, per tool |

**Acceptance**: Download PDF → buka di browser → teks bisa di-select + di-search.

**Effort**: ~3-4 hari

---

### Phase 1 Total: ~14-19 hari kerja

---

## Phase 2: Scanner Complete + Image Tools

### Milestone 2.1 — Scanner Fallback + PWA

**Goal**: Scanner robust — kalau auto gagal, user bisa atur manual. Kamera langsung dari browser.

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 2.1.1 | Manual crop UI | Canvas: 4 draggable corners, background image |
| 2.1.2 | Fallback pipeline | Auto → gagal → manual → kirim corners ke worker |
| 2.1.3 | Camera capture | PWA: akses kamera via `getUserMedia`, capture frame |
| 2.1.4 | Auto-exposure helper | Tap to focus + exposure lock |
| 2.1.5 | Batch multi-page | Upload multiple photos → 1 PDF result |

**Effort**: ~4-5 hari

---

### Milestone 2.2 — Image Tools

**Goal**: Compress, resize, convert image langsung di browser (WASM).

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 2.2.1 | WASM image crate | Compile `image` crate to WASM via `wasm-pack` |
| 2.2.2 | Compress UI | Slider kualitas %, preview perbandingan ukuran |
| 2.2.3 | Resize UI | Input dimensi, lock aspect ratio, preview |
| 2.2.4 | Convert UI | Pilih format output, preview |
| 2.2.5 | Client-side processing | Semua image tool jalan di browser — no upload needed |
| 2.2.6 | Fallback server-side | Kalau WASM gagal/browser tua → upload ke server worker |

**WASM Strategy**:
```rust
// apps/tools/backend/wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;

#[wasm_bindgen]
pub fn compress_jpeg(bytes: &[u8], quality: u8) -> Vec<u8> {
    let img = image::load_from_memory(bytes).unwrap();
    let mut output = Cursor::new(Vec::new());
    img.write_to(&mut output, ImageFormat::Jpeg).unwrap();
    // quality compression via mozjpeg or custom
    output.into_inner()
}
```

**Effort**: ~4-5 hari

---

### Milestone 2.3 — Background Removal

**Goal**: Hapus background foto pake AI model ONNX — jalan di Rust native.

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 2.3.1 | Download RMBG model | `rmbg-1.4.onnx` (atau model lebih kecil seperti `u2net`) |
| 2.3.2 | ONNX Runtime binding | `ort` crate — load model, run inference |
| 2.3.3 | Pre/post processing | Resize ke 1024x1024 → normalize → softmax → threshold |
| 2.3.4 | Mask application | Alpha channel: background transparent / warna solid |
| 2.3.5 | Image preview | Before/after dengan background removal |

**Model Options**:
| Model | Size | Quality | Notes |
|-------|------|---------|-------|
| RMBG-1.4 | ~50MB | Excellent | BRIA, butuh license untuk commercial |
| U-2-Net | ~170MB | Good | Open source, lebih besar |
| MODNet | ~25MB | Good | Ringan, cepat |
| Dis_seg | ~8MB | Decent | Paling kecil, cocok untuk VPS |

**Effort**: ~3-4 hari

---

### Milestone 2.4 — Batch Processing

**Goal**: Upload 10-20 foto sekaligus, diproses parallel, jadi 1 PDF.

**Tasks**:

| # | Task | Detail |
|---|------|--------|
| 2.4.1 | Batch upload UI | Drop zone accept multiple files, thumbnail list |
| 2.4.2 | Group job | 1 group job = N individual jobs, track per-item progress |
| 2.4.3 | Rayon parallel | Worker process multiple pages in parallel |
| 2.4.4 | PDF merger | `lopdf` merge multiple pages → 1 document |

**Effort**: ~3-4 hari

---

## Phase 3: PDF + Video/Audio Tools

### Milestone 3.1 — PDF Tools

| # | Task | Detail |
|---|------|--------|
| 3.1.1 | Merge PDF | Upload 2+ PDF, `lopdf` merge pages |
| 3.1.2 | Split PDF | Input pages "1-3,5,7-9", ekstrak + save jadi 1 file |
| 3.1.3 | Images to PDF | Upload images, sort order, jadi 1 PDF |
| 3.1.4 | PDF compress | Re-encode embedded images dengan kualitas lebih rendah |
| 3.1.5 | PDF to images | Tiap halaman → JPEG/PNG |

**Effort**: ~4-5 hari

---

### Milestone 3.2 — Video/Audio Tools

| # | Task | Detail |
|---|------|--------|
| 3.2.1 | Video compress | FFmpeg binding (`ffmpeg-next`), turunin bitrate + resolusi |
| 3.2.2 | Extract audio | MP4 → MP3 via FFmpeg |
| 3.2.3 | Trim video | Start/end time → cut segment |
| 3.2.4 | GIF maker | Video segment → GIF, atur FPS + dimensi |
| 3.2.5 | Audio convert | Format conversion via FFmpeg |

**Catatan**: Video processing heavy — butuh dedicated worker dengan resource lebih besar. Queue priority: video jobs ke stream terpisah dengan max 1 concurrent.

**Effort**: ~5-7 hari

---

### Milestone 3.3 — Final Polish

| # | Task | Detail |
|---|------|--------|
| 3.3.1 | Theme integration | Twilight Terminal theme dari hub |
| 3.3.2 | Responsive design | Mobile-first, touch-friendly crop |
| 3.3.3 | Error monitoring | Error tracking, alert kalau pipeline gagal |
| 3.3.4 | Load testing | k6: simulasi concurrent users, measure P50/P95/P99 latency |
| 3.3.5 | Dashboard integration | Link dari hub dashboard → tools stats |

**Effort**: ~3-4 hari

---

## Timeline Summary

```
Minggu 1:  Gateway + upload/download + edge detection + warp
Minggu 2:  Enhance + binarization + deskew + Next.js frontend
Minggu 3:  OCR + PDF + WebSocket + rate limit + cleanup
           ─── MVP LAUNCH (Document Scanner ready) ───
Minggu 4:  Manual crop fallback + PWA camera + batch
Minggu 5:  WASM image tools + compress/resize/convert
Minggu 6:  Background removal (ONNX) + PDF tools
           ─── V1 LAUNCH (Scanner + Image + PDF) ───
Minggu 7:  Video/audio tools + final polish
Minggu 8:  Load testing + bug fixes + deployment
```

## Critical Path

```
Edge Detection ──▶ Corner Detection ──▶ Perspective Warp
       │                                       │
       │                              ┌────────┘
       │                              ▼
       │                       Shadow Removal ──▶ Binarization ──▶ Deskew
       │                                                            │
       │                                              ┌─────────────┘
       │                                              ▼
       │                                       OCR ──▶ PDF Gen ──▶ Output
       │
       └───(Kalau gagal)─── Manual Crop ◀── Frontend Canvas
```

**Risks**:
1. Edge detection paling rentan gagal — pipeline harus graceful fallback ke manual crop
2. Homography SVD implementasi perlu numerik stabil — test dengan extreme perspective angles
3. OCR kualitas sangat tergantung pada binarization — Sauvola parameter perlu tuning
4. WASM image processing size besar (~2MB gzipped) — perlu code splitting + lazy load
