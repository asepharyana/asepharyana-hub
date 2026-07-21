# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Removed

- Removed deprecated services from apps, compose files, Dockerfiles, Traefik routes, and workflows.
- Removed stale monorepo orchestration configs and hook tooling from the root repo.
- Removed `.nvmrc` (duplicate of `.node-version`), `renovate.json` (using dependabot).
- Removed stale documentation: `dependency-map.md`, `observability.md`, `handoff-log.jsonl`, `quality-gates.json`, `workflow-state.json`.
- Removed deprecated `docs/superpowers/` design docs.
- Removed `infra/config/mysql/` (no active MySQL service) and `docs/config/squid.conf.archived`.
