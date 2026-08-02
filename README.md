# Architecture

## Hub Repository Structure Overview

```diff
asepharyana-hub/
├── apps/                    # Application services (Git submodules)
│   └── scraper/            # Web scraper service
├── docs/                    # Documentation
│   ├── adr/                # Architecture Decision Records
│   ├── add-new-app.md      # Guide for adding new services
│   └── superpowers/        # Project capabilities tracking
├── infra/                   # Infrastructure as code
│   ├── compose/            # Docker Compose files per service
│   ├── config/             # Infrastructure configuration
│   ├── docker/             # Dockerfiles per service
│   └── traefik/            # Traefik reverse proxy config
│       └── dynamic/        # Dynamic routing rules (YAML)
├── scripts/                 # Utility scripts
│   ├── git-hooks/          # Git hook scripts
│   ├── cleanup-ghcr.sh     # GHCR image cleanup
│   └── update-deps.sh      # Dependency update helper
├── .github/workflows/       # CI/CD pipelines
├── eslint.config.mjs        # Root ESLint config
├── package.json             # Root formatting/lint helper scripts
└── .prettierrc              # Prettier formatting rules
```

## Technology Stack

### Services

|| Service   | Path           | Language/Runtime | Framework | Database | Key Libraries |
||---------|----------------|------------------|-----------|----------|---------------|
|| **scraper** | `apps/scraper` | — | — | — | — |

### Infrastructure

|| Component          | Technology              | Purpose                                                          |
||---------------------|-------------------------|------------------------------------------------------------------|
|| Reverse Proxy       | Traefik v3.6            | TLS termination, routing, middleware (rate-limit, headers, auth) |
|| Container Runtime   | Docker + Docker Compose | Service isolation and orchestration                              |
|| Container Registry  | GHCR (ghcr.io)          | Docker image storage                                             |
|| Networking          | Tailscale               | Secure overlay network between VPS nodes                         |
|| Message Bus         | NATS + JetStream        | Event-driven pub/sub, job queues, streaming                      |
|| Runtime Sidecar     | Dapr                    | Service invocation, pub/sub abstraction, state management        |
|| Cache & State       | Redis (Alpine)          | Session store, rate limit counters, caching, Dapr state store    |
|| CI/CD               | GitHub Actions          | Build, test, deploy automation                                   |

### Infrastructure

### Traefik Reverse Proxy

Traefik runs as the entry point for all HTTP/S traffic. It is configured via:

- **Static config**: CLI arguments in `infra/compose/traefik.yml` — entry points, providers, plugins
- **Dynamic config**: `infra/traefik/dynamic/` — routers, services, middlewares, TLS
- **Docker provider**: Auto-discovers containers with `traefik.enable=true` labels
- **File provider**: Loads `apps.yaml` (routers/services), `middlewares.yaml`, `ssl.yaml`

Key middleware chains (`infra/traefik/dynamic/middlewares.yaml`):

- `secure-headers` — SSL redirect, HSTS, XSS protection, CSP
- `compress` — Gzip compression for responses over 256 bytes
- `rate-limit` — 100 avg / 50 burst requests
- `buffer` — 10MB request/response body limit
- `block-sensitive-paths` — blocks `.env`, `.git`, `/wp-admin` etc.
- `common-chain` — composes secure-headers + compress + retry + rate-limit + buffer

All services route through Traefik on port 443 (TLS), with automatic HTTP-to-HTTPS redirect.

### Docker Compose

Each service has its own Compose file under `infra/compose/`. All services join the `app-shared-net` external Docker network, enabling inter-service communication by container name.

Shared services:

- `infra/compose/shared.yml` — Redis (alias: `redis`)
- `infra/compose/traefik.yml` — Traefik reverse proxy

Service compose files are combined during deployment:

```bash
docker compose -f traefik.yml -f shared.yml -f scraper.yml up -d
```

### Tailscale Networking

### Arsitektur

Semua VPS terhubung via **Tailscale**. Setiap VPS punya IP Tailscale dan service berkomunikasi antar VPS melalui Tailscale network (`100.64.0.0/10`). Container-to-Tailscale connectivity requires a systemd service that adds a route to the main routing table:

```bash
ip route add 100.64.0.0/10 dev tailscale0 table main
```

