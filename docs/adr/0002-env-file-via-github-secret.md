# ADR 0002: Production `.env` via GitHub Encrypted Secret

> **LEGACY (2026-08-28):** Repo `asepharyana-hub` sudah dirombak → `asepharyana/infra`.
> Workflow lama yang SCP `.env` ke VPS tidak dipakai lagi (deploy app pindah ke repo masing-masing,
> secrets via Bitwarden `bws-exec`). ADR ini dipertahankan sebagai arsip.

## Status

Accepted (archived)

## Context

The project runs on a remote VPS (`orangevps`, IP `45.127.35.244`) that hosts multiple services via Docker Compose. These services require environment variables (database credentials, API keys, tokens) that must not be committed to the repository.

The production `.env` file on the VPS is **not** a copy of the committed `.env` in the repo root — it contains additional secrets (Portainer tokens, Discord bot tokens, etc.) that only exist in production.

Previously, the `.env` file on the VPS was edited manually via SSH, which led to drift between the local `.env` and the production `.env`. When the database server IP or port changed in the local `.env`, the production `.env` was not updated, causing service outages.

## Decision

The production `.env` file is stored as a **GitHub Actions encrypted secret** named `ENV_FILE_PRODUCTION`. During deployment, the `.github/workflows/deploy-docker.yml` workflow writes this secret to a file and SCPs it to the VPS.

### Flow

```
GitHub Secret (ENV_FILE_PRODUCTION)
       │
       ▼ (deploy-docker.yml)
echo "$ENV_FILE_PRODUCTION" > .env.prod
scp .env.prod  →  VPS:$VPS_TARGET_DIR/.env
       │
       ▼ (docker compose --env-file .env up)
Container reads $DATABASE_URL, $JWT_SECRET, etc.
```

### How to update

```bash
# 1. Read current content from the VPS
ssh root@45.127.35.244 "cat <VPS app dir, e.g. /home/code/hub>/.env"

# 2. Pipe updated content to the GitHub secret
#    (requires gh CLI with repo access)
cat /path/to/updated-env | gh secret set ENV_FILE_PRODUCTION --repo asepharyana/asepharyana-hub

# 3. Trigger a redeploy to push it to the VPS
gh workflow run deploy-docker.yml

# OR apply immediately on the VPS (for hotfix):
ssh root@45.127.35.244 "sed -i 's|OLD_VALUE|NEW_VALUE|' <VPS app dir, e.g. /home/code/hub>/.env"
# Then restart affected containers
```

## Server Topology

| Host | IP | Role |
|------|----|------|
| `orangevps` (VPS) | `45.127.35.244` | Docker host: Traefik, scraper-api, Redis, NATS, Dapr |
| `imrnes` (bare-metal) | `100.121.180.82` (Tailscale) | PostgreSQL (port 6432), Redis (port 6379), Browserless |

## Database

| Variable | Value |
|----------|-------|
| `DATABASE_URL` | `postgres://asephs:hunterz@100.121.180.82:6432/hub` |
| `REDIS_URL` | `redis://redis:6379` (Docker network, overridden per-service) |
| `EXTERNAL_BROWSERLESS_WS` | `ws://43.134.105.109:3001/?token=...` (external proxy) |

> **Important:** The Docker Compose `environment:` section uses variable interpolation (`${DATABASE_URL}`), which is resolved from the `--env-file .env` at compose time — NOT from the service's `env_file`. Both must be kept in sync.

## Docker Compose Project Structure

The VPS runs a single Docker Compose project named `compose` composed of multiple files:

```bash
<VPS app dir, e.g. /home/code/hub>/infra/compose/
├── traefik.yml         # Reverse proxy (TLS termination, routing)
├── shared.yml          # Redis
├── nats.yml            # NATS message broker + JetStream
├── dapr.yml            # Dapr placement service
├── scraper.yml         # Scraper API + Dapr sidecar
└── observability.yml   # OTel Collector + Jaeger + Dashboard
```

All files are always included together for dependency resolution:

```bash
docker compose \
  --env-file .env \
  -f infra/compose/traefik.yml \
  -f infra/compose/shared.yml \
  -f infra/compose/scraper.yml \
  -f infra/compose/nats.yml \
  -f infra/compose/dapr.yml \
  -f infra/compose/observability.yml \
  up -d
```

## GitHub Secrets Required

| Secret | Description |
|--------|-------------|
| `SSH_PRIVATE_KEY` | SSH key for VPS access |
| `VPS_HOST` | `45.127.35.244` |
| `VPS_USER` | `root` |
| `VPS_TARGET_DIR` | `<VPS app dir, e.g. /home/code/hub>` |
| `ENV_FILE_PRODUCTION` | Full `.env` content for production |

## Consequences

### Positive

- Environment is version-controlled via GitHub Secrets audit log.
- No risk of committing secrets to the repo.
- Deployment is fully automated — `.env` is pushed on every deploy.
- Easy to rotate secrets: update `ENV_FILE_PRODUCTION` and redeploy.

### Negative

- The secret is opaque — you cannot diff it or review changes via PR.
- If the secret falls out of sync with the local `.env`, services silently break on next deploy.
- Requires `gh` CLI or GitHub UI to update — not a simple file edit.

### Mitigations

- Keep the **committed `.env`** in the repo root as the source of truth for non-secret values (database URL, ports, API endpoints).
- Document any manual SSH hotfix at the same time as updating the GitHub secret.
- Run `gh secret set ENV_FILE_PRODUCTION` with the latest server `.env` content after any hotfix.
