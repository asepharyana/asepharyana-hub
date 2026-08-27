# Asepharyana Infra

Reverse-proxy & infrastructure config for [orangevps](https://asepharyana.my.id) (45.127.35.244).

> **Status (2026-08-28):** Repo ini dulunya monorepo `asepharyana-hub` dengan submodule aplikasi.
> Kini **murni repo infra**: Caddy reverse proxy (source of truth), firewall, drop-in systemd,
> dan docs. Build + deploy tiap aplikasi pindah ke repo masing-masing (self-contained CI).

## Repositori Aplikasi (self-contained build & deploy)

| Repo | Deskripsi | Deploy unit |
|------|-----------|-------------|
| [`asepharyana/hub`](https://github.com/asepharyana/hub) | Portfolio SPA (Next.js, port 4003, dashboard) | `hub` |
| [`asepharyana/scraper`](https://github.com/asepharyana/scraper) | Rust/Axum scraper API (port 4091) | `scraper` |
| [`asepharyana/llm-api`](https://github.com/asepharyana/llm-api) | Rust LLM API (llama.cpp, port 8080) | `llm-api` |

Tiap repo punya `flake.nix` + `.github/workflows/deploy.yml` sendiri:
`nix build .#<pkg>` → **push ke Attic binary cache** (`attic.asepharyana.my.id/asepharyana`) → VPS substitute via `nix-store --realise` → `nix-env --profile` → `systemctl restart`.
Push ke `main` (atau `workflow_dispatch`) langsung deploy; tidak ada lagi pointer submodule.

## Infra di Repo Ini

| Path | Isi |
|------|-----|
| `infra/caddy/Caddyfile.prod` | **Source of truth** `/etc/caddy/Caddyfile` (auto-deploy via CI) |
| `infra/firewall/firewall.sh` | deny-by-default iptables (SSH/80/443/4013/Tailscale/TCPShield) |
| `infra/firewall/99-*.conf` | sysctl hardenings |
| `infra/prometheus/targets.yml` | file_sd targets |
| `infra/systemd/scraper-otel.conf` | drop-in OTEL untuk scraper service |
| `docs/` | arsitektur + operasional (VPS) |

## CI/CD

| Workflow | Trigger | Aksi |
|----------|---------|------|
| `caddy-deploy.yml` | push main menyentuh `infra/**`, atau manual | sync `Caddyfile.prod` → `/etc/caddy/Caddyfile` → reload → verifikasi rute |

## Local Setup / Snapshot VPS

```bash
# Clone infra repo
git clone https://github.com/asepharyana/infra.git
# Diff config live vs repo
diff /etc/caddy/Caddyfile infra/caddy/Caddyfile.prod
# Koneksi VPS (public)
ssh code@45.127.35.244
```

## Menambahkan Service Baru / Subdomain

1. Aplikasi punya repo sendiri + `deploy.yml` (lihat template di repo app yang ada).
2. Registrasi unit systemd di VPS (manual/ops) → app jalan di port lokal.
3. Tambah site block di `infra/caddy/Caddyfile.prod` (pola `import proxy <port>`) → push → CI reload Caddy.
4. (Opsional) Tambah unit ke `MONITORED_UNITS` dashboard hub di repo `asepharyana/hub`.

Lihat `docs/add-new-app.md` untuk detail.