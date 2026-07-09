# Contributing to Asepharyana Hub

## Table of Contents

- [Prerequisites](#prerequisites)
- [Local Setup](#local-setup)
- [Development Workflow](#development-workflow)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Commit Message Format](#commit-message-format)
- [Pull Request Process](#pull-request-process)
- [Adding a New Service](#adding-a-new-service)

## Prerequisites

- **Git** with LFS support
- **Node.js** >= 22.11.0 (via `.node-version` or `.nvmrc`)
- **Bun** >= 1.3.11 (package manager)
- **Rust** >= 1.89.0 (for Rust services)
- **Docker** and **Docker Compose** (for shared infrastructure)

## Local Setup

### 1. Clone the Repository

```bash
git clone https://github.com/asepharyana/asepharyana-hub.git
cd asepharyana-hub
```

### 2. Initialize Submodules

This hub repo uses Git submodules for all application services:

```bash
git submodule update --init --recursive
```

This checks out all submodules at the pinned commit (not `main`). The submodules and their remotes are:

| Path             | Remote                                  |
| ---------------- | --------------------------------------- |
| `apps/elysia`    | `asepharyana/asepharyana-hub-elysia`    |
| `apps/scraper`   | `asepharyana/asepharyana-hub-scraper`   |
| `apps/react`     | `asepharyana/asepharyana-hub-react`     |
| `apps/rust-auth` | `asepharyana/asepharyana-hub-rust-auth` |

### 3. Install Dependencies per Service

Install dependencies for TypeScript/Bun services:

```bash
cd apps/elysia && bun install && cd ../..
cd apps/react && npm install && cd ../..
```

### 4. Start Shared Infrastructure

Start shared services (Redis) via Docker Compose:

```bash
docker compose -f infra/compose/shared.yml up -d
```

### 5. Configure Environment

Copy the example environment file and adjust as needed:

```bash
cp .env.example .env
```

Key variables to configure:

| Variable       | Description                                           |
| -------------- | ----------------------------------------------------- |
| `DATABASE_URL` | PostgreSQL connection (Tailscale IP to `imrnes` VPS)  |
| `REDIS_URL`    | Redis connection (`redis://localhost:6379` for local) |
| `JWT_SECRET`   | JWT signing secret                                    |
| `GITHUB_TOKEN` | GitHub personal access token                          |

## Development Workflow

### Running Services

**Rust API (rust-auth):**

```bash
cd apps/rust-auth
cargo run
```

**Elysia API (elysia):**

```bash
cd apps/elysia
bun run dev
```

**React Frontend (react):**

```bash
cd apps/react
npm run dev
```

### API Documentation

- Rust OpenAPI: `http://localhost:4091/docs`
- Elysia Swagger: `http://localhost:4092/docs`
- Elysia AsyncAPI: `http://localhost:4092/docs-ws`

## Coding Standards

### Linting

- **ESLint** with `@antfu/eslint-config` for TypeScript/JavaScript
- **Cargo Clippy** for Rust

Run linting:

```bash
# TypeScript/JavaScript
eslint . --no-error-on-unmatched-pattern

# Rust specific
cd apps/rust-auth && cargo clippy -- -D warnings
```

### Formatting

- **Prettier** for TypeScript/JavaScript/Markdown (config in `.prettierrc`)
  - Single quotes, 100 print width, 2-space indent, trailing commas
- **Cargo fmt** for Rust
- **EditorConfig** for general formatting (`.editorconfig`)

```bash
# Prettier
prettier --write .

# Rust
cd apps/rust-auth && cargo fmt
```

### Rust Configuration

Rust services use edition `2024` with stable toolchain (nightly features may be used).

## Commit Message Format

This project enforces **Conventional Commits** for all commit messages.

### Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

### Types

| Type       | Usage                                                       |
| ---------- | ----------------------------------------------------------- |
| `feat`     | A new feature                                               |
| `fix`      | A bug fix                                                   |
| `chore`    | Maintenance, config, tooling changes                        |
| `docs`     | Documentation only changes                                  |
| `refactor` | Code change that neither fixes a bug nor adds a feature     |
| `test`     | Adding or updating tests                                    |
| `ci`       | CI/CD configuration and scripts                             |
| `style`    | Formatting, missing semicolons, etc. (no production change) |
| `perf`     | Performance improvement                                     |

### Examples

```
feat(rust-auth): add OAuth2 Google login flow
fix(elysia): handle null JWT payload in auth middleware
chore: update eslint config to v10
docs: add API endpoint documentation for scraper
refactor(react): extract Header component from App
test(elysia): add unit tests for rate limiter
ci: migrate to CodeQL v3
```

### Scopes

Common scopes: `rust-auth`, `elysia`, `react`, `scraper`, `infra`, `ci`, `deps`

## Pull Request Process

1. **Create a branch** from `main` with a descriptive name:
   - `feat/my-feature`
   - `fix/issue-description`
   - `chore/update-config`

2. **Make your changes** following the coding standards above.

3. **Run checks locally** before pushing:

   ```bash
   cd apps/react && npx tsc --noEmit
   eslint . --no-error-on-unmatched-pattern
   ```

4. **Push and open a PR** against `main`. CI will automatically run:
   - **Lint** — ESLint across changed TypeScript files
   - **TypeCheck** — TypeScript compilation check
   - **Security** — CodeQL analysis (weekly schedule + PRs)

5. **Docker Build Pipeline** triggers on pushes to `main` when `apps/**` changes:
   - Detects which services changed
   - Builds Docker images for only those services
   - Pushes to GHCR with `latest` and `sha-<short>` tags
   - Updates compose manifests to use the new SHA tags

6. **Deployment Pipeline** triggers after a successful Docker build:
   - SSHes into the VPS (`orange`, Tailscale IP `100.96.248.86`)
   - Pulls updated Docker images
   - Recreates only the changed containers
   - All services share the `app-shared-net` Docker network

7. **Merge** after CI passes and you have at least one approval (if applicable). Use squash merge to keep history clean.

## Adding a New Service

See `docs/add-new-app.md` for the complete step-by-step guide. In summary:

1. Create the app in `apps/<name>`
2. Add it as a Git submodule in `.gitmodules`
3. Register it in `infra/compose/<name>.yml`
4. Add a Dockerfile at `infra/docker/<name>.Dockerfile`
5. Add Traefik routing config in `infra/traefik/dynamic/apps.yaml`
6. Add CI entries in `.github/workflows/docker-build-push.yml`
7. Add compose file to the deploy script in `deploy-docker.yml`
8. Add any required GitHub secrets for the service
