# Moonrepo Migration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the monorepo to moonrepo for centralized task orchestration, caching, and consistent dependency management across all 7 apps.

**Architecture:** moonrepo sits as a task orchestration layer above the existing Nix build system. `.moon/tasks/` holds shared task definitions (TypeScript and Rust). Each app gets a `moon.yml` with tags and inter-project dependencies. Nix, Docker Compose, git submodules, and infra remain untouched.

**Tech Stack:** moonrepo CLI, Node 22, Bun, TypeScript 5+, Cargo (Rust)

---

### Task 1: Install moonrepo CLI and scaffold .moon/ directory

**Files:**
- Create: `.moon/workspace.yml`
- Create: `.moon/toolchain.yml`
- Create: `.moon/tasks/` (directory)

- [ ] **Step 1: Install moonrepo CLI via curl**

```bash
curl -fsSL https://moonrepo.dev/install/moon.sh | bash
```

- [ ] **Step 2: Verify installation**

Run: `moon --version`
Expected: prints version number (e.g., `moon 1.x.x`)

- [ ] **Step 3: Create .moon/ scaffold directories**

```bash
mkdir -p .moon/tasks
```

- [ ] **Step 4: Commit**

```bash
git add .moon/
git commit -m "chore: scaffold .moon/ directory for moonrepo"
```

---

### Task 2: Configure workspace.yml

**Files:**
- Create: `.moon/workspace.yml`

- [ ] **Step 1: Write .moon/workspace.yml**

```yaml
# https://moonrepo.dev/docs/config/workspace
$schema: "https://moonrepo.dev/schemas/workspace.json"

projects:
  - "apps/*"

vcs:
  manager: "git"
  defaultBranch: "main"

runner:
  implicitDeps:
    # TypeScript apps: lint depends on build (for typecheck path)
    - "typescript-build.build"

  cacheTtl: 604800
```

- [ ] **Step 2: Validate config structure**

Run: `moon check`
Expected: no errors (will warn about missing project configs — expected)

- [ ] **Step 3: Commit**

```bash
git add .moon/workspace.yml
git commit -m "chore: configure moonrepo workspace with project glob"
```

---

### Task 3: Configure toolchain.yml

**Files:**
- Create: `.moon/toolchain.yml`

- [ ] **Step 1: Write .moon/toolchain.yml**

```yaml
# https://moonrepo.dev/docs/config/toolchain
$schema: "https://moonrepo.dev/schemas/toolchain.json"

node:
  version: "22.11.0"
  packageManager: "bun"
  bun:
    version: "1.3.11"

typescript:
  syncProjectReferences: true
  createMissingConfig: false
  routeOutDirToCache: false
```

- [ ] **Step 2: Commit**

```bash
git add .moon/toolchain.yml
git commit -m "chore: configure moonrepo toolchain (Node 22, Bun 1.3)"
```

---

### Task 4: Create shared TypeScript task definitions

**Files:**
- Create: `.moon/tasks/typescript-build.yml`
- Create: `.moon/tasks/typescript-lint.yml`
- Create: `.moon/tasks/typescript-test.yml`

- [ ] **Step 1: Write .moon/tasks/typescript-build.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  build:
    command: "bun run build"
    inputs:
      - "src/**/*"
      - "tsconfig.json"
      - "package.json"
    outputs:
      - ".next"
      - "dist"
      - ".output"
    options:
      cache: true
  dev:
    command: "bun run dev"
    local: true
    options:
      persistent: true
```

- [ ] **Step 2: Write .moon/tasks/typescript-lint.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  lint:
    command: "bun run lint"
    inputs:
      - "src/**/*"
      - "eslint.config.mjs"
      - "tsconfig.json"
    options:
      cache: false
  typecheck:
    command: "bun run check-types"
    inputs:
      - "src/**/*"
      - "tsconfig.json"
    options:
      cache: false
```

- [ ] **Step 3: Write .moon/tasks/typescript-test.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  test:
    command: "bun test"
    inputs:
      - "src/**/*"
      - "test/**/*"
      - "tests/**/*"
      - "vitest.config.ts"
    options:
      cache: false
  e2e:
    command: "noop"
    local: true
    options:
      cache: false