This is managed by `/etc/systemd/system/tailscale-routes.service` on the `orangevps` VPS.

### Data Flow

### Request Flow (Production)

```mermaid
sequenceDiagram
    participant User as Browser/Client
    participant DNS as Cloudflare DNS
    participant Traefik as Traefik Proxy
    participant App as Application Container
    participant DB as PostgreSQL (imrnes via Tailscale)
    participant Redis as Redis (imrnes via Tailscale)

    User->>DNS: asepharyana.my.id
    DNS->>User: A/AAAA record → orangevps VPS IP
    User->>Traefik: HTTPS request :443
    Traefik->>Traefik: TLS termination
    Traefik->>Traefik: Middleware chain (headers, rate-limit, buffer)
    Traefik->>App: HTTP reverse-proxy (internal network)

    alt Database query
        App->>DB: sqlx/Drizzle query via Tailscale
        DB-->>App: Result set
    else Cache lookup
        App->>Cache: GET/SET via Tailscale
        Cache-->>App: Cached value
    end

    App-->>Traefik: HTTP response
    Traefik-->>User: HTTPS response
```

### CI/CD Pipeline

```mermaid
flowchart LR
    A[Push to main] --> B{Changed paths?}
    B -->|apps/** or infra/docker/**| C[Build Docker Images]
    B -->|infra/compose/**| D[Deploy to VPS]
    B -->|apps/*/src/**/*.ts| E[Lint + TypeCheck]

    C --> F[Push to GHCR]
    F --> G[Update Compose tags]
    G --> D

    D --> H[SSH into VPS]
    H --> I[Pull images]
    I --> J[docker compose up -d]

    subgraph "Build Phase"
        C
        F
        G
    end

    subgraph "Deploy Phase"
        D
        H
        I
        J
    end
```

### Deployment Architecture

### Image Tags

- `latest` — mutable, for convenience
- `sha-<short-sha>` — immutable, for deterministic rollbacks
- Build cache: `sha-<short>-buildcache`

Registry: `ghcr.io/asepharyana/asepharyana-hub/<service>`

## Deployment Notes

- Pipeline memakai image tag berbasis commit SHA (`sha-<short-sha>`), bukan `latest`.
- Deploy Compose sekarang mencakup `infra/compose/*.yml` dan `deploy-docker.yml` akan berjalan langsung ketika `infra/compose/**` berubah.
- Selective deployment: hanya compose file yg berubah yang di-redeploy.

## Networking & Tailscale

### Arsitektur

Semua VPS terhubung via **Tailscale**. Setiap VPS punya IP Tailscale dan service berkomunikasi antar VPS melalui Tailscale network (`100.64.0.0/10`). Container-to-Tailscale connectivity requires a systemd service that adds a route to the main routing table:

```bash
ip route add 100.64.0.0/10 dev tailscale0 table main
```

This is managed by `/etc/systemd/system/tailscale-routes.service` on the `orangevps` VPS.

### Environment Variables

Service yang connect ke Tailscale IP:

```env
# PostgreSQL di imrnes
DATABASE_URL=postgres://user:***@100.121.180.82:6432/dbname

# Redis di imrnes
REDIS_URL=redis://100.121.180.82:6379
```

## Submodule Strategy

Each application lives in its own Git repository and is imported as a submodule into `apps/`. This approach:

- **Enables independent development** — each service can be developed, tested, and versioned separately
- **Pins exact commits** — the super-repository tracks exact submodule SHAs, enabling reproducible deployments
- **Supports `repository_dispatch`** — when a submodule receives a push, it can trigger the super-repository to build and deploy only that service

### Submodule Lifecycle

1. Developer pushes to a submodule (e.g., `apps/scraper`)
2. Submodule's GitHub Action dispatches `repository_dispatch` to the super-repo with the service name and new SHA
3. Super-repo detects the dispatch, waits for the SHA to be fetchable, then builds only that service
4. The compose manifest is updated and committed with the new SHA tag
5. The deploy workflow runs and updates only the changed containers

### Updating Submodules

```bash
# Update a single submodule to latest
cd apps/scraper
git checkout main
git pull
cd ../..
git add apps/scraper
git commit -m "chore(scraper): update submodule to latest"
```

## License

MIT