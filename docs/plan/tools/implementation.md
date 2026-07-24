# Implementation Plan — Granular Task Breakdown

Setiap task adalah unit kerja terkecil yang bisa dikerjakan dalam 1-4 jam. Format:

```
[ID] Task description
      Files: path/to/file
      Accept: criteria yang harus terpenuhi
```

---

## Phase 1: Foundation + Document Scanner (Prioritas)

### Milestone 1.1 — Rust Backend Skeleton

**Goal**: Gateway + Worker connected ke NATS + Redis, upload flow end-to-end.

#### 1.1.1 — Init Rust Workspace

```
[1.1.1] Buat Cargo workspace dengan 4 crate: common, gateway, workers, wasm
        Files:
          apps/tools/backend/Cargo.toml           (workspace definition)
          apps/tools/backend/common/Cargo.toml    (serde, uuid, chrono)
          apps/tools/backend/gateway/Cargo.toml   (axum, tokio, tower, async-nats, deadpool-redis, redis)
          apps/tools/backend/workers/Cargo.toml   (tokio, async-nats, redis, image, imageproc, lopdf, leptess, ort, rayon)
          apps/tools/backend/wasm/Cargo.toml      (wasm-bindgen, image, console-error-panic)
          apps/tools/backend/rust-toolchain.toml  (channel = "1.85")
        Accept: cargo build —release works untuk semua crate (walaupun main.rs masih empty)
```

#### 1.1.2 — Common Types

```
[1.1.2] Define shared types: JobStatus enum, Job struct, Tool enum, ScanOptions, ImageOptions,
        PdfOptions, UploadResponse, JobProgress
        Files:
          apps/tools/backend/common/src/lib.rs
          apps/tools/backend/common/src/types.rs
        Types:
          JobStatus { Queued, Processing { stage: String, progress: u8 }, Completed, Failed(String) }
          Tool { Scan, ImageCompress, ImageResize, ImageConvert, RemoveBg,
                 PdfMerge, PdfSplit, ImagesToPdf, PdfCompress, PdfToImages,
                 VideoCompress, AudioExtract, VideoTrim, GifMaker, AudioConvert }
          Job { id: Uuid, tool: Tool, status: JobStatus, file_path: PathBuf,
                result_path: Option<PathBuf>, file_size: u64, options: Value,
                created_at: DateTime<Utc>, ttl_seconds: u64 }
          JobProgress { job_id: Uuid, status: JobStatus, stage: String, progress: u8, message: String }
        Accept: Semua type implements Serialize + Deserialize + Debug + Clone
```

#### 1.1.3 — Common Errors

```
[1.1.3] Define error types dengan thiserror
        Files:
          apps/tools/backend/common/src/error.rs
        Errors:
          UploadError (InvalidMime, FileTooLarge, Io, VirusDetected)
          PipelineError (ImageLoad, EdgeDetection, Warp, Ocr, PdfGeneration, Timeout, Internal)
          NatsError (Publish, Subscribe, JetStream, Timeout)
          RedisError (Connection, Query, Serialization)
        Accept: Setiap error punya Display + Source chain yang jelas
```

#### 1.1.4 — NATS Subjects & Streams

```
[1.1.4] Define NATS subject constants + stream configuration
        Files:
          apps/tools/backend/common/src/nats.rs
        Subjects:
          tools.scan.jobs           → Queue for scan workers
          tools.scan.progress       → Fan-out progress events
          tools.image.jobs
          tools.image.progress
          tools.pdf.jobs
          tools.pdf.progress
          tools.video.jobs
          tools.video.progress
          tools.scheduler.cleanup   → Cron: cleanup expired files
        Streams:
          tools-jobs    (max_age: 24h, storage: file)
          tools-progress (max_age: 1h, storage: memory)
        Accept: Unit test verifikasi format subject string
```

#### 1.1.5 — Gateway Config

```
[1.1.5] Environment-based configuration loader
        Files:
          apps/tools/backend/gateway/src/config.rs
        Env vars:
          GATEWAY_PORT (default: 3001)
          NATS_URL (default: nats://localhost:4222)
          REDIS_URL (default: redis://localhost:6379)
          STORAGE_PATH (default: /data/tools)
          MAX_FILE_SIZE_MB (default: 50)
          JOB_TTL_SECONDS (default: 3600)
          RATE_LIMIT_PER_MINUTE (default: 30)
          RUST_LOG (default: info)
        Accept: AppConfig struct dengan semua field, load dari env + fallback default
```

#### 1.1.6 — Gateway: Upload Route (POST /api/upload)

```
[1.1.6] Multipart file upload handler
        Files:
          apps/tools/backend/gateway/src/routes/upload.rs
          apps/tools/backend/gateway/src/routes/mod.rs
        Logic:
          1. Extract multipart: file + tool + options
          2. Validate MIME type (image/* for scan/image tools, application/pdf for pdf tools,
             video/* for video tools, audio/* for audio tools)
          3. Check file size < MAX_FILE_SIZE_MB
          4. Scan magic bytes: verify actual content matches extension
          5. Save file ke {STORAGE_PATH}/upload/{uuid}.{ext}
          6. Generate job_id (Uuid v4)
          7. Save job metadata ke Redis: SET job:{job_id} → JSON
          8. Publish ke NATS: tools.{tool}.jobs → {job_id, file_path, options}
          9. Return 202: { job_id, status: "queued", tool, ws_url }
        Accept: curl upload → 202 + job_id. curl job_status → status field populated
```

#### 1.1.7 — Gateway: Job Status Route (GET /api/job/{id})

```
[1.1.7] Job status and metadata retrieval
        Files:
          apps/tools/backend/gateway/src/routes/job.rs
        Logic:
          1. Extract job_id from path
          2. GET job:{job_id} from Redis
          3. Return 404 kalau not found
          4. Return JSON: { job_id, status, tool, progress, stage, message,
                           result: Option<{ download_url, file_size, file_name, preview_url }>,
                           created_at, error: Option<String> }
        Endpoints:
          GET /api/job/{id}           → Single job status
          GET /api/job/{id}/preview   → Preview image (thumbnail)
        Accept: curl GET → full job status JSON
```

#### 1.1.8 — Gateway: Download Route (GET /api/download/{id})

```
[1.1.8] File download with streaming + auto-cleanup awareness
        Files:
          apps/tools/backend/gateway/src/routes/download.rs
        Logic:
          1. Extract job_id, get status from Redis
          2. Check status == completed → get result_path
          3. Stream file via tokio::fs::File → axum body stream
          4. Set Content-Disposition header with original filename
          5. Set Content-Type based on file extension
          6. Handle 404 (not found), 400 (not completed yet), 410 (expired)
        Accept: curl GET /api/download/{id} → file download dengan correct headers
```

#### 1.1.9 — Gateway: Health & Metrics

```
[1.1.9] Health check + Prometheus metrics endpoint
        Files:
          apps/tools/backend/gateway/src/routes/health.rs
          apps/tools/backend/gateway/src/metrics.rs
        Endpoints:
          GET /health          → 200 OK (used by Traefik health check)
          GET /metrics         → Prometheus text format
        Metrics (via custom counters, no external crate):
          tools_uploaded_files_total    → counter, labels: tool, status
          tools_jobs_total              → counter, labels: tool, status
          tools_processing_duration_ms  → histogram, labels: tool
          tools_queue_depth             → gauge, labels: tool
        Accept: /health returns 200, /metrics returns prometheus-format text
```

#### 1.1.10 — Gateway: NATS Publisher

```
[1.1.10] NATS connection management + publish helpers
        Files:
          apps/tools/backend/gateway/src/nats/mod.rs
          apps/tools/backend/gateway/src/nats/publisher.rs
        Functions:
          connect_nats(url: &str) → Result<Connection>
          publish_job(nats: &Connection, tool: Tool, payload: &Job) → Result<()>
          publish_progress(nats: &Connection, progress: &JobProgress) → Result<()>
        Accept: Integration test: publish message → consume back via subscriber
```

#### 1.1.11 — Gateway: Redis Client