```

- [ ] **Step 4: Commit**

```bash
git add .moon/tasks/typescript-build.yml .moon/tasks/typescript-lint.yml .moon/tasks/typescript-test.yml
git commit -m "chore: add shared TypeScript task definitions"
```

---

### Task 5: Create shared Rust task definitions

**Files:**
- Create: `.moon/tasks/rust-build.yml`
- Create: `.moon/tasks/rust-test.yml`
- Create: `.moon/tasks/rust-lint.yml`

- [ ] **Step 1: Write .moon/tasks/rust-build.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  build:
    command: "cargo build --release"
    platform: system
    inputs:
      - "src/**/*"
      - "Cargo.toml"
      - "Cargo.lock"
      - "build.rs"
    outputs:
      - "target/release/*"
    options:
      cache: true
      envFile: false
  dev:
    command: "cargo run"
    platform: system
    local: true
    options:
      persistent: true
      envFile: false
```

- [ ] **Step 2: Write .moon/tasks/rust-test.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  test:
    command: "cargo test"
    platform: system
    inputs:
      - "src/**/*"
      - "Cargo.toml"
      - "Cargo.lock"
      - "tests/**/*"
    options:
      cache: false
      envFile: false
```

- [ ] **Step 3: Write .moon/tasks/rust-lint.yml**

```yaml
# https://moonrepo.dev/docs/config/tasks
$schema: "https://moonrepo.dev/schemas/tasks.json"

tasks:
  lint:
    command: "cargo clippy -- -D warnings"
    platform: system
    inputs:
      - "src/**/*"
      - "Cargo.toml"
    options:
      cache: false
      envFile: false
  fmt-check:
    command: "cargo fmt --check"
    platform: system
    inputs:
      - "src/**/*"
    options:
      cache: false
      envFile: false
```

- [ ] **Step 4: Commit**

```bash
git add .moon/tasks/rust-build.yml .moon/tasks/rust-test.yml .moon/tasks/rust-lint.yml
git commit -m "chore: add shared Rust task definitions"
```

---

### Task 6: Create per-app moon.yml for TypeScript apps

**Files:**
- Create: `apps/nextjs/moon.yml`
- Create: `apps/elysia/moon.yml`
- Create: `apps/solidjs/moon.yml`

- [ ] **Step 1: Write apps/nextjs/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "typescript"
platform: "node"
tags:
  - "lang:typescript"
  - "type:frontend"

dependsOn:
  - id: "rust-auth"
  - id: "elysia"
```

- [ ] **Step 2: Write apps/elysia/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "typescript"
platform: "bun"
tags:
  - "lang:typescript"
  - "type:backend"
```

- [ ] **Step 3: Write apps/solidjs/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "typescript"
platform: "bun"
tags:
  - "lang:typescript"
  - "type:frontend"

dependsOn:
  - id: "elysia"
  - id: "rust-auth"
```

- [ ] **Step 4: Commit**

```bash
git add apps/nextjs/moon.yml apps/elysia/moon.yml apps/solidjs/moon.yml
git commit -m "chore: add moon.yml for TypeScript apps (nextjs, elysia, solidjs)"
```

---

### Task 7: Create per-app moon.yml for Rust apps

**Files:**
- Create: `apps/rust/moon.yml`
- Create: `apps/rust-auth/moon.yml`
- Create: `apps/leptos/moon.yml`

- [ ] **Step 1: Write apps/rust/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "rust"
platform: "system"
tags:
  - "lang:rust"
  - "type:backend"

fileGroups:
  sources:
    - "src/**/*.rs"
    - "Cargo.toml"
    - "Cargo.lock"
    - "build.rs"
    - "rustfmt.toml"
```

- [ ] **Step 2: Write apps/rust-auth/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "rust"
platform: "system"
tags:
  - "lang:rust"
  - "type:backend"

fileGroups:
  sources:
    - "src/**/*.rs"
    - "Cargo.toml"
    - "Cargo.lock"
```

- [ ] **Step 3: Write apps/leptos/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "rust"
platform: "system"
tags:
  - "lang:rust"
  - "type:frontend"

dependsOn:
  - id: "rust"

fileGroups:
  sources:
    - "src/**/*.rs"
    - "Cargo.toml"
    - "Cargo.lock"
    - "Trunk.toml"
    - "rust-toolchain.toml"
```

- [ ] **Step 4: Commit**

```bash
git add apps/rust/moon.yml apps/rust-auth/moon.yml apps/leptos/moon.yml
git commit -m "chore: add moon.yml for Rust apps (rust, rust-auth, leptos)"
```

---

### Task 8: Create moon.yml for 9router

**Files:**
- Create: `apps/9router/moon.yml`

- [ ] **Step 1: Write apps/9router/moon.yml**

```yaml
# https://moonrepo.dev/docs/config/project
$schema: "https://moonrepo.dev/schemas/project.json"

