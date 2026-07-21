---
name: commit-convention
description: Enforce commit message convention untuk Asepharyana Hub
---

# Commit Convention — Asepharyana Hub

## Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

## Types

| Type       | Usage                                |
| ---------- | ------------------------------------ |
| `feat`     | Fitur baru                           |
| `fix`      | Bug fix                              |
| `chore`    | Maintenance, config, tooling         |
| `docs`     | Dokumentasi                          |
| `refactor` | Perubahan kode tanpa fungsional baru |
| `test`     | Nambah/update test                   |
| `ci`       | CI/CD workflows                      |
| `perf`     | Optimasi performa                    |
| `style`    | Formatting (tanda kutip, dll)        |

## Scopes

| Scope         | Area                              |
| ------------- | --------------------------------- |
| `scraper`     | apps/scraper submodule           |
| `infra`       | infra/ (compose, traefik, docker) |
| `ci`          | .github/workflows/                |
| `dapr`        | Dapr config & sidecar             |
| `nats`        | NATS message bus                  |
| `docs`        | Dokumentasi                       |
| `deps`        | Dependencies                      |
| `scripts`     | Utility scripts                   |
| `root`        | Root config files                 |

## Contoh

```
feat(scraper): add anime detail caching via Dapr pubsub
fix(infra): correct NATS CLI flags for JetStream
chore(deps): update biome to v2.5.3
docs(infra): add deployment order for Dapr services
ci(deploy): add nats.yml to ALL_COMPOSE_FILES
refactor(scraper): migrate EventBus from tokio broadcast to Dapr pubsub
```

## Aturan

1. **Wajib** menyertakan scope dalam tanda kurung
2. **Wajib** `Co-Authored-By` untuk commit yang digenerate AI
3. **Gunakan imperative mood**: "add" bukan "added" / "adds"
4. **Jangan capitalize** type: `feat:` bukan `Feat:`
5. **No period** di akhir subject baris
6. Body explain **why** dan **what**, bukan **how**
7. Refer issue dengan `Closes #123` atau `Fixes #123` di footer
