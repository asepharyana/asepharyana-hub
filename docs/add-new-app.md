# Menambahkan Aplikasi Baru ke Deployment

Dokumen ini menjelaskan langkah menambahkan service baru ke `asepharyana-hub`. Root repo berfungsi sebagai hub: source aplikasi berada di `apps/<nama-app>` sebagai submodule, sedangkan Docker Compose, Traefik, dan workflow deploy tetap berada di root repo.

## 1. Buat repo aplikasi

Buat repo baru di GitHub dengan pola nama:

```text
https://github.com/asepharyana/asepharyana-hub-<nama-app>.git
```

Lalu tambahkan ke root hub sebagai submodule:

```bash
git submodule add https://github.com/asepharyana/asepharyana-hub-<nama-app>.git apps/<nama-app>
git submodule update --init --recursive
```

## 2. Tambahkan Dockerfile

Tambahkan Dockerfile runtime di `infra/docker/<nama-app>.Dockerfile`.

Gunakan root repo sebagai build context agar Dockerfile bisa mengakses submodule path:

```bash
docker build -f infra/docker/<nama-app>.Dockerfile -t <nama-app>:local .
```

## 3. Tambahkan Compose file

Buat `infra/compose/<nama-app>.yml`:

```yaml
services:
  <nama-app>:
    container_name: <nama-app>
    image: ghcr.io/asepharyana/asepharyana-hub/<nama-app>:sha-<short-sha>
    restart: always
    networks:
      app-shared-net:
        aliases:
          - <nama-app>
    env_file:
      - ../../.env

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

Gunakan `app-shared-net` agar service dapat diakses oleh Traefik dan service lain.

## 3.5. Tambahkan Dapr sidecar (wajib untuk pub/sub)

Setiap service yang ingin menggunakan Dapr pub/sub atau service invocation harus punya sidecar.
Tambah di `infra/compose/<nama-app>.yml`:

```yaml
  <nama-app>-dapr:
    container_name: <nama-app>-dapr
    image: daprio/daprd:latest
    restart: always
    depends_on:
      dapr-placement:
        condition: service_healthy
      nats:
        condition: service_healthy
      otel-collector:
        condition: service_started
    networks:
      - app-shared-net
    command:
      - './daprd'
      - '--app-id=<nama-app>'
      - '--app-port=<port>'
      - '--dapr-http-port=3500'
      - '--dapr-grpc-port=50001'
      - '--placement-host-address=dapr-placement:50005'
      - '--config=/dapr/config.yaml'
      - '--resources-path=/dapr/components'
    volumes:
      - ../../infra/dapr:/dapr:ro
```

Pastikan juga app container punya `depends_on` ke dapr-placement, nats, dan otel-collector:
```yaml
    depends_on:
      dapr-placement:
        condition: service_healthy
      nats:
        condition: service_healthy
      otel-collector:
        condition: service_started
```

## 4. Tambahkan route Traefik

Update `infra/traefik/dynamic/apps.yaml`:

```yaml
http:
  routers:
    <nama-app>:
      rule: 'Host(`<subdomain>.asepharyana.my.id`) || Host(`<subdomain>.asepharyana.web.id`)'
      entryPoints:
        - websecure
      tls: {}
      middlewares:
        - common-chain@file
      service: <nama-app>-service

  services:
    <nama-app>-service:
      loadBalancer:
        servers:
          - url: 'http://<nama-app>:<port>'
```

## 5. Update workflow build

Update `.github/workflows/docker-build-push.yml`:

1. Tambahkan path detection untuk `apps/<nama-app>` dan `infra/docker/<nama-app>.Dockerfile`.
2. Tambahkan service ke matrix build.
3. Tambahkan mapping Dockerfile di step `Docker metadata`.
4. Tambahkan mapping compose file dan submodule path di step `Update tags and submodules`.

## 6. Update workflow deploy

Tambahkan compose file baru ke `ALL_COMPOSE_FILES` di `.github/workflows/deploy-docker.yml`:

```bash
infra/compose/<nama-app>.yml
```

## 7. Update dokumentasi

Update file berikut bila service baru mengubah arsitektur publik:

- `README.md`
- `ARCHITECTURE.md`
- `infra/README.md`
- `.gitmodules`

## 8. Validasi

Jalankan validasi YAML dan compose rendering:

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

for f in infra/compose/*.yml; do
  docker compose -f "$f" config >/dev/null && echo "OK $f"
done
```