```
[1.1.11] Redis connection pool + job CRUD operations
        Files:
          apps/tools/backend/gateway/src/redis/mod.rs
          apps/tools/backend/gateway/src/redis/job.rs
          apps/tools/backend/gateway/src/redis/ratelimit.rs
        Functions:
          connect_redis(url: &str) → Result<ConnectionManager>
          job_create(redis, job) → Result<()>
          job_get(redis, job_id) → Result<Job>
          job_update(redis, job_id, status) → Result<()>
          job_delete(redis, job_id) → Result<()>
          rate_limit_check(redis, ip, tool) → Result<bool>  // sliding window
        TTL: Set TTL JOB_TTL_SECONDS on job_create
        Accept: Redis integration test: create → get → update → delete → not found
```

#### 1.1.12 — Gateway: Main Bootstrap

```
[1.1.12] Axum app assembly + graceful shutdown
        Files:
          apps/tools/backend/gateway/src/main.rs
        Logic:
          1. Load config
          2. Init Redis connection pool
          3. Init NATS connection
          4. Build Axum router with all routes
          5. Spawn NATS progress consumer (subscribe tools.*.progress, caches in Redis)
          6. Start HTTP server on configured port
          7. Graceful shutdown on SIGINT/SIGTERM
        States: SharedState { redis, nats, config } wrapped in Arc
        Accept: cargo run —bin gateway → server listening on :3001
```

#### 1.1.13 — Workers: NATS Consumer

```
[1.1.13] NATS JetStream consumer for job queues
        Files:
          apps/tools/backend/workers/src/main.rs
          apps/tools/backend/workers/src/nats/mod.rs
          apps/tools/backend/workers/src/nats/consumer.rs
        Logic:
          1. Connect to NATS + Redis
          2. Subscribe to JetStream streams: tools.*.jobs
          3. For each message:
             a. Deserialize job payload
             b. Match tool → dispatch to appropriate handler
             c. Acknowledge after handler returns
             d. NACK with delay on failure (for retry)
          4. Max delivery: 3, then dead-letter
        Max concurrency: configurable (TOOLS_WORKER_CONCURRENCY, default: 4)
        Accept: Workers start, consume NATS messages, dispatch to tool handlers
```

#### 1.1.14 — Workers: Progress Publisher

```
[1.1.14] Progress reporting from worker to NATS → Gateway → WebSocket
        Files:
          apps/tools/backend/workers/src/nats/progress.rs
        Functions:
          report_progress(redis, nats, job_id, status, stage, progress, message)
          1. Update Redis: job:{job_id} status + progress
          2. Publish to NATS: tools.{tool}.progress → {job_id, status, stage, progress, message}
        Helper: ProgressReporter struct yang implements Clone, bisa dipass ke pipeline
        Accept: Worker updates progress → Gateway receives → WebSocket forwards
```

#### 1.1.15 — Worker: Scanner Stub

```
[1.1.15] Scanner worker yang bisa menerima job dan update progress
        Files:
          apps/tools/backend/workers/src/scanner/mod.rs
        Logic:
          1. Receive job: { job_id, file_path, options }
          2. report_progress(queued → processing:10%)
          3. sleep 2s (simulasi pipeline)
          4. report_progress(processing:50%)
          5. sleep 2s
          6. Copy input file to output (simulasi hasil)
          7. report_progress(completed:100%)
          8. Update Redis with result_path
        Accept: Upload → NATS queue → worker consume → progress update → completed
```

---

### Milestone 1.2 — Scanner Pipeline Core

**Goal**: Foto miring → lurus + bersih + hitam-putih (belum OCR/PDF).

#### 1.2.1 — Preprocess: Load & Resize

```
[1.2.1] Load image from file, resize if too large, convert to grayscale
        Files:
          apps/tools/backend/workers/src/scanner/preprocess.rs
        Functions:
          load_image(path: &Path) → Result<DynamicImage>
          safe_resize(img: DynamicImage, max_dim: u32) → DynamicImage
          to_grayscale(img: &DynamicImage) → GrayImage
        Rules:
          - Resize if max(width, height) > 2000px → scale down, preserve aspect ratio
          - Lanczos3 filter untuk downscale (sharpest)
          - Support input: JPEG, PNG, WebP, HEIC (if feature enabled)
        Accept: Unit test: 12MP image → resize to ≤2000px, verify aspect ratio preserved
```

#### 1.2.2 — Edge Detection: Canny + Morphological Close

```
[1.2.2] Canny edge detection with morphological operations to connect broken edges
        Files:
          apps/tools/backend/workers/src/scanner/edge.rs
        Functions:
          detect_edges(img: &GrayImage) → GrayImage
          morphological_close(edges: &GrayImage, kernel_size: u8) → GrayImage
        Algorithm:
          1. Gaussian blur (sigma=1.0) on grayscale
          2. Canny with low_threshold=50, high_threshold=150
          3. Morphological close: dilate → erode with 5x5 kernel
          4. If edge_count < 1% of total pixels → retry Canny(20, 80)
        Accept: Unit test: known test images → edge image with continuous document borders
```

#### 1.2.3 — Corner Detection: Largest Rectangle Contour

```
[1.2.3] Find 4 corners of the document from edge image
        Files:
          apps/tools/backend/workers/src/scanner/corners.rs
        Functions:
          find_contours(edges: &GrayImage) → Vec<Contour>
          largest_rectangular_contour(contours: &[Contour]) -> Option<Contour>
          approx_polygon(contour: &Contour, num_vertices: u32) → Option<Vec<(f64,f64)>>
          order_corners(points: Vec<(f64,f64)>) -> [(f64,f64); 4]
          detect_corners(img: &GrayImage) -> Result<[(f64,f64); 4], FallbackReason>
        Algorithm:
          1. imageproc::contours::find_contours
          2. Filter by area > 20% of total image
          3. Top 5 largest by contour area
          4. For each: approx polygon, find 4-vertex polygon
          5. Order: top-left, top-right, bottom-right, bottom-left
        Accept: Unit test: 5 test images (normal, dark, angle, shadow, cluttered bg)
                → correct corners or explicit fallback
```

#### 1.2.4 — Perspective Warp: DLT Homography

```
[1.2.4] Compute homography matrix via DLT + SVD, apply perspective warp
        Files:
          apps/tools/backend/workers/src/scanner/warp.rs
        Functions:
          compute_homography(src: &[(f64,f64);4], dst: &[(f64,f64);4]) → [[f64;3];3]
          invert_homography(h: &[[f64;3];3]) → [[f64;3];3]
          apply_homography(h: &[[f64;3];3], x: f64, y: f64) → (f64, f64)
          bilinear_interpolate(img: &GrayImage, x: f64, y: f64) → Luma<u8>
          warp_perspective(img: &DynamicImage, corners: [(f64,f64);4]) → DynamicImage
        Algorithm:
          DLT (Direct Linear Transform):
            - 4 point correspondences → 8x9 matrix A
            - SVD (via ndarray + ndarray-linalg or nalgebra)
            - H = last column of V, reshape to 3x3
          Backward mapping:
            - For each output pixel (x,y), compute source (sx,sy) via H_inv
            - Bilinear interpolate from source
        Accept: Unit test: 4 corners of known grid → warped image is perfectly rectangular
```

#### 1.2.5 — Shadow Removal: Illumination Correction

```
[1.2.5] Remove uneven lighting, shadows, and glare
        Files:
          apps/tools/backend/workers/src/scanner/shadow.rs
        Functions:
          gaussian_blur_large(img: &GrayImage, radius: f64) → GrayImage
          subtract_background(img: &GrayImage, background: &GrayImage) -> GrayImage
          apply_clahe(img: &GrayImage, tile_size: u8, clip_limit: u8) -> GrayImage
          remove_shadow(img: &GrayImage) → GrayImage
        Algorithm (primary):
          1. Large Gaussian blur (radius = max_dim/50, min 15px) = illumination estimate
          2. Subtract: pixel = max(0, original - background + mean(background))
          3. CLAHE: 8x8 tiles, clip limit 3
        Algorithm (fallback - Retinex):
          1. log(I) = log(R) + log(L)
          2. log(R) = log(I) - log(Gaussian*I)
          3. exp(R), normalize to [0,255]
        Accept: Unit test: image with shadow gradient → uniform illumination
```

#### 1.2.6 — Binarization: Sauvola Local Threshold

