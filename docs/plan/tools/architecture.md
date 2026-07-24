# Architecture

## System Overview

```
┌────────────────────────────────────────────────────────────────┐
│  BROWSER                                                       │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────────┐  │
│  │ Upload     │  │ Camera     │  │ Preview + Download     │  │
│  │ (drag/drop)│  │ (PWA)      │  │ (streaming)            │  │
│  └─────┬──────┘  └─────┬──────┘  └───────────┬────────────┘  │
│        │               │                     │                │
│        ▼               ▼                     ▼                │
│  ┌──────────────────────────────────────────────────────┐    │
│  │ WebSocket (progress:  processing/step/percentage)    │    │
│  └──────────────────────────────────────────────────────┘    │
└──────────────────────────┬───────────────────────────────────┘
                           │ HTTPS / WSS
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  TRAEFIK (tools.asepharyana.my.id)                             │
│  Middleware chain: secure-headers → compress → rate-limit      │
└──────────────────────────┬────────────────────────────────────┘
                           │
                           ▼
┌────────────────────────────────────────────────────────────────┐
│  tools-app (Next.js 16 / TypeScript)                           │
│                                                               │
│  ┌──────────────────┐  ┌─────────────────┐                   │
│  │  Pages/Routes     │  │  API Routes     │                   │
│  │  / → home         │  │  POST /api/upload ──▶ file          │
│  │  /scan → scanner  │  │  GET  /api/job/:id ─▶ status        │
│  │  /image → image   │  │  WS   /api/job/:id/ws ─▶ progress   │
│  │  /pdf → pdf tools │  │  GET  /api/download/:id ─▶ file     │
│  └──────────────────┘  └─────────────────┘                   │
│                                                               │
│  Upload validation: MIME type, size limit (50MB), virus scan   │
│  Temp storage bridge ke worker via HTTP/NATS                   │
└──────────────────┬────────────────────────────────────────────┘
                   │ HTTP (internal)
                   ▼
┌────────────────────────────────────────────────────────────────┐
│  API GATEWAY (Rust / Axum)                                     │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐  │
│  │  Upload      │  │  Job Manager │  │  Download          │  │
│  │  (streaming  │  │  (CRUD job   │  │  (stream file,     │  │
│  │   chunked)   │  │   status)    │  │   auto-delete)     │  │
│  └──────┬───────┘  └──────┬───────┘  └────────────────────┘  │
│         │                 │                                    │
│         ▼                 ▼                                    │
│  ┌────────────────────────────────────────────────────┐       │
│  │  NATS JetStream                                    │       │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐      │       │
│  │  │ scan.    │ │ image.   │ │ pdf.         │      │       │
│  │  │ jobs     │ │ jobs     │ │ jobs         │      │       │
│  │  └────┬─────┘ └────┬─────┘ └──────┬───────┘      │       │
│  │       │            │              │               │       │
│  │       ▼            ▼              ▼               │       │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐      │       │
│  │  │ scan.    │ │ image.   │ │ pdf.         │      │       │
│  │  │ progress │ │ progress │ │ progress     │      │       │
│  │  └──────────┘ └──────────┘ └──────────────┘      │       │
│  └────────────────────────────────────────────────────┘       │
│                                                               │
│  ┌────────────────────────────────────────────────────┐       │
│  │  Cache (Redis)                                     │       │
│  │  - Job metadata (status, progress, timestamps)     │       │
│  │  - Rate limiting (sliding window per IP/tool)      │       │
│  │  - Result metadata (file path, size, type)         │       │
│  └────────────────────────────────────────────────────┘       │
└──────────────────┬────────────────────────────────────────────┘
                   │ consume NATS queue
                   ▼
┌────────────────────────────────────────────────────────────────┐
│  WORKER POOL (Rust / Tokio + Rayon)                            │
│                                                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌──────────────┐  │
│  │  Scan Worker     │  │  Image Worker   │  │  PDF Worker  │  │
│  │  ×4 instances    │  │  ×2 instances   │  │  ×2 instances│  │
│  │                  │  │                 │  │              │  │
│  │  1. Load image   │  │  1. Load image  │  │  1. Load PDF │  │
│  │  2. Edge detect  │  │  2. Compress    │  │  2. Merge/   │  │
│  │  3. Warp         │  │     /resize/    │  │     split    │  │
│  │  4. Enhance      │  │     convert     │  │  3. Save     │  │
│  │  5. OCR          │  │  3. Save       │  │  4. Update   │  │
│  │  6. Gen PDF      │  │  4. Update     │  │     job      │  │
│  │  7. Update job   │  │     job status │  │     status   │  │
│  │  └───────────────┘  └─────────────────┘  └──────────────┘  │
│                   │                                            │
│                   ▼                                            │
│  ┌────────────────────────────────────────────────────┐       │
│  │  Temp Storage (filesystem volume / S3-compatible)  │       │
│  │  Auto-cleanup: job TTL 1 jam, NATS cron tiap 10m  │       │
│  └────────────────────────────────────────────────────┘       │
└────────────────────────────────────────────────────────────────┘
```

