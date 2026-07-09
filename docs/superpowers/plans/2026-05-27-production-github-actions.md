# Production GitHub Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework GitHub Actions build/deploy workflows into a production baseline with reliable React submodule updates, least-privilege permissions, clear deploy behavior, and current official action versions.

**Architecture:** Keep two workflows: `docker-build-push.yml` for detect/build/manifest updates, and `deploy-docker.yml` for VPS deployment. Add dispatch submodule SHA readiness checks before parent pointer updates, and remove recursive submodule checkout from deploy runner.

**Tech Stack:** GitHub Actions YAML, GitHub-hosted Ubuntu runners, Docker Buildx, GHCR, git submodules, Docker Compose over SSH.

---

## File Structure

- Modify `.github/workflows/docker-build-push.yml`: add default permissions, validate repository dispatch payloads, wait for submodule SHAs, keep selective matrix builds, harden manifest update.
- Modify `.github/workflows/deploy-docker.yml`: add default permissions, remove recursive checkout, keep auto deploy from successful build, make deploy logs clearer.
- Modify `.github/dependabot.yml`: add GitHub Actions update config so official actions stay current.

---

### Task 1: Harden build workflow permissions and dispatch validation

**Files:**
- Modify: `.github/workflows/docker-build-push.yml`

- [ ] **Step 1: Add workflow-level read permissions**

At top level, after `concurrency`, add:

```yaml
permissions:
  contents: read
```

Expected shape:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: false

permissions:
  contents: read

env:
  REGISTRY: ghcr.io
```

- [ ] **Step 2: Replace dispatch parser with payload validation**

In `.github/workflows/docker-build-push.yml`, replace `Parse repository_dispatch payload` step body with:

```yaml
      - name: Parse repository_dispatch payload
        id: dispatch
        if: github.event_name == 'repository_dispatch'
        env:
          SERVICE: ${{ github.event.client_payload.service }}
          SHA: ${{ github.event.client_payload.sha }}
        run: |
          set -euo pipefail

          if [ -z "${SERVICE:-}" ]; then
            echo "::error::repository_dispatch payload missing service"
            exit 1
          fi

          if [ -z "${SHA:-}" ]; then
            echo "::error::repository_dispatch payload missing sha"
            exit 1
          fi

          case "$SERVICE" in
            rust-api|elysia-api|react-web|9router) ;;
            *)
              echo "::error::Unsupported service '$SERVICE'. Expected one of: rust-api, elysia-api, react-web, 9router"
              exit 1
              ;;
          esac

          case "$SHA" in
            *[!0-9a-fA-F]*|???????????????????????????????????????|?????????????????????????????????????????*)
              echo "::error::Invalid sha '$SHA'. Expected 40 hex characters"
              exit 1
              ;;
          esac

          declare -a SERVICES=("rust-api" "elysia-api" "react-web" "9router")
          for svc in "${SERVICES[@]}"; do
            if [ "$SERVICE" = "$svc" ]; then
              echo "${svc}=true" >> "$GITHUB_OUTPUT"
            else
              echo "${svc}=false" >> "$GITHUB_OUTPUT"
            fi
          done
```

- [ ] **Step 3: Run YAML syntax check**

Run:

```bash
python - <<'PY'
from pathlib import Path
import yaml
for path in Path('.github/workflows').glob('*.yml'):
    yaml.safe_load(path.read_text())
    print(f'OK {path}')
