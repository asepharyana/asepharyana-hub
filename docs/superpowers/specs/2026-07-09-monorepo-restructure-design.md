# Monorepo Restructure: ultimate-asepharyana.tech → asepharyana-hub

**Date:** 2026-07-09
**Status:** Approved
**Owner:** @asepharyana

## Background

Proyek ini sebelumnya bernama `ultimate-asepharyana.tech` — sebuah monorepo yang berisi beberapa service aplikasi, infrastruktur, dokumentasi, dan utility scripts. Karena GitHub org `MythEclipse` kena banned, semua remote repos tidak bisa diakses. Perlu dilakukan restruktur dan rename project secara menyeluruh.

## Goals

1. Rename project dari `ultimate-asepharyana.tech` ke `asepharyana-hub`
2. Hapus service docker-manager dan teleuploader dari proyek
3. Hapus semua pointer `.git` submodule (clean slate)
4. Update `.gitmodules` dengan remote baru ke `github.com/asepharyana/*`
5. Update semua referensi: workflows, konfigurasi, dokumentasi, image names
6. Siapkan struktur lokal — inisialisasi git dan push dilakukan terpisah

## Service yang tetap dipertahankan

| Service | Path | Git remote baru |
|---------|------|----------------|
| Elysia API | `apps/elysia` | `asepharyana/asepharyana-hub-elysia` |
| React Frontend | `apps/react` | `asepharyana/asepharyana-hub-react` |
| Scraper | `apps/scraper` | `asepharyana/asepharyana-hub-scraper` |
| Rust Auth | `apps/rust-auth` | `asepharyana/asepharyana-hub-rust-auth` |

## Service yang dihapus

| Service | Path |
|---------|------|
| Docker Manager | `apps/docker-manager/` |
| TeleUploader | `apps/teleuploader/` |

## File yang akan dihapus

- `apps/docker-manager/` (seluruh direktori)
- `apps/teleuploader/` (seluruh direktori)
- `infra/compose/docker-manager.yml`
- `infra/compose/teleuploader.yml`
- `infra/docker/docker-manager.Dockerfile`
- `infra/docker/teleuploader.Dockerfile`

## File pointer `.git` submodule yang dihapus

- `apps/elysia/.git`
- `apps/react/.git`
- `apps/scraper/.git`
- `apps/rust-auth/.git`

## Rename mapping

| Lokasi | Dari | Ke |
|--------|------|----|
| `package.json` `name` | `ultimate-asepharyana.tech` | `asepharyana-hub` |
| `README.md` | judul & path references | `asepharyana-hub` |
| `ARCHITECTURE.md` | directory tree, paths | `asepharyana-hub` |
| `CONTRIBUTING.md` | repo names & paths | `asepharyana-hub-*` |
| `infra/README.md` | deskripsi | `asepharyana-hub` |
| `.env.example` | `TRAEFIK_CONFIG_PATH` | `asepharyana-hub` |
| Perlengkapan infra Traefik | path config references | `asepharyana-hub` |
| `scripts/cleanup-ghcr.sh` | `MythEclipse` | `asepharyana` |
| `docs/add-new-app.md` | path references | `asepharyana-hub` |
| `docs/adr/*.md` | path references | `asepharyana-hub` |

## Image name migration (GHCR)

| Lama | Baru |
|------|------|
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/elysia-api:*` | `ghcr.io/asepharyana/asepharyana-hub/elysia-api:*` |
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/react-web:*` | `ghcr.io/asepharyana/asepharyana-hub/react-web:*` |
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/scraper-api:*` | `ghcr.io/asepharyana/asepharyana-hub/scraper-api:*` |
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/rust-auth:*` | `ghcr.io/asepharyana/asepharyana-hub/rust-auth:*` |
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/docker-manager:*` | — (dihapus) |
| `ghcr.io/mytheclipse/ultimate-asepharyana.tech/teleuploader:*` | — (dihapus) |

## Workflow changes

### `docker-build-push.yml`
- `IMAGE_NAME_PREFIX`: `mytheclipse/ultimate-asepharyana.tech` → `asepharyana/asepharyana-hub`
- Hapus service entries: `docker-manager`, `teleuploader`
- Update repo URLs di `wait-submodule-ref` dari `MythEclipse/*` ke `asepharyana/*`
- Update `update-manifest` phase — hapus service docker-manager & teleuploader

### `deploy-docker.yml`
- Update `git remote add origin`
- Hapus docker-manager & teleuploader dari compose list dan checkout

### `update-submodule.yml`
- Hapus service docker-manager & teleuploader
- Update nama workflow

## `.gitmodules`

Hanya berisi 4 apps dengan remote baru:

```ini
[submodule "apps/elysia"]
	path = apps/elysia
	url = https://github.com/asepharyana/asepharyana-hub-elysia.git
[submodule "apps/react"]
	path = apps/react
	url = https://github.com/asepharyana/asepharyana-hub-react.git
[submodule "apps/scraper"]
	path = apps/scraper
	url = https://github.com/asepharyana/asepharyana-hub-scraper.git
[submodule "apps/rust-auth"]
	path = apps/rust-auth
	url = https://github.com/asepharyana/asepharyana-hub-rust-auth.git
```

## Execution plan

1. Hapus docker-manager & teleuploader direktori + file infra
2. Hapus `.git` pointer di submodule
3. Update `.gitmodules`
4. Update `package.json`
5. Update `README.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`
6. Update `infra/compose/*.yml` image tags
7. Update `.env.example`, `infra/README.md`, docs traefik
8. Update `docker-build-push.yml`
9. Update `deploy-docker.yml`
10. Update `update-submodule.yml`
11. Update `scripts/cleanup-ghcr.sh`
12. Update `docs/add-new-app.md`
13. Update `docs/adr/*.md` path references (historical)

## Post-execution state

- Root direktori `asepharyana-hub/` dengan source code apps utuh (tanpa git)
- 4 app submodule terdaftar di `.gitmodules` dengan remote baru
- Infra/docs/scripts tetap menyatu di root
- 0 references ke `MythEclipse/ultimate-asepharyana.tech` di file konfigurasi
- Siap untuk `git init && git add && git commit` kapan saja
