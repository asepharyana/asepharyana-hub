# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

Asepharyana Hub is a **hub monorepo** for Asep Haryana Saputra's portfolio ecosystem. Application services live in separate repos imported as Git submodules under `apps/`. Infrastructure (Docker Compose, Traefik, Dapr) lives in `infra/`.

```
asepharyana-hub/
├── apps/              # Git submodules — each app is its own repo
│   └── scraper/       # Rust scraper API (asepharyana-hub-scraper)
├── docs/              # ADRs, deployment guide, new-app guide
├── infra/
│   ├── compose/       # One Docker Compose file per service
│   ├── dapr/          # Dapr config + component definitions
│   ├── docker/        # Dockerfiles per service
│   └── traefik/       # Reverse proxy config (static + dynamic)
├── scripts/           # Utility scripts (cleanup, update-deps, git hooks)
└── .github/workflows/ # CI/CD pipelines
```

### Submodule Strategy
- Each app in `apps/` is a separate Git repo imported as a submodule. Code changes happen in the submodule repo, not here.
- Submodule pointers are updated by CI/CD (via `repository_dispatch` or manual commit).
- Current submodule: `apps/scraper` → `asepharyana/asepharyana-hub-scraper`.

### Infrastructure Stack
- **Traefik v3.6** — reverse proxy, TLS termination, middleware chain (rate-limit, headers, buffer, block-sensitive-paths)
- **NATS + JetStream** — message broker with persistent streaming
- **Dapr** — sidecar runtime (pub/sub abstraction, state management, service invocation)
- **Redis (Alpine)** — cache, session store, Dapr state store & pub/sub backend
- **Tailscale** — secure overlay network between VPS nodes (PostgreSQL on `imrnes`, containers on `orangevps`)

### Networking
- All containers join `app-shared-net` (external Docker bridge network). Service discovery via Docker DNS (container name aliases).
- Traefik handles all external HTTP/S traffic on port 443.
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

docker build -f infra/docker/scraper.Dockerfile -t scraper-api:latest .  # Build image
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
| `docker-build-push.yml` | Push to main touching `apps/**`/`infra/**`, or `repository_dispatch` | Build Docker images per changed service, push to GHCR, update compose manifests |
| `deploy-docker.yml` | After build completes, or push touching `infra/**` | SSH to VPS (orangevps), pull images, restart containers selectively |
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

### Traefik Routing
- Routers + services defined in `infra/traefik/dynamic/apps.yaml`
- Subdomain pattern: `<service>.asepharyana.my.id` and `<service>.asepharya.web.id`
- TLS certs from volume mounts (not auto-ACME)
- Middleware chain: `secure-headers` → `compress` → `retry` → `rate-limit` → `buffer`

### Image Tagging
- `sha-<short-sha>` — immutable, for deterministic rollbacks
- `latest` — mutable, for convenience
- Registry: `ghcr.io/asepharyana/asepharyana-hub/<service>`
- Build cache: `sha-<short>-buildcache` (registry-based caching)

## Adding a New Service

1. Create a separate repo for the app code
2. Add as submodule: `git submodule add <url> apps/<name>`
3. Create Dockerfile in `infra/docker/`
4. Create compose file in `infra/compose/` (app + Dapr sidecar)
5. Add Traefik router in `infra/traefik/dynamic/apps.yaml`
6. Add build job in `.github/workflows/docker-build-push.yml`
7. See `docs/add-new-app.md` for full guide

## Commit Convention

Format: `<type>(<scope>): <description>`

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `style`
Scopes: `scraper`, `infra`, `ci`, `dapr`, `nats`, `docs`, `deps`, `scripts`, `root`

Scope is required. Use imperative mood. No period at end of subject line. Co-Authored-By footer for AI-generated commits.