PY
```

Expected:

```text
OK .github/workflows/deploy-docker.yml
OK .github/workflows/docker-build-push.yml
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/docker-build-push.yml
git commit -m "ci: validate dispatch payloads"
```

---

### Task 2: Add submodule SHA readiness wait before build/update

**Files:**
- Modify: `.github/workflows/docker-build-push.yml`

- [ ] **Step 1: Add readiness job after changes job**

Insert this job between `changes` and `build`:

```yaml
  wait-submodule-ref:
    needs: [changes]
    if: github.event_name == 'repository_dispatch'
    runs-on: ubuntu-latest
    steps:
      - name: Wait for submodule ref
        env:
          SERVICE: ${{ github.event.client_payload.service }}
          SHA: ${{ github.event.client_payload.sha }}
        run: |
          set -euo pipefail

          case "$SERVICE" in
            "rust-api") REPO="https://github.com/MythEclipse/ultimate-asepharyana-tech-rust.git" ;;
            "elysia-api") REPO="https://github.com/MythEclipse/ultimate-asepharyana-tech-elysia.git" ;;
            "react-web") REPO="https://github.com/MythEclipse/ultimate-asepharyana-tech-react.git" ;;
            "9router") REPO="https://github.com/MythEclipse/9router.git" ;;
            *)
              echo "::error::Unsupported service '$SERVICE'"
              exit 1
              ;;
          esac

          echo "Waiting for $SERVICE ref $SHA in $REPO"
          for attempt in {1..30}; do
            if git ls-remote --exit-code "$REPO" "$SHA" >/dev/null 2>&1; then
              echo "Submodule ref $SHA is fetchable for $SERVICE"
              exit 0
            fi
            echo "Attempt $attempt/30: $SHA not visible yet; waiting 10s"
            sleep 10
          done

          echo "::error::Submodule ref $SHA for $SERVICE was not fetchable after 300s"
          exit 1
```

- [ ] **Step 2: Make build wait for readiness job without blocking push/manual events**

Change build job header from:

```yaml
  build:
    needs: [changes]
```

to:

```yaml
  build:
    needs: [changes, wait-submodule-ref]
    if: |
      always() &&
      needs.changes.result == 'success' &&
      (needs.wait-submodule-ref.result == 'success' || needs.wait-submodule-ref.result == 'skipped') &&
      needs.changes.outputs.matrix != '[]'
```

Remove existing build-level line:

```yaml
    if: needs.changes.outputs.matrix != '[]'
```

- [ ] **Step 3: Make update-manifest wait for readiness job**

Change update-manifest header from:

```yaml
  update-manifest:
    needs: [changes, build]
    if: |
      always() &&
      (needs.build.result == 'success' || needs.build.result == 'skipped')
```

to:

```yaml
  update-manifest:
    needs: [changes, wait-submodule-ref, build]
    if: |
      always() &&
      needs.changes.result == 'success' &&
      (needs.wait-submodule-ref.result == 'success' || needs.wait-submodule-ref.result == 'skipped') &&
      (needs.build.result == 'success' || needs.build.result == 'skipped')
```

- [ ] **Step 4: Run YAML syntax check**

Run same command from Task 1 Step 3.

Expected both workflow files print `OK`.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/docker-build-push.yml
git commit -m "ci: wait for submodule refs before builds"
```

---

### Task 3: Harden manifest update and submodule checkout

**Files:**
- Modify: `.github/workflows/docker-build-push.yml`

- [ ] **Step 1: Add job permissions to build and manifest jobs**

Ensure build job contains:

```yaml
    permissions:
      contents: read
      packages: write
```

Ensure update-manifest job contains:

```yaml
    permissions:
      contents: write
```

- [ ] **Step 2: Replace dispatch submodule checkout block**

Inside `Update tags and submodules`, replace the repository_dispatch submodule update block with:

```bash
              if [ "${{ github.event_name }}" == "repository_dispatch" ] && [ "${{ github.event.client_payload.service }}" == "$id" ]; then
                SHA_DISPATCH="${{ github.event.client_payload.sha }}"
                SUB_PATH="${PATHS[$id]}"
                if [ -n "$SHA_DISPATCH" ]; then
                  echo "Updating submodule $SUB_PATH to $SHA_DISPATCH"
                  git submodule update --init "$SUB_PATH"
                  git -C "$SUB_PATH" fetch origin "$SHA_DISPATCH"
                  git -C "$SUB_PATH" checkout "$SHA_DISPATCH"
                  git add "$SUB_PATH"
                  CHANGED=true
                fi
              fi
```

- [ ] **Step 3: Add pull/rebase retry before push**

Replace:

```bash
            git commit -m "chore: update manifests and submodules [skip ci]"
            git pull --rebase origin main
            git push origin main
```

with:

```bash
            git commit -m "chore: update manifests and submodules [skip ci]"

            for attempt in {1..3}; do
              if git pull --rebase origin main && git push origin main; then
                exit 0
              fi
              echo "Manifest push attempt $attempt/3 failed; retrying"
              git rebase --abort || true
              git pull --rebase origin main || true
              sleep 5
            done

            echo "::error::Failed to push manifest update after 3 attempts"
            exit 1
```

