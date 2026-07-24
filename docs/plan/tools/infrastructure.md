# Infrastructure & Deployment

## Docker Image Architecture

Project ini punya **satu Docker image** dengan multi-stage build. Backend Rust + Tesseract + ONNX model plus frontend Next.js.

### Dockerfile Structure

```dockerfile
# ============================================================
# Stage 1: Build Rust Backend
# ============================================================
FROM rust:1.85-slim-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY backend/ .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY backend/ .
RUN cargo build --release --bin gateway --bin workers

# ============================================================
# Stage 2: Build Next.js Frontend
# ============================================================
FROM oven/bun:1.3 AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile
COPY frontend/ .
RUN bun run build

# ============================================================
# Stage 3: Production Runtime
# ============================================================
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    tesseract-ocr \
    tesseract-ocr-eng \
    tesseract-ocr-ind \
    ca-certificates \
    fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Rust binaries
COPY --from=builder /app/target/release/gateway /app/gateway
COPY --from=builder /app/target/release/workers /app/workers

# Copy Next.js build
COPY --from=frontend-builder /app/.next /app/.next
COPY --from=frontend-builder /app/public /app/public
COPY --from=frontend-builder /app/package.json /app/package.json
COPY --from=frontend-builder /app/node_modules /app/node_modules

# Copy ONNX model (for background removal)
COPY models/ /app/models/

# Create temp storage directory
RUN mkdir -p /data/tools && chmod 1777 /data/tools

# Environment
ENV TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
ENV TOOLS_STORAGE_PATH=/data/tools
ENV TOOLS_GATEWAY_PORT=3001
ENV TOOLS_WORKER_CONCURRENCY=4
ENV RUST_LOG=info

# Expose port
EXPOSE 3001

# Run both gateway and workers via supervisor script
COPY scripts/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

CMD ["/app/entrypoint.sh"]
```

### Entrypoint Script

```bash
#!/bin/bash
# Start Gateway (Axum HTTP server)
/app/gateway &
GATEWAY_PID=$!

# Start Worker(s)
/app/workers &
WORKER_PID=$!

# Handle graceful shutdown
trap "kill $GATEWAY_PID $WORKER_PID; exit 0" SIGINT SIGTERM

# Wait for either process to exit
wait -n $GATEWAY_PID $WORKER_PID

# If one exits, kill the other
kill $GATEWAY_PID $WORKER_PID 2>/dev/null
exit 1
```

### Image Size Estimates

| Component | Size |
|-----------|------|
| Rust binary (gateway) | ~8 MB |
| Rust binary (workers) | ~15 MB |
| Next.js build | ~10 MB |
| Tesseract + data | ~25 MB |
| ONNX model | ~50 MB |
| Base (Debian slim) | ~80 MB |
| **Total** | **~188 MB** |

> ONNX model opsional — bisa di-download runtime daripada di-include di image.

---

## Docker Compose

```yaml
# infra/compose/tools.yml
services:
  tools:
    container_name: tools
    image: ghcr.io/asepharyana/asepharyana-hub/tools:sha-xxxxxxx
    restart: always
    networks:
      app-shared-net:
        aliases:
          - tools
    env_file:
      - ../../.env
    environment:
      - REDIS_URL=redis://redis:6379
      - NATS_URL=nats://nats:4222
      - TOOLS_STORAGE_PATH=/data/tools
      - TOOLS_GATEWAY_PORT=3001
      - TOOLS_WORKER_CONCURRENCY=4
      - RUST_LOG=info
    volumes:
      - tools_data:/data/tools
    ports:
      - "3001:3001"
    depends_on:
      redis:
        condition: service_started
      nats:
        condition: service_started

volumes:
  tools_data:

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

### Environment Variables (`../../.env`)

```bash
# Tools
TOOLS_GATEWAY_PORT=3001
TOOLS_WORKER_CONCURRENCY=4
TOOLS_STORAGE_PATH=/data/tools
TOOLS_JOB_TTL_SECONDS=3600
TOOLS_RATE_LIMIT_PER_MINUTE=30
TOOLS_MAX_FILE_SIZE_MB=50
TOOLS_OCR_LANG=eng+ind

