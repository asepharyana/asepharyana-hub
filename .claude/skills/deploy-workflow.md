---
name: deploy-workflow
description: Panduan deploy, CI/CD, dan Nix/systemd patterns untuk Asepharyana Hub
---

# Deploy & Workflow — Asepharyana Hub

## CI/CD Pipeline

### Build Pipeline (`docker-build-push.yml`)
Trigger: push ke `main` yang touch `apps/**`, `infra/**`, `infra/docker/**`

1. **changes** — detect service mana yg berubah via git diff
2. **wait-submodule-ref** — (repository_dispatch only) tunggu SHA commit fetchable
3. **build** — matrix build per service, push ke GHCR (`sha-<short>` + `latest`)
4. **update-manifest** — update image tag di compose file, commit + push

### Deploy Pipeline (`deploy-docker.yml`)
Trigger: build selesai, atau push ke `main` touch `infra/**`

1. SSH ke `orangevps` (via `secrets.VPS_HOST`)
2. Sync repo (`git fetch --depth=1 + reset`)
3. Login ke GHCR
4. Deteksi compose file yg berubah
5. Pull images + restart container selektif

### Secrets Required
| Secret | Untuk |
|--------|-------|
| `SSH_PRIVATE_KEY` | SSH ke VPS |
| `VPS_HOST` | IP/host VPS (tailscale IP) |
| `VPS_USER` | SSH user, biasanya `root` |
| `VPS_TARGET_DIR` | Lokasi repo di VPS |
| `ENV_FILE_PRODUCTION` | .env content untuk production |

### Selective Deployment
- Hanya compose file yg berubah yang di-redeploy
- Selective: `UP_FLAGS="-d"` (tanpa `--remove-orphans`)
- Full deploy: `UP_FLAGS="-d --remove-orphans"`

## Docker Patterns

### Build dengan cargo-chef (Rust)
```dockerfile
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0 AS chef
WORKDIR /app
FROM chef AS planner
COPY apps/scraper .
RUN cargo chef prepare --recipe-path recipe.json
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY apps/scraper .
RUN cargo build --release
```

### Runtime minimal untuk Rust binary
```dockerfile
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl libssl3 && rm -rf /var/lib/apt/lists/*
```

## Image Tagging
- `sha-<short-sha>` — immutable, untuk rollback
- `latest` — mutable, untuk convenience
- Build cache: `sha-<short>-buildcache`
- Registry: `ghcr.io/asepharyana/asepharyana-hub/<service>`

## Manual Deploy Steps
```bash
# 1. Login GHCR
echo $GITHUB_TOKEN | docker login ghcr.io -u asepharyana --password-stdin

# 2. Full stack
docker compose -f infra/compose/traefik.yml \
  -f infra/compose/shared.yml \
  -f infra/compose/nats.yml \
  -f infra/compose/dapr.yml \
  -f infra/compose/scraper.yml \
  --env-file .env up -d --remove-orphans

# 3. Selective (hanya satu service)
docker compose -f infra/compose/scraper.yml --env-file .env up -d
```

## Troubleshooting

### Container reach Tailscale
Pastikan route ke Tailscale di main table:
```bash
ip route add 100.64.0.0/10 dev tailscale0 table main
systemctl restart tailscale-routes
```

### Healthcheck gagal di scratch images
NATS dan Dapr placement pake scratch — tidak bisa healthcheck. Cukup `service_started` di depends_on.

### Dapr sidecar crash
```bash
docker logs scraper-api-dapr | grep -iE "fatal|error"
```
Penyebab umum: komponen config salah, NATS/Dapr placement belum siap.
