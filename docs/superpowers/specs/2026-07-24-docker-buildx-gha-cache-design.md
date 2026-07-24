# Docker Buildx GHA layer cache for release images

**Status:** Implemented (absorbed into [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); not a separate indexed feature surface)
**Date:** 2026-07-24
**Module:** `.github/workflows/docker.yml`
**Approach:** Persist Buildx layer cache via GitHub Actions cache (`type=gha`, `mode=max`), scoped per image

---

## Problem

`docker.yml` builds and pushes `mtgfr-server` and `mtgfr-web` on every `v*` tag using
`docker/build-push-action`, but never sets `cache-from` / `cache-to`. Each job runs on a
fresh `ubuntu-latest` runner, so BuildKit starts cold. The Docker Build job summary
cache percentage stays near zero even when lockfiles and base stages are unchanged.
Pushing the final image to GHCR does not feed Buildx layer reuse.

---

## Goals

- Restore Buildx layers across release builds so unchanged stages (base images, `apt` /
  `bun install`, unchanged `COPY` prefixes) hit cache.
- Keep server and web caches independent so one image’s layers do not evict the other’s.
- Document the behavior on the production-topology surface spec (no standalone indexed
  feature surface for this).

## Non-goals

- Registry cache (`type=registry`) or dual backends.
- `buildkit-cache-dance` / persisting Dockerfile `--mount=type=cache` Cargo mounts across
  jobs (BuildKit does not put cache mounts into GHA cache by default).
- Dockerfile ARG/ENV reorder to keep `APP_VERSION` / `GIT_COMMIT` from busting the web
  `bun run build` layer.
- Running image builds on PRs (still tag-only).
- Guaranteeing a high summary percentage on every release when sources always change.

---

## Behavior

### Workflow

On each `v*` tag build of `mtgfr-server` and `mtgfr-web`:

1. Buildx **imports** cache with `cache-from: type=gha,scope=<image-scope>`.
2. Buildx **exports** all stages with `cache-to: type=gha,mode=max,scope=<image-scope>`.
3. Scopes are `mtgfr-server` and `mtgfr-web` respectively.
4. Workflow `permissions` include `actions: write` so the restricted token can write the
   Actions cache (alongside existing `contents: read` and `packages: write`).

### Expected cache behavior

- **Hit:** unchanged early layers (base `FROM`, `apt-get` / `bun install`, copies whose
  inputs are unchanged).
- **Miss (acceptable):** layers after changed sources; web `bun run build` when
  `APP_VERSION` / `GIT_COMMIT` or client tree changes; server `cargo build` when crates
  change. Cargo registry/`target` cache mounts remain ephemeral without cache-dance.

### Spec home

Implementation updates **production-topology-and-operations** under Release and CI /
Implementation Decisions. This design file is the brainstorming record only; it is not
a separate indexed feature surface.

---

## Testing Decisions

- Assert workflow YAML contains `cache-from` / `cache-to` with `type=gha`, `mode=max`,
  both scopes, and `actions: write` (small script or grep-based check run in CI or locally
  before merge). No live Buildx run required in PR CI (tag-only workflow).

---

## Out of Scope

- Inline cache (`type=inline` is `mode=min` only — wrong for multi-stage).
- Warming cache from PR builds.
- Changing image tags, GHCR visibility, or Dockerfile stages.
