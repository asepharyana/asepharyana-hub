# ─── Stage 1: Build ─────────────────────────────────────────────────────────
FROM oven/bun:1 AS builder
WORKDIR /app
COPY apps/react/package.json apps/react/bun.lock ./
RUN bun install --frozen-lockfile
COPY apps/react .
RUN bun run build

# ─── Stage 2: Runtime (Bun static server) ──────────────────────────────────
FROM oven/bun:1-alpine
WORKDIR /app
COPY infra/docker/react-server.js ./server.js
COPY --from=builder /app/dist ./dist

EXPOSE 80
CMD ["bun", "server.js"]