## Component Diagram

```
┌────────────────────────────────────────────┐
│  apps/tools                                 │
│                                            │
│  ├── frontend/                             │
│  │   ├── pages/         ← Next.js pages    │
│  │   ├── components/    ← React components │
│  │   ├── lib/           ← utilities        │
│  │   └── public/        ← static assets    │
│  │                                            │
│  ├── backend/          ← Rust workspace     │
│  │   ├── gateway/      ← Axum API server    │
│  │   ├── workers/      ← Processing workers │
│  │   │   ├── scanner/  ← Document scanner   │
│  │   │   ├── image/    ← Image tools        │
│  │   │   └── pdf/      ← PDF tools          │
│  │   └── common/       ← Shared libs        │
│  │                                            │
│  └── Dockerfile                              │
└────────────────────────────────────────────┘
```

## Data Flow (Document Scanner — Flow Lengkap)

```
1. User buka tools.asepharyana.my.id/scan
2. Upload foto via drag-drop atau kamera HP (PWA)
3. Next.js route handler menerima file
   ├─ Validasi: MIME type (image/*), max 50MB, virus header scan
   └─ Upload chunked ke Gateway internal (HTTP POST)

4. Gateway menerima stream:
   ├─ Simpan ke temp storage
   ├─ Buat job record di Redis: {id, tool: "scan", status: "queued", progress: 0}
   └─ Publish ke NATS: tools.scan.jobs {job_id, file_path, options}

5. Scan Worker consume dari NATS:
   ├─ Update Redis: status = "processing", progress = 10
   ├─ Load image (image-rs)
   ├─ Pipeline (detail di pipeline.md):
   │   1. Edge detection   ──▶ progress 25
   │   2. Perspective warp  ──▶ progress 40
   │   3. Shadow removal    ──▶ progress 55
   │   4. Binarization      ──▶ progress 70
   │   5. Contrast/sharpen  ──▶ progress 80
   │   6. OCR               ──▶ progress 90
   │   7. Generate PDF      ──▶ progress 95
   ├─ Simpan file hasil ke temp storage
   ├─ Update Redis: status = "completed", progress = 100, result_path, ocr_text
   └─ Publish ke NATS: tools.scan.progress {job_id, status, progress}

6. WebSocket handler di Gateway:
   ├─ Subscribe NATS topics tools.scan.progress
   ├─ Forward ke browser user (per-job-id filter)
   └─ Browser update progress bar + preview

7. User download PDF:
   ├─ GET /api/download/:job_id
   ├─ Gateway stream file dari temp storage
   └─ Browser save file
```

## Tech Stack

### Frontend (Next.js + TypeScript)

| Library | Fungsi |
|---------|--------|
| Next.js 16 | App router, API routes |
| shadcn/ui + Tailwind v4 | UI components |
| Framer Motion | Animasi progress, transisi |
| Canvas API | Preview crop manual, image manipulation client-side |
| WebSocket API | Real-time progress |

### Backend (Rust)

| Crate | Fungsi |
|-------|--------|
| `axum` | HTTP server (Gateway) |
| `tokio` | Async runtime |
| `image` | Image I/O, resize, convert, compress |
| `imageproc` | Edge detection, contour, thresholding |
| `lopdf` | PDF generation, merge, split, compress |
| `leptess` | Tesseract OCR binding |
| `ort` | ONNX Runtime (background removal) |
| `async-nats` | NATS JetStream client |
| `deadpool-redis` | Redis connection pool |
| `redis` | Redis async client |
| `rayon` | Parallel processing (batch, pixel ops) |
| `serde` | Serialization |
| `tracing` + `opentelemetry` | Observability |
| `uuid` | Job ID generation |

