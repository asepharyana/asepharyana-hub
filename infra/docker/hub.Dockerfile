# ── Build stage ──
FROM oven/bun:1.2 AS builder
WORKDIR /app
COPY apps/hub/package.json apps/hub/bun.lock ./
RUN bun install --frozen-lockfile
COPY apps/hub .
RUN bun run build

# ── Runtime ──
FROM oven/bun:1.2 AS runtime
RUN addgroup -S appgroup && adduser -S appuser -G appgroup
WORKDIR /app
COPY --from=builder /app/.next ./.next
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/public ./public
COPY --from=builder /app/package.json ./
RUN rm -rf .next/cache && chown -R appuser:appgroup .next
USER appuser
EXPOSE 3000
ENV PORT=3000 NODE_ENV=production
CMD ["bun", "run", "start"]
