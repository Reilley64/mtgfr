# CI and Release

**Status:** Current (as of 2026-07-26; verify uses `mtgfr-ci` + migrate job)
**Module:** `.github/workflows/`, root `package.json`, `.husky/commit-msg`,
`.cursor/scripts/wire-cloud-git-hooks.sh`

Related: [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
(apply / Argo roll after images exist), [observability-ops](2026-07-20-observability-ops.md).

---

## Problem Statement

Image builds and releases must be traceable to a specific git commit. Merge-to-main must verify
server and client, cut semver tags from Angular commit messages, and publish GHCR images without
manual version tagging — while PR CI stays fast and cancel-superseded.

---

## Solution

**semantic-release** (default config, Angular commit convention) is the only writer of `v*` tags
and GitHub Releases. Push of those tags runs **`docker.yml`** to build/push `mtgfr-server` and
`mtgfr-web` to GHCR with Buildx GHA layer cache. PRs run **`ci.yml`** (commitlint on PR title +
reusable **`verify-jobs.yml`**). Operators then pin image tags in `iac/terraform.tfvars` and
`terraform apply` ([production-topology-and-operations](2026-07-20-production-topology-and-operations.md)).

---

## User Stories

- As an **operator**, I merge a `feat:` PR to `main`; CI runs verify, semantic-release cuts a
  `v*` tag, GHCR builds and pushes `mtgfr-server` and `mtgfr-web` images.
- As a **developer**, I merge a `fix:` PR; a `v*.*.patch` tag is cut and docker images build on
  that tag without hand-pushed version tags.
- As a **contributor**, my PR title is linted for Angular convention; superseded pushes cancel
  in-progress CI via concurrency.

---

## Behavior

### Commit convention and hooks

**Commit convention:** Angular format (`feat:`, `fix:`, `build:`, `ci:`, `docs:`, `refactor:`,
`test:`, `perf:`, `style:`; breaking changes via `BREAKING CHANGE:` footer). Enforced on each
commit by commitlint via Husky `commit-msg` (`.husky/commit-msg`). In Cursor Cloud, after
`npm clean-install`, `.cursor/scripts/wire-cloud-git-hooks.sh` restores the agent hooks
dispatcher and chains `.husky` as the original hooks path so commitlint still runs. **PRs are
squash-merged** — the squash commit subject is the PR title; semantic-release analyzes that
line only. PR CI therefore lint-checks the **PR title** only (`commitlint` job), not the
branch commit range.

### `ci.yml` (PRs)

**`ci.yml`** (PRs): `concurrency` group `ci-${{ github.ref }}` with
`cancel-in-progress: true` so superseded pushes cancel. Jobs: `changes`
(`dorny/paths-filter` for `iac/**` + `.github/workflows/ci.yml`), Commitlint (PR title),
`verify-jobs.yml`, and terraform (only when `changes.outputs.iac == 'true'`).

### `verify-jobs.yml` (reusable)

**`verify-jobs.yml`** (reusable):
- `verify-server`: pass-marker gate + parallel lint / nextest / migrate + mark +
  aggregator. Pass marker `verify-server-v3-*` hashes `crates/**`, `proto/**`,
  Cargo/Toasty lockfiles, `toasty/**`, `.config/nextest.toml`, `justfile`, this
  workflow, `docs/CR_INDEX.md`, and `scripts/gen_cr_index.py`. Key is computed
  on a clean checkout at restore and again at save (identical inputs; do not
  re-`hashFiles` after mutating the tree).
  - `verify-server-gate`: `actions/cache/restore@v5` on `.ci-pass`; emits
    `cache-hit`.
  - On miss: `verify-server-lint`, `verify-server-test` (matrix partitions
    `1`/`2`/`3`), and `verify-server-migrate` run in parallel inside
    `ghcr.io/reilley64/mtgfr-ci:latest` (`container.options: --user root` so the
    GHA workspace mount is writable). Each uses `Swatinem/rust-cache`
    `shared-key: verify-server`.
    - Lint: CR index + fmt + clippy (tools from the image; no host rustup/protoc
      installs).
    - Test shards: `cargo nextest run --profile ci --partition count:i/3` only —
      **no** Postgres service (tests use in-memory SQLite). Per-shard JUnit
      upload + test summary.
    - Migrate: Postgres 16 service + `just migrate` only;
      `DATABASE_URL=postgresql://mtgfr:mtgfr@postgres:5432/mtgfr` (service
      hostname, not `localhost`).
  - `verify-server-mark`: `actions/cache/save@v5` only when gate miss and lint +
    all test shards + migrate succeeded.
  - Aggregator job `Verify (server)`: green on cache hit, or on miss when lint +
    tests + migrate + mark succeeded.
  - On hit: lint, test, migrate, and mark jobs are skipped (`if:`); Postgres does
    not start for skipped migrate.
- `verify-client`: Bun-only `just client-check` (tokens + mana-oracle + buf
  codegen + format + lint + typecheck + vitest). Pass marker
  `verify-client-v3-*` hashes `client/**`, `proto/**`,
  `design.tokens.json`, `.bun-version`, `justfile`, and this workflow — not
  `crates/**` (wire codegen does not compile Rust). On miss: Vitest JUnit
  (`client/junit.xml`) upload + test summary. No Rust toolchain on this job.

### `verify-and-release.yml` (push to main)

**`verify-and-release.yml`** (push to main): `verify-jobs.yml` then `npx semantic-release`
(default config, no `.releaserc`). Requires `RELEASE_TOKEN` (PAT: `contents` + `workflow`) so
the `v*` tag push can cascade `docker.yml`.

### `docker.yml` (push of `v*` tags)

**`docker.yml`** (push of `v*` tags): parallel jobs `docker-server` and `docker-web`
build/push GHCR images tagged `${GITHUB_REF_NAME#v}`; `docker-visibility` runs after
both (`needs: [docker-server, docker-web]`, `continue-on-error`) to mark packages
public. `GITHUB_TOKEN` permissions: `contents: read`, `packages: write`,
`actions: write`. Each build imports/exports Buildx layers via GitHub Actions cache
(`cache-from` / `cache-to` `type=gha`, `mode=max`) with per-image scopes
`mtgfr-server` and `mtgfr-web`. Dockerfile `--mount=type=cache` Cargo mounts are not
persisted across jobs.

### `ci-image.yml` (CI toolchain image)

**`ci-image.yml`:** on push to `main`/`master` when `docker/ci/**` or this workflow
changes, and on `workflow_dispatch`, builds/pushes `ghcr.io/<owner>/mtgfr-ci:latest`
with Buildx GHA cache scope `mtgfr-ci`, then attempts to mark the package public
(`docker-ci-visibility`, `continue-on-error`). Server verify miss-path jobs pull that
image (`ghcr.io/reilley64/mtgfr-ci:latest`).

### Root package / semantic-release

**Root `package.json`:** `private: true`; `"semantic-release": "^24"` in `devDependencies`.
Not published to npm. `@semantic-release/npm` bumps `package.json` version only (private).

### End-to-end release flow

1. Merge `feat:` PR → squash commit on `main`.
2. `verify-and-release.yml` runs verify → semantic-release → `v*.*.* ` tag → GitHub Release.
3. `docker.yml` builds GHCR images on that tag.
4. Operator updates `server_image` / `web_image` in `terraform.tfvars`, runs `terraform apply`.
5. Argo syncs: migrate Jobs → new API Deployment (wave 0) → Service retarget (wave 1) → PruneLast.
6. Old pods drain in-process on SIGTERM.

(Steps 4–6 detail: [production-topology-and-operations](2026-07-20-production-topology-and-operations.md).)

---

## Implementation Decisions

- **Server pass-marker restore/save split**: `verify-server-gate` restores only;
  `verify-server-mark` saves only after lint + all nextest shards + migrate
  succeed. Lint, test, and migrate jobs share `Swatinem/rust-cache`
  `shared-key: verify-server` and run in `mtgfr-ci` (`--user root`).
- **Migrate isolated from nextest**: Postgres exists only on
  `verify-server-migrate`; nextest shards have no DB service.
- **No `.releaserc`, no custom release rules**: semantic-release default config only. Version
  bumps follow the built-in Angular analyzer. `@semantic-release/git` not used (no committed
  `CHANGELOG.md`).
- **Buildx GHA layer cache for release images**: `docker.yml` uses
  `type=gha,mode=max` with scopes `mtgfr-server` / `mtgfr-web` so multi-stage builder
  layers survive across `v*` tag builds on ephemeral runners. Requires `actions: write`.
  Cache mounts (Cargo registry/`target`) are out of scope without cache-dance.
- **PR title is the release subject**: squash-merge means semantic-release sees only the PR
  title; title PRs with `feat:` / `fix:` (or a `BREAKING CHANGE:` footer) when the merge
  should cut a release; `build:` / `ci:` / `docs:` / `refactor:` / `test:` / `style:` /
  `perf:` alone verify green and skip a version bump.

---

## Testing Decisions

- CI itself is the verification surface: `verify-jobs.yml` gates merge and release.
- `iac/` terraform validate runs from `ci.yml` when iac paths change (plan not run in CI).
- Local equivalents: `just server-check`, `just client-check`, `just check`.

---

## Out of Scope

- Hand-creating or pushing `v*` tags (forbidden — semantic-release only).
- Moving `latest` GHCR tag (pin explicit semver in `terraform.tfvars`).
- Persisting Dockerfile Cargo `--mount=type=cache` across jobs without cache-dance.
- Cluster apply / Argo roll mechanics ([production-topology-and-operations](2026-07-20-production-topology-and-operations.md)).

---

## Further Notes

- The `RELEASE_TOKEN` repo secret (PAT: `contents` + `workflow`) is required for semantic-release
  to push `v*` tags that cascade `docker.yml`. The default `GITHUB_TOKEN` cannot trigger cascade
  workflow runs on tag push.
- Image Dockerfiles and distroless runtime notes remain in
  [production-topology-and-operations](2026-07-20-production-topology-and-operations.md)
  (Container images).
