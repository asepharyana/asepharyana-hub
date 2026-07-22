# Development Guide

Panduan setup lingkungan development lokal untuk kontributor `asepharyana-hub`.

## Prasyarat

| Tool | Versi Minimal | Catatan |
|------|---------------|---------|
| Git | 2.40+ | Submodule support |
| Docker | 24+ | Dengan Docker Compose v2 plugin |
| Rust | 1.85+ | Hanya untuk `apps/scraper` |
| Bun | 1.x | Root tooling (Biome) |
| Dapr CLI | 1.14+ | Opsional, untuk development dengan Dapr |

## Setup Awal

```bash
# 1. Clone repo
git clone https://github.com/asepharyana/asepharyana-hub.git
cd asepharyana-hub

# 2. Init submodules
make init-submodules

# 3. Setup environment
cp .env.example .env
# Edit .env sesuai kebutuhan lokal

# 4. Install root dependencies
bun install
```

## Menjalankan Infrastruktur Lokal

Beberapa service membutuhkan Redis. Jalankan dengan:

```bash
make dev
# atau equivalen:
docker compose -f infra/compose/shared.yml up -d
```

Ini akan menjalankan Redis Alpine di `localhost:6379`.

### (Opsional) NATS Lokal

Jika service membutuhkan pub/sub:

```bash
docker compose -f infra/compose/nats.yml up -d
# NATS client: localhost:4222
# NATS monitor: localhost:8222
```

### (Opsional) Dapr Placement Lokal

Jika service membutuhkan sidecar Dapr:

```bash
docker compose -f infra/compose/dapr.yml up -d
# Dapr placement: localhost:50005
```

## Menjalankan Service Lokal

### Scraper API (Rust)

```bash
# Pastikan Redis sudah running (make dev)
cd apps/scraper

# Cargo run
cargo run

# Dengan Dapr sidecar (jika placement running)
dapr run \
  --app-id scraper-api \
  --app-port 4091 \
  --dapr-http-port 3500 \
  --resources-path ../../infra/dapr/components \
  -- cargo run
```

### Dengan Docker Compose (Full Stack)

Untuk menjalankan semua service sekaligus:

```bash
docker compose \
  -f infra/compose/shared.yml \
  -f infra/compose/nats.yml \
  -f infra/compose/dapr.yml \
  -f infra/compose/scraper.yml \
  --env-file .env \
  up -d
```

Untuk service baru, tambahkan compose file-nya ke daftar.

## Update Submodules

### Pull latest dari semua submodule

```bash
make update-submodules
# atau:
git submodule update --remote --merge --recursive
```

### Check status submodule

```bash
make status
# atau:
git submodule status
```

### Sync .env ke submodule

```bash
bash scripts/2updateenv.sh
# Copy .env root ke apps/*/
```

## Linting & Formatting

Root repo menggunakan **Biome** untuk linting dan formatting:

```bash
bun run check    # Lint + format + write
bun run ci       # CI mode (no write, exit code on issues)
bun run lint     # Lint only
bun run format   # Format only
```

## Build Docker Image Lokal

```bash
# Scraper API
docker build -f infra/docker/scraper.Dockerfile -t scraper-api:local .

# Service baru: tambahkan Dockerfile di infra/docker/
```

## Testing

Saat ini belum ada test runner di root level. Masing-masing submodule mengelola testing sendiri:

```bash
# Scraper API (Rust)
cd apps/scraper && cargo test
```

## Validasi YAML

Sebelum commit perubahan infra, validasi semua file YAML:

```bash
python -c "
import pathlib, yaml
for p in pathlib.Path('infra').rglob('*.yml'):
    with open(p) as f: yaml.safe_load(f)
    print(f'OK {p}')
for p in pathlib.Path('infra').rglob('*.yaml'):
    with open(p) as f: yaml.safe_load(f)
    print(f'OK {p}')
"

for f in infra/compose/*.yml; do
  docker compose -f "$f" config >/dev/null && echo "OK $f"
done
```

## Git Workflow

### Commit Convention

```
<type>(<scope>): <description>
```

Type: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `style`
Scope: `scraper`, `infra`, `ci`, `dapr`, `nats`, `docs`, `deps`, `scripts`, `root`

Contoh:
```
feat(scraper): add image cache endpoint
fix(infra): correct Traefik rate-limit config
chore(deps): bump biome to 2.5.0
```

### Branch Strategy

- `main` — production branch, push triggers CI/CD
- Fitur baru: branch dari `main`, PR ke `main`
- Submodule development: dilakukan di repo masing-masing, hub hanya update pointer

## Deployment ke VPS

Push ke `main` otomatis trigger CI/CD. Untuk trigger manual:

```bash
gh workflow run deploy-docker.yml
```

Lihat `docs/DEPLOYMENT.md` untuk detail.