```
[1.2.6] Convert grayscale to clean black-and-white using adaptive threshold
        Files:
          apps/tools/backend/workers/src/scanner/binarize.rs
        Functions:
          compute_integral_image(img: &GrayImage) → Vec<u64>
          compute_integral_image_sq(img: &GrayImage) -> Vec<u64>
          local_stats(integral: &[u64], integral_sq: &[u64],
                      x: i32, y: i32, half_win: i32, w: i32, h: i32) -> (f64, f64)
          sauvola_threshold(img: &GrayImage, window_size: u32, k: f64) -> GrayImage
          otsu_threshold(img: &GrayImage) -> GrayImage  // fallback
        Algorithm:
          Sauvola: T = m * (1 + k * (s/R - 1))
          - m = local mean (from integral image)
          - s = local std dev (from integral image squared)
          - k = 0.2 (tunable)
          - R = 128 (max std dev for 8-bit)
          - window_size = max(width, height) / 30, clamped to [15, 100]
        Accept: Unit test: 5 test images → binary output, text readable, background clean white
```

#### 1.2.7 — Deskew: Hough Transform Line Detection

```
[1.2.7] Detect and correct small rotation (<5°) of text lines
        Files:
          apps/tools/backend/workers/src/scanner/deskew.rs
        Functions:
          probabilistic_hough_lines(img: &GrayImage, threshold: u32,
                                     min_line_length: f64, max_gap: f64) -> Vec<Line>
          median_angle(lines: &[Line]) -> f64
          rotate_image(img: &GrayImage, angle_degrees: f64) -> GrayImage
          deskew(img: &GrayImage) -> GrayImage
        Algorithm:
          1. Probabilistic Hough line transform
          2. Filter: keep lines with angle between -45° and +45° (skip vertical)
          3. Compute median angle
          4. If |angle| > 0.5° → rotate with Lanczos3, crop to fit
        Accept: Unit test: rotated text image 3° → deskewed to <0.5° residual rotation
```

#### 1.2.8 — Image Enhancement: Sharpening + Contrast

```
[1.2.8] Apply final sharpening and contrast optimization
        Files:
          apps/tools/backend/workers/src/scanner/enhance.rs
        Functions:
          unsharp_mask(img: &GrayImage, sigma: f64, amount: f64) -> GrayImage
          adjust_contrast(img: &GrayImage, factor: f64) -> GrayImage
          remove_noise(img: &GrayImage, threshold: u8) -> GrayImage
          enhance_final(img: &GrayImage) -> GrayImage
        Algorithm (Unsharp mask):
          blurred = gaussian_blur(img, sigma=1.0)
          mask = img - blurred
          result = img + amount * mask      // amount = 1.0 (default)
        Accept: Unit test: blurry text → sharpened text, verify no ringing artifacts
```

#### 1.2.9 — Pipeline Assembly

```
[1.2.9] Connect all pipeline stages with progress reporting
        Files:
          apps/tools/backend/workers/src/scanner/pipeline.rs
          apps/tools/backend/workers/src/scanner/mod.rs (update)
        Functions:
          ScanPipeline::process(input_path, options, progress: ProgressReporter) → Result<ScanResult>
        Stages with progress:
          0%  → load + preprocess
          15% → edge detection
          25% → corner detection
          35% → perspective warp
          50% → shadow removal
          65% → binarization
          75% → deskew
          85% → final enhance
          100% → complete
        ScanResult:
          { output_image_path, page_count: 1, image_dimensions, processing_time_ms }
        Accept: Full pipeline test with 10 diverse test images → consistent quality output
```

---

### Milestone 1.3 — Next.js Frontend Foundation

**Goal**: User bisa upload foto, lihat progress, download hasil.

#### 1.3.1 — Init Next.js App

```
[1.3.1] Create Next.js 16 app with Tailwind v4 + shadcn/ui + TypeScript strict
        Files:
          apps/tools/frontend/package.json
          apps/tools/frontend/next.config.ts
          apps/tools/frontend/tsconfig.json
          apps/tools/frontend/postcss.config.mjs
          apps/tools/frontend/components.json
          apps/tools/frontend/biome.json
          apps/tools/frontend/src/app/globals.css
        Setup:
          bun create next-app@latest --typescript --tailwind --eslint
          bun add @shadcn/react lucide-react class-variance-authority clsx tailwind-merge framer-motion
          npx shadcn@latest init
          Add custom CSS variables + Twilight Terminal theme
        Accept: bun dev → localhost:3002, halaman kosong dengan Tailwind + shadcn working
```

#### 1.3.2 — Root Layout + Theme Provider

```
[1.3.2] Layout dengan header, footer, theme provider, fonts
        Files:
          apps/tools/frontend/src/app/layout.tsx
          apps/tools/frontend/src/app/providers.tsx
          apps/tools/frontend/src/components/tools/header.tsx
          apps/tools/frontend/src/components/tools/footer.tsx
          apps/tools/frontend/src/lib/utils.ts
        Features:
          - Root layout with metadata (title: "Tools — Asep Haryana")
          - ThemeProvider (next-themes) wrapping children
          - Geist sans font (same as hub portfolio)
          - Header: logo "Tools", navigation links, theme toggle, GitHub link
          - Footer: copyright, powered by Rust + Next.js badge
          - cn() utility from tailwind-merge
        Accept: All pages render with header + footer, theme toggle works
```

#### 1.3.3 — Landing Page (/) with Tool Cards

```
[1.3.3] Card grid showing all available tools with icons
        Files:
          apps/tools/frontend/src/app/page.tsx
          apps/tools/frontend/src/components/tools/tool-card.tsx
          apps/tools/frontend/src/components/tools/tool-grid.tsx
        Data:
          tools = [
            { id: "scan", title: "Document Scanner", desc: "...", icon: ScanIcon, href: "/scan", phase: 1 },
            { id: "image-compress", title: "Compress Image", desc: "...", icon: ... },
            ...
          ]
        Features:
          - Grid responsive: 1 col mobile, 2 col tablet, 3 col desktop
          - Each card: icon, title, description, link
          - Phase badges: "Available", "Coming Soon"
          - Framer Motion stagger animation on mount
        Accept: / renders grid of tool cards, each card is clickable link
```

#### 1.3.4 — Upload Zone Component

```
[1.3.4] Drag & drop upload zone with file validation
        Files:
          apps/tools/frontend/src/components/tools/upload-zone.tsx
          apps/tools/frontend/src/hooks/use-upload.ts
        Features:
          - Drag & drop area with dashed border
          - Click to open file picker
          - Accept attribute berdasarkan tool (image/*, application/pdf, video/*, audio/*)
          - Validate: file type, max size (50MB), max count (50 for batch)
          - Show file name, size, type after selection
          - Error state: invalid type, too large, too many
          - Drag over highlight animation
          - Loading spinner during upload
          - Upload progress percentage (from XHR or fetch)
          - Cancel upload button
        Accept: Drag image file → uploads to server → returns job_id
```

#### 1.3.5 — Progress Bar Component

```
[1.3.5] Animated progress bar with stage label from WebSocket
        Files:
          apps/tools/frontend/src/components/tools/progress-bar.tsx
          apps/tools/frontend/src/hooks/use-job-status.ts
        WebSocket hook (useJobStatus):
          - Connect to /api/job/{id}/ws
          - Auto-reconnect on disconnect (3 retries)
          - Parse JobProgress messages
          - Update state: status, progress, stage, message
          - Cleanup on unmount
        ProgressBar:
          - Animated bar (Framer Motion width animation)
          - Stage label: "Detecting edges...", "Correcting perspective...", etc.
          - Percentage number
          - Status badge: Processing (amber pulse), Completed (green), Failed (red)
          - Error state with retry button
        Accept: Upload → progress bar animates from 0-100% with stage labels
```

#### 1.3.6 — Preview Before/After Component

```
[1.3.6] Image preview with before/after comparison slider
        Files:
          apps/tools/frontend/src/components/tools/preview-before-after.tsx
        Features:
          - Two image layers: original (left) vs processed (right)
          - Draggable slider divider
          - Click on left = show original, click right = show processed
          - Zoom: scroll to zoom, drag to pan
          - File size comparison badge: "2.4 MB → 340 KB"
          - Responsive: fill container width
        Accept: Component renders with two image URLs, slider interaction works
```

#### 1.3.7 — Result Preview Component

```
[1.3.7] Result display: preview, download, info
        Files:
          apps/tools/frontend/src/components/tools/result-preview.tsx
        Features:
          - Show processed file preview (image or icon for PDF/video/audio)
          - File info: name, size, dimensions, pages (for PDF)
          - Download button with file type icon
          - Download as ZIP for batch results
          - Copy share link button (if applicable)
          - "Process another" button → reset to upload state
          - Auto-download option checkbox
        Accept: Job completes → result card shows with download button → click downloads file
```

