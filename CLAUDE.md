# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Asepharyana Hub is a **hub monorepo** for Asep Haryana Saputra's portfolio ecosystem. Application services live in separate repos imported as Git submodules under `apps/`. Production infrastructure: Caddy reverse proxy + Nix/systemd services (Docker/Traefik removed 2026-08-02; legacy configs under `infra/` marked LEGACY).

```
asepharyana-hub/
├── apps/              # Git submodules — each app is its own repo
│   ├── hub/           # Personal portfolio SPA (asepharyana-hub-hub)
│   └── scraper/       # Rust scraper API (asepharyana-hub-scraper)
├── docs/              # ADRs, deployment guide, new-app guide
├── infra/
│   ├── compose/       # Docker Compose files (LEGACY — Docker dihapus)
│   ├── dapr/          # Dapr config + component definitions
│   ├── docker/        # Dockerfiles (LEGACY)
│   └── traefik/       # Reverse proxy config (static + dynamic)
├── scripts/           # Utility scripts (cleanup, update-deps, git hooks)
└── .github/workflows/ # CI/CD pipelines
```

### Submodule Strategy
- Each app in `apps/` is a separate Git repo imported as a submodule. Code changes happen in the submodule repo, not here.
- Submodule pointers are updated by CI/CD (via `repository_dispatch` or manual commit).
- Current submodules:
  - `apps/hub` → `asepharyana/asepharyana-hub-hub`
  - `apps/scraper` → `asepharyana/asepharyana-hub-scraper`
  - `apps/llm-api` → `asepharyana/asepharyana-hub-llm-api`
  - `apps/tools` → `asepharyana/asepharyana-hub-tools`.

### Infrastructure Stack
- **Caddy 2.11.4** — reverse proxy, TLS termination (auto-LE), HTTP/3, zstd/gzip, keep-alive tuning (`/etc/caddy/Caddyfile`, ref `infra/caddy/Caddyfile.prod`)
- **NATS + JetStream** — message broker with persistent streaming
- **Dapr** — sidecar runtime (pub/sub abstraction, state management, service invocation)
- **Redis (Alpine)** — cache, session store, Dapr state store & pub/sub backend
- **Prometheus** — metrics backend with `file_sd_configs` target files.
- **Jaeger** — distributed tracing backend (all-in-one), OTLP receiver
- **Tailscale** — secure overlay network between VPS nodes (PostgreSQL on `imrnes`, containers on `orangevps`)

### Monitoring
- **Hub dashboard** at `/dashboard` (Next.js client page, auto-refresh 15s)
- **Dashboard API** at `/api/dashboard` — returns JSON with systemd services, Jaeger traces, Prometheus metrics (RPS, latency, errors, node CPU/RAM/Disk)
- **Prometheus** scrapes node-exporter + app metrics endpoints

### Networking
- All services run as Nix/systemd units; inter-service via 127.0.0.1:<port>.
- Caddy handles all external HTTP/S traffic on port 443 (and HTTP/3 UDP).
- Cross-VPS traffic (DB, Redis) goes through Tailscale (`100.64.0.0/10`). Container-to-Tailscale connectivity requires a route in the main routing table (managed by `tailscale-routes.service`).

## Commands

```bash
make init-submodules     # Initialize submodules after clone
make dev                 # Start shared dev infrastructure (Redis)
make update-submodules   # Update all submodules to latest

bun run check            # Biome lint + format + write
bun run ci               # Biome CI mode (no writes, exit code on issues)
bun run format           # Format only
bun run lint             # Lint only

# Nix build (produksi): nix build .#default --impure --option sandbox false
```

### Validate YAML
```bash
python -c "import pathlib, yaml; [yaml.safe_load(open(p)) for p in pathlib.Path('infra').rglob('*.yml')]"
for f in infra/compose/*.yml; do docker compose -f "$f" config >/dev/null && echo "OK $f"; done
```

## CI/CD Workflows

| Workflow | Trigger | Action |
|----------|---------|--------|
| `lint.yml` | PR/push to main touching `*.json`, `*.js`, `biome.json` | `bun run ci` (Biome lint) |
| `deploy.yml` | Push to main | nix build → nix copy ssh:// → systemctl restart |
| `docker-build-push.yml` | LEGACY (Docker dihapus) | LEGACY |
| `security.yml` | PR to main + weekly Monday | CodeQL analysis (Rust) |
| `update-submodule.yml` | `repository_dispatch` | Update submodule pointer in hub repo |

### Deployment Order
1. `shared.yml` (Redis)
2. `nats.yml` (NATS + JetStream)
3. `dapr.yml` (Dapr placement)
4. `traefik.yml` (Reverse proxy)
5. Service compose files (app + Dapr sidecar)

### Secrets Required for Deploy
`SSH_PRIVATE_KEY`, `VPS_HOST`, `VPS_USER`, `VPS_TARGET_DIR`, `ENV_FILE_PRODUCTION`

## Infrastructure Patterns

### Compose File Pattern
Each service gets one compose file. Containers join `app-shared-net` with a `container_name` alias for DNS. The network is declared `external: true`.

### Dapr Sidecar Pattern
Each app gets a companion `daprd` sidecar container. Dapr components (pubsub, statestore) are mounted from `infra/dapr/components/`. The sidecar communicates with NATS for pub/sub and Dapr placement for actor coordination.

### Caddy Routing
- Site blocks in `/etc/caddy/Caddyfile` (ref `infra/caddy/Caddyfile.prod`)
- Subdomain pattern: `<service>.asepharyana.my.id` and `<service>.asepharya.web.id`
- Auto-TLS via Let's Encrypt
- Shared handler snippet `(proxy)`: `encode zstd gzip` + security headers + keep-alive tuning

### Image Tagging
- `sha-<short-sha>` — immutable, for deterministic rollbacks
- `latest` — mutable, for convenience
- Registry: `ghcr.io/asepharyana/asepharyana-hub/<service>`
- Build cache: `sha-<short>-buildcache` (registry-based caching)

## Adding a New Service

1. Create a separate repo for the app code
2. Add as submodule: `git submodule add <url> apps/<name>`
3. Create Nix flake package + systemd unit
4. Create compose file in `infra/compose/` (app + Dapr sidecar)
5. Add Caddy site block in `/etc/caddy/Caddyfile`
6. Add build job in `.github/workflows/docker-build-push.yml`
7. See `docs/add-new-app.md` for full guide

## Commit Convention

Format: `<type>(<scope>): <description>`

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `style`
Scopes: `scraper`, `infra`, `ci`, `dapr`, `nats`, `docs`, `deps`, `scripts`, `root`

Scope is required. Use imperative mood. No period at end of subject line. Co-Authored-By footer for AI-generated commits.
