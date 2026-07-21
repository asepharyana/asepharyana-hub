# Architecture

## Hub Repository Structure Overview

```
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

| Service   | Language/Runtime | Framework | Database | Key Libraries |
| --------- | ---------------- | --------- | -------- | ------------- |
| **scraper** | _(submodule)_  | —         | —        | —             |

### Infrastructure

| Component          | Technology              | Purpose                                                          |
| ------------------ | ----------------------- | ---------------------------------------------------------------- |
| Reverse Proxy      | Traefik v3.6            | TLS termination, routing, middleware (rate-limit, headers, auth) |
| Container Runtime  | Docker + Docker Compose | Service isolation and orchestration                              |
| Container Registry | GHCR (ghcr.io)          | Docker image storage                                             |
| Networking         | Tailscale               | Secure overlay network between VPS nodes                         |
| Message Bus        | NATS + JetStream        | Event-driven pub/sub, job queues, streaming                      |
| Runtime Sidecar    | Dapr                    | Service invocation, pub/sub abstraction, state management        |
| Cache & State      | Redis (Alpine)          | Session store, rate limit counters, caching, Dapr state store    |
| CI/CD              | GitHub Actions          | Build, test, deploy automation                                   |

## Infrastructure

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

```mermaid
graph TB
    subgraph "Tailnet (100.64.0.0/10)"
        IMRNES["imrnes (100.121.180.82)"]
        ORANGEVPS["orangevps (100.79.111.61)"]
        ARCH["archlinux (100.84.39.83)"]
    end

    subgraph "imrnes Services"
        PG[(PostgreSQL)]
        REDIS[Redis]
    end

    subgraph "orangevps Containers"
        TRAEFIK[Traefik :443]
        SCRAPER[scraper-api :4091]
    end

    TRAEFIK --> SCRAPER

    style IMRNES fill:#3a7,color:#fff
    style ORANGEVPS fill:#37a,color:#fff
    style ARCH fill:#773,color:#fff
```

Container-to-Tailscale connectivity requires a systemd service that adds a route to the main routing table:

```
ip route add 100.64.0.0/10 dev tailscale0 table main
```

This is managed by `/etc/systemd/system/tailscale-routes.service` on the `orangevps` VPS.

## Data Flow

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

## Deployment Architecture

### Image Tags

- Every push to `main` triggers Docker builds for changed services
- Images are tagged with both `latest` and `sha-<short-sha>` (e.g., `sha-b0ef947`)
- Compose files are auto-updated to pin the new SHA tag
- This enables deterministic rollbacks by reverting the compose file change

### VPS Deployment

The `orangevps` VPS (Tailscale `100.79.111.61`) hosts all application containers:

1. GitHub Actions SSHes into the VPS
2. Production secrets are written as `.env`
3. The repo is synchronized via `git pull`
4. Changed compose files are detected by `git diff`
5. Docker images are pulled (with retry logic for transient failures)
6. Old containers are removed by `container_name`
7. `docker compose up -d` brings up the new containers
8. Traefik automatically detects the new containers via Docker provider

### Selective Deployment

The deploy workflow supports selective updates — if only one compose file changed, only the corresponding service is pulled and recreated, avoiding disruption to other services.

```mermaid
graph TB
    subgraph "Orange VPS"
        DIR[/root/asepharyana-hub/]
        ENV[.env]
        COMPOSE[infra/compose/*.yml]
        NET[app-shared-net]

        DIR -->|git pull| COMPOSE
        ENV -->|docker compose --env-file| COMPOSE
        COMPOSE -->|docker compose pull| IMAGES[(GHCR Images)]
        COMPOSE -->|docker compose up -d| CONT[Containers]
        CONT --> NET
    end

    subgraph "GitHub Actions"
        BUILD[Build & Push]
        DEPLOY[Deploy Workflow]
        BUILD -->|trigger| DEPLOY
        DEPLOY -->|SSH| DIR
    end

    IMAGES -->|registry| GHCR[ghcr.io/asepharyana]
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

## Service Mesh & Inter-Service Communication

### HTTP (External + Internal via Traefik)
External traffic and internal HTTP calls route through Traefik. Services on `app-shared-net` can also communicate directly by container name.

### Event-Driven (NATS + Dapr)
NATS with JetStream provides a persistent message backbone. Each service has a Dapr sidecar that abstracts pub/sub, service invocation, and state management.

```mermaid
graph TB
    subgraph "External"
        WWW[Internet]
    end

    subgraph "Orange VPS"
        TRAEFIK[Traefik :443]
        
        subgraph "app-shared-net"
            NATS[NATS + JetStream<br/>:4222]
            DAPR_PLACEMENT[Dapr Placement<br/>:50005]

            subgraph "Service: scraper-api"
                SCRAPER[scraper-api<br/>:4091]
                DAPR_SIDECAR[Dapr Sidecar<br/>:3500]
                SCRAPER --- DAPR_SIDECAR
            end
        end

        DAPR_SIDECAR -.->|gRPC pub/sub| NATS
        DAPR_SIDECAR -.->|placement| DAPR_PLACEMENT
    end

    WWW -->|HTTPS| TRAEFIK
    TRAEFIK --> SCRAPER
```

### Communication Patterns

| Pattern | Mechanism | Use Case |
|---------|-----------|----------|
| External HTTP | Traefik → Service | User requests, API calls |
| Internal HTTP | Service → Service (via Traefik or direct) | Synchronous queries |
| Pub/Sub Event | Dapr sidecar → NATS JetStream | Async notifications, image cache events |
| Service Invocation | Dapr sidecar gRPC | Cross-service RPC with retry & observability |
| State Store | Dapr → Redis | Shared state, job progress |

## Observability

- **Traefik access logs**: JSON format, logged at INFO level
- **Traefik access logs**: JSON format, logged at INFO level
- **Dashboard**: Traefik dashboard at `traefik.asepharyana.my.id` (secured)