#### 1.3.8 — Scanner Page (/scan)

```
[1.3.8] Full scanner page: upload → progress → result
        Files:
          apps/tools/frontend/src/app/scan/page.tsx
          apps/tools/frontend/src/app/scan/result/[id]/page.tsx
        Page states:
          1. UPLOAD: UploadZone + options (OCR toggle, enhance toggle, DPI selector)
          2. PROCESSING: ProgressBar + stage label + cancel button
          3. RESULT: PreviewBeforeAfter + ResultPreview + "Process Another"
          4. ERROR: Error message with retry + feedback button
        Options panel:
          - Enable OCR (toggle, default: on)
          - Auto-enhance (toggle, default: on)
          - Output format (PDF, JPEG, PNG — default: PDF)
          - DPI (150, 200, 300, 400 — default: 300)
        Flow:
          Upload → POST /api/upload → get job_id → connect WS → show progress →
          complete → show result with preview + download
        Accept: Full user flow: upload → progress → download PDF
```

#### 1.3.9 — API Route: Upload Proxy

```
[1.3.9] Next.js API route that proxies upload to Rust backend
        Files:
          apps/tools/frontend/src/app/api/upload/route.ts
        Logic:
          - Accept multipart/form-data from browser
          - Forward to http://tools:3001/api/upload (Rust gateway)
          - On 202: return { job_id, ws_url } to client
          - On 4xx/5xx: return error to client
          - Handle timeout, connection refused gracefully
        Accept: POST via browser → proxied to Rust → returns job_id
```

#### 1.3.10 — API Route: Job Status & Download Proxy

```
[1.3.10] Proxy job status + WebSocket + download to Rust backend
        Files:
          apps/tools/frontend/src/app/api/job/[id]/route.ts
          apps/tools/frontend/src/app/api/job/[id]/ws/route.ts
          apps/tools/frontend/src/app/api/download/[id]/route.ts
        Features:
          GET /api/job/{id} → proxy to Rust
          WS /api/job/{id}/ws → proxy WebSocket (Next.js can't do WS in app router,
                                so use upgrade header or direct client WS to Rust port)
          GET /api/download/{id} → stream from Rust
        Note: WebSocket langsung dari client ke Rust gateway port (3001),
              bukan via Next.js. CORS sudah dihandle di Rust.
        Accept: WS connection works: client → Rust gateway → NATS progress → browser
```

---

### Milestone 1.4 — OCR + Searchable PDF + Infrastructure

**Goal**: Output searchable PDF, realtime WebSocket progress, auto-cleanup, rate limiting.

#### 1.4.1 — Tesseract OCR Integration

```
[1.4.1] OCR text extraction from processed image
        Files:
          apps/tools/backend/workers/src/scanner/ocr.rs
        Functions:
          init_tesseract(lang: &str) -> Result<LepTess>
          ocr_text(tess: &mut LepTess, img: &GrayImage) -> Result<String>
          ocr_words(tess: &mut LepTess, img: &GrayImage) -> Result<Vec<OcrWord>>
          ocr_with_language(img: &GrayImage, lang: &str) -> Result<OcrResult>
        OcrWord: { text: String, bbox: {x,y,w,h}, confidence: i32 }
        OcrResult: { full_text: String, words: Vec<OcrWord>, confidence: f32 }
        Languages: "eng+ind" (English + Indonesian)
        TESSDATA_PREFIX: /usr/share/tesseract-ocr/5/tessdata
        PSM mode: 3 (automatic), fallback 6 (single text block)
        Accept: Unit test: known text image → OCR returns text with >80% confidence
```

#### 1.4.2 — Searchable PDF Generation

```
[1.4.2] Generate PDF with visible image + invisible text layer
        Files:
          apps/tools/backend/workers/src/scanner/pdf.rs
        Functions:
          compress_image_jpeg(img: &GrayImage, quality: u8) -> Result<Vec<u8>>
          generate_pdf_page(image_data: &[u8], words: &[OcrWord],
                            page_width_pt: f64, page_height_pt: f64) -> Result<Document>
          generate_searchable_pdf(image_data: &[u8], ocr_text: &str,
                                   words: &[OcrWord]) -> Result<Vec<u8>>
        PDF structure:
          - Page with MediaBox A4 (595.28 x 841.89) or fit to image aspect ratio
          - Image XObject (JPEG DCTDecode, 300 DPI equivalent)
          - Content stream:
            1. Place image at full page: q {w} 0 0 {h} 0 0 cm /Im0 Do Q
            2. Invisible text: 3 Tr (rendering mode 3 = neither fill nor stroke)
            3. Each word positioned at its bbox, converted pixels → points
          - Metadata: Producer, CreationDate
        Accept: Generated PDF → open in browser → text is selectable + searchable
```

#### 1.4.3 — Scanner Pipeline: OCR + PDF Integration

```
[1.4.3] Connect OCR + PDF generation into the main pipeline
        Files:
          apps/tools/backend/workers/src/scanner/pipeline.rs (update)
        Updated stages:
          75% → deskew
          82% → OCR
          90% → PDF generation
          100% → save + complete
        Pipeline now returns ScanResult:
          { output_path, page_count, file_size, ocr_text, processing_time_ms }
        Accept: Full pipeline test: input image → output searchable PDF file
```

#### 1.4.4 — WebSocket Progress Forwarding (Gateway)

```
[1.4.4] Gateway subscribes to NATS progress and forwards via WebSocket
        Files:
          apps/tools/backend/gateway/src/routes/ws.rs
        Functions:
          ws_handler(ws: WebSocketUpgrade, job_id: Path<String>, state: SharedState)
          handle_ws(mut ws: WebSocket, job_id: String, nats: Connection, redis: ConnectionManager)
        Logic:
          1. Accept WebSocket upgrade
          2. Subscribe to NATS: tools.*.progress.{job_id}
          3. Forward each message as JSON to WebSocket client
          4. On connection close: unsubscribe from NATS
          5. Keepalive ping every 30 seconds
          6. Send initial status from Redis on connect
        Accept: Client connects via WS → receives progress messages in realtime
```

#### 1.4.5 — Auto-Cleanup Scheduler

```
[1.4.5] Scheduled cleanup of expired files and Redis keys
        Files:
          apps/tools/backend/workers/src/scheduler/mod.rs
          apps/tools/backend/workers/src/scheduler/cleanup.rs
        Logic:
          1. Subscribe to NATS cron: tools.scheduler.cleanup (every 10 min)
          2. Scan {STORAGE_PATH} for files older than JOB_TTL_SECONDS
          3. Delete expired files
          4. SCAN Redis for job:* keys with TTL expired, delete orphans
          5. Log: deleted N files, freed M bytes, deleted K orphan keys
          6. Prometheus: tools_cleanup_deleted_files counter
        Periodic trigger: NATS cron via tools.scheduler.cleanup subject
        Accept: Upload file → wait TTL → file deleted automatically
```

#### 1.4.6 — Rate Limiting

```
[1.4.6] Rate limiting per IP per tool via Redis sliding window
        Files:
          apps/tools/backend/gateway/src/redis/ratelimit.rs (update)
          apps/tools/backend/gateway/src/routes/upload.rs (middleware)
        Algorithm (Sliding Window):
          Key: ratelimit:{ip}:{tool}:{minute_bucket}
          - ZADD current timestamp
          - ZREMRANGEBYSCORE older than 60s
          - ZCOUNT → if > RATE_LIMIT_PER_MINUTE → reject
        Response on reject: 429 Too Many Requests
          { error: "rate_limit_exceeded", retry_after_seconds: 60 }
        Middleware: Add to upload route as tower Layer
        Accept: curl 31x in 60s → 429 on 31st request
```

#### 1.4.7 — Error Handling & Validation (Gateway Middleware)

```
[1.4.7] Global error handling and input validation middleware
        Files:
          apps/tools/backend/gateway/src/middleware/mod.rs
          apps/tools/backend/gateway/src/middleware/error_handler.rs
          apps/tools/backend/gateway/src/middleware/request_id.rs
        Features:
          - Request ID middleware (X-Request-Id header, uuid v4)
          - JSON error response format: { error: string, code: string, request_id: string }
          - 400: invalid input (missing file, missing tool, invalid options)
          - 404: job not found / file expired
          - 413: file too large
          - 429: rate limited
          - 500: internal error (logged, not exposed to client)
          - Panic recovery layer
        Accept: curl with missing fields → 400 JSON error. curl bad tool → 400.
```

