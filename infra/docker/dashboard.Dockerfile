# ── Build stage ──
FROM golang:1.24-alpine AS builder
WORKDIR /build
COPY infra/dashboard/ ./
RUN CGO_ENABLED=0 go build -ldflags="-s -w" -o dashboard .

# ── Runtime (scratch — ~7MB total) ──
FROM scratch
COPY --from=builder /build/dashboard /dashboard
EXPOSE 8080
CMD ["/dashboard"]
