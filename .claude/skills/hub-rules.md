---
name: hub-rules
description: Aturan repository, arsitektur hub, submodule, dan workflow Asepharyana Hub
---

# Asepharyana Hub — Repository Rules

## Struktur Repository

```
asepharyana-hub/
├── apps/              # Git submodules — source code aplikasi
├── docs/              # Dokumentasi, ADR, deployment guide
├── infra/             # Infrastructure as code
│   ├── compose/       # Satu compose file per service
│   ├── dapr/          # Dapr component configs
│   ├── docker/        # Dockerfiles (LEGACY — Docker dihapus)
│   ├── traefik/       # Traefik config (LEGACY — diganti Caddy)
│   └── caddy/         # Caddyfile.prod (reverse proxy produksi)
├── scripts/           # Utility scripts (cleanup, update-deps)
└── .github/workflows/ # CI/CD pipelines
```

### Aturan Submodule
- Setiap aplikasi di `apps/` adalah **submodule** ke repo terpisah.
- Perubahan kode aplikasi dilakukan di **repo masing-masing**, bukan di sini.
- Submodule pointer diupdate oleh CI/CD (bukan manual).

## Infrastructure Patterns

### Networking
- Semua service Nix/systemd, inter-service via 127.0.0.1:<port>
- Caddy sebagai ingress untuk HTTP/S eksternal (auto-TLS LE, HTTP/3)
- Tailscale untuk cross-VPS (PostgreSQL, Redis)

### Compose File Pattern
```yaml
services:
  <service>:
    container_name: <service>
    image: ghcr.io/asepharyana/asepharyana-hub/<service>:sha-<sha>
    restart: always
    networks:
      app-shared-net:
        aliases:
          - <service>
    env_file:
      - ../../.env

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

### Dapr Sidecar Pattern
```yaml
  <service>-dapr:
    container_name: <service>-dapr
    image: daprio/daprd:latest
    restart: always
    depends_on:
      nats:
        condition: service_started
      dapr-placement:
        condition: service_started
    networks:
      - app-shared-net
    command:
      - './daprd'
      - '--app-id=<service>'
      - '--app-port=<port>'
      - '--dapr-http-port=3500'
      - '--dapr-grpc-port=50001'
      - '--placement-host-address=dapr-placement:50005'
      - '--resources-path=/components'
    volumes:
      - ../../infra/dapr/components:/components
```

### Caddy Routing
- Site block di `/etc/caddy/Caddyfile` (ref `infra/caddy/Caddyfile.prod`)
- Subdomain pattern: `<service>.asepharyana.my.id` + `<service>.asepharyana.web.id`
- TLS cert dari volume mount (bukan auto-acme)

### CI/CD
- `docker-build-push.yml` — build per service, push ke GHCR, update compose manifest
- `deploy-docker.yml` — SSH ke orangevps, pull images, restart
- Selective deploy: hanya compose file yg berubah

## Deployment Order
1. `shared.yml` (Redis)
2. `nats.yml` (NATS message bus)
3. `dapr.yml` (Dapr placement)
4. Caddy (reverse proxy)
5. Service compose files (apps + Dapr sidecar)