---

### Phase 1 Complete: Document Scanner MVP

**Acceptance criteria**:
1. User buka tools.asepharyana.my.id → landing page with tool cards
2. Click "Document Scanner" → halaman scan
3. Upload foto dokumen HP (miring, bayangan) → preview upload
4. Progress bar animasi: edge → warp → enhance → binarize → OCR → PDF
5. Download searchable PDF → teks bisa di-copy
6. Rate limit: 30 uploads/min/IP
7. File auto-delete after 1 hour

---

## Phase 2: Scanner Complete + Image Tools

### Milestone 2.1 — Scanner Robustness (Manual Crop + Camera + Batch)

#### 2.1.1 — Manual Crop Canvas

```
[2.1.1] Interactive canvas with 4 draggable corner handles
        Files:
          apps/tools/frontend/src/components/tools/crop-editor.tsx
          apps/tools/frontend/src/hooks/use-crop-editor.ts
        Features:
          - Render uploaded image on Canvas
          - 4 draggable corner handles (circular, Luma color)
          - Connect corners with dashed lines
          - Zoom: scroll to zoom, drag canvas to pan
          - Grid overlay (rule of thirds) for alignment
          - Touch support: pinch zoom, drag handles
          - Double-click to auto-detect corners again
          - Reset button
          - Confirm button → sends corner coordinates + image to server
        API: POST /api/scan/manual-crop
          { job_id, corners: [{x,y},{x,y},{x,y},{x,y}] }
        Accept: User adjust corners → confirm → server applies warp with given corners
```

#### 2.1.2 — Fallback Pipeline: Auto → Manual → Process

```
[2.1.2] When auto edge detection fails, fall back to manual crop
        Files:
          apps/tools/backend/gateway/src/routes/upload.rs (update)
          apps/tools/frontend/src/app/scan/page.tsx (update)
        Flow:
          1. Upload → queue job → worker coba auto-detect
          2. Auto gagal → worker set status = "needs_manual_crop"
             → return corner confidence < 0.6
          3. Gateway returns: { job_id, status: "needs_manual_crop",
                                manual_crop_url: "/scan/crop/{job_id}" }
          4. Frontend redirect to CropEditor
          5. User adjust corners → POST manual-crop → worker resume pipeline
        Accept: Upload bad photo → auto redirect to manual crop → complete pipeline
```

#### 2.1.3 — PWA Camera Capture

```
[2.1.3] Direct camera capture from browser (PWA)
        Files:
          apps/tools/frontend/src/components/tools/camera-capture.tsx
          apps/tools/frontend/src/hooks/use-camera.ts
        Features:
          - Access rear camera via getUserMedia
          - Live preview in viewfinder
          - Auto-focus on tap
          - Capture button → freeze frame
          - Aspect ratio guide overlay (A4: 1:1.414)
          - Reject blurry photos (Laplacian variance check via Canvas API)
          - Auto-capture when document detected (stabilize → snap)
          - Switch front/rear camera
          - Torch/flash toggle (if supported)
          - Zoom slider (if supported)
        Accept: Click "Use Camera" → camera opens → capture → photo uploaded
```

#### 2.1.4 — Batch Multi-Page

```
[2.1.4] Multiple pages → one PDF, parallel processing
        Files:
          apps/tools/frontend/src/components/tools/file-list.tsx
          apps/tools/backend/workers/src/scanner/pipeline.rs (update)
        Frontend:
          - Upload multiple files (drag multiple or multi-select)
          - Thumbnail list with drag-to-reorder
          - Remove individual pages
          - Add more pages button
          - Upload all → single group job → N individual jobs
          - Per-page progress status in thumbnail list
        Backend:
          - Group job: { group_id, job_ids: [...], total: N, completed: 0 }
          - Process each page via Rayon parallel for
          - Merge all PDF pages into single document via lopdf
          - Report: per-page progress + overall progress % (completed/total)
        Accept: Upload 5 photos → all processed in parallel → single 5-page PDF
```

#### 2.1.5 — Scan Options Panel

```
[2.1.5] Enhanced options panel for scanner
        Files:
          apps/tools/frontend/src/components/tools/scan-options.tsx
          apps/tools/frontend/src/app/scan/page.tsx (update)
        Options:
          - Output: PDF, JPEG, PNG
          - Quality: 1-100 (slider, default 90)
          - OCR: on/off (default: on)
          - Language: English, Indonesian, Both (default: Both)
          - Auto-enhance: on/off (default: on)
          - Color mode: Black & White, Grayscale, Color (default: B&W)
          - Page size: A4, Letter, Auto-fit (default: A4)
          - DPI: 150/200/300/400 (default: 300)
        Accept: Change options → upload → output sesuai pilihan
```

---

### Milestone 2.2 — WASM Image Tools (Compress, Resize, Convert)

#### 2.2.1 — WASM Rust Crate

```
[2.2.1] Compile image processing to WebAssembly via wasm-pack
        Files:
          apps/tools/backend/wasm/Cargo.toml
          apps/tools/backend/wasm/src/lib.rs
          apps/tools/backend/wasm/src/compress.rs
          apps/tools/backend/wasm/src/resize.rs
          apps/tools/backend/wasm/src/convert.rs
        Functions:
          #[wasm_bindgen]
          fn compress_jpeg(bytes: &[u8], quality: u8) -> Result<Vec<u8>>
          fn compress_png(bytes: &[u8], effort: u8) -> Result<Vec<u8>>
          fn compress_webp(bytes: &[u8], quality: u8) -> Result<Vec<u8>>
          fn resize_image(bytes: &[u8], width: u32, height: u32, fit: String) -> Result<Vec<u8>>
          fn convert_format(bytes: &[u8], target_format: String) -> Result<Vec<u8>>
          fn get_image_info(bytes: &[u8]) -> Result<JsValue>  // { width, height, format, size }
        Build: wasm-pack build --release --target web
        Output: pkg/ directory (wasm binary + JS glue)
        Accept: wasm-pack build succeeds. Node.js test: compress results smaller than input
```

#### 2.2.2 — WASM Integration in Frontend

```
[2.2.2] Load and invoke WASM module from Next.js
        Files:
          apps/tools/frontend/src/lib/wasm/loader.ts
          apps/tools/frontend/src/lib/wasm/image-processor.ts
        Functions:
          async initWasm() → WASM module instance (lazy load on first use)
          async compressInBrowser(file: File, quality: number) → Blob
          async resizeInBrowser(file: File, width: number, height: number) → Blob
          async convertInBrowser(file: File, format: string) → Blob
        Strategy:
          - Dynamic import: await import('./pkg/image_wasm.js')
          - Lazy init: only load when user visits image tool page
          - Code splitting: WASM chunk loaded separately (~1.5MB gzipped)
          - Cache: once loaded, keep in memory
          - Fallback: if WASM fails → upload to server worker
        Accept: Browser loads WASM → image processing runs client-side, no upload
```

#### 2.2.3 — Image Compress Page (/image/compress)

```
[2.2.3] Full compress page with quality slider and live comparison
        Files:
          apps/tools/frontend/src/app/image/compress/page.tsx
          apps/tools/frontend/src/components/tools/quality-slider.tsx
        Features:
          - UploadZone (accepts image/*)
          - Quality slider: 1-100, live preview update
          - File size display: original vs compressed (estimated)
          - PreviewBeforeAfter (original vs compressed)
          - Format selector: JPEG, PNG, WebP
          - Download button
          - Batch mode: compress all images in folder
        WASM flow: upload → load in WASM → compress → preview → download (no server)
        Server fallback: upload → Rust worker compress → download
        Accept: Upload photo → adjust quality → live preview → download compressed version
```

#### 2.2.4 — Image Resize Page (/image/resize)

