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
RUN cargo build --release --bin tools-gateway --bin tools-workers

# ============================================================
# Stage 2: Build Next.js Frontend
# ============================================================
FROM oven/bun:1.3 AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/bun.lock ./
RUN bun install --frozen-lockfile || bun install
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
COPY --from=builder /app/target/release/tools-gateway /app/gateway
COPY --from=builder /app/target/release/tools-workers /app/workers

# Copy Next.js build
COPY --from=frontend-builder /app/.next /app/.next
COPY --from=frontend-builder /app/public /app/public
COPY --from=frontend-builder /app/package.json /app/package.json
COPY --from=frontend-builder /app/node_modules /app/node_modules
COPY --from=frontend-builder /app/next.config.ts /app/next.config.ts

# Create temp storage directory
RUN mkdir -p /data/tools && chmod 1777 /data/tools

# Environment
ENV TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
ENV STORAGE_PATH=/data/tools
ENV GATEWAY_PORT=3001
ENV TOOLS_WORKER_CONCURRENCY=4
ENV RUST_LOG=info

# Expose port
EXPOSE 3001

# Copy entrypoint
COPY scripts/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

CMD ["/app/entrypoint.sh"]