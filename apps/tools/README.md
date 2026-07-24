# Tools — Document Scanner & Media Processing

Self-hosted, no-install document scanner dan media processing tools yang jalan di browser. Alternatif dari CamScanner, ilovepdf, compressjpeg — tanpa upload ke pihak ketiga.

## Tech Stack

- **Frontend**: Next.js 16 + TypeScript + shadcn/ui + Tailwind v4 + Framer Motion
- **Backend**: Rust (Axum gateway + worker pool with Tokio)
- **Queue**: NATS JetStream (job queue + progress pub/sub)
- **Cache**: Redis (job metadata, rate limiting)
- **Image Processing**: `image` + `imageproc` crates (edge detection, warp, binarization, deskew)
- **OCR**: Tesseract via `leptess` crate (optional feature)

## Architecture

```
Browser → Next.js (frontend) → Rust Gateway (Axum) → NATS Queue → Workers (Tokio+Rayon)
                                              ↕                ↕
                                           Redis            Temp Storage
```

## Directory Structure

```
apps/tools/
├── frontend/          # Next.js 16 SPA
│   ├── src/
│   │   ├── app/       # Pages + API routes
│   │   ├── components/# shadcn/ui components
│   │   └── hooks/     # Custom hooks (useJobStatus, useUpload)
│   ├── package.json
│   └── next.config.ts
├── backend/           # Rust workspace
│   ├── common/        # Shared types, errors, NATS constants
│   ├── gateway/       # Axum API server (upload, job, WS, download)
│   ├── workers/       # Processing workers (scanner, image, PDF)
│   └── wasm/          # WASM image processing (future)
├── scripts/
│   └── entrypoint.sh
└── Dockerfile
```

## Development

```bash
# Start Redis + NATS
docker compose -f infra/compose/shared.yml -f infra/compose/nats.yml up -d

# Start Rust workers
cd apps/tools/backend
REDIS_URL=redis://localhost:6379 NATS_URL=nats://localhost:4222 cargo run --bin workers

# Start Rust gateway (another terminal)
REDIS_URL=redis://localhost:6379 NATS_URL=nats://localhost:4222 \
  STORAGE_PATH=/tmp/tools cargo run --bin gateway

# Start Next.js frontend (another terminal)
cd apps/tools/frontend
bun dev --port 3002
```

## Build

```bash
docker build -f infra/docker/tools.Dockerfile -t tools:latest .
```

## Pipeline Stages (Document Scanner)

1. **Preprocess** — Load, resize (max 2000px), grayscale
2. **Edge Detection** — Canny with adaptive threshold + morphological close
3. **Corner Detection** — Contour analysis with fallback chain
4. **Perspective Warp** — DLT homography + bilinear interpolation
5. **Shadow Removal** — Background subtraction + CLAHE
6. **Binarization** — Sauvola local threshold (integral image accelerated)
7. **Deskew** — Hough transform line detection
8. **Enhance** — Unsharp mask + contrast adjustment
9. **OCR** — Tesseract (English + Indonesian)
10. **PDF Generation** — Searchable PDF with invisible text layer
