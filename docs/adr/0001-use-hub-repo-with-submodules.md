# ADR 0001: Infra Repo — Reverse Proxy Config Only (Submodules Removed)

## Status

**Superseded** (2026-08-28) — lihat ADR ini sebagai arsip keputusan awal.

## Context (aslinya)

Proyek awal memakai `asepharyana-hub` sebagai monorepo: aplikasi di `apps/<service>` sebagai
git submodule, infra (compose/traefik/dokumen/CI) terpusat di root. Keputusan itu masuk akal
saat semua service berbagi satu deployment surface.

## Decision (aslinya)

Gunakan `asepharyana-hub` sebagai root hub: app code submodule, infra + CI di root.

## Superseded By

Mulai **2026-08-28** repo dirombak:

- **Parent `asepharyana-hub` → `asepharyana/infra`** — murni config reverse proxy (Caddy),
  firewall, systemd drop-ins, docs. CI hanya untuk deploy Caddy.
- **App repos di-rename & self-contained**: `hub`, `scraper`, `tools`, `llm-api`.
  Masing-masing punya `flake.nix` + `.github/workflows/deploy.yml` sendiri
  (`nix build → nix copy → nix-env --profile → systemctl restart`).
- **Submodule dihapus** — tidak ada lagi pointer submodule / repository_dispatch chain.
- `update-submodule.yml`, `notify-parent.yml`, matrix `nix-build.yml` dihapus.

## Consequences

### Positive
- CI tiap app independen: push ke repo app langsung build+deploy, tak perlu 2 hop.
- Parent kecil & fokus: diff Caddyfile mudah di-audit.
- Tanpa submodule = tanpa `dubious ownership` / pointer drift / fetchGit pin.

### Negative
- Koordinasi cross-repo manual (app + Caddy bila perlu port baru).
- Repo lama `asepharyana-hub-*` redirect ke nama baru (GitHub auto).