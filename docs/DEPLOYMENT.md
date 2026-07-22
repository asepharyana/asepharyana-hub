# Deployment Guide

Panduan deploy aplikasi apapun menggunakan **Docker + Docker Compose + GitHub Actions + VPS**.

## Arsitektur

```
GitHub Repo ──► GitHub Actions ──► Registry (GHCR / Docker Hub / ECR / dll.)
                                                       │
                                                       ▼
                                                  VPS (<VPS_HOST>)
                                                docker compose pull + up
```

## Prerequisites

- Docker Engine >= 24.x
- Docker Compose v2 (plugin)
- Git
- Akun GitHub dengan akses repo
- SSH key di `~/.ssh/<KEY_NAME>` (default: `id_ed25519`)

## Konfigurasi VPS Target

Buat berkas `~/orangevps` (atau sesuaikan dengan env Anda):

```text
ssh <USER>@<VPS_HOST>
```

Contoh isi `~/orangevps`:

```text
ssh root@45.127.35.244
```

| Parameter | Nilai | Contoh |
|-----------|-------|--------|
| User | `<USER>` | `root` |
| Host | `<VPS_HOST>` | `45.127.35.244` |
| SSH Key | `~/.ssh/<KEY_NAME>` | `~/.ssh/id_ed25519` |
| Target Dir di VPS | `<VPS_TARGET_DIR>` | `/opt/app` atau `/root/app` |

> Tip: Jika SSH key menggunakan nama selain default, sesuaikan path dan `ssh -i` sesuai.

## Registry

Pilih registry untuk menyimpan image Docker. Sesuaikan dengan proyek:

| Registry | URL | Auth |
|----------|-----|------|
| GitHub Container Registry | `ghcr.io` | `GITHUB_TOKEN` |
| Docker Hub | `docker.io` | username / PAT |
| AWS ECR | `<account>.dkr.ecr.<region>.amazonaws.com` | `aws ecr get-login-password` |
| Google GCR | `gcr.io` | `gcloud auth print-access-token` |
| Azure ACR | `<registry>.azurecr.io` | `az acr login` |

Contoh namespace untuk GHCR:

```text
Registry : ghcr.io
Namespace: <GITHUB_USERNAME_OR_ORG>
Repo     : <REPO_NAME>
```

Pastikan package/visibility di registry mengizinkan akses pull dari VPS.

---

## Deploy Otomatis (Recommended)

Gunakan GitHub Actions untuk otomatisasi build, push, dan deploy.

### Workflow 1: Build dan Push Image

File: `.github/workflows/docker-build-push.yml`

```yaml
name: Build and Push Docker Images

on:
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read
  packages: write

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - uses: docker/login-action@v4
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - uses: docker/setup-buildx-action@v4

      - uses: docker/build-push-action@v7
        with:
          context: .
          file: Dockerfile
          push: true
          tags: |
            ghcr.io/${{ github.repository }}/<SERVICE_NAME>:latest
            ghcr.io/${{ github.repository }}/<SERVICE_NAME>:sha-${{ github.sha }}
          cache-from: type=registry,ref=ghcr.io/${{ github.repository }}/<SERVICE_NAME>:buildcache
          cache-to: type=registry,ref=ghcr.io/${{ github.repository }}/<SERVICE_NAME>:buildcache,mode=max
```

Ubah `<SERVICE_NAME>` sesuai service (misal: `app`, `web`, `api`). Jika monorepo, gunakan matrix strategy untuk build beberapa service sekaligus.

### Workflow 2: Deploy ke VPS

File: `.github/workflows/deploy-docker.yml`