```
[2.2.4] Resize page with dimension input and aspect ratio lock
        Files:
          apps/tools/frontend/src/app/image/resize/page.tsx
          apps/tools/frontend/src/components/tools/dimension-input.tsx
        Features:
          - UploadZone
          - Input: width + height, auto-fill from original
          - Aspect ratio lock toggle (🔗/🔓)
          - Preset sizes: 800x600, 1024x768, 1920x1080, Instagram (1080x1080), Custom
          - Fit modes: exact (stretch), contain (fit within, add bg), cover (crop to fill)
          - Preview: resized dimensions overlay
          - Download (or ZIP for batch)
          - Batch: resize all images to same dimensions
        Accept: Upload photo → set 800px width → lock aspect → download resized image
```

#### 2.2.5 — Image Convert Page (/image/convert)

```
[2.2.5] Format conversion page
        Files:
          apps/tools/frontend/src/app/image/convert/page.tsx
        Features:
          - UploadZone
          - From: auto-detected from file
          - To: JPEG, PNG, WebP, GIF, BMP, TIFF
          - Quality slider (for lossy formats)
          - PreviewBeforeAfter
          - Download
          - Batch: convert all files in folder
        Accept: Upload HEIC → convert to JPEG → download
```

#### 2.2.6 — Server Fallback for WASM

```
[2.2.6] Server-side processing when WASM unavailable
        Files:
          apps/tools/backend/workers/src/image/mod.rs
          apps/tools/backend/workers/src/image/compress.rs
          apps/tools/backend/workers/src/image/resize.rs
          apps/tools/backend/workers/src/image/convert.rs
        Logic:
          - Same pipeline as WASM but runs natively
          - Use image crate with mozjpeg feature for optimal JPEG
          - Resize with Lanczos3 filter
          - Convert via image crate format support
        Frontend detection:
          - try/catch WASM init → if fails, enable "Server Process" button
          - Upload → NATS → worker → result (same as scanner flow)
        Accept: Disable WASM in browser → upload fallback works → same quality output
```

---

### Milestone 2.3 — Background Removal (ONNX)

#### 2.3.1 — ONNX Model Download & Setup

```
[2.3.1] Download and package background removal model
        Files:
          Dockerfile (update — or runtime download script)
          models/download-models.sh
        Models:
          - Primary: MODNet (~25MB, good quality, fast)
          - Alternative: DIS_seg (~8MB, decent quality, very fast)
        Setup:
          - Download model file to models/
          - Verify SHA256 checksum
          - Load model at worker startup
          - One-time init, cache in memory
        ONNX session options:
          - InterOpNumThreads: 2
          - IntraOpNumThreads: 4
          - GraphOptimizationLevel: ORT_ENABLE_ALL
        Accept: Worker starts → loads ONNX model → ready for inference
```

#### 2.3.2 — Pre/Post Processing

```
[2.3.2] Image preprocessing and postprocessing for ONNX model
        Files:
          apps/tools/backend/workers/src/image/remove_bg.rs
        Functions:
          preprocess_for_model(img: &DynamicImage, target_size: u32) -> Result<Array4<f32>>
            - Resize to model input size (1024x1024 for MODNet)
            - Normalize: /255.0, mean=[0.5,0.5,0.5], std=[0.5,0.5,0.5]
            - Convert to CHW format (ort::Tensor)
          postprocess_mask(output: &ort::Tensor, original_size: (u32,u32)) -> GrayImage
            - Sigmoid → threshold 0.5 → resize to original dimensions
            - Apply optional smoothing (Gaussian blur sigma=1)
          apply_alpha(img: &DynamicImage, mask: &GrayImage, bg_color: Option<[u8;3]>) -> DynamicImage
            - If bg_color = None → RGBA with transparent background
            - If bg_color = Some([r,g,b]) → composite onto solid color
        Accept: Unit test: portrait photo → mask correctly separates foreground
```

#### 2.3.3 — Remove Background Page (/image/remove-bg)

```
[2.3.3] Background removal page
        Files:
          apps/tools/frontend/src/app/image/remove-bg/page.tsx
        Features:
          - UploadZone
          - Processing via server worker (too heavy for WASM)
          - PreviewBeforeAfter: original → transparent bg
          - Background selector: transparent, white, color picker
          - Download as PNG (transparent) or JPEG (with bg color)
          - Download HD (full resolution) or Web (compressed)
          - Batch processing
          - Progress bar during inference
        Accept: Upload photo → background removed → download with transparent PNG
```

---

### Phase 2 Complete: Document Scanner Robust + Image Tools

---

## Phase 3: PDF Tools

### Milestone 3.1 — PDF Worker

#### 3.1.1 — PDF Merge Worker

```
[3.1.1] Merge multiple PDF files into one
        Files:
          apps/tools/backend/workers/src/pdf/mod.rs
          apps/tools/backend/workers/src/pdf/merge.rs
          apps/tools/frontend/src/app/pdf/merge/page.tsx
        Logic:
          1. Upload 2+ PDF files
          2. Load each via lopdf::Document::load
          3. Iterate pages from each document, append to output
          4. Save merged document
          5. Preserve: page size (use largest MediaBox per page), fonts (embedded)
        Frontend:
          - UploadZone (multiple, accept .pdf)
          - Drag-to-reorder uploaded files
          - Remove individual files
          - Download merged PDF
        Accept: Upload 3 PDFs → download 1 PDF with all pages in order
```

#### 3.1.2 — PDF Split Worker

```
[3.1.2] Extract specific pages from a PDF
        Files:
          apps/tools/backend/workers/src/pdf/split.rs
          apps/tools/frontend/src/app/pdf/split/page.tsx
        Functions:
          parse_page_range(input: &str, total_pages: u32) -> Result<Vec<u32>>
          split_pdf(input_path: &Path, pages: &[u32]) -> Result<Vec<u8>>
        Page syntax: "1-3,5,7-9,12" → [1,2,3,5,7,8,9,12]
        Frontend:
          - UploadZone (single PDF)
          - PDF preview with page thumbnails
          - Click pages to select/deselect
          - Or type page range input
          - Download extracted pages as single PDF
        Accept: Upload 20-page PDF → extract pages 3-7 → download 5-page PDF
```

#### 3.1.3 — Images to PDF Worker

```
[3.1.3] Convert multiple images into a single PDF
        Files:
          apps/tools/backend/workers/src/pdf/images_to_pdf.rs
          apps/tools/frontend/src/app/pdf/images-to-pdf/page.tsx
        Functions:
          images_to_pdf(image_paths: &[PathBuf], options: PdfOptions) -> Result<Vec<u8>>
        Logic:
          1. Load each image via image::open
          2. Convert to JPEG (for PDF embedding)
          3. Create PDF page per image
          4. Fit image to A4 / Letter / Original size
          5. Optional: margin, alignment
        Frontend:
          - UploadZone (multiple, accept image/*)
          - Thumbnail grid with drag-to-reorder
          - Page size: A4, Letter, Original
          - Orientation: Auto, Portrait, Landscape
          - Margin: 0-50mm
          - Download PDF
        Accept: Upload 5 photos → download 1 PDF, each photo = 1 page
```

#### 3.1.4 — PDF Compress Worker

```
[3.1.4] Reduce PDF file size by recompressing embedded images
        Files:
          apps/tools/backend/workers/src/pdf/compress.rs
          apps/tools/frontend/src/app/pdf/compress/page.tsx
        Algorithm:
          1. Open PDF with lopdf
          2. Find all image XObject streams
          3. For each: decode → re-encode JPEG with lower quality
          4. Replace stream in PDF
          5. Remove unused objects
          6. Linearize PDF for fast web viewing
        Compression levels:
          - Maximum (q=30): smallest size, visible quality loss
          - Balanced (q=50): good balance (default)
          - Minimal (q=80): slight size reduction, near-lossless
        Frontend:
          - UploadZone (single PDF)
          - Compression level selector
          - Preview: original size vs estimated compressed size
          - Download compressed PDF
        Accept: Upload 10MB PDF with images → compress → <3MB output
```

#### 3.1.5 — PDF to Images Worker

```
[3.1.5] Convert each PDF page to an image
        Files:
          apps/tools/backend/workers/src/pdf/to_images.rs
          apps/tools/frontend/src/app/pdf/pdf-to-images/page.tsx
        Functions:
          pdf_page_to_image(input_path: &Path, page_num: u32, dpi: u32) -> Result<Vec<u8>>
          pdf_to_images(input_path: &Path, dpi: u32, format: &str) -> Result<Vec<Vec<u8>>>
        Logic:
          1. Render PDF page to image (via lopdf rasterize or poppler)
          2. Output format: JPEG, PNG, WebP
          3. DPI: 72, 150, 200, 300 (default: 200)
        Frontend:
          - UploadZone (single PDF)
          - DPI selector
          - Format selector
          - Preview: page thumbnails
          - Download as ZIP (all images) or per-page
        Accept: Upload PDF → download ZIP of page images
```

