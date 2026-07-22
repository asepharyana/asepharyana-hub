# Backup & Disaster Recovery

## Aset yang Perlu di-Backup

| Aset | Lokasi | Frekuensi | Metode |
|------|--------|-----------|--------|
| Database PostgreSQL | `imrnes` (100.121.180.82:6432) | Harian | `pg_dump` |
| Volume Redis | `orangevps` (Docker volume) | Opsional | Redis RDB / AOF |
| Volume NATS JetStream | `orangevps` (Docker volume) | Opsional | File copy |
| Docker Compose manifests | GitHub (hub repo) | Real-time | Git |
| Environment variables | GitHub secret `ENV_FILE_PRODUCTION` | Manual | `gh secret set` |
| TLS certificates | `orangevps` (`/root/*.pem`, `*.key`) | Saat renew | SCP |
| Tailscale auth | Tailscale admin console | - | Cloud-managed |
| GitHub Actions secrets | GitHub UI | Manual | Backup list |

## Database PostgreSQL (Prioritas Tertinggi)

### Backup Manual

```bash
# Dari orangevps (via Tailscale)
pg_dump -h 100.121.180.82 -p 6432 -U asephs -d hub \
  --no-owner --no-acl \
  -F c -f /root/db-backups/hub-$(date +%Y%m%d-%H%M%S).dump

# Atau dari imrnes langsung
pg_dump -U asephs -d hub \
  -F c -f /backup/hub/hub-$(date +%Y%m%d-%H%M%S).dump
```

### Restore

```bash
# Drop dan recreate database
dropdb -h 100.121.180.82 -p 6432 -U asephs hub
createdb -h 100.121.180.82 -p 6432 -U asephs hub

# Restore dari dump
pg_restore -h 100.121.180.82 -p 6432 -U asephs -d hub \
  --no-owner --no-acl \
  /path/to/backup/hub-20260101-120000.dump
```

### Backup Otomatis (via Cron di imrnes)

```bash
# /etc/cron.d/hub-db-backup
0 2 * * * root pg_dump -U asephs -d hub -F c -f /backup/hub/hub-$(date +\%Y\%m\%d).dump && find /backup/hub -name "hub-*.dump" -mtime +30 -delete
```

## Volume Docker

### Redis

Redis data bisa di-recover dari NATS events (event sourcing). Jika tidak ada persistence requirement, cukup restart:

```bash
docker volume rm redis_data
docker compose -f infra/compose/shared.yml up -d
```

Jika perlu backup:

```bash
# Save RDB snapshot
docker exec redis redis-cli SAVE

# Copy dari volume
docker run --rm -v redis_data:/data -v /backup:/backup alpine cp /data/dump.rdb /backup/redis-$(date +%Y%m%d).rdb
```

### NATS JetStream

```bash
# Backup volume
docker run --rm -v nats_data:/data -v /backup:/backup alpine \
  tar czf /backup/nats-$(date +%Y%m%d).tar.gz -C /data .
```

## Environment Variables

### Backup `.env` dari VPS

```bash
# Simpan current .env dari VPS
ssh root@45.127.35.244 "cat /root/asepharyana-hub/.env" > .env.backup.$(date +%Y%m%d)

# Update GitHub secret
cat .env.backup.$(date +%Y%m%d) | gh secret set ENV_FILE_PRODUCTION --repo asepharyana/asepharyana-hub
```

### Restore `.env` jika hilang

```bash
# Buat .env baru dari template
cp .env.example .env

# Edit secrets (manual dari password manager atau GitHub secret)
# Atau download dari GitHub secret
gh secret list --repo asepharyana/asepharyana-hub
```

## TLS Certificates

### Backup

```bash
# Di orangevps
tar czf /root/cert-backup-$(date +%Y%m%d).tar.gz \
  /root/asepharyana.my.id.pem \
  /root/asepharyana.my.id.key \
  /root/asepharyana.web.id.pem \
  /root/asepharyana.web.id.key \
  /root/asepharyana-hub/infra/traefik/dynamic/ssl.yaml

# SCP ke local
scp root@45.127.35.244:/root/cert-backup-*.tar.gz .
```

### Restore

```bash
# SCP ke VPS
scp cert-backup-20260101.tar.gz root@45.127.35.244:/root/

# Extract
ssh root@45.127.35.244 "tar xzf /root/cert-backup-20260101.tar.gz -C / && docker restart traefik"
```

## Disaster Recovery Scenarios

### Skenario 1: VPS (orangevps) mati total

**Dampak:** Semua service down.

**Recovery:**

```bash
# 1. Provision VPS baru (atau restore dari snapshot)
# 2. Install Docker + Tailscale
# 3. Clone repo
git clone https://github.com/asepharyana/asepharyana-hub.git /root/asepharyana-hub

# 4. Setup Tailscale, route service
# 5. Restore .env
echo "<ENV_FILE_PRODUCTION>" > /root/asepharyana-hub/.env

# 6. Restore TLS certs
# 7. Create network
docker network create app-shared-net

# 8. Start services sesuai urutan
cd /root/asepharyana-hub
for f in shared.yml nats.yml dapr.yml traefik.yml scraper.yml; do
  docker compose -f infra/compose/$f --env-file .env up -d
done

# 9. Update DNS jika IP baru
```

### Skenario 2: Database (imrnes) mati total

**Dampak:** Semua service yang butuh database error.

**Recovery:**

```bash
# 1. Fix imrnes atau provision server baru
# 2. Setup PostgreSQL
# 3. Restore dari backup terakhir
# 4. Update Tailscale IP jika perlu
# 5. Update .env dan GitHub secret
# 6. Redeploy
```

### Skenario 3: GitHub repository hilang

**Dampak:** Kehilangan CI/CD, tapi Docker images masih ada di GHCR.

**Recovery:**

```bash
# 1. Create repo baru di GitHub
# 2. Push dari local clone
git remote add origin-new https://github.com/asepharyana/asepharyana-hub-new.git
git push origin-new main

# 3. Re-create GitHub secrets
# 4. Re-create workflows
# 5. Update VPS remote
ssh root@45.127.35.244 "cd /root/asepharyana-hub && git remote set-url origin https://github.com/asepharyana/asepharyana-hub-new.git"
```

### Skenario 4: GHCR registry tidak bisa diakses

**Dampak:** Tidak bisa pull image.

**Recovery:**

```bash
# 1. Build image langsung di VPS
docker build -f infra/docker/scraper.Dockerfile -t ghcr.io/asepharyana/asepharyana-hub/scraper-api:local .

# 2. Update compose file untuk sementara
sed -i 's|image: ghcr.io/.*|image: ghcr.io/asepharyana/asepharyana-hub/scraper-api:local|' infra/compose/scraper.yml

# 3. Start
docker compose -f infra/compose/scraper.yml up -d
```

### Skenario 5: Semua server mati (total loss)

**Recovery:**

```bash
# 1. Provision VPS baru
# 2. Provision server database baru
# 3. Setup Tailscale
# 4. Clone repo, restore .env, certs
# 5. Restore database dari backup (jika ada)
# 6. Jika tidak ada backup database:
#    - Build image dari GHCR
#    - Start service dengan database kosong
#    - Data akan terisi ulang dari scraping
```

## Checklist Pencegahan

- [ ] Cron job backup database berjalan
- [ ] Backup `.env` disimpan di luar VPS (password manager)
- [ ] TLS certificates backup disimpan di luar VPS
- [ ] GitHub secrets terdaftar (tidak hanya diingat)
- [ ] Docker images bisa di-rebuild dari CI (GHCR sebagai source of truth)
- [ ] Tailscale admin access via multiple accounts