# Infra (reuse existing)
REDIS_URL=redis://redis:6379
NATS_URL=nats://nats:4222
```

---

## CI/CD Integration

### Docker Build Workflow

Tambah service `tools` di `.github/workflows/docker-build-push.yml`:

```yaml
# Di job "changes" step "Detect changed services"
changed() {
  printf '%s\n' "$CHANGED_FILES" | grep -Eq "$1" && echo true || echo false
}
echo "tools=$(changed '^(apps/tools(/|$)|\.github/workflows/docker-build-push\.yml$|infra/docker/tools\.Dockerfile$)')" >> "$GITHUB_OUTPUT"

# Di job "build" step "Set matrix"
if [ "${{ steps.filter.outputs['tools'] == 'true' || steps.dispatch.outputs['tools'] == 'true' || github.event_name == 'workflow_dispatch' }}" == "true" ]; then
  add_service "tools" "docker-tools" "apps/tools"
fi

# Di job "build" step "Docker metadata"
case "$SVC_NAME" in
  "tools") echo "dockerfile=infra/docker/tools.Dockerfile" >> $GITHUB_OUTPUT ;;
esac

# Di job "update-manifest"
SERVICES["tools"]="tools.yml"
PATHS["tools"]="apps/tools"
```

### Deploy Workflow

Tambah di `.github/workflows/deploy-docker.yml`:
```yaml
# Tidak perlu perubahan — deploy-docker.yml auto-detect compose file changes.
# Kalau compose/tools.yml berubah, service tools akan di-restart.
```

### Service Registration (update infra/traefik/dynamic/apps.yaml)

```yaml
tools:
  rule: 'Host(`tools.asepharyana.my.id`) || Host(`tools.asepharyana.web.id`)'
  entryPoints:
    - websecure
  tls: {}
  middlewares:
    - common-chain@file
  service: tools-service

# ...di bagian services:
tools-service:
  loadBalancer:
    servers:
      - url: 'http://tools:3001'
```

---

## Monitoring

### Prometheus Metrics

Tambahkan label Prometheus ke container tools:

```yaml
# Di compose tools.yml
labels:
  - 'prometheus.io/scrape=true'
  - 'prometheus.io/port=3001'
  - 'prometheus.io/path=/metrics'
```

### Dashboard Integration

Tambah card di dashboard hub yang sudah ada:

```tsx
// Di dashboard hub — tambah section "Tools Usage"
// Data dari /api/dashboard → Prometheus query:
//   rate(tools_jobs_total[24h]) — jobs per tool per hari
//   sum(increase(tools_jobs_total[7d])) — total jobs minggu ini
//   tools_jobs_in_flight — current processing
```

---

## Storage Architecture

### Temp Storage

```
/data/tools/
├── upload/          # Uploaded files
│   └── {job_id}.{ext}
├── processing/      # Intermediate files (stage-by-stage)
│   └── {job_id}/
│       ├── 00_original.png
│       ├── 01_grayscale.png
│       ├── 02_edges.png
│       ├── 03_warped.png
│       └── ...
└── output/          # Final output
    └── {job_id}.pdf
```

### Cleanup Strategy

| Mekanisme | Timing |
|-----------|--------|
| NATS cron job | Setiap 10 menit |
| Scan files >1 jam | `find /data/tools -mmin +60 -delete` |
| Redis job keys >1 jam | `SCAN 0 MATCH job:*` → TTL check → DEL |
| Storage low warning | Alert via Notification Hub (future) |

---

## Resource Estimation (VPS orangevps)

### Current Usage

| Service | CPU | RAM | Disk |
|---------|-----|-----|------|
| Traefik | 0.1 | 50 MB | 10 MB |
| NATS | 0.05 | 30 MB | 10 MB |
| Redis | 0.05 | 10 MB | 5 MB |
| Dapr Placement | 0.02 | 20 MB | 5 MB |
| Scraper API | 0.1 | 30 MB | 50 MB |
| Hub | 0.05 | 120 MB | 200 MB |
| Jaeger | 0.1 | 200 MB | 500 MB |
| Prometheus | 0.1 | 150 MB | 1 GB |
| Node Exporter | 0.02 | 10 MB | 5 MB |
| OTel Collector | 0.05 | 50 MB | 10 MB |
| **Total Current** | **~0.64** | **~670 MB** | **~1.8 GB** |

### Tools Addition

| Resources | Estimate | Notes |
|-----------|----------|-------|
| CPU | +1.0 core (burst) | Pipeline processing berat di CPU. Scoring, warp, OCR semua CPU-bound. |
| RAM | +300 MB | Rust binary + image processing buffers + Tesseract + ONNX |
| Disk | +5 GB | Temp files, bisa lebih untuk batch processing. Butuh auto-cleanup ketat. |
| **Total After** | **~1.64 cores** | **~970 MB RAM** | **~6.8 GB disk** |

> **Catatan**: Kalau VPS cuma punya 1-2 cores, processing akan antri. NATS queue handle ini. Untuk production, pastikan CPU ada >2 cores.

### Scalability

```
VPS 1 core:
  - Scanner: ~5-8 detik per page
  - Concurrent: 1 job at a time
  - Antrian: NATS queue buffer unlimited

