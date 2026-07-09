# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Restructured the repository into a lightweight hub repo with standalone app submodules.
- Simplified root tooling to plain `package.json` scripts and per-service commands.
- Kept infrastructure, deployment workflows, and documentation in the root hub repo.

### Removed

- Removed deprecated services from apps, compose files, Dockerfiles, Traefik routes, and workflows.
- Removed stale monorepo orchestration configs and hook tooling from the root repo.
