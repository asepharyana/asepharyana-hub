# ADR 0003: Rename Repositori & Pisahkan CI per Aplikasi

## Status

Accepted (2026-08-28)

## Context

Monorepo `asepharyana-hub` (app submodule + infra + CI terpusat) punya kelemahan:
- CI build+deploy semua app menyatu di parent (`nix-build.yml` matrix) — setiap push app
  butuh 2 hop (notify-parent → update-submodule → nix-build), rawan drift pointer.
- Nama `asepharyana-hub` ambigu (parent & app prefix sama), dan submodule menambah kompleksitas.

## Decision

Rombak total:

| Lama | Baru | Peran |
|------|------|-------|
| `asepharyana-hub` | `asepharyana/infra` | Reverse proxy (Caddy) + firewall + systemd + docs. CI: caddy-deploy saja. |
| `asepharyana-hub-hub` | `asepharyana/hub` | Portfolio SPA. CI mandiri (deploy.yml). |
| `asepharyana-hub-scraper` | `asepharyana/scraper` | Rust scraper API. CI mandiri. |
| `asepharyana-hub-tools` | `asepharyana/tools` | Tools stack (gateway/workers/frontend). CI mandiri. |
| `asepharyana-hub-llm-api` | `asepharyana/llm-api` | Rust LLM API. CI mandiri. |
| `asepharyana-hub-guide` | `asepharyana/hub-guide` | (tidak di-root; plugin guide — diarsipkan) |

Setiap app repo mendapat:
- `flake.nix` (derivasi build sendiri, tanpa fetchGit submodule)
- `.github/workflows/deploy.yml` (nix build → nix copy → nix-env --profile → systemctl restart)
- Secret `SSH_PRIVATE_KEY`, `VPS_HOST`, `VPS_USER`

Parent `infra` mendapat:
- Hapus semua submodule + `update-submodule.yml` + matrix `nix-build.yml`
- `.github/workflows/caddy-deploy.yml` (sync Caddyfile → reload → verify)
- Docs diarahkan ulang.

## Consequences

- **Positif**: CI per-app independen & cepat; parent kecil; tanpa submodule = tanpa fetchGit pin
  / dubious-ownership / pointer churn. Rename GitHub auto-redirect URL lama.
- **Negatif**: koordinasi manual bila app butuh port baru di Caddy; workflow lama di
  downstream (skill/cron) perlu update referensi.

## Referensi

- `docs/add-new-app.md` — proses menambah service baru
- `infra/caddy/Caddyfile.prod` — pola site block