### Infrastructure

| Komponen | Fungsi |
|----------|--------|
| NATS JetStream | Job queue, progress pub/sub, scheduler |
| Redis | Job metadata, rate limiting, cache |
| PostgreSQL | Opsional — audit log, usage statistics |
| Tesseract | OCR engine (data files di Docker image) |
| Prometheus | Metrics (jobs/min, queue depth, latency per stage) |

## Job Queue (NATS Streams & Consumers)

### Streams

```
tools-scan-jobs       → 1 stream, mirror to all scan workers
tools-image-jobs      → 1 stream, mirror to all image workers
tools-pdf-jobs        → 1 stream, mirror to all pdf workers
tools-progress        → 1 stream, all progress events (key-value by job_id)
tools-scheduler       → 1 stream, cron events
```

### Subjects

```
tools.scan.jobs.{job_id}         → job submission
tools.scan.progress.{job_id}     → progress update (fan-out ke Gateway)
tools.image.jobs.{job_id}        → job submission
tools.image.progress.{job_id}    → progress update
tools.pdf.jobs.{job_id}          → job submission
tools.pdf.progress.{job_id}      → progress update
tools.scheduler.cleanup          → cleanup expired files (every 10 min)
```

## Redis Schema

```
job:{id}                      → Hash {status, tool, progress, file_path, result_path, ocr_text, created_at, ttl}
rate_limit:{ip}:{tool}        → Sorted Set (sliding window)
file_meta:{hash}              → String {original_name, size, mime}
```

## Metrics (Prometheus)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `tools_jobs_total` | Counter | `tool`, `status` | Total jobs processed |
| `tools_jobs_in_flight` | Gauge | `tool` | Currently processing jobs |
| `tools_queue_depth` | Gauge | `tool` | NATS queue depth |
| `tools_processing_duration` | Histogram | `tool`, `stage` | Duration per stage |
| `tools_file_size_bytes` | Histogram | `tool` | Upload file size distribution |
| `tools_rate_limit_hits` | Counter | `tool` | Rate limit violations |

## Directory Structure

```
apps/tools/
├── frontend/
│   ├── src/
│   │   ├── app/
│   │   │   ├── page.tsx              # Landing page
│   │   │   ├── scan/
│   │   │   │   ├── page.tsx          # Scanner page
│   │   │   │   └── result/[id]/
│   │   │   │       └── page.tsx      # Result page
│   │   │   ├── image/
│   │   │   │   ├── compress/page.tsx
│   │   │   │   ├── resize/page.tsx
│   │   │   │   ├── convert/page.tsx
│   │   │   │   └── remove-bg/page.tsx
│   │   │   ├── pdf/
│   │   │   │   ├── merge/page.tsx
│   │   │   │   ├── split/page.tsx
│   │   │   │   ├── images-to-pdf/page.tsx
│   │   │   │   └── compress/page.tsx
│   │   │   ├── api/
│   │   │   │   ├── upload/route.ts
│   │   │   │   ├── job/[id]/route.ts
│   │   │   │   │   └── ws/route.ts
│   │   │   │   └── download/[id]/route.ts
│   │   │   ├── layout.tsx
│   │   │   └── globals.css
│   │   ├── components/
│   │   │   ├── upload-zone.tsx       # Drag & drop area
│   │   │   ├── progress-bar.tsx      # WebSocket-connected progress
│   │   │   ├── preview.tsx           # Before/after preview
│   │   │   ├── crop-editor.tsx       # Manual corner adjustment
│   │   │   ├── tool-layout.tsx       # Consistent tool page layout
│   │   │   └── camera-capture.tsx    # PWA camera interface
│   │   ├── hooks/
│   │   │   ├── use-job-status.ts     # WebSocket connection
│   │   │   ├── use-upload.ts         # Upload with progress
│   │   │   └── use-camera.ts         # Camera access
│   │   └── lib/
│   │       ├── utils.ts
│   │       └── types.ts
│   ├── next.config.ts
│   ├── package.json
│   └── tsconfig.json
│
├── backend/
│   ├── Cargo.toml
│   ├── gateway/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── upload.rs
│   │       │   ├── job.rs
│   │       │   ├── download.rs
│   │       │   └── ws.rs
│   │       ├── nats/
│   │       │   ├── mod.rs
│   │       │   └── publisher.rs
│   │       ├── redis/
│   │       │   ├── mod.rs
│   │       │   ├── job.rs
│   │       │   └── ratelimit.rs
│   │       ├── metrics.rs
│   │       └── config.rs
│   │
│   ├── workers/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── scanner/
│   │       │   ├── mod.rs
│   │       │   ├── pipeline.rs
│   │       │   ├── edge.rs          # Edge detection
│   │       │   ├── warp.rs          # Perspective correction
│   │       │   ├── enhance.rs       # Shadow removal, B&W, contrast
│   │       │   ├── ocr.rs           # Tesseract wrapper
│   │       │   └── pdf.rs           # Generate searchable PDF
│   │       ├── image/
│   │       │   ├── mod.rs
│   │       │   ├── compress.rs
│   │       │   ├── resize.rs
│   │       │   ├── convert.rs
│   │       │   └── remove_bg.rs
│   │       ├── pdf/
│   │       │   ├── mod.rs
│   │       │   ├── merge.rs
│   │       │   ├── split.rs
│   │       │   ├── extract.rs
│   │       │   └── compress.rs
│   │       ├── nats/
│   │       │   ├── mod.rs
│   │       │   └── consumer.rs
│   │       └── config.rs
│   │
│   └── common/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── types.rs             # Shared types (JobStatus, Job, etc.)
│           ├── error.rs             # Error types
│           └── nats.rs              # NATS subject constants
│
├── Dockerfile
├── compose.yml                       # Local dev compose
└── README.md
```

