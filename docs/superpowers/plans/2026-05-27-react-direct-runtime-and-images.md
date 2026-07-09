# React Direct Runtime and Images Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve the React SPA without nginx and remove frontend image-cache proxy calls so browser uses direct image/API URLs.

**Architecture:** The React Docker image still builds with Bun/Vite, but runtime uses `vite preview` from Bun instead of nginx. `CachedImage` normalizes image URLs and renders them directly, keeping fallback/retry UI but removing `/proxy/image-cache` auditing. Traefik keeps routing `react-web:80`, so compose and dynamic routing stay stable.

**Tech Stack:** Bun, Vite, React, TypeScript, Docker, Docker Compose, Traefik.

---

## Tasks

### Task 1: Switch React Runtime From nginx to Bun/Vite Preview

Modify `infra/docker/react.Dockerfile` so runtime stage uses `oven/bun:1-alpine`, installs production deps, copies `/app/dist`, exposes 80, and runs `bunx vite preview --host 0.0.0.0 --port 80`. Verify nginx runtime/copy lines are gone and Vite preview command exists.

### Task 2: Remove Image Cache Proxy From CachedImage

Modify `apps/react/src/components/ui/cached-image.tsx` to remove `API_BASE_URL` import, `/proxy/image-cache` POST, audit state/function, and auditing overlay. Keep direct normalized image URLs, retry behavior, and fallback image.

### Task 3: Verify Build and Config

Run static checks for both tasks, `bun --cwd apps/react run build`, `docker build -f infra/docker/react.Dockerfile -t react-web:test .`, start test container on `18080:80`, curl root, clean container, and run `git diff --check`.

### Task 4: Commit and Push

Commit changed files: `infra/docker/react.Dockerfile`, `apps/react/src/components/ui/cached-image.tsx`, `docs/superpowers/plans/2026-05-27-react-direct-runtime-and-images.md`. Push branch only after verification if user asks; do not push from worktree unless explicitly instructed.
