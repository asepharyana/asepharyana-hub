# Menambahkan Service Baru / Subdomain

Panduan untuk menambahkan service baru di ekosistem `asepharyana/infra` (2026-08-28+, pasca monorepo).

## Prinsip

- **Aplikasi hidup di repo sendiri** (`hub`, `scraper`, `tools`, `llm-api`) dengan
  `flake.nix` + `.github/workflows/deploy.yml` mandiri. Repo infra TIDAK berisi kode app.
- Repo infra (`asepharyana/infra`) hanya mengatur **reverse proxy & config VPS**.

## Langkah

1. **Buat repo aplikasi** (contoh pola: `asepharyana/scraper`).
2. **Tambahkan `flake.nix`** di repo app — derivasi Nix (lihat template di repo app yang ada:
   Next.js/bun atau Rust/cargo). Nama paket = nama unit systemd.
3. **Tambahkan `.github/workflows/deploy.yml`** (pola `nix build .#<pkg>` → `nix copy ssh://`
   → `nix-env --profile /nix/var/nix/profiles/<pkg> --set` → `systemctl restart <pkg>`).
   Secrets yang dibutuhkan: `SSH_PRIVATE_KEY`, `VPS_HOST`, `VPS_USER`.
4. **Di VPS**: buat user systemd + unit (mis. `/etc/systemd/system/<app>.service`,
   `ExecStart=/usr/local/bin/bws-exec <app> /nix/var/nix/profiles/<app>/bin/<app>`),
   pastikan app jalan di port lokal.
5. **Tambah site block** di `infra/caddy/Caddyfile.prod` (pola `import proxy <port>`),
   push ke `main` → CI `caddy-deploy.yml` sync + reload + verifikasi rute.
6. **(Opsional)** Tambah unit ke `MONITORED_UNITS` di hub dashboard
   (repo `asepharyana/hub`, `src/app/api/dashboard/route.ts`).

## Contoh site block Caddy

```caddyfile
nama-app.asepharyana.my.id {
	import proxy <PORT>
}
```

## Verifikasi

```bash
# Dari local
curl -s -o /dev/null -w '%{http_code}\n' https://nama-app.asepharyana.my.id/

# Dari VPS
systemctl status <app>