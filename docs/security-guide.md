# Security Guide

Praktik keamanan untuk infrastruktur `asepharyana-hub`.

## Ringkasan

| Area | Status | Prioritas |
|------|--------|-----------|
| Secrets management | GitHub encrypted secrets | Tinggi |
| TLS termination | Traefik + cert volume mounts | Tinggi |
| Container security | Non-root user (scraper-api) | Sedang |
| Network security | Tailscale overlay, app-shared-net | Sedang |
| Access control | SSH key, GitHub permissions | Sedang |
| Monitoring | Belum ada alert system | Rendah |
| Firewall | UFW/iptables (manual) | Sedang |
| Backup | lihat `docs/backup-recovery.md` | Sedang |

## Secrets Management

### Yang Tidak Boleh di-Commit

- [ ] `.env` production (disimpan sebagai GitHub secret `ENV_FILE_PRODUCTION`)
- [ ] SSH private keys
- [ ] API tokens, JWT secret
- [ ] Docker registry tokens
- [ ] Database passwords
- [ ] TLS certificate private keys

### GitHub Secrets

Setting di Settings > Secrets and variables > Actions:

| Secret | Tujuan | Rotasi |
|--------|--------|--------|
| `SSH_PRIVATE_KEY` | Akses SSH ke VPS | 6 bulan |
| `VPS_HOST` | IP VPS | Tidak berubah |
| `VPS_USER` | User SSH | Tidak berubah |
| `VPS_TARGET_DIR` | Directory di VPS | Tidak berubah |
| `ENV_FILE_PRODUCTION` | Full `.env` production | Saat ada perubahan |

### Update Secrets dengan aman

```bash
# Baca current .env dari VPS via SSH
ssh root@45.127.35.244 "cat /root/asepharyana-hub/.env" | gh secret set ENV_FILE_PRODUCTION --repo asepharyana/asepharyana-hub --repos
```

### Production `.env` tidak boleh di-commit

`.env` di root repo adalah untuk development lokal. Production `.env` hanya ada di:
1. GitHub secret `ENV_FILE_PRODUCTION`
2. File `/root/asepharyana-hub/.env` di VPS (hasil SCP dari CI/CD)

## TLS / SSL

### Konfigurasi

```yaml
# Traefik TLS certs dari file mount (bukan auto-ACME)
volumes:
  - ${TRAEFIK_CERT_MY_ID_PEM:-/root/asepharyana.my.id.pem}:/etc/traefik/certs/asepharyana.my.id.pem:ro
  - ${TRAEFIK_CERT_MY_ID_KEY:-/root/asepharyana.my.id.key}:/etc/traefik/certs/asepharyana.my.id.key:ro
```

### Best Practices

- Certificates disimpan di host (`/root/`), bukan di repo
- Volume mount read-only (`:ro`)
- Private key hanya bisa dibaca oleh root (chmod 600)
- Renew certificates sebelum expired (monitor expiry)
- Dua domain: `asepharyana.my.id` + `asepharyana.web.id`

## Container Security

### Non-Root User

Scraper API berjalan sebagai `appuser` (UID 1001):

```dockerfile
RUN groupadd -g 1001 appgroup && \
    useradd -u 1001 -g appgroup -s /bin/sh appuser
USER appuser
```

Service baru harus mengikuti pattern yang sama.

### Read-Only Filesystem

Untuk container yang tidak perlu write ke filesystem:

```yaml
services:
  app:
    image: app:latest
    read_only: true
    tmpfs:
      - /tmp
```

### Docker Socket

Hanya Traefik yang perlu akses ke Docker socket (read-only):

```yaml
volumes:
  - /var/run/docker.sock:/var/run/docker.sock:ro
```

Service lain tidak boleh mount Docker socket.

### Image Security

- Build dari base image resmi dan minimal (`debian:bookworm-slim`, `redis:alpine`, `nats:latest`)
- Multi-stage build untuk production image (tidak include build tools)
- Update base image secara berkala

## Network Security

### Firewall (UFW/iptables)

Di VPS (`orangevps`):

```bash
# Hanya buka port yang diperlukan
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp       # SSH
sudo ufw allow 80/tcp       # HTTP redirect
sudo ufw allow 443/tcp      # HTTPS
sudo ufw allow 4222/tcp     # NATS (jika perlu external akses)
sudo ufw enable
```

Di `imrnes`:

```bash
# Hanya dari Tailscale interface
sudo ufw allow in on tailscale0 to any port 6432 proto tcp  # PostgreSQL
sudo ufw allow in on tailscale0 to any port 6379 proto tcp  # Redis
sudo ufw enable
```

### Network Segmentation

- Semua container di network `app-shared-net` (internal bridge)
- Tidak ada port yang di-expose ke host kecuali Traefik (80,443)
- Redis hanya accessible via Docker DNS (`redis:6379`) — tidak di-expose
- Database hanya via Tailscale — tidak accessible dari public internet

### SSH Hardening

Konfigurasi di `/etc/ssh/sshd_config`:

```
Port 22
PermitRootLogin prohibit-password
PasswordAuthentication no
PubkeyAuthentication yes
AllowUsers root
MaxAuthTries 3
ClientAliveInterval 300
ClientAliveCountMax 2
```

## Access Control

### GitHub Repository

- `contents: write` hanya untuk workflow `update-manifest` dan `update-submodule`
- `packages: write` hanya untuk workflow `build`
- `security-events: write` hanya untuk workflow `security`
- Branch protection di `main`: require PR review, status checks

### VPS

- SSH hanya dengan key-based authentication
- Key disimpan di GitHub secret, bukan di repo
- Rotate SSH key secara berkala (minimal 6 bulan)
- Jangan gunakan password login

## Monitoring Keamanan

### Saat Ini

- Traefik access logs (format JSON, buffer size 100)
- Docker logs via `docker logs`
- CodeQL analysis untuk Rust code (setiap PR + weekly)

### Rekomendasi

- [ ] Alert untuk SSH failed login (fail2ban)
- [ ] Log monitoring (Loki / Promtail)
- [ ] Container vulnerability scanning (Trivy / Snyk)
- [ ] Certificate expiry monitoring
- [ ] Disk usage alert
- [ ] Unauthorized access detection

## Checklist Security

- [ ] SSH password authentication disabled
- [ ] Root login via SSH key only
- [ ] UFW/iptables configured
- [ ] Docker socket only mounted where necessary (read-only)
- [ ] Container berjalan sebagai non-root user
- [ ] `.env` tidak di-commit
- [ ] GitHub secrets ter-encrypt
- [ ] TLS certificates valid dan belum expired
- [ ] CodeQL analysis berjalan
- [ ] Backup database berjalan
- [ ] SSH key di-rotate
- [ ] Docker image di-scan untuk vulnerability
