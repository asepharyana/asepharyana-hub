# ============================================================
# Stage 1: Build Rust Backend (with cargo-chef caching)
# ============================================================
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY apps/tools/backend/ .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY apps/tools/backend/ .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin tools-gateway --bin tools-workers && \
    cp /app/target/release/tools-gateway /app/tools-gateway-bin && \
    cp /app/target/release/tools-workers /app/tools-workers-bin

# ============================================================
# Stage 2: Build Next.js Frontend
# ============================================================
FROM oven/bun:1.3 AS frontend-builder
WORKDIR /app

# Copy package files first for layer caching
COPY apps/tools/frontend/package.json apps/tools/frontend/bun.lock ./
RUN bun install --frozen-lockfile

COPY apps/tools/frontend/ .
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

# Copy Rust binaries (cp'd from cache mount in builder stage)
COPY --from=builder /app/tools-gateway-bin /app/gateway
COPY --from=builder /app/tools-workers-bin /app/workers

# Copy Next.js build
COPY --from=frontend-builder /app/.next /app/.next
COPY --from=frontend-builder /app/public /app/public
COPY --from=frontend-builder /app/package.json /app/package.json
COPY --from=frontend-builder /app/node_modules /app/node_modules
COPY --from=frontend-builder /app/next.config.ts /app/next.config.ts

# Create temp storage directory
RUN mkdir -p /data/tools && chmod 1777 /data/tools

# Copy entrypoint
COPY apps/tools/scripts/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# Environment
ENV TESSDATA_PREFIX=/usr/share/tesseract-ocr/5/tessdata
ENV STORAGE_PATH=/data/tools
ENV GATEWAY_PORT=3001
ENV TOOLS_WORKER_CONCURRENCY=4
ENV RUST_LOG=info

# Expose port
EXPOSE 3001

CMD ["/app/entrypoint.sh"]