## API Design

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/upload` | Upload file, create job |
| `GET` | `/api/job/:id` | Get job status + result metadata |
| `WS` | `/api/job/:id/ws` | WebSocket — realtime progress |
| `GET` | `/api/download/:id` | Download result file |
| `DELETE` | `/api/job/:id` | Cancel job, delete files |
| `GET` | `/health` | Health check |

### Upload Request

```
POST /api/upload
Content-Type: multipart/form-data

{
  file: <binary>,
  tool: "scan" | "image-compress" | "image-resize" | "image-convert" | "remove-bg" |
        "pdf-merge" | "pdf-split" | "images-to-pdf" | "pdf-compress",
  options?: {           // tool-specific options
    quality?: 80,       // compress quality
    width?: 1920,       // resize width
    format?: "webp",    // convert format
    pages?: "1,3-5",    // PDF split pages
    dpi?: 300,          // scan DPI
    enhance?: true,     // scan auto-enhance
    ocr?: true          // scan OCR
  }
}
```

### Response (202 Accepted)

```json
{
  "job_id": "uuid",
  "status": "queued",
  "tool": "scan",
  "ws_url": "/api/job/uuid/ws",
  "created_at": "2026-07-24T10:00:00Z",
  "estimated_seconds": 5
}
```

### WebSocket Messages

```json
// Server → Client
{
  "type": "progress",
  "job_id": "uuid",
  "status": "processing",
  "progress": 45,
  "stage": "warp",
  "message": "Meluruskan perspektif dokumen..."
}

{
  "type": "complete",
  "job_id": "uuid",
  "status": "completed",
  "progress": 100,
  "result": {
    "download_url": "/api/download/uuid",
    "file_name": "scan_20260724.pdf",
    "file_size": 1245678,
    "pages": 1,
    "ocr_text": "Nama: Asep...",
    "preview_url": "/api/job/uuid/preview"
  }
}

{
  "type": "error",
  "job_id": "uuid",
  "status": "failed",
  "error": "Edge detection failed: cannot find document boundary"
}
```

## Integration with Existing Portfolio

| Area | Detail |
|------|--------|
| **Domain** | `tools.asepharyana.my.id` — tambah entry di `infra/traefik/dynamic/apps.yaml` |
| **Dashboard** | Link ke tools stats di dashboard hub yang sudah ada |
| **Docker Compose** | `infra/compose/tools.yml` — pola sama kaya `hub.yml` |
| **CI/CD** | Tambah service `tools` di `docker-build-push.yml` |
| **Style** | Ulang Twilight Terminal theme dari hub, konsisten visual branding |
| **Monitoring** | Reuse existing Prometheus + Grafana, tambah metrics tools |