```yaml
name: Deploy Docker to VPS

on:
  workflow_run:
    workflows: ['Build and Push Docker Images']
    types: [completed]
  push:
    branches: [main]
  workflow_dispatch:

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v6

      - name: Deploy to VPS
        env:
          SSH_PRIVATE_KEY: ${{ secrets.SSH_PRIVATE_KEY }}
          VPS_HOST: ${{ secrets.VPS_HOST }}
          VPS_USER: ${{ secrets.VPS_USER }}
          VPS_TARGET_DIR: ${{ secrets.VPS_TARGET_DIR }}
          ENV_FILE_PRODUCTION: ${{ secrets.ENV_FILE_PRODUCTION }}
        run: |
          set -euo pipefail

          mkdir -p ~/.ssh
          echo "$SSH_PRIVATE_KEY" > ~/.ssh/id_rsa
          chmod 600 ~/.ssh/id_rsa
          ssh-keyscan -H -t ed25519,rsa "$VPS_HOST" >> ~/.ssh/known_hosts

          SSH_OPTS=(-o ControlMaster=auto -o ControlPath=/tmp/ssh-%r@%h:%p -o ControlPersist=600 -o StrictHostKeyChecking=yes)

          ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" "mkdir -p $VPS_TARGET_DIR && mkdir -p $VPS_TARGET_DIR/infra/compose"
          echo "$ENV_FILE_PRODUCTION" > .env.prod
          scp "${SSH_OPTS[@]}" .env.prod "$VPS_USER@$VPS_HOST:$VPS_TARGET_DIR/.env"

          ssh "${SSH_OPTS[@]}" "$VPS_USER@$VPS_HOST" bash -s <<'EOF'
            set -euo pipefail
            cd "$VPS_TARGET_DIR"

            docker network inspect app-shared-net >/dev/null 2>&1 || docker network create app-shared-net

            if [ ! -d ".git" ]; then
              git init
              git remote add origin https://github.com/<GITHUB_USER>/<REPO_NAME>.git
            fi
            git fetch origin main --depth=1 || true
            git reset --hard FETCH_HEAD

            docker compose --env-file .env pull
            docker compose --env-file .env up -d --remove-orphans
          EOF
```

### Secrets GitHub yang Diperlukan

Buka **Settings > Secrets and variables > Actions**:

| Secret | Deskripsi |
|--------|-----------|
| `SSH_PRIVATE_KEY` | Isi dengan `cat ~/.ssh/<KEY_NAME>` |
| `VPS_HOST` | IP atau domain VPS |
| `VPS_USER` | User SSH (misal: `root`, `ubuntu`, `deploy`) |
| `VPS_TARGET_DIR` | Direktori aplikasi di VPS |
| `ENV_FILE_PRODUCTION` | Isi dengan environment production |

### Trigger Manual

```bash
gh workflow run deploy-docker.yml
```

---

## Dockerfile Patterns

Pilih pattern sesuai jenis aplikasi.

### Pattern 1: Multi-stage Build (SPA / static assets)

```dockerfile
FROM oven/bun:1 AS builder
WORKDIR /app
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile
COPY . .
RUN bun run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
```

### Pattern 2: Single-stage (runtime image)

```dockerfile
FROM oven/bun:1
WORKDIR /app
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile
COPY . .
EXPOSE 3000
CMD ["bun", "run", "start"]
```

### Pattern 3: Compiled binary (Rust / Go / Zig)

```dockerfile
FROM rust:1 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/app /usr/local/bin/app
EXPOSE 8080
CMD ["app"]
```

---

## Docker Compose Patterns

### Single service

```yaml
services:
  app:
    container_name: app
    image: registry.example.com/org/app:latest
    restart: always
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
```

### Multi-service dengan shared network

```yaml
services:
  app:
    container_name: app
    image: registry.example.com/org/app:latest
    restart: always
    networks: [app-shared-net]

  redis:
    container_name: redis
    image: redis:7-alpine
    restart: always
    networks: [app-shared-net]

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

### Dengan reverse proxy (Traefik / Caddy / Nginx)

```yaml
services:
  app:
    container_name: app
    image: registry.example.com/org/app:latest
    restart: always
    networks: [app-shared-net]
    labels:
      - 'traefik.enable=true'
      - 'traefik.http.routers.app.rule=Host(`app.example.com`)'
      - 'traefik.http.routers.app.entrypoints=websecure'
      - 'traefik.http.routers.app.tls=true'
      - 'traefik.http.services.app.loadbalancer.server.port=3000'

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

---

## Deploy Manual (Lokal)

### 1. Build dan Push ke Registry

Login ke registry:

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u <GITHUB_USERNAME> --password-stdin
```

Build dan push:

```bash
docker build -t ghcr.io/<GITHUB_USERNAME>/<REPO_NAME>/<SERVICE_NAME>:latest -f Dockerfile .