VPS 4+ core:
  - Scanner: ~2-3 detik per page
  - Concurrent: 4 jobs parallel (1 per worker)
  - Rayon: parallel per-page dalam batch
```

---

## Security Considerations

| Area | Mitigation |
|------|-----------|
| **Upload validation** | MIME type check (whitelist), magic bytes verification, max size 50MB |
| **Path traversal** | Job ID = UUID v4, no user-controlled filenames in storage |
| **Command injection** | No shell commands — semua processing via Rust crates, FFmpeg via crate binding |
| **Temporary files** | Auto-cleanup, random filenames, restricted permissions (0600) |
| **Rate limiting** | Redis sliding window: 30 requests/min/IP per tool, 429 response |
| **CORS** | Origin terbatas ke domain portfolio |
| **Resource exhaustion** | Max image dimension 8000px, max file count per batch 50, worker concurrency limit |
| **OCR data** | Tesseract data dari package manager, no user-trained models |
| **ONNX model** | Model dari source terpercaya, verify checksum |

---

## Rollback Strategy

1. **Image tag**: `tools:sha-<short>` immutable — tinggal update compose file ke tag sebelumnya
2. **Data**: Files auto-expire dalam 1 jam — no persistent data migration needed
3. **Traefik**: Cukup restart, TLS certs ga berubah
4. **Monitor**: Prometheus metrics akan langsung show error rate spike

---

## Development Setup (Local)

Untuk development tanpa Docker:

```bash
# Terminal 1: Redis + NATS
docker compose -f infra/compose/shared.yml -f infra/compose/nats.yml up -d

# Terminal 2: Rust workers
cd apps/tools/backend
REDIS_URL=redis://localhost:6379 NATS_URL=nats://localhost:4222 \
cargo run --bin workers

# Terminal 3: Rust gateway
REDIS_URL=redis://localhost:6379 NATS_URL=nats://localhost:4222 \
TOOLS_STORAGE_PATH=/tmp/tools \
cargo run --bin gateway

# Terminal 4: Next.js
cd apps/tools/frontend
bun dev --port 3002
```

### Test Pipeline Locally (tanpa NATS/Redis)

Untuk development pipeline image processing doang:

```rust
// Di workers/src/scanner/pipeline.rs — test function
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_pipeline() {
        let pipeline = ScanPipeline::default();
        let result = pipeline.process_sync(
            "test_images/scan_miring.jpg",
            ScanOptions { ocr: false, enhance: true }
        );
        assert!(result.is_ok());
        assert!(result.unwrap().output_path.exists());
    }

    #[test]
    fn test_edge_detection_variations() {
        // Test dengan berbagai kondisi: kertas putih, background ramai, sudut ekstrim
        for case in &["normal.jpg", "dark.jpg", "angle45.jpg", "shadow.jpg"] {
            let img = image::open(format!("test_images/{}", case)).unwrap();
            let corners = detect_corners_with_fallback(&img.grayscale().into_luma8());
            assert!(corners.is_ok(), "Failed on: {}", case);
        }
    }
}
```

Test images kumpulin dari foto dokumen real di berbagai kondisi — ini penting buat tuning parameter.