---

## Phase 4: Video/Audio Tools

### Milestone 4.1 — FFmpeg Worker

#### 4.1.1 — FFmpeg Binding & Worker Setup

```
[4.1.1] Integrate FFmpeg via ffmpeg-next crate
        Files:
          apps/tools/backend/workers/Cargo.toml (add ffmpeg-next)
          apps/tools/backend/workers/src/video/mod.rs
          apps/tools/backend/workers/src/audio/mod.rs
          Dockerfile (update — install ffmpeg package)
        Setup:
          - apt-get install ffmpeg
          - ffmpeg-next version 7.x
          - Initialize: ffmpeg::init() at worker startup
          - Test: version check
        Accept: Worker starts → ffmpeg initialized → transcoding functions available
```

#### 4.1.2 — Video Compress

```
[4.1.2] Reduce video file size by lowering bitrate and/or resolution
        Files:
          apps/tools/backend/workers/src/video/compress.rs
          apps/tools/frontend/src/app/video/compress/page.tsx
        Options:
          - Target size (approximate): 10MB, 25MB, 50MB, 100MB, Custom
          - Resolution: 480p, 720p, 1080p, Original
          - Quality: Low, Medium, High (CRF: 28, 23, 18)
          - Codec: H.264, H.265/HEVC, VP9
        FFmpeg command (via ffmpeg-next API):
          - Transcode video stream: libx264, crf, preset medium
          - Scale if needed: scale=iw/2:ih/2
          - Copy audio stream (or compress with aac)
        Frontend:
          - UploadZone (accept video/*, max 500MB)
          - Options: target size, resolution, quality
          - Progress: from FFmpeg stderr parsed progress
          - Preview: thumbnail + duration, original vs estimated size
          - Download compressed video
        Note: Video processing is HEAVY → max 1 concurrent video job, queue via NATS
        Accept: Upload 500MB 1080p video → compress → <100MB 720p output
```

#### 4.1.3 — Audio Extract

```
[4.1.3] Extract audio track from video file
        Files:
          apps/tools/backend/workers/src/video/audio_extract.rs
          apps/tools/frontend/src/app/video/audio-extract/page.tsx
        Options:
          - Format: MP3, AAC, WAV, FLAC, OGG
          - Quality: 128k, 192k, 320k (for lossy) / Lossless (for FLAC/WAV)
        FFmpeg:
          - Stream copy if format compatible, else transcode
          - Extract best audio stream
        Frontend:
          - UploadZone (accept video/*)
          - Format + quality selector
          - Progress
          - Download extracted audio
        Accept: Upload MP4 → download MP3 with correct audio
```

#### 4.1.4 — Video Trim

```
[4.1.4] Cut a segment from a video
        Files:
          apps/tools/backend/workers/src/video/trim.rs
          apps/tools/frontend/src/app/video/trim/page.tsx
        Options:
          - Start time (HH:MM:SS or seconds)
          - End time / Duration
        FFmpeg:
          -ffmpeg -i input -ss start -to end -c copy output (fast seek)
          Or re-encode for precise seeking: -ss start -i input -t duration -c libx264
        Frontend:
          - UploadZone (accept video/*)
          - Preview player with timeline
          - Set start/end via sliders or input
          - Preview trim result
          - Download trimmed video
        Accept: Upload 10min video → trim 2:30-5:00 → download 2.5min video
```

#### 4.1.5 — GIF Maker

```
[4.1.5] Convert video segment to animated GIF
        Files:
          apps/tools/backend/workers/src/video/gif.rs
          apps/tools/frontend/src/app/video/gif-maker/page.tsx
        Options:
          - Start time, Duration (max 10s for GIF)
          - FPS: 10, 15, 20, 24, 30
          - Width: 320, 480, 640, 800
          - Dither: on/off
          - Colors: 64, 128, 256
        FFmpeg:
          ffmpeg -i input -ss start -t duration -vf "fps=15,scale=480:-1:flags=lanczos,palettegen" palette.png
          ffmpeg -i input -i palette.png -ss start -t duration -lavfi "fps=15,scale=480:-1:flags=lanczos[x];[x][1:v]paletteuse" output.gif
        Frontend:
          - UploadZone (accept video/*)
          - Start time + duration sliders
          - FPS + width + dither options
          - Preview: animated GIF in browser
          - Download GIF
        Accept: Upload 30s video → trim 2s → download animated GIF
```

#### 4.1.6 — Audio Convert

```
[4.1.6] Convert audio between formats
        Files:
          apps/tools/backend/workers/src/audio/convert.rs
          apps/tools/frontend/src/app/audio/convert/page.tsx
        Formats:
          Input: MP3, WAV, FLAC, AAC, OGG, M4A, WMA
          Output: MP3, WAV, FLAC, AAC, OGG
        Options:
          - Bitrate: 128k, 192k, 256k, 320k
          - Sample rate: 44100, 48000, 96000
          - Channels: Mono, Stereo (downmix if source is 5.1)
        Frontend:
          - UploadZone (accept audio/*)
          - From (auto-detected) → To selector
          - Bitrate + sample rate options
          - Download converted audio
        Accept: Upload FLAC → convert to 320kbps MP3 → download
```

---

## Phase 5: Deployment & Polish

### Milestone 5.1 — Infrastructure

#### 5.1.1 — Docker Multi-Stage Build

```
[5.1.1] Production Dockerfile
        Files:
          infra/docker/tools.Dockerfile
        Stages:
          1. chef (cargo-chef install)
          2. planner (recipe.json)
          3. builder (cargo build —release)
          4. wasm-builder (wasm-pack build)
          5. frontend-builder (bun build)
          6. runtime: debian:bookworm-slim + tesseract + ffmpeg + ONNX model
        Runtime dependencies:
          - tesseract-ocr + tessdata (eng, ind)
          - ffmpeg
          - ca-certificates
          - libfontconfig1 (for lopdf)
        Dockerignore:
          - node_modules, target, .next, .git
        Build: docker build -f infra/docker/tools.Dockerfile -t tools:latest .
        Accept: docker build succeeds, image size <400MB
```

#### 5.1.2 — Docker Compose File

```
[5.1.2] Compose file for production deployment
        Files:
          infra/compose/tools.yml
        Service definition:
          container_name: tools
          image: ghcr.io/asepharyana/asepharyana-hub/tools:sha-xxxxx
          restart: always
          networks: app-shared-net (alias: tools)
          env_file: ../../.env
          volumes: tools_data:/data/tools
          ports: 3001:3001
          depends_on: [redis, nats]
          labels: prometheus.io/scrape=true, prometheus.io/port=3001
        Volume: tools_data (docker volume)
        Accept: docker compose up → tools container running, connected to redis + nats
```

#### 5.1.3 — Traefik Routing

```
[5.1.3] Add Traefik router and service for tools
        Files:
          infra/traefik/dynamic/apps.yaml (update)
        Router:
          tools:
            rule: Host(`tools.asepharyana.my.id`) || Host(`tools.asepharyana.web.id`)
            entryPoints: websecure
            tls: {}
            middlewares: common-chain@file
            service: tools-service
        Service:
          tools-service:
            loadBalancer:
              servers:
                - url: http://tools:3001
        Accept: tools.asepharyana.my.id → loads tools frontend
```

#### 5.1.4 — CI/CD Workflow Integration

```
[5.1.4] Add tools service to existing build + deploy workflows
        Files:
          .github/workflows/docker-build-push.yml (update)
          .github/workflows/deploy-docker.yml (check — auto-detects compose changes)
        Changes:
          - Detect changed service (apps/tools/**)
          - Build matrix: add tools service
          - Dockerfile: tools.Dockerfile
          - Path: apps/tools
          - Compose file: tools.yml
          - Update manifest: sed image tag in compose
        Accept: Push to main with apps/tools changes → CI builds + deploys tools
```

#### 5.1.5 — Environment Variables Setup

