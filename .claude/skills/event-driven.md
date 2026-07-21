---
name: event-driven
description: Event-driven patterns dengan Dapr + NATS untuk Asepharyana Hub
---

# Event-Driven Architecture — Asepharyana Hub

## Stack
- **Message Backbone**: NATS + JetStream (untuk streaming & job queue)
- **Pub/Sub Runtime**: Dapr sidecar per service (pubsub via Redis built-in)
- **State Store**: Dapr → Redis

## Event Topics Convention

```
hub.<domain>.<action>

Contoh:
hub.image.cached       → Image selesai di-cache ke CDN
hub.image.repaired     → Image diperbaiki (CNAME change)
hub.scrape.anime.done  → Scrape anime selesai
hub.system.alert       → Error/alert dari service
```

## CloudEvents Format

```json
{
  "specversion": "1.0",
  "type": "hub.image.cached",
  "source": "scraper-api",
  "subject": "anime-poster",
  "id": "uuid-v4",
  "time": "2026-07-21T10:00:00Z",
  "datacontenttype": "application/json",
  "data": { ... }
}
```

## Publish Event (Rust via HTTP API)

Gunakan `reqwest` langsung ke Dapr sidecar (SDK Rust masih experimental):

```rust
let event = serde_json::json!({
    "specversion": "1.0",
    "type": "hub.image.cached",
    "source": "scraper-api",
    "id": Uuid::new_v4().to_string(),
    "time": chrono::Utc::now().to_rfc3339(),
    "datacontenttype": "application/json",
    "data": { "original_url": url, "cdn_url": cdn_url }
});

reqwest::Client::new()
    .post("http://localhost:3500/v1.0/publish/pubsub/hub.image.cached")
    .json(&event)
    .send()
    .await?;
```

## Service Invocation

```bash
curl http://localhost:3500/v1.0/invoke/<app-id>/method/<path>
```

## State Store

```bash
# Set
curl -X POST http://localhost:3500/v1.0/state/statestore \
  -H "Content-Type: application/json" \
  -d '[{"key": "mykey", "value": "myvalue"}]'

# Get
curl http://localhost:3500/v1.0/state/statestore/mykey

# Delete
curl -X DELETE http://localhost:3500/v1.0/state/statestore/mykey
```

## Scraper Event Integration

File yang perlu dimodifikasi untuk event-driven:

| File | Perubahan |
|------|-----------|
| `src/events/bus.rs` | Ganti backend dari tokio broadcast ke Dapr pub/sub |
| `src/bootstrap/mod.rs` | Init DaprClient, inject ke AppState |
| `src/presentation/state.rs` | Tambah `dapr_client` field |
| `src/proxy/use_cases.rs` | Publish `ImageRepaired` & `ImageCached` events |
| `src/infrastructure/services/images/cache.rs` | Emit event tiap cache selesai |
| `Cargo.toml` | Tambah `reqwest`, `uuid`, `chrono` (jika belum ada) |

## Event Handlers (Subscribe)

Buat `src/subscribers/` untuk handler:

```rust
// src/subscribers/image_handler.rs
pub async fn handle_image_cached(event: CloudEvent) -> Result<()> {
    // Log, notifikasi, update status
}
```

Daftarkan subscribers di `bootstrap/mod.rs` dengan spawn task:
```rust
tokio::spawn(async move {
    let mut stream = dapr_client.subscribe("pubsub", "hub.image.cached");
    while let Some(event) = stream.next().await {
        handle_image_cached(event).await;
    }
});
```

## Testing Event-Driven Code

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_event() {
        let client = MockDaprClient::new();
        client.expect_publish()
            .with(...)
            .returning(|_| Ok(()));
        // ... test
    }
}
```
