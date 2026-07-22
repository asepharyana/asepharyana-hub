# CI/CD Pipeline

Dokumentasi pipeline CI/CD untuk `asepharyana-hub`. Terdiri dari 5 GitHub Actions workflow yang saling terhubung.

## Workflow Overview

```
                        ┌─────────────┐
                        │  Lint       │ (PR/push → Biome)
                        └──────┬──────┘
                               │
             Push ke main ─────┼────── repository_dispatch
                               │
                        ┌──────▼──────────────────┐
                        │  docker-build-push.yml   │
                        │                         │
                        │  Phase 1: Detect        │
                        │  Phase 2: Build & Push  │
                        │  Phase 3: Update        │
                        │    manifests            │
                        └──────┬──────────────────┘
                               │ workflow_run
                        ┌──────▼──────────────┐
                        │  deploy-docker.yml   │
                        │  SSH → VPS           │
                        │  Pull → Restart      │
                        └─────────────────────┘

  repository_dispatch ──► update-submodule.yml
    (dari submodule)        (update pointer → commit)
                                  │
                                  ▼
                          docker-build-push.yml
                          (triggered by push)
```

## Workflow Detail

### 1. Lint (`lint.yml`)

**Trigger:** PR/push ke `main` yang mengubah `*.json`, `*.js`, `biome.json`

**Aksi:**
- Checkout repo dengan submodules
- Setup Bun
- `bun install --frozen-lockfile`
- `bun run ci` (Biome CI mode)

**Permissions:** read-only

### 2. Build and Push Docker Images (`docker-build-push.yml`)

**Trigger:**
- Push ke `main` yang mengubah `apps/**`, `infra/**`, atau file workflow
- `repository_dispatch` tipe `submodule-updated`
- `workflow_dispatch` (manual)

**Concurrency:** Satu workflow per branch (cancel-in-progress=false)

#### Phase 1: Detect Changes

Job `changes` mendeteksi service mana yang perlu di-build:

- **Push event:** `git diff --name-only` antara `before` dan `after` SHA
- **repository_dispatch:** Parse payload `{service, sha}` dan validasi
- **workflow_dispatch:** Build semua service

Output format matrix:
```json
[{"id":"scraper-api","target":"docker-scraper","path":"apps/scraper"}]
```

#### Phase 2: Build & Push (Matrix)

Job `build` berjalan paralel per service (matrix strategy):

1. Checkout repo + sync submodule
2. Jika `repository_dispatch`, checkout submodule ke SHA tertentu
3. Login ke GHCR
4. Setup Docker Buildx
5. Build & push dengan tag:
   - `ghcr.io/asepharyana/asepharyana-hub/<service>:latest`
   - `ghcr.io/asepharyana/asepharyana-hub/<service>:sha-<shortsha>`
6. Build cache: registry-based (`:<service>:buildcache`)

#### Phase 3: Update Manifests

Job `update-manifest`:

1. Update image tag di compose file (`infra/compose/<service>.yml`)
2. Jika `repository_dispatch`, update submodule pointer
3. Commit dengan message `chore: update manifests and submodules [skip ci]`
4. Push dengan retry (3 attempts, rebase jika conflict)

### 3. Deploy Docker to VPS (`deploy-docker.yml`)

**Trigger:**
- `workflow_run` setelah `docker-build-push.yml` selesai
- Push ke `main` yang mengubah `infra/**`
- `workflow_dispatch` (manual)

**Concurrency:** Satu deployment dalam satu waktu (`group: deploy-vps`)

**Aksi di VPS (via SSH):**

```
1. Setup SSH multiplexing
2. SCP .env dari GitHub secret ke VPS
3. Docker login ke GHCR
4. Git sync (fetch + reset --hard)
5. Detect changed files:
   ├─ Compose stack changes → selective container update
   ├─ Traefik dynamic config → SIGHUP
   └─ Other infra → full deploy
6. Pull images (retry 3x)
7. Remove stale containers
8. Up services
9. SIGHUP Traefik jika perlu
```

### 4. Security Scan (`security.yml`)

