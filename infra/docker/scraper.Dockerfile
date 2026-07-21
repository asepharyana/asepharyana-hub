# ── Build stage: cargo-chef for dependency caching ──
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY apps/scraper .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

COPY apps/scraper .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/scraper /app/scraper

# ── Runtime image ──
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 1001 appgroup && \
    useradd -u 1001 -g appgroup -s /bin/sh appuser

WORKDIR /app
COPY --from=builder /app/scraper /app/scraper
USER appuser

EXPOSE 4091
CMD ["./scraper"]
