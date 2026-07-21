# Menambahkan Dapr ke Service Baru

Panduan integrasi Dapr runtime sidecar untuk service di `asepharyana-hub`.

## Prasyarat

- NATS server berjalan (`infra/compose/nats.yml`)
- Dapr placement service berjalan (`infra/compose/dapr.yml`)

## 1. Compose File

Setiap service butuh sidecar container Dapr. Contoh:

```yaml
services:
  app:
    container_name: app
    image: ghcr.io/asepharyana/asepharyana-hub/app:latest
    restart: always
    depends_on:
      dapr-placement:
        condition: service_healthy
      nats:
        condition: service_healthy
    networks:
      app-shared-net:
        aliases:
          - app
    env_file:
      - ../../.env

  app-dapr:
    container_name: app-dapr
    image: daprio/daprd:latest
    restart: always
    depends_on:
      dapr-placement:
        condition: service_healthy
      nats:
        condition: service_healthy
    networks:
      - app-shared-net
    command:
      - './daprd'
      - '--app-id=app'
      - '--app-port=3000'
      - '--dapr-http-port=3500'
      - '--dapr-grpc-port=50001'
      - '--placement-host-address=dapr-placement:50005'
      - '--resources-path=/components'
    volumes:
      - ../../infra/dapr/components:/components

networks:
  app-shared-net:
    name: app-shared-net
    external: true
```

## 2. Mengakses Dapr dari Service

### Via HTTP API (semua bahasa)

Sidecar listen di `localhost:3500`:

```bash
# Publish event
curl -X POST http://localhost:3500/v1.0/publish/pubsub/hub.event.type \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'

# Service invocation
curl http://localhost:3500/v1.0/invoke/app/method/endpoint

# State store
curl -X POST http://localhost:3500/v1.0/state/statestore \
  -H "Content-Type: application/json" \
  -d '[{"key": "mykey", "value": "myvalue"}]'
```

### Via Dapr SDK (Rust)

Tambah ke `Cargo.toml`:

```toml
dapr-sdk = { version = "0.15", features = ["pubsub", "http"] }
tokio-stream = "0.1"
```

Contoh publish event:

```rust
use dapr_sdk::client::{Client, Event};
use dapr_sdk::DaprClient;

let client = DaprClient::new("127.0.0.1", 3500).await?;
client.publish_event("pubsub", "hub.image.cached", serde_json::json!({
    "original_url": url,
    "cdn_url": cdn_url,
})).await?;
```

Contoh subscribe event:

```rust
let mut stream = client.subscribe_events("pubsub", "hub.image.cached").await?;
while let Some(event) = stream.next().await {
    let data: MyEvent = serde_json::from_slice(&event.data)?;
    // handle event
}
```

## 3. Event Topics Convention

Gunakan prefix `hub.` untuk semua event:

| Topic | Payload | Description |
|-------|---------|-------------|
| `hub.image.cached` | `{original_url, cdn_url, source}` | Image selesai di-cache |
| `hub.image.repaired` | `{old_url, new_url}` | CNAME image diperbaiki |
| `hub.scrape.anime.done` | `{source, slug, duration}` | Scrape anime selesai |
| `hub.system.alert` | `{service, level, message}` | Error/alert dari service |

## 4. Local Development

Untuk development tanpa Docker:

```bash
# 1. Install Dapr CLI
# 2. Init Dapr local
dapr init

# 3. Run service dengan sidecar
dapr run --app-id app --app-port 3000 --dapr-http-port 3500 \
  --resources-path ./infra/dapr/components \
  -- cargo run
```

## 5. Verifikasi

```bash
# Sidecar health
curl http://localhost:3500/v1.0/healthz

# Publish test event
curl -X POST http://localhost:3500/v1.0/publish/pubsub/hub.test \
  -H "Content-Type: application/json" \
  -d '{"test": true}'

# NATS stream stats
curl http://localhost:8222/jszetstream
```