- [ ] **Step 4: Run YAML syntax check**

Run same command from Task 1 Step 3.

Expected both workflow files print `OK`.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/docker-build-push.yml
git commit -m "ci: harden manifest updates"
```

---

### Task 4: Make deploy checkout submodule-free and least privilege

**Files:**
- Modify: `.github/workflows/deploy-docker.yml`

- [ ] **Step 1: Add workflow-level read permissions**

After `concurrency`, add:

```yaml
permissions:
  contents: read
```

Expected shape:

```yaml
concurrency:
  group: deploy-vps
  cancel-in-progress: false

permissions:
  contents: read
```

If current `cancel-in-progress` is `true`, change it to `false`.

- [ ] **Step 2: Make checkout non-recursive**

Replace checkout step:

```yaml
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          submodules: recursive
```

with:

```yaml
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 1
          submodules: false
```

- [ ] **Step 3: Add deploy context log**

At start of `Deploy with Docker Compose on VPS` run script, after `set -euo pipefail`, add:

```bash
          echo "Deploy event: ${{ github.event_name }}"
          echo "Deploy ref: ${{ github.ref }}"
          echo "Deploy sha: ${{ github.sha }}"
```

- [ ] **Step 4: Run YAML syntax check**

Run same command from Task 1 Step 3.

Expected both workflow files print `OK`.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/deploy-docker.yml
git commit -m "ci: avoid submodule checkout during deploy"
```

---

### Task 5: Add GitHub Actions Dependabot updates

**Files:**
- Modify: `.github/dependabot.yml`

- [ ] **Step 1: Add github-actions ecosystem**

Append this update entry under `updates:`:

```yaml
  - package-ecosystem: 'github-actions'
    directory: '/'
    schedule:
      interval: weekly
    groups:
      github-actions:
        patterns:
          - '*'
```

Expected file shape:

```yaml
version: 2
updates:
  - package-ecosystem: 'devcontainers'
    directory: '/'
    schedule:
      interval: weekly

  - package-ecosystem: 'github-actions'
    directory: '/'
    schedule:
      interval: weekly
    groups:
      github-actions:
        patterns:
          - '*'
```

- [ ] **Step 2: Run YAML syntax check**

Run:

```bash
python - <<'PY'
from pathlib import Path
import yaml
paths = [Path('.github/dependabot.yml'), *Path('.github/workflows').glob('*.yml')]
for path in paths:
    yaml.safe_load(path.read_text())
    print(f'OK {path}')
PY
```

Expected:

```text
OK .github/dependabot.yml
OK .github/workflows/deploy-docker.yml
OK .github/workflows/docker-build-push.yml
```

- [ ] **Step 3: Commit**

```bash
git add .github/dependabot.yml
git commit -m "ci: enable github actions dependency updates"
```

---

### Task 6: Final validation

**Files:**
- Validate: `.github/workflows/docker-build-push.yml`
- Validate: `.github/workflows/deploy-docker.yml`
- Validate: `.github/dependabot.yml`

- [ ] **Step 1: Run YAML syntax check**

Run:

```bash
python - <<'PY'
from pathlib import Path
import yaml
paths = [Path('.github/dependabot.yml'), *Path('.github/workflows').glob('*.yml')]
for path in paths:
    yaml.safe_load(path.read_text())
    print(f'OK {path}')
PY
```

Expected all files print `OK`.

- [ ] **Step 2: Check workflows recognized by GitHub CLI**

Run:

```bash
gh workflow list
```

Expected output includes:

```text
Build and Push Docker Images
Deploy Docker to VPS
```

- [ ] **Step 3: Inspect final diff**

Run:

```bash
git diff -- .github/workflows .github/dependabot.yml
```

Expected:

- `docker-build-push.yml` has dispatch validation, `wait-submodule-ref`, job permissions, and manifest push retry.
- `deploy-docker.yml` has non-recursive checkout and read-only permissions.
- `dependabot.yml` has `github-actions` updates.

- [ ] **Step 4: Commit any final validation fixes**

If Step 1 or Step 2 required fixes, commit them:

```bash
git add .github/workflows .github/dependabot.yml
git commit -m "ci: finalize production workflow hardening"
```

If no fixes were needed, do not create an empty commit.