docker push ghcr.io/<GITHUB_USERNAME>/<REPO_NAME>/<SERVICE_NAME>:latest
```

Tag tambahan dengan SHA commit:

```bash
SHORT_SHA=$(git rev-parse --short HEAD)
docker tag ghcr.io/<GITHUB_USERNAME>/<REPO_NAME>/<SERVICE_NAME>:latest \
  ghcr.io/<GITHUB_USERNAME>/<REPO_NAME>/<SERVICE_NAME>:sha-${SHORT_SHA}
docker push ghcr.io/<GITHUB_USERNAME>/<REPO_NAME>/<SERVICE_NAME>:sha-${SHORT_SHA}
```

### 2. Pull dan Deploy di VPS

SSH ke VPS:

```bash
ssh -i ~/.ssh/<KEY_NAME> <USER>@<VPS_HOST>
```

Clone repo (jika belum):

```bash
git clone https://github.com/<GITHUB_USER>/<REPO_NAME>.git <VPS_TARGET_DIR>
cd <VPS_TARGET_DIR>
```

Buat shared network (hanya sekali):

```bash
docker network create app-shared-net
```

Siapkan environment:

```bash
cp .env.example .env
# Edit .env sesuai nilai production
nano .env
```

Login ke registry di VPS:

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u <GITHUB_USERNAME> --password-stdin
```

Pull gambar terbaru:

```bash
cd <VPS_TARGET_DIR>
docker compose -f docker-compose.yml --env-file .env pull
```

Deploy (up):

```bash
docker compose -f docker-compose.yml --env-file .env up -d --remove-orphans
```

Verifikasi:

```bash
docker compose -f docker-compose.yml ps
docker compose -f docker-compose.yml logs -f <SERVICE_NAME>
```

---

## Deployment Order (Manual)

Jika deploy bertahap, gunakan urutan ini:

```bash
# 1. Shared services (Redis, database, dll.)
docker compose -f infra/compose/shared.yml up -d

# 2. Reverse proxy
docker compose -f infra/compose/traefik.yml up -d

# 3. Aplikasi
docker compose \
  -f infra/compose/app1.yml \
  -f infra/compose/app2.yml \
  up -d
```

---

## Perintah Berguna di VPS

```bash
# Lihat semua container
docker ps -a

# Log service
docker logs -f <container_name>

# Restart satu service
docker compose -f <compose_file> up -d --force-recreate

# Hapus network lama (hati-hati)
docker network rm app-shared-net
docker network create app-shared-net

# Bersihkan image unused
docker image prune -a -f
docker system prune -a -f
```

---

## Troubleshooting

### Image tidak bisa di-pull

Pastikan sudah login ke registry di VPS:

```bash
docker logout ghcr.io
echo $GITHUB_TOKEN | docker login ghcr.io -u <GITHUB_USERNAME> --password-stdin
```

Periksa visibility package di registry (harus `Public` atau akses diberikan).

### Port sudah dipakai

```bash
docker ps | grep :80
docker ps | grep :443
```

### Reverse proxy tidak routing

Periksa label di compose file dan pastikan shared network ada:

```bash
docker network inspect app-shared-net
docker logs traefik
```

---

## Environment Variable Management

### Pola 1: `.env` di VPS (recommended untuk production)

```bash
# Di VPS
cd <VPS_TARGET_DIR>
cp .env.example .env
# Edit sesuai production
nano .env
```

CI/CD upload `.env` via secret, tidak simpan di repo.

### Pola 2: Docker secrets (Swarm mode)

```yaml
services:
  app:
    image: app:latest
    secrets:
      - db_password

secrets:
  db_password:
    file: ./secrets/db_password.txt
```

### Pola 3: External secret manager

- **HashiCorp Vault**: inject via env atau file
- **AWS Secrets Manager**: `aws secretsmanager get-secret-value`
- **Doppler / Infisical**: unified secret management

---

## Tagging dan Versioning

### Strategy yang umum

| Strategy | Contoh tag | Kegunaan |
|----------|-----------|----------|
| Latest + SHA | `latest`, `sha-abc1234` | CI/CD cepat, traceable |
| SemVer | `1.2.3`, `1.2`, `1` | Release publik |
| Git tag mirror | `v1.2.3` | Sync dengan git tag |
| Branch mirror | `main`, `develop` | Preview / staging |

### Contoh git tag driven deploy

```bash
git tag v1.2.3
git push origin v1.2.3
```

CI/CD membaca tag, build image dengan tag yang sama, dan deploy.

