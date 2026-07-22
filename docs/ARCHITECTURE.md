# Arsitektur asepharyana-hub

## Topologi Fisik

Dua node terhubung via **Tailscale** overlay network:

```
┌──────────────────────────────┐       ┌──────────────────────────────┐
│         orangevps (VPS)       │       │        imrnes (Bare-metal)   │
│  IP: 45.127.35.244           │       │  Tailscale: 100.121.180.82   │
│  Tailscale: 100.x.x.x        │◄──────┤                              │
│                              │       │  Layanan:                    │
│  Layanan:                    │       │  ├─ PostgreSQL (port 6432)   │
│  ├─ Traefik (port 80/443)    │       │  └─ Redis (port 6379)        │
│  ├─ NATS + JetStream         │       │                              │
│  ├─ Dapr Placement           │       └──────────────────────────────┘
│  ├─ Redis (cache, Dapr)      │
│  ├─ Scraper API + Dapr       │
│  └─ Hub (Next.js SPA)       │
└──────────────────────────────┘
```

### Konektivitas Container ke Tailscale

Container di `orangevps` tidak bisa langsung mencapai IP Tailscale (`100.x.x.x`). Route Tailscale harus ditambahkan ke tabel routing utama (`main`) via `tailscale-routes.service` agar traffic dari container bisa melewati host ke Tailscale.

## Alur Request HTTP (External)

```
Internet
   │
   ▼ Port 443
Traefik (v3.6)
   ├─ TLS termination (sertifikat dari volume mount)
   ├─ Middleware chain: secure-headers → compress → retry → rate-limit → buffer
   ├─ Plugin: real-ip (Cloudflare), block-sensitive-paths
   │
   ▼ Router matching
Host(`scraper.asepharyana.my.id`) || Host(`api.asepharyana.my.id') → scraper-api
Host(`hub.asepharyana.my.id`) → hub
   │
   ▼ Service load balancer
http://scraper-api:4091
   │
   ▼
Scraper API (Rust / Axum)
   ├─ Health check: GET /, respon 200
   ├─ REST endpoints
   ├─ Database via `DATABASE_URL` (Tailscale → PostgreSQL di imrnes)
   ├─ Cache via `REDIS_URL` (Redis lokal di container)
   └─ Pub/sub via Dapr sidecar (localhost:3500)
```

## Infrastruktur Internal

### Docker Compose Project

Semua service berjalan dalam satu Docker Compose project bernama `compose` dan bergabung di network `app-shared-net`:

| File | Service | Peran |
|------|---------|-------|
| `traefik.yml` | `traefik` | Reverse proxy, TLS termination, middleware |
| `shared.yml` | `redis` | Cache, session store, backend Dapr pub/sub & state |
| `nats.yml` | `nats` | Message broker + JetStream persistent streaming |
| `dapr.yml` | `dapr-placement` | Koordinasi actor placement untuk sidecar Dapr |
| `scraper.yml` | `scraper-api` + `scraper-api-dapr` | Aplikasi Rust + sidecar Dapr |
| `hub.yml` | `hub` | Next.js SPA portfolio |

### Dapr Sidecar Pattern

Setiap aplikasi yang menggunakan Dapr mendapat sidecar container `daprd`:

```
┌─────────────────────┐
│  scraper-api        │
│  (app port 4091)    │
└────────┬────────────┘
         │ localhost:3500 (HTTP)
         │ localhost:50001 (gRPC)
┌────────▼────────────┐
│  scraper-api-dapr   │
│  (daprd sidecar)    │
│                     │
│  Dapr components:   │
│  ├─ pubsub.redis    │
│  └─ state.redis     │
└─────────────────────┘
```

Komponen Dapr:

| Komponen | Tipe | Backend |
|----------|------|---------|
| `pubsub` | `pubsub.redis` | `redis:6379` |
| `statestore` | `state.redis` | `redis:6379` (prefix `dapr`) |

### NATS + JetStream

NATS berjalan dengan flag `-js` untuk mengaktifkan JetStream. Persistent stream disimpan di volume `nats_data`. Dapr pub/sub routing:

```
Service → Dapr sidecar (pubsub.redis) → Redis streams
```

> **Catatan:** Saat ini Dapr pub/sub menggunakan Redis, bukan NATS. Jika ingin migrasi ke NATS untuk pub/sub, komponen Dapr perlu diganti dengan `pubsub.nats`.

## Arsitektur CI/CD

```
Push ke main (apps/**, infra/**)
        │
        ▼
docker-build-push.yml
   ├─ Phase 1: Detect changed services
   ├─ Phase 2: Build & Push image ke GHCR
   └─ Phase 3: Update compose manifest + submodule pointer
        │
        ▼ (workflow_run trigger)
deploy-docker.yml
   ├─ SSH ke orangevps
   ├─ Git sync, pull images
   ├─ Remove stale containers
   └─ Selective restart service
```

Submodule update dari remote repo via `repository_dispatch`:

```
Push ke asepharyana-hub-scraper
        │
        ▼ (repository_dispatch)
update-submodule.yml
   ├─ Update submodule pointer
   └─ Commit & push ke hub repo
        │
        ▼ (repository_dispatch trigger)
docker-build-push.yml
   └─ Build, push, deploy
```

## Image Tagging Strategy

| Tag | Contoh | Penggunaan |
|-----|--------|------------|
| `sha-<short>` | `sha-a3c5d74` | Immutable, deterministic rollback |
| `latest` | `latest` | Mutable, convenience |
| `buildcache` | `sha-a3c5d74-buildcache` | Registry-based build cache (internal) |

## Networking

### Port Map

| Port | Service | Deskripsi |
|------|---------|-----------|
| 443 | Traefik | HTTPS eksternal |
| 80 | Traefik | Redirect ke HTTPS |
| 4222 | NATS | Client connections |
| 8222 | NATS | HTTP monitor / health |
| 6379 | Redis | Internal container network |
| 3500 | Dapr sidecar | Dapr HTTP API (per service) |
| 50001 | Dapr sidecar | Dapr gRPC API (per service) |
| 50005 | Dapr placement | Actor placement |
| 4091 | Scraper API | Aplikasi HTTP |

## Event Topics Convention

Semua event menggunakan prefix `hub.`:

| Topic | Payload | Deskripsi |
|-------|---------|-----------|
| `hub.image.cached` | `{original_url, cdn_url, source}` | Image selesai di-cache |
| `hub.image.repaired` | `{old_url, new_url}` | CNAME image diperbaiki |
| `hub.scrape.anime.done` | `{source, slug, duration}` | Scrape anime selesai |
| `hub.system.alert` | `{service, level, message}` | Error/alert dari service |

## Service Registry (Traefik)

Domain routing:

| Subdomain | Service | URL Backend |
|-----------|---------|-------------|
| `scraper.*` | Scraper API | `http://scraper-api:4091` |
| `api.*` | Scraper API (alias) | `http://scraper-api:4091` |
| `traefik.*` | Traefik Dashboard | `api@internal` |

Semua domain tersedia di:
- `<service>.asepharyana.my.id`
- `<service>.asepharyana.web.id`
