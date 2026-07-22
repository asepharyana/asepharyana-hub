# Tailscale Networking

Dokumentasi setup dan troubleshooting konektivitas Tailscale antara node `orangevps` (VPS) dan `imrnes` (bare-metal).

## Topologi

```
orangevps (VPS)
  ├─ Tailscale IP: 100.x.x.x (dynamic)
  ├─ Public IP: 45.127.35.244
  ├─ Docker containers (app-shared-net)
  │    └─ perlu akses ke imrnes via Tailscale
  └─ tailscale-routes.service
       └─ menambahkan route 100.x.x.x ke tabel routing main

imrnes (Bare-metal)
  ├─ Tailscale IP: 100.121.180.82
  ├─ Layanan:
  │    ├─ PostgreSQL (port 6432)
  │    └─ Redis (port 6379)
  └─ Layanan hanya listen di Tailscale interface
```

## Masalah: Container Tidak Bisa Mencapai Tailscale IP

Docker container secara default hanya bisa mencapai IP di Docker bridge network dan network host. Tailscale menggunakan interface virtual `tailscale0` yang tidak secara otomatis di-route ke container.

### Solusi: `tailscale-routes.service`

Service systemd yang menambahkan route Tailscale ke tabel routing `main` agar traffic dari container bisa melewati host ke Tailscale.

```ini
# /etc/systemd/system/tailscale-routes.service
[Unit]
Description=Add Tailscale routes to main routing table
After=tailscaled.service
Requires=tailscaled.service

[Service]
Type=oneshot
RemainAfterExit=yes
ExecStart=/bin/sh -c 'ip rule add from all lookup main priority 10000 2>/dev/null; ip route add 100.64.0.0/10 dev tailscale0 table main 2>/dev/null || true'
ExecStop=/bin/sh -c 'ip rule del from all lookup main priority 10000 2>/dev/null; ip route del 100.64.0.0/10 dev tailscale0 table main 2>/dev/null || true'
RemainAfterExit=yes

[Install]
WantedBy=multi-user.target
```

### Verifikasi

```bash
# Cek apakah route sudah ada
ip route show table main | grep tailscale

# Test dari dalam container
docker run --rm alpine ping -c 3 100.121.180.82

# Test koneksi PostgreSQL dari container
docker run --rm alpine sh -c "apk add postgresql-client && psql -h 100.121.180.82 -p 6432 -U asephs -d hub -c 'SELECT 1'"
```

## Setup Tailscale di Node Baru

### 1. Install Tailscale

```bash
curl -fsSL https://tailscale.com/install.sh | sh
```

### 2. Authenticate

```bash
sudo tailscale up --advertise-routes=<LAN_SUBNET_CIDR>
```

Untuk node yang hanya sebagai client (tidak advertise routes):

```bash
sudo tailscale up
```

### 3. Enable dan Start

```bash
sudo systemctl enable --now tailscaled
```

### 4. Setup Route Service (khusus node dengan Docker)

```bash
# Buat service file
sudo nano /etc/systemd/system/tailscale-routes.service
# Paste content di atas

sudo systemctl daemon-reload
sudo systemctl enable --now tailscale-routes.service
```

### 5. Konfigurasi ACL di Tailscale Admin

Pastikan ACL di [Tailscale Admin Console](https://login.tailscale.com/admin/acls) mengizinkan traffic antar node:

```json
{
  "acls": [
    {"action": "accept", "src": ["*"], "dst": ["*:*"]}
  ]
}
```

Atau jika ingin lebih ketat:

```json
{
  "acls": [
    {"action": "accept", "src": ["tag:server"], "dst": ["tag:server:*"]}
  ]
}
```

## Konfigurasi iptables/ufw

Pastikan port yang diperlukan terbuka di `imrnes`:

```bash
# PostgreSQL
sudo ufw allow in on tailscale0 to any port 6432 proto tcp

# Redis
sudo ufw allow in on tailscale0 to any port 6379 proto tcp
```

Atau menggunakan iptables langsung:

```bash
sudo iptables -A INPUT -i tailscale0 -p tcp --dport 6432 -j ACCEPT
sudo iptables -A INPUT -i tailscale0 -p tcp --dport 6379 -j ACCEPT
```

## Troubleshooting

### Container timeout connect ke Tailscale IP

```bash
# 1. Cek apakah route service berjalan
systemctl status tailscale-routes.service

# 2. Cek route di host
ip route show table main | grep 100.64

# 3. Cek apakah host bisa ping ke target
ping 100.121.180.82

# 4. Test dari container dengan --network host
docker run --rm --network host alpine ping -c 3 100.121.180.82

# 5. Pastikan tidak ada firewall blocking
iptables -L FORWARD -n -v
```

### Tailscale disconnect

```bash
# Cek status
tailscale status

# Restart
sudo systemctl restart tailscaled
```

### IP Tailscale berubah

Tailscale IP bisa berubah jika node dire-auth. Update:

1. `.env` production di VPS (via GitHub secret `ENV_FILE_PRODUCTION`)
2. Database connection strings
3. Redis connection strings
4. Trigger redeploy

### MagicDNS tidak resolve

```bash
# Cek DNS
tailscale dns status

# Flush DNS cache
sudo resolvectl flush-caches
```

## Catatan Keamanan

- Interface Tailscale (`tailscale0`) hanya boleh diakses oleh node yang terautentikasi dalam network yang sama
- Jangan expose port database ke public interface (`eth0`), hanya ke Tailscale
- Gunakan ACL untuk membatasi akses antar node jika diperlukan
- Rotate auth key secara berkala di Tailscale admin console
