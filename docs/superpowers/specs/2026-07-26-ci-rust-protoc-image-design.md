# CI Rust + protoc image (design)

**Status:** Approved design input (2026-07-26).
**Surfaces:** `ci-and-release` (`.github/workflows/`, `docker/ci/`).

Related: [ci-and-release](2026-07-20-ci-and-release.md), [gha-server-verify-sharding](2026-07-26-gha-server-verify-sharding-design.md).

Ships as **two PRs**. This document is the design input for both; only **PR 1** is in scope for the first implementation plan.

---

## Problem Statement

Cold server verify still pays per-job setup for Rust toolchain, `protoc`, `just`, and nextest on the lint job and every nextest shard. That cost is duplicated across parallel jobs and is mostly identical each run. A thin prebuilt image removes that install tax without changing the pass-marker or shard model.

Separately (PR 2): every test shard today starts Postgres even though nearly all tests use in-memory SQLite; CI Postgres exists mainly to smoke `just migrate`.

## Goal

- **PR 1:** Publish `ghcr.io/<owner>/mtgfr-ci:latest` with the server-verify toolchain baked in; do **not** switch verify yet.
- **PR 2:** Run lint / nextest / migrate jobs in that container; isolate migrate (+ Postgres) in its own job so nextest shards need no database service.

## Locked decisions

| Decision | Choice |
|---|---|
| Rollout | Two PRs (image+publish first; verify switch second) |
| Image name | `ghcr.io/<lowercase-owner>/mtgfr-ci` |
| Tag | `:latest` only for verify pinning |
| Publish trigger | Push to `main` when `docker/ci/**` or the publish workflow changes; plus `workflow_dispatch` |
| Base | Thin `ubuntu:24.04` + rustup (not `.cursor/Dockerfile`, not `rust:` official image) |
| PR 2 DB job | Postgres + `just migrate` only (no nextest filter set) |
| Cargo cache | Keep `Swatinem/rust-cache` `shared-key: verify-server` after the switch |

## Approaches considered

1. **Thin ubuntu + rustup image (chosen)** — Small, explicit toolchain; publish on main path changes.
2. `FROM rust:bookworm` — Less DIY; fatter / more tag churn.
3. Reuse `.cursor/Dockerfile` — Couples Cloud agent image to CI; oversized pulls.

## Design — PR 1 (this implementation)

### Image (`docker/ci/Dockerfile`)

Bake:

- `build-essential`, `pkg-config`, `ca-certificates`, `curl`, `git`, `protobuf-compiler`, `python3` (CR index scripts)
- Rust **stable** via rustup (`minimal` profile) + `rustfmt` + `clippy`
- `just` and `cargo-nextest` (pin versions or install from known release/crates in the Dockerfile)

Do **not** bake: Bun/Node, Postgres server, app source, Cargo `target/`, client toolchain.

Run as a non-root user with tools on `PATH` (suitable for later GHA `container:` jobs).

### Publish workflow (`.github/workflows/ci-image.yml`)

- **Triggers:** `push` to `main` with paths `docker/ci/**`, `.github/workflows/ci-image.yml`; `workflow_dispatch`
- **Permissions:** `contents: read`, `packages: write`, `actions: write` (GHA Buildx cache)
- **Steps:** checkout → Buildx → GHCR login → build/push `mtgfr-ci:latest` with cache scope `mtgfr-ci` → mark package public (same pattern as `docker.yml` visibility job for `mtgfr-server` / `mtgfr-web`)
- **Does not** modify `verify-jobs.yml`

### Docs (PR 1)

Update living [ci-and-release](2026-07-20-ci-and-release.md): document that `mtgfr-ci` is built/pushed on main when `docker/ci` or `ci-image.yml` changes; verify still uses host-installed toolchain until PR 2. Index this design in the specs README.

### Verification (PR 1)

1. Merge (or `workflow_dispatch` on `main` after merge) → workflow green → `ghcr.io/<owner>/mtgfr-ci:latest` exists and is public.
2. Smoke: `docker run --rm ghcr.io/<owner>/mtgfr-ci:latest bash -lc 'rustc --version && protoc --version && just --version && cargo nextest --version'`

### Out of scope (PR 1)

- Switching verify jobs to `container:`
- Changing nextest shard count or pass-marker semantics
- Baking `target/` or Postgres into the image

---

## Design — PR 2 (follow-up PR)

### Containerize verify

On miss path, `verify-server-lint`, `verify-server-test` (matrix), and the new migrate job set:

```yaml
container:
  image: ghcr.io/<lowercase-owner>/mtgfr-ci:latest
```

Drop host steps that install rustup, protoc, just, and nextest. Keep checkout, rust-cache (`shared-key: verify-server`), and JUnit upload/summary.

### Migrate job

New job (e.g. `verify-server-migrate`):

- Needs gate; skipped on pass-marker hit
- Postgres 16 service (same credentials as today)
- `just migrate` only
- Uses CI image
- **No** nextest

Test shards **remove** `services: postgres` and the migrate step. Engine/server tests continue to use in-memory SQLite as today.

### Pass marker / aggregator

Mark saves only when gate was a miss **and** lint + **all** nextest shards + **migrate** succeeded. Aggregator `Verify (server)` treats migrate like the other required miss-path jobs (skipped on hit ⇒ green; unexpected skip/fail on miss ⇒ red).

### Postgres networking

With `container:` + `services:`, use the Compose-style hostname `postgres` in `DATABASE_URL` for the migrate job (not `localhost`). Document the URL in the living spec when PR 2 ships.

### Docs (PR 2)

Update `ci-and-release` Behavior to the final job graph (gate → lint ∥ test shards ∥ migrate → mark → aggregator) and container pin.

### Out of scope (PR 2)

- Nextest filters for “DB tests” (none required; migrate-only)
- More than three shards
- Maelstrom / larger runners

---

## Error / degradation

| Condition | Behavior |
|---|---|
| PR 1 publish fails | `:latest` unchanged; verify unaffected (still host installs) |
| PR 2 pulls missing/private image | Verify jobs fail at pull — package must stay public after PR 1 |
| Migrate fails (PR 2) | Aggregator red; pass marker not saved |
| One nextest shard fails (PR 2) | Same as today; marker not saved |

## Further Notes

- Public-repo standard hosted runners do not bill Actions minutes; image work is for wall-clock / simpler jobs, not spend.
- Rust **stable** inside the image advances only when the Dockerfile is rebuilt (path change or `workflow_dispatch`), not on every CI run — intentional.
