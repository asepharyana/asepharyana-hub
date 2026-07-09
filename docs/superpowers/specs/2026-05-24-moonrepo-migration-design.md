# Moonrepo Migration Design

**Date:** 2026-05-24
**Status:** approved

## Context

Ultimate Asepharyana Tech is a polyglot monorepo with 7 applications managed as git submodules: nextjs (React/Next.js), elysia (Bun/Elysia), solidjs (SolidStart), rust (Axum), rust-auth (Axum auth service), leptos (Leptos WASM), and 9router. Nix flakes handle system dependencies and Docker image builds. Docker Compose manages deployment.

## Problem

- No centralized task orchestration — running build/lint/test across all apps is manual and error-prone
- No dependency caching — builds are not incremental across apps
- Inconsistent developer experience — each app has its own conventions, onboarding is slow

## Goal

Adopt moonrepo for task orchestration, caching, and dependency graph management across all 7 apps while preserving the existing Nix build system, Docker Compose deployment, and git submodule structure.

## Architecture

### Directory Structure

```
ultimate-asepharyana.tech/
├── .moon/
│   ├── workspace.yml
│   ├── toolchain.yml
│   └── tasks/
│       ├── typescript-build.yml
│       ├── typescript-lint.yml
│       ├── typescript-test.yml
│       ├── rust-build.yml
│       ├── rust-test.yml
│       └── rust-lint.yml
├── apps/
│   ├── nextjs/moon.yml
│   ├── elysia/moon.yml
│   ├── solidjs/moon.yml
│   ├── rust/moon.yml
│   ├── rust-auth/moon.yml
│   ├── leptos/moon.yml
│   └── 9router/moon.yml
├── .moon/workspace.yml          # unchanged — Nix build + deployment
├── flake.nix                    # unchanged
├── infra/                       # unchanged — Docker Compose, Traefik, nginx
└── scripts/                     # unchanged
```

### Tag Taxonomy

| Tag | Projects |
|-----|----------|
| `lang:typescript` | nextjs, elysia, solidjs |
| `lang:rust` | rust, rust-auth, leptos |
| `type:frontend` | nextjs, solidjs, leptos |
| `type:backend` | rust, elysia, rust-auth |
| `type:router` | 9router |

### Toolchain

- moonrepo manages Node 22, Bun, and TypeScript versions via `.moon/toolchain.yml` for consistent access across all TypeScript apps
- Rust toolchain remains managed by Nix/devShell — moonrepo does not touch Rust installation
- Nix devShell gains the moon CLI binary

### Project Dependencies

- **nextjs** depends on `rust-auth` and `elysia` (frontend consumes both APIs)
- **solidjs** depends on `elysia` and `rust-auth`
- **leptos** depends on `rust` (CSR frontend consumes rust backend API)
- **elysia**, **rust**, **rust-auth**, **9router** — no project dependencies

## Task Definitions

### Shared Tasks: TypeScript

All TypeScript apps inherit from `.moon/tasks/typescript-*.yml`:

| Task | Command | Inputs | Outputs |
|------|---------|--------|---------|
| `build` | `bun run build` | `src/**/*`, `tsconfig.json`, `package.json` | `.next`, `dist`, `.output` |
| `lint` | `bun run lint` | `src/**/*`, `eslint.config.mjs`, `tsconfig.json` | — |
| `typecheck` | `bun run check-types` | `src/**/*`, `tsconfig.json` | — |
| `test` | `bun test` | `src/**/*`, `test/**/*` | — |
| `e2e` | overridden per app | — | — |

### Shared Tasks: Rust

All Rust apps inherit from `.moon/tasks/rust-*.yml`, using system tasks:

| Task | Command | Inputs | Outputs |
|------|---------|--------|---------|
| `build` | `cargo build --release` | `src/**/*`, `Cargo.toml`, `Cargo.lock`, `build.rs` | `target/release/*` |
| `test` | `cargo test` | `src/**/*`, `Cargo.toml`, `Cargo.lock`, `tests/**/*` | — |
| `lint` | `cargo clippy -- -D warnings` | `src/**/*`, `Cargo.toml` | — |
| `fmt-check` | `cargo fmt --check` | `src/**/*` | — |

### Implicit Dependencies

Configured in `workspace.yml`: `lint` implicitly depends on `build` for TypeScript apps (typecheck path), and `build` propagates to dependents when source inputs change.

## Key Commands

```bash
moon run :build                    # build all changed apps + dependents
moon run :lint                     # lint all
moon run :test                     # test all
moon run --tag lang:rust :test     # test Rust apps only
moon run --affected :build         # build only what changed
moon query projects --tag lang:typescript  # list TypeScript projects
```

## What Does NOT Change

- **Nix flakes** — system dependencies, Docker image builds, devShell remain as-is
- **Docker Compose** — deployment continues through existing compose files in `infra/compose/`
- **Git submodules** — all 7 app repos stay as submodules under `apps/`
- **infra/** — Traefik and Docker Compose deployment remain under `infra/`; legacy Grafana/Prometheus/Alertmanager configs have been removed
- **scripts/** — existing utility scripts unchanged

## Migration Steps (High-Level)

1. Install moonrepo CLI, create `.moon/` scaffolding
2. Create shared task definitions in `.moon/tasks/`
3. Configure `workspace.yml` with project list, tags, and dependency constraints
4. Configure `toolchain.yml` for Node 22 + Bun
5. Add `moon.yml` to each of the 7 app directories
6. Validate: `moon check`, `moon run :build`, `moon run :lint`, `moon run :test`
7. Add moon CLI to Nix devShell
8. Commit and document
