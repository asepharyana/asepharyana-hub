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
- **Node.js** >= 22.11.0 (via `.node-version`)
- **Bun** >= 1.3.11 (package manager)
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
| `apps/scraper`   | `asepharyana/asepharyana-hub-scraper`   |

### 3. Install Dependencies per Service

Install dependencies for TypeScript/Bun services:

```bash
cd apps/scraper && bun install && cd ../..
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
| `GITHUB_TOKEN` | GitHub personal access token                          |

## Development Workflow

### Running Services

Refer to each service's own documentation for setup and development instructions.

### API Documentation

Refer to each service's own documentation for API docs and endpoints.

## Coding Standards

### Linting

- **Biome** for TypeScript/JavaScript formatting and linting

Run linting:

```bash
# TypeScript/JavaScript
bun run check
```

### Formatting

- **Biome** for TypeScript/JavaScript
- **EditorConfig** for general formatting (`.editorconfig`)

```bash
# Format all
bun run format
```



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
feat(scraper): add new data source integration
chore: update biome config to v10
docs: add API endpoint documentation for scraper
ci: migrate to CodeQL v3
```

### Scopes

Common scopes: `scraper`, `infra`, `ci`, `deps`

## Pull Request Process

1. **Create a branch** from `main` with a descriptive name:
   - `feat/my-feature`
   - `fix/issue-description`
   - `chore/update-config`

2. **Make your changes** following the coding standards above.

3. **Run checks locally** before pushing:

    ```bash
    bun run check
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
   - SSHes into the VPS (`orangevps`, Tailscale IP `100.79.111.61`)
   - Pulls updated Docker images
   - Recreates only the changed containers
   - All services share the `app-shared-net` Docker network

7. **Merge** after CI passes and you have at least one approval (if applicable). Use squash merge to keep history clean.

## Adding a New Service

See `docs/add-new-app.md` for the complete step-by-step guide.
