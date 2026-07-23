# ── Build stage ──
FROM node:22-alpine AS builder
WORKDIR /app
COPY apps/hub/package.json apps/hub/bun.lock ./
RUN npm install --frozen-lockfile
COPY apps/hub .
RUN npm run build

# ── Runtime ──
FROM node:22-alpine AS runtime
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
CMD ["npm", "run", "start"]
