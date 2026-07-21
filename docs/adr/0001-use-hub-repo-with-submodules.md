# ADR 0001: Use a Hub Repository with App Submodules

## Status

Accepted

## Context

The project contains multiple independent application services that share one deployment surface: Docker Compose, Traefik routing, GitHub Actions workflows, and operational documentation.

The services should be developed and versioned independently, while deployment infrastructure should remain centralized so production routing and compose manifests stay consistent.

## Decision

Use `asepharyana-hub` as the root hub repository.

- Application code lives under `apps/<service>` as Git submodules.
- Infrastructure lives in the root repo under `infra/`.
- Documentation lives in the root repo under `docs/`.
- CI/CD workflows live in the root repo under `.github/workflows/`.
- Root tooling stays minimal: `package.json`, Prettier, ESLint, Makefile helpers, and deployment scripts.

Current app submodules:

| Service     | Path           | Remote                                  |
| ----------- | -------------- | --------------------------------------- |
| Scraper API | `apps/scraper` | `asepharyana/asepharyana-hub-scraper`   |

## Consequences

### Positive

- Each app can evolve in its own repository.
- The hub pins exact submodule revisions for reproducible deployments.
- Deployment infrastructure remains centralized and easier to audit.
- Root tooling stays lightweight and does not impose one build system on every service.

### Negative

- Developers must understand Git submodule workflows.
- Updating a service requires updating the submodule pointer in the hub repo.
- Cross-service changes require coordinating commits across multiple repositories.

### Mitigations

- Keep `.gitmodules` accurate and minimal.
- Use `scripts/sync-submodules.sh` for local checkout consistency.
- Document service-addition steps in `docs/add-new-app.md`.
- Keep GitHub Actions responsible for Docker image builds, compose tag updates, and deployments.