**Trigger:**
- PR ke `main`
- Jadwal: Setiap Senin (`0 6 * * 1`)

**Aksi:**
- Checkout dengan fetch-depth 2
- CodeQL init untuk Rust
- `cargo build` di `apps/scraper`
- CodeQL analyze

### 5. Update Submodule Pointer (`update-submodule.yml`)

**Trigger:** `repository_dispatch` tipe `submodule-updated`

**Aksi:**
1. Validasi payload (`service`, `sha`)
2. Map service ke submodule path (e.g., `scraper-api` → `apps/scraper`)
3. Update submodule ke SHA yang diberikan
4. Commit sebagai `monrepo-bot` dengan message:
   `chore: update <service> to <shortsha>`
5. Push dengan retry (3 attempts)

## Flow Submodule Update

Flow lengkap ketika code berubah di submodule repo:

```
1. Developer push ke asepharyana-hub-scraper
2. GitHub Action di scraper repo kirim repository_dispatch
   ke asepharyana-hub
3. update-submodule.yml terima dispatch, update pointer
4. Commit masuk ke hub repo main
5. Commit ini trigger docker-build-push.yml
   (push ke main dengan path apps/scraper/**)
6. Build image baru, update compose file
7. Deploy ke VPS
```

## Secrets yang Diperlukan

| Secret | Workflow | Deskripsi |
|--------|----------|-----------|
| `SSH_PRIVATE_KEY` | deploy-docker | SSH key untuk akses VPS |
| `VPS_HOST` | deploy-docker | IP VPS (`45.127.35.244`) |
| `VPS_USER` | deploy-docker | User SSH (`root`) |
| `VPS_TARGET_DIR` | deploy-docker | Dir di VPS (`/root/asepharyana-hub`) |
| `ENV_FILE_PRODUCTION` | deploy-docker | Full `.env` production |

## Menambahkan Service Baru ke Pipeline

Untuk menambahkan service baru, update:

### `docker-build-push.yml`

1. **Phase 1 — `changes` job:** Tambah detection logic untuk service baru:

```yaml
echo "new-service=$(changed '^(apps/new-service(/|$)|\.github/workflows/docker-build-push\.yml$|infra/docker/new-service\.Dockerfile$)')" >> "$GITHUB_OUTPUT"
```

2. **Phase 1 — `repository_dispatch`:** Tambah case:

```yaml
case "$SERVICE" in
  scraper-api|new-service) ;;
```

3. **Phase 1 — `set-matrix`:** Tambah service:

```bash
if [ "${{ ...['new-service'] == 'true' ... }}" == "true" ]; then add_service "new-service" "docker-new-service" "apps/new-service"; fi
```

4. **Phase 2 — `meta` step:** Tambah mapping Dockerfile:

```bash
"new-service") echo "dockerfile=infra/docker/new-service.Dockerfile" >> $GITHUB_OUTPUT ;;
```

5. **Phase 3 — `update-manifest`:** Tambah mapping:

```bash
SERVICES["new-service"]="new-service.yml"
PATHS["new-service"]="apps/new-service"
```

### `deploy-docker.yml`

Tambah compose file ke `ALL_COMPOSE_FILES`:

```bash
ALL_COMPOSE_FILES="infra/compose/traefik.yml infra/compose/shared.yml infra/compose/scraper.yml infra/compose/nats.yml infra/compose/dapr.yml infra/compose/new-service.yml"
```

## Rollback

### Rollback Image

```bash
# Cari SHA tag sebelumnya di GHCR packages
# Update compose file ke tag tersebut
sed -i 's|sha-badcommit|sha-goodcommit|g' infra/compose/scraper.yml
git commit -am "fix: rollback scraper-api to sha-goodcommit"
git push
```

### Rollback via Git Revert

```bash
git revert HEAD
git push origin main
# Pipeline otomatis build dan deploy
```

## Monitoring Pipeline

```bash
# Cek status workflow terbaru
gh run list --limit 5

# Lihat log workflow tertentu
gh run view <run-id> --log

# Trigger workflow manual
gh workflow run deploy-docker.yml
```
