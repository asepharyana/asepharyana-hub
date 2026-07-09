# Use cargo-chef for dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /app

FROM chef AS planner
COPY apps/rust-auth .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Utilize buildkit cache mounts for cargo
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo chef cook --release --recipe-path recipe.json

# Build application
COPY apps/rust-auth .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    cp target/release/rust-auth /app/rust-auth

# Final runtime image
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Add non-root user
RUN groupadd -g 1001 appgroup && \
    useradd -u 1001 -g appgroup -s /bin/sh appuser

WORKDIR /app
COPY --from=builder /app/rust-auth /app/rust-auth

# Run as non-root
USER appuser

EXPOSE 3000
CMD ["./rust-auth"]
