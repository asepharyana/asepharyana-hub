# Hapus Nix, Docker-Only

**Goal:** Remove all Nix configuration and adapt CI/dev to Docker-only.

**Scope:**
- Delete: `flake.nix`, `flake.lock`, `nix/`, `.envrc`, `.direnv/`
- Modify: `.github/workflows/docker-build-push.yml` — remove Nix steps, move `rust-api` to Dockerfile build
- Modify: `.vscode/settings.json` — remove `nixEnvSelector`
- Modify: `README.md` — replace `nix build` with `docker build`

**CI Changes:**
- `rust-api` uses `infra/docker/rust.Dockerfile` (already exists)
- Remove `cachix/install-nix-action`, `cachix/cachix-action`, `nix build` step
- Remove `flake.nix`, `flake.lock`, `nix/**` from path triggers
- Update path filters: `nix/apps/*.nix` → `infra/docker/*.Dockerfile`
- Remove `repository_dispatch` Nix `--override-input` logic
- Add `rust-api` to Dockerfile case + push branch

**Dev Workflow:**
- Rust toolchain via `rustup`/`rust-toolchain.toml` in `apps/rust/`
- Dev via `docker compose` instead of `process-compose`
