# Asepharyana Hub

Hub repo untuk ekosistem portfolio dan layanan pendukung milik Asep Haryana Saputra.
Aplikasi dipisah sebagai submodule agar frontend, API, dan service pendukung bisa dikembangkan serta di-deploy secara independen.

## Services

| Service     | Path             | Default Local Port | Notes                                                                         |
| :---------- | :--------------- | :----------------- | :---------------------------------------------------------------------------- |
| Rust API    | `apps/rust-auth` | `4091`             | API utama (Axum + SeaORM), scraping, image proxy/cache, metrics, OpenAPI docs |
| Elysia API  | `apps/elysia`    | `4092`             | API realtime/auth/chat/quiz (Elysia + Bun + Drizzle + Redis)                  |
| React Web   | `apps/react`     | `3000`             | Frontend React/Vite                                                           |
| Scraper     | `apps/scraper`   | —                  | Web scraper service                                                           |

## Infrastructure

File compose berada di `infra/compose/`:

- `traefik.yml`: reverse proxy Traefik untuk semua layanan.
- `shared.yml`: Redis.
- `rust-auth.yml`, `elysia.yml`, `react.yml`, `scraper.yml`: manifest deploy per service (image GHCR bertag SHA).

Dockerfile per service berada di `infra/docker/`.

## Docker Image Builds

Build image via Dockerfile:

```bash
docker build -f infra/docker/rust.Dockerfile -t rust-auth:latest .
docker build -f infra/docker/elysia.Dockerfile -t elysia-api:latest .
docker build -f infra/docker/react.Dockerfile -t react-web:latest .
docker build -f infra/docker/scraper.Dockerfile -t scraper-api:latest .
```

Tag and push:

```bash
SHORT_SHA=$(git rev-parse --short HEAD)

docker tag rust-auth:latest ghcr.io/asepharyana/asepharyana-hub/rust-auth:sha-$SHORT_SHA
docker push ghcr.io/asepharyana/asepharyana-hub/rust-auth:sha-$SHORT_SHA
# repeat for elysia-api, react-web, scraper-api
```

## Local Development

### 1) Jalankan dependency bersama

```bash
docker compose -f infra/compose/shared.yml up -d
```

### 2) Jalankan service yang dibutuhkan

```bash
# Rust API
cd apps/rust-auth
cargo run

# Elysia API
cd apps/elysia
bun install
bun run dev

# React web
cd apps/react
npm install
npm run dev
```

## API Docs and Monitoring

- Rust OpenAPI: `/docs`
- Elysia Swagger: `/docs`
- Elysia AsyncAPI viewer: `/docs-ws`

## Deployment Notes

- Pipeline memakai image tag berbasis commit SHA (`sha-<short-sha>`), bukan `latest`.
- Deploy Compose sekarang mencakup `infra/compose/*.yml` dan `deploy-docker.yml` akan berjalan langsung ketika `infra/compose/**` berubah.

## Networking & Tailscale

### Arsitektur

Semua VPS terhubung via **Tailscale**. Setiap VPS punya IP Tailscale dan service berkomunikasi antar VPS melalui Tailscale network (`100.64.0.0/10`).

| VPS               | Tailscale IP    | Service                                |
| :---------------- | :-------------- | :------------------------------------- |
| `imrnes`          | `100.108.1.124` | PostgreSQL (`hub`), Redis              |
| `orange`          | `100.96.248.86` | App containers (Traefik, 9Router, dll) |
| `archlinux`       | `100.114.19.66` | _(development machine)_                |
| `laptop-2f6e1iph` | `100.86.195.29` | _(offline)_                            |

### Container → Tailscale Connectivity

Docker containers di bridge network (`app-shared-net`) **tidak otomatis bisa access Tailscale IPs** karena Tailscale menggunakan **custom policy routing** (routes di `table 52`, bukan `main` table).

#### Fix: Tailscale Route di Main Table

Agar container bisa reach Tailscale IPs (untuk DB, Redis, dll), tambahkan route ke `main` routing table:

```bash
# Manual (hilang setelah reboot)
ip route add 100.64.0.0/10 dev tailscale0 table main

# Persistent (systemd service)
# Sudah dikonfigurasi sebagai /etc/systemd/system/tailscale-routes.service
# Service ini berjalan otomatis setelah tailscaled start
systemctl enable tailscale-routes.service
systemctl start tailscale-routes.service
```

#### Environment Variables

Service yang connect ke Tailscale IP:

```env
# PostgreSQL di imrnes
DATABASE_URL=postgres://user:pass@100.108.1.124:5432/dbname

# Redis di imrnes
REDIS_URL=redis://100.108.1.124:6379
```

#### Persistent Systemd Service

File: `/etc/systemd/system/tailscale-routes.service`

```ini
[Unit]
Description=Add Tailscale routes to main routing table
After=tailscaled.service
Requires=tailscaled.service

[Service]
Type=oneshot
ExecStart=/bin/bash -c '/usr/sbin/ip route add 100.64.0.0/10 dev tailscale0 table main 2>/dev/null || /usr/sbin/ip route replace 100.64.0.0/10 dev tailscale0 table main'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

Install & enable:

```bash
sudo tee /etc/systemd/system/tailscale-routes.service > /dev/null << 'EOF'
[Unit]
Description=Add Tailscale routes to main routing table
After=tailscaled.service
Requires=tailscaled.service

[Service]
Type=oneshot
ExecStart=/bin/bash -c '/usr/sbin/ip route add 100.64.0.0/10 dev tailscale0 table main 2>/dev/null || /usr/sbin/ip route replace 100.64.0.0/10 dev tailscale0 table main'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable tailscale-routes.service
sudo systemctl start tailscale-routes.service
```

#### Troubleshooting

```bash
# Cek Tailscale peers
tailscale status

# Cek route table 52 (Tailscale internal)
ip route show table 52

# Cek route table main (yang dipakai container)
ip route show table main | grep 100.

# Test connectivity dari dalam container
docker exec <container> node -e "
const net = require('net');
const c = new net.Socket();
c.setTimeout(5000);
c.connect(5432, '100.108.1.124', () => { console.log('OK'); c.end(); });
c.on('error', e => { console.log('FAIL:', e.code); });
c.on('timeout', () => { console.log('TIMEOUT'); c.destroy(); });
"

# Cek service tailscale-routes
systemctl status tailscale-routes.service
```

## Menambahkan Aplikasi Baru

Panduan langkah demi langkah untuk menambahkan aplikasi baru ada di `docs/add-new-app.md`.

## License

MIT

# PR-Agent V3 Test
Testing with corrected model prefix.
