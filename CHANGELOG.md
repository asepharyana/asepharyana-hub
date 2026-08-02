# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2026-08-02]

### Changed

- **Infra overhaul**: Docker + Traefik dihapus dari produksi → Caddy 2.11.4 (reverse proxy, auto-TLS LE, HTTP/3) + Nix/systemd services.
- **Port migration**: semua service pindah ke port 4000-an (hub 4003, tools 4007/4008, scraper 4091, llm-api 4010, dll).
- **DB via PgBouncer pool**: semua service konek ke imrnes 100.121.180.82:6432 (bukan :5432 langsung).
- **Secrets**: Bitwarden Secrets Manager (BWS) sebagai central secret store, wrapper bws-exec.
- **Flake**: dibatasi x86_64-linux (nixpkgs 26.11 drop darwin).

## [Unreleased]

### Changed

- Restructured the repository into a lightweight hub repo with standalone app submodules.
- Simplified root tooling to plain `package.json` scripts and per-service commands.
- Kept infrastructure, deployment workflows, and documentation in the root hub repo.
- Cleaned up Traefik SSL config: removed legacy `asephstech`/`asephscloud` cert references, synced volume mounts, fixed `api.insecure`.
- Pruned `.env.example` from 145 legacy vars (Firebase, Discord, Coolify, Portainer, YouTube, etc.) to 30 focused vars.
- Optimized `scraper.Dockerfile`: removed Node.js and Chromium from runtime image.
- Simplified CI/CD workflows: removed orphan container reference, commented code blocks.
- Cleaned up scripts: removed stale MySQL config, fixed package references, simplified update-deps.
- Added NATS + JetStream message broker infrastructure (`infra/compose/nats.yml`).
- Added Dapr runtime infrastructure: placement service, sidecar pattern, pub/sub + state store components.
- Integrated Dapr sidecar into scraper service (`infra/compose/scraper.yml`).
- Updated deployment order: shared → NATS → Dapr → Traefik → apps.
- Added `docs/add-dapr-service.md` guide for adding Dapr to new services.

### Removed

- Removed deprecated services from apps, compose files, Dockerfiles, Traefik routes, and workflows.
- Removed stale monorepo orchestration configs and hook tooling from the root repo.
- Removed `.nvmrc` (duplicate of `.node-version`), `renovate.json` (using dependabot).
- Removed stale documentation: `dependency-map.md`, `observability.md`, `handoff-log.jsonl`, `quality-gates.json`, `workflow-state.json`.
- Removed deprecated `docs/superpowers/` design docs.
- Removed `infra/config/mysql/` (no active MySQL service) and `docs/config/squid.conf.archived`.
