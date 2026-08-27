# Troubleshooting

Kumpulan solusi untuk masalah umum di infrastruktur `asepharyana/infra` (orangevps).

> **Catatan (2026-08-02):** Produksi sekarang Caddy + Nix/systemd. Section Traefik/Docker di bawah adalah LEGACY — Docker dan Traefik dihapus dari produksi; gunakan hanya sebagai referensi historis.

## Daftar Isi

- [Deployment](#deployment)
- [Dapr](#dapr)
- [NATS](#nats)
- [Tailscale / Networking](#tailscale--networking)
- [Caddy](#caddy)
- [Database](#database)
- [Submodule](#submodule)

---

## Deployment

### Workflow deploy gagal: "Secrets not fully configured"

**Penyebab:** Salah satu GitHub secrets tidak diset.

**Solusi:** Cek secrets di Settings > Secrets and variables > Actions:

| Secret | Status |
|--------|--------|
| `SSH_PRIVATE_KEY` | Wajib |
| `VPS_HOST` | Wajib (`45.127.35.244`) |
| `VPS_USER` | Wajib (`root`) |
| ~~`VPS_TARGET_DIR`~~ | Tidak dipakai lagi (CI baru tidak checkout VPS) |
| `ENV_FILE_PRODUCTION` | Wajib |

### Workflow build gagal: "Submodule commit not fetchable"

**Penyebab:** Commit SHA dari `repository_dispatch` belum tersedia di remote submodule repo (eventual consistency).

**Solusi:** Workflow akan retry hingga 5 menit. Jika masih gagal:

```bash
# Cek apakah commit ada di remote
git ls-remote https://github.com/asepharyana/scraper.git <SHA>

# Trigger ulang dispatch dari submodule repo, atau push langsung ke hub
```

### Push manifest gagal: conflict di main

**Penyebab:** Ada commit lain yang masuk sebelum workflow selesai.

**Solusi:** Workflow otomatis retry rebase 3 kali. Jika semua gagal:

```bash
# Manual fix di lokal
git pull --rebase origin main
# resolve conflict
git push origin main
```

---

## Dapr

### Dapr sidecar tidak connect ke placement

**Gejala:** Container `scraper-api-dapr` restart loop. Log: `failed to connect to placement`

**Diagnosis:**

```bash
# Cek log sidecar
docker logs scraper-api-dapr --tail 50

# Cek apakah placement service running
docker ps -a | grep dapr-placement
docker logs dapr-placement --tail 20

# Cek konektivitas
docker exec scraper-api-dapr curl -s http://dapr-placement:50005
```

**Solusi:**

```bash
# Restart placement dulu, lalu sidecar
docker compose -f infra/compose/dapr.yml up -d --force-recreate
sleep 5
docker compose -f infra/compose/scraper.yml up -d --force-recreate scraper-api-dapr
```

### Dapr pub/sub tidak bekerja

**Gejala:** Event di-publish tapi tidak sampai ke subscriber.

**Diagnosis:**

```bash
# Cek komponen Dapr
curl http://localhost:3500/v1.0/components

# Cek health sidecar
curl http://localhost:3500/v1.0/healthz

# Cek Redis (backend pub/sub)
docker exec redis redis-cli ping
```

**Solusi:**

```bash
# Restart sidecar
docker restart scraper-api-dapr

# Jika Redis bermasalah, restart juga
docker restart redis
```

### Dapr state store error: "key not found"

**Penyebab:** Key belum ada di state store, atau prefix berbeda.

**Diagnosis:**

```bash
# Cek state langsung di Redis
docker exec redis redis-cli KEYS 'dapr*'

# State store menggunakan prefix "dapr"
# Format key: dapr || <app-id> || <key>
```

---

## NATS

### NATS tidak bisa start

**Gejala:** Container NATS restart loop.

**Diagnosis:**

```bash
docker logs nats --tail 50
```

**Solusi:** Kemungkinan korupsi data JetStream:

```bash
# Backup dulu volume data
docker run --rm -v nats_data:/data -v /tmp:/backup alpine cp -r /data /backup/nats_data_backup

# Hapus volume dan recreate
docker compose -f infra/compose/nats.yml down
docker volume rm asepharyana-hub_nats_data
docker compose -f infra/compose/nats.yml up -d
```

### JetStream stream overflow

**Gejala:** Disk penuh, NATS lambat.

**Diagnosis:**

```bash
# Cek ukuran volume
docker system df | grep nats_data
du -sh /var/lib/docker/volumes/nats_data/_data/

# Cek stream info
nats stream list
nats stream info <stream-name>
```

**Solusi:**

```bash
# Purge stream tertentu (data hilang)
nats stream purge <stream-name>

# Atau tambah limit stream via NATS config
```

### "Slow Consumer" warning

**Gejala:** Log NATS menampilkan "slow consumer".

**Diagnosis:**

```bash
curl http://localhost:8222/varz | jq '.slow_consumers'
```

**Solusi:**
- Scale consumer (tambah worker)
- Percepat processing message
- Kurangi ukuran payload

---

## Traefik

### Traefik tidak routing ke service

**Gejala:** 404 atau 503 dari Traefik.

**Diagnosis:**

```bash
# Cek apakah service container running
docker ps -a | grep scraper-api

# Cek log Traefik
docker logs traefik --tail 50

# Cek apakah container ada di network yang benar
docker network inspect app-shared-net | grep scraper-api

# Test routing langsung
curl -H "Host: scraper.asepharyana.my.id" http://localhost/
```

**Solusi:**

```bash
# Pastikan service terdaftar di apps.yaml
# Pastikan container join app-shared-net
# Restart Traefik
docker compose -f infra/compose/traefik.yml up -d --force-recreate
```

### TLS certificate error

**Gejala:** Browser menampilkan warning certificate.

**Diagnosis:**

```bash
# Cek sertifikat di host
ls -la /root/asepharyana.my.id.pem
openssl x509 -in /root/asepharyana.my.id.pem -text -noout | head -20

# Cek apakah Traefik bisa mount
docker exec traefik ls -la /etc/traefik/certs/
```

**Solusi:**
- Update sertifikat di host
- Restart Traefik
- Jika path berbeda, set environment variable `TRAEFIK_CERT_*`

### Rate limit terlalu ketat

**Gejala:** Request legitimate di-block.

**Diagnosis:**

```bash
# Cek rate-limit config di middlewares.yaml
# Current: average 100, burst 50
```

**Solusi:** Ubah nilai `average` dan `burst` di `infra/traefik/dynamic/middlewares.yaml`, lalu reload:

```bash
docker kill --signal HUP traefik
# atau
docker exec traefik kill -HUP 1
```

---

## Tailscale / Networking

### Container tidak bisa connect ke Tailscale IP

**Gejala:** Timeout saat container connect ke `100.121.180.82:6432`.

**Diagnosis:**

```bash
# Cek route service dari host
systemctl status tailscale-routes.service

# Cek route di host
ip route show table main | grep 100.64

# Cek koneksi dari host
ping 100.121.180.82

# Test dari container (dengan --network host)
docker run --rm --network host alpine ping -c 3 100.121.180.82
```

**Solusi:**

```bash
# Restart route service
sudo systemctl restart tailscale-routes.service

# Atau tambah route manual
sudo ip rule add from all lookup main priority 10000
sudo ip route add 100.64.0.0/10 dev tailscale0 table main
```

### Database connection refused

**Gejala:** Service tidak bisa konek ke PostgreSQL.

**Diagnosis:**

```bash
# Cek apakah DB listen di Tailscale (dari imrnes)
ss -tlnp | grep 6432

# Cek dari orangevps
nc -zv 100.121.180.82 6432

# Cek firewall di imrnes
sudo ufw status
sudo iptables -L -n | grep 6432
```

**Solusi:**

```bash
# Di imrnes: pastikan PostgreSQL bind ke Tailscale interface
# Di postgresql.conf:
listen_addresses = 'localhost,100.121.180.82'

# Di pg_hba.conf:
host    hub     asephs      100.0.0.0/8     md5

# Restart PostgreSQL
sudo systemctl restart postgresql
```

### Redis connection refused dari container

**Gejala:** Service tidak bisa connect ke `redis://redis:6379`.

**Diagnosis:**

```bash
# Cek apakah container Redis running
docker ps -a | grep redis

# Cek apakah container target join network yang sama
docker inspect <container> | grep -A5 Networks

# Cek DNS resolve dari container
docker exec <container> getent hosts redis
```

**Solusi:**

```bash
# Pastikan Redis ada di network app-shared-net
docker network inspect app-shared-net | grep redis

# Jika tidak, attach
docker network connect app-shared-net redis
```

---

## Docker / Container

### Container restart loop

**Diagnosis:**

```bash
docker logs <container> --tail 50
docker inspect <container> | jq '.[].State'
```

**Penyebab umum:**
- Health check gagal
- Dependency service belum siap
- Environment variable tidak diset

### Image pull gagal dari GHCR

**Gejala:** `docker pull` gagal di VPS.

**Diagnosis:**

```bash
# Cek login
cat ~/.docker/config.json | grep ghcr

# Cek visibility package
# Buka https://github.com/orgs/asepharyana/packages
```

**Solusi:**

```bash
# Re-login
echo $GITHUB_TOKEN | docker login ghcr.io -u asepharyana --password-stdin

# Pastikan package visibility public atau di-share ke org
```

### Disk penuh

**Gejala:** Container crash, write error.

**Diagnosis:**

```bash
df -h
docker system df
du -sh /var/lib/docker/
```

**Solusi:**

```bash
# Bersihkan container/image/volume yang tidak dipakai
docker system prune -a -f

# Hapus image lama
docker image prune -a -f

# Lihat volume terbesar
docker system df -v | grep -E "(nats_data|redis_data)"
```

---

## Database

### Koneksi PostgreSQL lambat

**Gejala:** Query time high, connection timeout.

**Diagnosis:**

```bash
# Dari container, test latency
docker exec scraper-api ping -c 5 100.121.180.82

# Cek koneksi aktif
docker exec scraper-api psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity;"
```

**Solusi:**
- Cek Tailscale latency
- Adjust connection pool size
- Cek resource PostgreSQL di `imrnes`

### Migration gagal

**Gejala:** Service error setelah image update.

**Diagnosis:**

```bash
# Cek log service
docker logs scraper-api --tail 100 | grep -i migration
```

**Solusi:**
- Migration ada di submodule `apps/scraper`, bukan di hub
- Pastikan schema sesuai dengan versi code
- Rollback image jika migration tidak backward-compatible

---

## Submodule

### HEAD detached di submodule

**Gejala:** `git status` di `apps/scraper` menunjukkan "HEAD detached".

**Penyebab:** Normal. Submodule selalu dalam keadaan detached HEAD karena mengacu pada commit spesifik.

**Solusi:** Jangan commit perubahan dari dalam submodule. Selalu bekerja di repo asli.

### Submodule tidak ter-update setelah pull

```bash
git submodule update --init --recursive
```

### Konflik submodule saat rebase/merge

```bash
# Resolve dengan memilih versi yang benar
git add apps/scraper
git rebase --continue
```