---

## Rollback

### Rollback via registry

```bash
# Lihat tag yang tersedia
docker manifest inspect ghcr.io/org/app:latest
# atau lihat UI registry

# Di VPS, edit compose file ke tag sebelumnya
# lalu:
docker compose --env-file .env pull
docker compose --env-file .env up -d --remove-orphans
```

### Rollback via git

```bash
git revert HEAD
git push origin main
# CI/CD otomatis build dan deploy versi sebelumnya
```

---

## Health Checks

### Di Dockerfile

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1
```

### Di Docker Compose

```yaml
services:
  app:
    image: app:latest
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
```

---

## Monitoring & Observability

```bash
# Log aggregated
docker compose logs -f --tail=100

# Resource usage
docker stats

# Disk usage
docker system df

# Cleanup
docker system prune -a -f
```

---

## Catatan Keamanan

- Jangan commit `.env` atau SSH private key ke repo.
- Gunakan GitHub Secrets (atau secret manager) untuk credential di CI/CD.
- Rotate token dan key secara berkala.
- Batasi akses SSH ke VPS (ubah port default, gunakan fail2ban).
- Set `StrictHostKeyChecking=yes` pada SSH opsional deployment.

---

---

## Proyek Ini: asepharyana-hub

> Dokumentasi spesifik untuk repo ini. Lihat juga [ADR-0002](adr/0002-env-file-via-github-secret.md).

### Topologi

| Host | IP | Peran |
|------|----|-------|
| `orangevps` (VPS) | `45.127.35.244` | Docker host: Traefik, scraper-api, Redis, NATS, Dapr |
| `imrnes` (bare-metal) | `100.121.180.82` (Tailscale) | PostgreSQL (port 6432), Redis (port 6379) |

### Environment Variables

**Production `.env` tidak pernah di-commit.** File ini disimpan sebagai GitHub secret `ENV_FILE_PRODUCTION` dan di-SCP ke VPS saat deploy via `deploy-docker.yml`.

Cara update:
```bash
# Baca current .env dari VPS
ssh root@45.127.35.244 "cat /root/asepharyana-hub/.env"

# Update GitHub secret (dari output di atas)
cat > /tmp/env-updated << 'EOF'
<paste content, edit, lalu>
EOF
cat /tmp/env-updated | gh secret set ENV_FILE_PRODUCTION --repo asepharyana/asepharyana-hub
```

**Jangan manual edit `.env` di VPS tanpa update GitHub secret juga** — nanti ke- overwrite pas deploy berikutnya.

### Database

| Variable | Value |
|----------|-------|
| `DATABASE_URL` | `postgres://asephs:hunterz@100.121.180.82:6432/hub` |
| `REDIS_URL` | `redis://redis:6379` (Docker network) |

### Kompose

Proyek compose bernama `compose`, terdiri dari 5 file yang selalu di-include bersamaan:

```bash
/root/asepharyana-hub/infra/compose/
├── traefik.yml     # Reverse proxy
├── shared.yml      # Redis
├── nats.yml        # NATS
├── dapr.yml        # Dapr placement
└── scraper.yml     # Scraper API
```

Perintah restart setelah update `.env` di VPS:
```bash
cd /root/asepharyana-hub
docker compose \
  -p compose \
  --env-file .env \
  -f infra/compose/traefik.yml \
  -f infra/compose/shared.yml \
  -f infra/compose/scraper.yml \
  -f infra/compose/nats.yml \
  -f infra/compose/dapr.yml \
  up -d --remove-orphans
```

---

## Checklist Deploy Proyek Baru

1. [ ] Dockerfile ditest lokal (`docker build`, `docker run`)
2. [ ] Docker Compose file valid (`docker compose config`)
3. [ ] `.dockerignore` sesuai (node_modules, .git, .env)
4. [ ] Registry dibuat (GHCR package / Docker Hub repo / ECR / dll.)
5. [ ] GitHub Actions workflow dibuat dengan permission `packages: write`
6. [ ] VPS siap: Docker, Docker Compose, SSH key
7. [ ] Shared network dibuat (`docker network create`)
8. [ ] `.env` production di-VPS atau via secret manager
9. [ ] Reverse proxy (Traefik / Caddy / Nginx) routing ke container
10. [ ] Health check endpoint aktif
