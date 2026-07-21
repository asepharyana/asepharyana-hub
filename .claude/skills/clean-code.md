---
name: clean-code
description: Clean Code, Clean Architecture, SOLID principles untuk proyek Asepharyana Hub
---

# Clean Code — Asepharyana Hub

## Prinsip Dasar

### 1. Naming
- **Gunakan nama yang mengungkapkan intensi**: `calculateTotal`, `fetchAnimeData`, `ImageCacheRepository` — bukan `calc`, `getData`, `Repo`.
- **Boolean prefix** dengan `is`, `has`, `should`: `isHealthy`, `hasPoster`, `shouldRetry`.
- **Hindari singkatan** kecuali sangat umum (`config`, `url`, `db`).

### 2. Fungsi
- **Satu fungsi = satu tanggung jawab**. Max 20 baris.
- **Nama fungsi sebagai kata kerja**: `validateToken()`, `cacheImage()`.
- **Parameter minimal**: max 3 parameter. Lebih dari itu bungkus jadi struct/object.

### 3. Clean Architecture Layers

```
Domain       → Entities, Repository traits, Error enums
Application  → Use cases
Infrastructure → Implementasi konkret (SeaORM, Redis, HTTP clients)
Presentation → Axum handlers, DTOs, middleware
```

Dependency rule: **kode layer dalam tidak tahu tentang layer luar**. Domain gak import framework.

### 4. Error Handling
- **Gunakan `thiserror`** untuk domain errors, bukan `anyhow` untuk library code.
- **`anyhow` hanya untuk** binary/app entry point dan test.
- **Convert domain errors ke HTTP** di presentation layer, bukan di use case.

### 5. Testing
- **TDD mindset**: tulis test sebelum implementasi untuk logic bisnis.
- **Unit test untuk use case + domain** (tanpa infra).
- **Integration test untuk repository** (dengan test container).
- **Mock trait**, bukan struct konkret.

## Untuk Proyek Ini

### Scraper Service (Rust)
- `src/domain/` — tipe data murni, trait, error — **tanpa framework**
- `src/application/` — use cases, orchestrasi, **tanpa HTTP**
- `src/infrastructure/` — implementasi repository, cache, HTTP client
- `src/presentation/` — handler Axum, routing, middleware

### Event-Driven (Dapr + NATS)
- Event schema pake CloudEvents format
- Topic naming: `hub.<domain>.<action>` (e.g. `hub.image.cached`)
- Handler hanya untuk satu tipe event, pisah file per domain

### Infra Config (YAML)
- Satu compose file per service
- Networking via `app-shared-net`
- Image tag selalu `sha-<short-sha>`, bukan `latest` di production
