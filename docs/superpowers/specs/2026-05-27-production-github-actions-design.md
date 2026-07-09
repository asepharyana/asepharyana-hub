# Production GitHub Actions redesign

## Goal

Rework `.github/workflows` into a production baseline that balances reliability, security, speed, and clear failures while keeping automatic deploys from `main`.

## Current problems

- Deploy runner checks out submodules recursively even though deploy work happens on the VPS. Fresh submodule SHAs can fail with `not our ref` before deploy starts.
- Repository dispatch can update a parent submodule pointer before the submodule SHA is fetchable by GitHub Actions.
- Build and deploy permissions are broader than needed at workflow level.
- Failure logs do not clearly separate build, manifest update, submodule readiness, and deployment phases.

## Chosen approach

Use a split pipeline:

1. `docker-build-push.yml` remains the build and manifest update pipeline.
2. `deploy-docker.yml` remains deploy-only.
3. Repository dispatch waits for the requested submodule SHA to be fetchable before building and updating the parent pointer.
4. Deploy checkout no longer uses recursive submodules; the VPS updates submodules after resetting to `origin/main`.

## Build workflow design

Triggers:

- `push` to `main` for app/package/docker/workflow changes.
- `repository_dispatch` with `service` and `sha` payload.
- `workflow_dispatch` for manual full builds.

Jobs:

- `changes`: detects the service matrix and validates dispatch payloads.
- `build`: builds and pushes only selected service images using Docker Buildx registry cache.
- `update-manifest`: updates compose image tags and, for dispatch events, updates the matching submodule pointer.

Repository dispatch handling:

- For dispatch events, wait until `git ls-remote` or equivalent fetch confirms the payload SHA exists in the service submodule remote.
- Retry for a bounded timeout and fail with an explicit message if the SHA never becomes visible.
- Only after readiness is confirmed, checkout the SHA in the submodule and commit the parent pointer update.

## Deploy workflow design

Triggers:

- `workflow_run` from successful `Build and Push Docker Images` on `main`.
- `push` to `main` for `infra/compose/**` and deploy workflow changes.
- `workflow_dispatch`.

Behavior:

- Keep automatic deploy from `main`.
- Keep a single deploy concurrency group.
- Use non-recursive checkout on the runner.
- On the VPS, fetch/reset `origin/main`, update submodules, compute changed compose stacks, pull images, and run Docker Compose.

## Permissions and action trust

Defaults:

```yaml
permissions:
  contents: read
```

Job-specific permissions:

- Build job: `contents: read`, `packages: write`.
- Manifest update job: `contents: write`.
- Deploy job: `contents: read`.

Action pinning:

- GitHub-owned and Docker official actions may use major versions such as `actions/checkout@v4` and `docker/build-push-action@v6`.
- Any future third-party action should be pinned to a full commit SHA.

## Reliability details

- Keep `set -euo pipefail` in shell steps.
- Add bounded retry around submodule SHA readiness.
- Add clear logs for selected services, image tags, compose files changed, and dispatch payload values.
- Avoid recursive submodule checkout in deploy to eliminate fresh-SHA checkout race.
- Manifest commits continue using `[skip ci]` to prevent build loops.

## Speed details

- Keep selective matrix builds.
- Keep Docker Buildx registry cache.
- Keep deploy checkout shallow and submodule-free.
- Avoid rebuilding from compose-only manifest commits.

## Validation

Before marking implementation complete:

- Validate workflow YAML syntax.
- Run `gh workflow list` or equivalent sanity checks.
- Verify build workflow still detects React updates.
- Verify deploy workflow no longer fails during runner checkout for fresh submodule SHAs.