```
[5.1.5] Add tools env vars to .env.example
        Files:
          .env.example (update)
        Vars:
          # Tools
          TOOLS_GATEWAY_PORT=3001
          TOOLS_WORKER_CONCURRENCY=4
          TOOLS_STORAGE_PATH=/data/tools
          TOOLS_JOB_TTL_SECONDS=3600
          TOOLS_RATE_LIMIT_PER_MINUTE=30
          TOOLS_MAX_FILE_SIZE_MB=50
          TOOLS_OCR_LANG=eng+ind
        Accept: .env.example updated with tools section
```

---

### Milestone 5.2 — Frontend Polish

#### 5.2.1 — Theme Integration

```
[5.2.1] Apply Twilight Terminal theme consistent with portfolio hub
        Files:
          apps/tools/frontend/src/app/globals.css (update)
        Theme vars (from hub):
          --background / --foreground
          --primary / --primary-foreground
          --card / --card-foreground
          --muted / --muted-foreground
          Glass effect: .glass { backdrop-filter: blur }
          Gradient text: .gradient-text
          Terminal cursor blink animation
        Same dark/light mode switch mechanism
        Accept: tools subdomain → visual style consistent with hub portfolio
```

#### 5.2.2 — Responsive Mobile Design

```
[5.2.2] All pages responsive for mobile devices
        Files: All page/component files (review)
        Requirements:
          - UploadZone: full-width on mobile, tap-friendly
          - Tool cards: single column on mobile
          - ProgressBar: always visible, top-fixed on scroll
          - CropEditor: touch-drag handles, pinch-zoom
          - CameraCapture: fullscreen viewfinder
          - FileList: compact thumbnail list on mobile
          - Buttons: min 44px touch target
          - Bottom sheet instead of modal for options
          - Safe area insets for notch devices
        Accept: Lighthouse mobile audit >80 for all pages
```

#### 5.2.3 — Loading States & Skeleton

```
[5.2.3] Skeleton loading states for all pages
        Files:
          apps/tools/frontend/src/components/tools/skeleton.tsx
          apps/tools/frontend/src/app/scan/page.tsx (update)
          apps/tools/frontend/src/app/image/compress/page.tsx (update)
          (all other tool pages)
        Components:
          SkeletonCard (pulse animation)
          SkeletonUploadZone
          SkeletonProgressBar
          SkeletonPreview
        Accept: All pages show skeleton while loading data/WASM
```

#### 5.2.4 — Error Boundaries

```
[5.2.4] React error boundaries per page + global
        Files:
          apps/tools/frontend/src/components/tools/error-boundary.tsx
          apps/tools/frontend/src/app/layout.tsx (wrap with ErrorBoundary)
          Each tool page: wrap with ErrorBoundary
        Behavior:
          - Catch React render errors
          - Show friendly error message with tool name
          - "Try Again" button
          - "Report Issue" link (GitHub)
          - Log error details to console (future: telemetry)
        Accept: Force render error → error boundary shows, app doesn't crash
```

#### 5.2.5 — PWA Manifest

```
[5.2.5] Progressive Web App configuration
        Files:
          apps/tools/frontend/public/manifest.json
          apps/tools/frontend/src/app/layout.tsx (add manifest link + meta tags)
          apps/tools/frontend/public/icons/ (app icons: 192x192, 512x512)
        Manifest:
          name: "Tools — Asep Haryana"
          short_name: "Tools"
          description: "Document Scanner, Image & PDF Tools"
          start_url: /
          display: standalone
          background_color: #0a0a1a (dark theme)
          theme_color: #0a0a1a
          icons: 192x192, 512x512
        Accept: Lighthouse PWA audit >80
```

---

### Milestone 5.3 — Monitoring & Observability

#### 5.3.1 — Prometheus Alerts

```
[5.3.1] Alert rules for tools service
        Files:
          infra/otel/prometheus.yml (update — or separate alert file)
        Rules:
          - High error rate: rate(tools_jobs_total{status="failed"}[5m]) > 0.1
          - Queue buildup: tools_queue_depth > 50
          - Slow processing: tools_processing_duration_ms{quantile="0.95"} > 10000
          - Low disk space: (disk_free_bytes / disk_total_bytes) < 0.1  (if node_exporter)
        Accept: Rules loaded in Prometheus, alert firing correctly
```

#### 5.3.2 — Dashboard Integration

```
[5.3.2] Add tools metrics to hub dashboard
        Files:
          apps/hub/src/app/dashboard/page.tsx (update)
          apps/hub/src/app/api/dashboard/route.ts (update)
        Add to dashboard:
          - Card: "Tools" service status (running/degraded/down)
          - Quick stats: Total jobs today, Active jobs, Storage used
          - Link to tools.asepharyana.my.id
        Accept: Dashboard shows tools service status and stats
```

#### 5.3.3 — Structured Logging

```
[5.3.3] Structured JSON logging for production
        Files:
          apps/tools/backend/common/src/logging.rs (or in each crate)
          apps/tools/backend/gateway/src/main.rs (logging init)
          apps/tools/backend/workers/src/main.rs (logging init)
        Config:
          - Default: human-readable (development)
          - JSON mode: RUST_LOG_FORMAT=json (production)
        Fields per log:
          - timestamp (ISO 8601)
          - level (INFO, WARN, ERROR)
          - service (gateway / workers / pipeline)
          - request_id (if within request context)
          - job_id (if within job context)
          - message
          - duration_ms (for completed processing)
        Accept: RUST_LOG_FORMAT=json → JSON-structured log output
```

---

## Effort Summary

| Phase | Milestone | Tasks | Estimated Hours | Total Days |
|-------|-----------|-------|-----------------|------------|
| 1 | 1.1 Rust Backend Skeleton | 15 | ~45 | 6 |
| 1 | 1.2 Scanner Pipeline Core | 9 | ~50 | 7 |
| 1 | 1.3 Next.js Frontend | 10 | ~30 | 4 |
| 1 | 1.4 OCR + PDF + Infra | 7 | ~25 | 4 |
| **Phase 1 Total** | **41** | **~150** | **~21** |
| 2 | 2.1 Scanner Robustness | 5 | ~25 | 4 |
| 2 | 2.2 WASM Image Tools | 6 | ~30 | 4 |
| 2 | 2.3 Background Removal | 3 | ~15 | 2 |
| **Phase 2 Total** | **14** | **~70** | **~10** |
| 3 | 3.1 PDF Tools | 5 | ~25 | 4 |
| **Phase 3 Total** | **5** | **~25** | **~4** |
| 4 | 4.1 FFmpeg Worker | 6 | ~35 | 5 |
| **Phase 4 Total** | **6** | **~35** | **~5** |
| 5 | 5.1 Infrastructure | 5 | ~15 | 2 |
| 5 | 5.2 Frontend Polish | 5 | ~20 | 3 |
| 5 | 5.3 Monitoring | 3 | ~10 | 2 |
| **Phase 5 Total** | **13** | **~45** | **~7** |
| **Grand Total** | **79 tasks** | **~325 hours** | **~47 days** |

> **MVP** (Phase 1 only): 41 tasks, ~21 days
> **Full release** (Phase 1-5): 79 tasks, ~47 days

---

## Critical Path (Phase 1)

```
Day 1-2:   1.1.1 → 1.1.2 → 1.1.3 → 1.1.4 → 1.1.5     (workspace + common)
Day 3-5:   1.1.6 → 1.1.7 → 1.1.8 → 1.1.10 → 1.1.14   (gateway routes + redis)
Day 5-6:   1.1.9 → 1.1.11 → 1.1.12 → 1.1.13          (gateway + workers connect)
Day 7-10:  1.2.1 → 1.2.2 → 1.2.3 → 1.2.4              (edge → warp)
Day 10-12: 1.2.5 → 1.2.6 → 1.2.7 → 1.2.8              (shadow → binarize → enhance)
Day 12:    1.2.9                                        (pipeline assembly)
Day 13-16: 1.3.1 → 1.3.2 → 1.3.3 → 1.3.4               (frontend pages)
Day 14-17: 1.3.5 → 1.3.6 → 1.3.7 → 1.3.8               (components)
Day 15-17: 1.3.9 → 1.3.10                                (API proxy routes)
Day 18-19: 1.4.1 → 1.4.2 → 1.4.3                        (OCR + PDF)
Day 19:    1.4.4                                        (WebSocket)
Day 20:    1.4.5 → 1.4.6 → 1.4.7                       (cleanup + rate limit + errors)
```

> **MVP launch**: Day ~21 — Document Scanner live di tools.asepharyana.my.id