type: "application"
language: "unknown"
platform: "system"
tags:
  - "type:router"

fileGroups:
  sources:
    - "src/**/*"
    - "next.config.mjs"
    - "package.json"
```

- [ ] **Step 2: Commit**

```bash
git add apps/9router/moon.yml
git commit -m "chore: add moon.yml for 9router"
```

---

### Task 9: Update flake.nix — add moon CLI to devShell

**Files:**
- Modify: `flake.nix`

- [ ] **Step 1: Add moon to nativeBuildInputs in flake.nix**

Find the `devShells.default` block in `flake.nix`. Add `moon` to `nativeBuildInputs`:

```
devShells.default = pkgs.mkShell {
  name = "ultimate-asepharyana-dev";
  nativeBuildInputs = with pkgs; [
    rustToolchain
    bun
    nodejs_22
    pkg-config
    openssl
    trunk
    wasm-bindgen-cli
    binaryen
    process-compose
    mysql84
    redis
    minio-client
    gh
    git
    moon                 # <-- add this line
  ];
  # ... shellHook unchanged
};
```

- [ ] **Step 2: Verify Nix can still evaluate the flake**

Run: `nix flake check --no-build 2>&1 | head -20`
Expected: no evaluation errors

- [ ] **Step 3: Commit**

```bash
git add flake.nix
git commit -m "chore: add moon CLI to Nix devShell"
```

---

### Task 10: Update .gitignore

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Add moonrepo cache entries to .gitignore**

Append to `.gitignore`:

```gitignore
# moonrepo
.moon/cache
.~moon
```

- [ ] **Step 2: Commit**

```bash
git add .gitignore
git commit -m "chore: add moonrepo cache entries to .gitignore"
```

---

### Task 11: Validate installation with moon check

**Files:** (none — validation only)

- [ ] **Step 1: Run moon check**

```bash
moon check
```

Expected: `OK` or zero errors. If warnings about unresolved project IDs appear, verify that `apps/*` glob in `workspace.yml` matches all project directories.

- [ ] **Step 2: Run moon query projects**

```bash
moon query projects
```

Expected: lists all 7 projects with their tags: nextjs, elysia, solidjs, rust, rust-auth, leptos, 9router

- [ ] **Step 3: Verify tag queries work**

```bash
moon query projects --tag lang:typescript
```

Expected: nextjs, elysia, solidjs

```bash
moon query projects --tag lang:rust
```

Expected: rust, rust-auth, leptos

- [ ] **Step 4: Commit (if any fixes were needed)**

No commit needed if check passes clean.

---

### Task 12: Test — moon run build on TypeScript apps

**Files:** (none — test only)

- [ ] **Step 1: Build nextjs**

```bash
moon run nextjs:build
```

Expected: `next build` runs successfully, outputs to `.next/`

- [ ] **Step 2: Build elysia**

```bash
moon run elysia:build
```

Expected: `bun build` runs successfully, outputs to `dist/`

- [ ] **Step 3: Build solidjs**

```bash
moon run solidjs:build
```

Expected: `vinxi build` runs successfully, outputs to `.output/`

- [ ] **Step 4: Verify caching on second build (nextjs)**

```bash
moon run nextjs:build
```

Expected: `Cached` — no rebuild, uses moonrepo cache

---

### Task 13: Test — moon run lint and test

**Files:** (none — test only)

- [ ] **Step 1: Run lint across TypeScript apps**

```bash
moon run :lint
```

Expected: all apps with a `lint` task run it. Note failures as they exist pre-migration (not caused by moonrepo).

- [ ] **Step 2: Run tests across TypeScript apps**

```bash
moon run :test
```

Expected: all apps with a `test` task run it.

- [ ] **Step 3: Run tag-scoped commands**

```bash
moon run --tag lang:typescript :build
```

Expected: builds nextjs, elysia, solidjs only (not Rust apps).

---

### Task 14: Final commit and documentation

**Files:**
- Modify: `.gitignore` (if any final updates)

- [ ] **Step 1: Final moon check**

```bash
moon check
```

Expected: clean, no errors.

- [ ] **Step 2: Commit any remaining changes**

```bash
git status
```

If nothing outstanding, move on.

- [ ] **Step 3: Verify the full workspace is clean**

```bash
git status
```

Expected: working tree clean.
