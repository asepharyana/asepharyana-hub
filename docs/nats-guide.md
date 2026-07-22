# NATS + JetStream Guide

Dokumentasi konfigurasi, penggunaan, dan troubleshooting NATS di infrastruktur `asepharyana-hub`.

## Arsitektur

NATS berjalan di container `nats` dengan JetStream diaktifkan (`-js`). Data persistent disimpan di volume Docker `nats_data`.

```
Service ──► NATS (port 4222) ──► JetStream (disk)
                 │
                 ├─ Monitoring HTTP: port 8222
                 └─ Client connections: port 4222
```

### Hubungan dengan Dapr

Saat ini Dapr pub/sub menggunakan **Redis** (`pubsub.redis`), bukan NATS. NATS berfungsi sebagai message broker independen untuk:

- Event streaming antar service
- Persistent job queues
- Pub/sub untuk service yang tidak menggunakan Dapr

Jika ingin Dapr menggunakan NATS sebagai backend pub/sub, ganti komponen `pubsub.yaml`:

```yaml
apiVersion: dapr.io/v1alpha1
kind: Component
metadata:
  name: pubsub
spec:
  type: pubsub.nats
  version: v1
  metadata:
    - name: natsURL
      value: nats://nats:4222
```

## Konfigurasi Compose

File: `infra/compose/nats.yml`

```yaml
services:
  nats:
    container_name: nats
    image: nats:latest
    restart: always
    networks:
      app-shared-net:
        aliases:
          - nats
    ports:
      - '4222:4222'   # client connections
      - '8222:8222'   # HTTP monitor
    command:
      - '-js'         # enable JetStream
      - '-sd'
      - '/data'       # storage directory
    volumes:
      - nats_data:/data
```

## CLI Tools

### Install NATS CLI

```bash
# Linux
curl -sf https://bin.nats.dev/nats | sh
sudo mv nats /usr/local/bin/

# Atau via package manager
# brew install nats-io/nats-tools/nats  (macOS)
```

### Koneksi ke NATS

```bash
# Dari host (port 4222 ter-expose)
nats context save hub --server nats://localhost:4222 --description "Hub Production"
nats context select hub

# Test koneksi
nats server check
nats server info
```

### Manage Streams (JetStream)

```bash
# List semua stream
nats stream list

# Lihat detail stream
nats stream info <stream-name>

# Buat stream
nats stream add <stream-name> \
  --subjects "hub.>" \
  --storage file \
  --max-msgs 1000000 \
  --max-bytes 1G \
  --retention limits

# Hapus stream
nats stream rm <stream-name>

# Purge (hapus semua message, retain stream)
nats stream purge <stream-name>
```

### Pub/Sub

```bash
# Subscribe ke subject
nats sub "hub.>"
nats sub "hub.image.cached"

# Publish message
nats pub "hub.test" '{"message": "hello"}'
nats pub "hub.image.cached" '{"original_url": "https://example.com/img.jpg", "cdn_url": "https://cdn.example.com/img.jpg"}'

# Request-reply
nats request "hub.service.do" '{"task": "process"}'
```

### Monitoring via HTTP API

```bash
# Server info
curl http://localhost:8222/

# JetStream info
curl http://localhost:8222/jszetstream

# Stream detail
curl http://localhost:8222/jszetstream?stream=<stream-name>

# Consumer info
curl http://localhost:8222/jszetstream?stream=<stream-name>&consumer=<consumer-name>

# Server stats
curl http://localhost:8222/varz

# Connections
curl http://localhost:8222/connz
```

## Event Topics Convention

Semua topik menggunakan prefix `hub.`:

| Subject | Payload | Deskripsi |
|---------|---------|-----------|
| `hub.image.cached` | `{original_url, cdn_url, source}` | Image selesai di-cache |
| `hub.image.repaired` | `{old_url, new_url}` | CNAME image diperbaiki |
| `hub.scrape.anime.done` | `{source, slug, duration}` | Scrape anime selesai |
| `hub.system.alert` | `{service, level, message}` | Error/alert dari service |
| `hub.test` | Any | Testing |

### Wildcard Subjects

NATS mendukung wildcard:

- `hub.>` — semua event hub (multi-level)
- `hub.image.*` — semua event image (single-level)
- `hub.*.done` — semua event yang selesai (single-level)

## JetStream Configuration

### Storage

Data JetStream disimpan di volume Docker `nats_data`.

Lokasi di VPS:
```bash
docker volume inspect nats_data
# atau
ls -la /var/lib/docker/volumes/nats_data/_data/
```

### Memory & Limits

NATS tidak memiliki konfigurasi limit memori default. Untuk production, pertimbangkan:

```yaml
command:
  - '-js'
  - '-sd'
  - '/data'
  - '--max_pending_size=64MB'
  - '--max_payload=1MB'
```

Atau gunakan NATS configuration file:

```yaml
# nats-server.conf
jetstream:
  max_memory_store: 256MB
  max_file_store: 10GB
```

## Troubleshooting

### Stream data tidak muncul

```bash
# 1. Cek koneksi NATS
nats server check

# 2. Cek apakah JetStream aktif
curl http://localhost:8222/jszetstream

# 3. Cek stream dan message count
nats stream list

# 4. Subscribe langsung untuk test
nats sub ">"
```

### NATS tidak bisa start

```bash
# Cek log
docker logs nats

# Cek apakah port 4222 sudah dipakai
ss -tlnp | grep 4222

# Cek volume data korup
docker run --rm -v nats_data:/data alpine ls -la /data

# Restart
docker compose -f infra/compose/nats.yml up -d --force-recreate
```

### Disk JetStream penuh

```bash
# Cek ukuran volume
docker system df -v | grep nats_data

# Purge stream jika perlu
nats stream purge <stream-name>

# Atau hapus volume (data hilang!)
docker compose -f infra/compose/nats.yml down
docker volume rm nats_data
docker compose -f infra/compose/nats.yml up -d
```

### Slow consumer

```bash
# Cek consumer lag
nats stream info <stream-name>
# Lihat fields: "Pending" dan "Acknowledgment"

# Lihat stats server
curl http://localhost:8222/varz | jq '.slow_consumers'
```

## Migration: Redis Pub/Sub ke NATS

Jika ingin migrasi dari Dapr pub/sub Redis ke NATS:

1. Buat stream NATS untuk topik `hub.>`
2. Update `infra/dapr/components/pubsub.yaml` dari `pubsub.redis` ke `pubsub.nats`
3. Deploy ulang semua service (Dapr sidecar akan reconnect)
4. Verifikasi event flow

```yaml
# infra/dapr/components/pubsub.yaml (setelah migrasi)
apiVersion: dapr.io/v1alpha1
kind: Component
metadata:
  name: pubsub
spec:
  type: pubsub.nats
  version: v1
  metadata:
    - name: natsURL
      value: nats://nats:4222
```
