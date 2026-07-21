# Infrastructure

Docker Compose and Traefik configuration for `asepharyana-hub`.

## Layout

```text
infra/
├── compose/                 # One compose file per stack/service
│   ├── traefik.yml          # Public reverse proxy
│   ├── shared.yml           # Shared Redis
│   └── scraper.yml          # Scraper API
├── docker/                  # Dockerfiles and image runtime helpers
├── traefik/                 # Static and dynamic Traefik configuration
│   ├── dynamic/             # Routers, services, middlewares, TLS certs
│   └── TRAEFIK_ENV_CONFIG.md
└── config/                  # Service bootstrap configuration
```

Archived configs that are not deployed live under `docs/config/`.

## First-time setup

Create the shared Docker network before starting any service:

```bash
docker network create app-shared-net
```

Create `.env` from `.env.example` and fill production values. Do not commit `.env`.

## Deployment order

The GitHub deploy workflow combines the active compose files automatically. For manual deployment, use this order:

```bash
docker compose -f infra/compose/shared.yml up -d
docker compose -f infra/compose/traefik.yml up -d

docker compose \
  -f infra/compose/scraper.yml \
  up -d
```

## Environment variables

Common variables used by infra compose files:

```env
DATABASE_URL=
GITHUB_TOKEN=
SHARED_REDIS_EXPOSE=127.0.0.1:6379:6379
```

Traefik certificate path variables are optional because `infra/compose/traefik.yml` provides production-compatible defaults. See `infra/traefik/TRAEFIK_ENV_CONFIG.md` for the full list.

## Traefik

Traefik reads dynamic config from `infra/traefik/dynamic/`:

- `apps.yaml` — routers and upstream services
- `middlewares.yaml` — shared middleware chains
- `ssl.yaml` — TLS certificates for `asepharyana.my.id` and `asepharyana.web.id`

## Validation

Run syntax checks after editing infra YAML:

```bash
python - <<'PY'
import pathlib, yaml
for path in pathlib.Path('infra').rglob('*.yml'):
    with path.open() as fh:
        yaml.safe_load(fh)
    print(f'OK {path}')
for path in pathlib.Path('infra').rglob('*.yaml'):
    with path.open() as fh:
        yaml.safe_load(fh)
    print(f'OK {path}')
PY
```

Check compose rendering when Docker is available:

```bash
for f in infra/compose/*.yml; do
  docker compose -f "$f" config >/dev/null && echo "OK $f"
done
```
