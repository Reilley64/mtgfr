# CI Improvement Wave 3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cut release Docker wall-clock by building `mtgfr-server` and `mtgfr-web` in parallel jobs, keeping per-image Buildx GHA caches and GHCR visibility step.

**Architecture:** Replace the single sequential `docker` job with three jobs: `docker-server`, `docker-web` (parallel), and `docker-visibility` (`needs` both). Keep tag-only `v*` trigger, permissions, and `type=gha` scopes. Extend the docker-cache guard + add `scripts/check-ci-wave3.sh`. Do **not** redesign pass-markers, path filters, or the release cascade — recent PR CI evidence (med ~84s success, ~12% cancel) does not justify that rewrite.

**Tech Stack:** GitHub Actions, Docker Buildx / `docker/build-push-action@v6`, bash guards

## Evidence (Wave 3 gate)

| Signal | Observation | Decision |
|---|---|---|
| PR CI success duration | med ~84s, p90 ~98s (n=25 success of 50) | No larger runners / verify redesign |
| PR cancel rate | ~12% | Wave 1 concurrency working |
| Docker wall-clock | ~7–9 min single job, sequential server then web | **Parallelize Docker jobs** |
| Pass-marker / release cascade | No bottleneck evidence | Out of scope |

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md` (Wave 3 — parallel Docker only)
- Surface spec: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Keep `on.push.tags: ["v*"]` only (no `create` double-trigger)
- Keep per-image GHA cache scopes `mtgfr-server` / `mtgfr-web`, `mode=max`
- Do not change Dockerfiles, image tag scheme, or `RELEASE_TOKEN` cascade
- Do not redesign `verify-jobs.yml` pass-markers or path filters
- Angular commit subjects (`ci:`, `docs:`, `test:`)
- Wave 1 + Wave 2 guards must remain green

## File map

| File | Role |
|---|---|
| `.github/workflows/docker.yml` | Parallel `docker-server` / `docker-web` + `docker-visibility` |
| `scripts/check-docker-workflow-cache.sh` | Also assert parallel job ids |
| `scripts/check-ci-wave3.sh` | Wave 3 wiring guard |
| `.github/workflows/ci.yml` | `ci-wave3-guard` job |
| `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md` | Document parallel Docker |
| `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md` | Status → Wave 3 implemented |

---

### Task 1: Failing Wave 3 guard + extend docker-cache script

**Files:**
- Create: `scripts/check-ci-wave3.sh`
- Modify: `scripts/check-docker-workflow-cache.sh`
- Test: both scripts (wave3 FAIL until Task 2; docker-cache may FAIL after its own new asserts)

**Interfaces:**
- Consumes: `.github/workflows/docker.yml`, `.github/workflows/ci.yml`
- Produces: exit 0 when parallel jobs + wave3 CI guard present; still requires GHA cache patterns

- [ ] **Step 1: Extend `scripts/check-docker-workflow-cache.sh`**

After the existing `need` calls, add:

```bash
need 'docker-server:'
need 'docker-web:'
need 'docker-visibility:'
need 'needs:[[:space:]]*\[docker-server,[[:space:]]*docker-web\]'
```

Keep the final `ok: docker.yml Buildx GHA cache wiring present` line.

- [ ] **Step 2: Create `scripts/check-ci-wave3.sh`**

```bash
#!/usr/bin/env bash
# Assert CI Wave 3 wiring (parallel Docker jobs).
set -euo pipefail

docker=".github/workflows/docker.yml"
ci=".github/workflows/ci.yml"

need_file() {
  local f=$1
  if [[ ! -f "$f" ]]; then
    echo "missing $f" >&2
    exit 1
  fi
}

need_grep() {
  local f=$1
  local pat=$2
  local label=$3
  if ! grep -qE "$pat" "$f"; then
    echo "$label: missing pattern in $f: $pat" >&2
    exit 1
  fi
}

need_file "$docker"
need_file "$ci"

need_grep "$docker" '^  docker-server:' "docker-server job"
need_grep "$docker" '^  docker-web:' "docker-web job"
need_grep "$docker" '^  docker-visibility:' "docker-visibility job"
need_grep "$docker" 'needs:[[:space:]]*\[docker-server,[[:space:]]*docker-web\]' "visibility needs both builds"
need_grep "$docker" 'file:[[:space:]]*docker/server/Dockerfile' "server Dockerfile"
need_grep "$docker" 'file:[[:space:]]*docker/web/Dockerfile' "web Dockerfile"
need_grep "$docker" 'scope=mtgfr-server' "server cache scope"
need_grep "$docker" 'scope=mtgfr-web' "web cache scope"

# Must not keep a monolithic sequential job named exactly `docker:` at jobs root
if grep -qE '^  docker:' "$docker"; then
  echo "docker.yml still has monolithic job 'docker:' — expected parallel jobs" >&2
  exit 1
fi

need_grep "$ci" 'check-ci-wave3\.sh' "wave3 guard step"

./scripts/check-docker-workflow-cache.sh
./scripts/check-ci-wave2.sh

echo "ok: CI Wave 3 wiring present"
```

```bash
chmod +x scripts/check-ci-wave3.sh
```

- [ ] **Step 3: Run — expect FAIL**

```bash
./scripts/check-docker-workflow-cache.sh
./scripts/check-ci-wave3.sh
```

Expected: docker-cache exits 1 (missing `docker-server:`); wave3 exits 1.

- [ ] **Step 4: Commit**

```bash
git add scripts/check-docker-workflow-cache.sh scripts/check-ci-wave3.sh
git commit -m "test: add CI Wave 3 parallel Docker wiring guards"
```

---

### Task 2: Parallelize `docker.yml`

**Files:**
- Modify: `.github/workflows/docker.yml`
- Test: `./scripts/check-docker-workflow-cache.sh` → PASS; `./scripts/check-ci-wave3.sh` still FAIL until Task 3 (missing ci-wave3-guard)

**Interfaces:**
- Produces: three jobs as below; shared permissions/env/trigger unchanged

- [ ] **Step 1: Replace `.github/workflows/docker.yml` with**

```yaml
name: Docker

# Build + push GHCR images for `v*` tags.
# Verify already ran `just check` on this commit (verify-and-release → semantic-release).
# semantic-release must create tags with `RELEASE_TOKEN` (PAT: contents + workflow) —
# `GITHUB_TOKEN` cannot cascade workflow runs. Do not also listen for `create`: GitHub
# emits both `create` and `push` for the same tag, which doubles the build.
# Wire clients (Effect-gRPC) and tonic stubs are regenerated inside each Dockerfile —
# nothing under `client/src/wire/generated/` or Cargo `OUT_DIR` is committed.
# Wave 3: build server and web images in parallel; visibility step waits on both.

on:
  push:
    tags:
      - "v*"

permissions:
  contents: read
  packages: write
  actions: write

env:
  REGISTRY: ghcr.io

jobs:
  docker-server:
    name: Build mtgfr-server
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5

      - name: Image names (GHCR requires lowercase)
        id: image
        run: |
          owner=$(echo '${{ github.repository_owner }}' | tr '[:upper:]' '[:lower:]')
          echo "server=ghcr.io/${owner}/mtgfr-server" >> "$GITHUB_OUTPUT"
          echo "version=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push mtgfr-server
        uses: docker/build-push-action@v6
        with:
          context: .
          file: docker/server/Dockerfile
          push: true
          tags: ${{ steps.image.outputs.server }}:${{ steps.image.outputs.version }}
          cache-from: type=gha,scope=mtgfr-server
          cache-to: type=gha,mode=max,scope=mtgfr-server
          build-args: |
            APP_VERSION=${{ steps.image.outputs.version }}
            GIT_COMMIT=${{ github.sha }}

  docker-web:
    name: Build mtgfr-web
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5

      - name: Image names (GHCR requires lowercase)
        id: image
        run: |
          owner=$(echo '${{ github.repository_owner }}' | tr '[:upper:]' '[:lower:]')
          echo "web=ghcr.io/${owner}/mtgfr-web" >> "$GITHUB_OUTPUT"
          echo "version=${GITHUB_REF_NAME#v}" >> "$GITHUB_OUTPUT"

      - uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push mtgfr-web
        uses: docker/build-push-action@v6
        with:
          context: .
          file: docker/web/Dockerfile
          push: true
          tags: ${{ steps.image.outputs.web }}:${{ steps.image.outputs.version }}
          cache-from: type=gha,scope=mtgfr-web
          cache-to: type=gha,mode=max,scope=mtgfr-web
          build-args: |
            VITE_CARD_CDN=${{ vars.VITE_CARD_CDN }}
            APP_VERSION=${{ steps.image.outputs.version }}
            GIT_COMMIT=${{ github.sha }}

  docker-visibility:
    name: Make GHCR packages public
    needs: [docker-server, docker-web]
    runs-on: ubuntu-latest
    continue-on-error: true
    steps:
      - name: Make GHCR packages public
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          owner=$(echo '${{ github.repository_owner }}' | tr '[:upper:]' '[:lower:]')
          for pkg in mtgfr-server mtgfr-web; do
            gh api --method PATCH \
              "/user/packages/container/${pkg}" \
              -f visibility=public || \
            gh api --method PATCH \
              "/orgs/${owner}/packages/container/${pkg}" \
              -f visibility=public || true
          done
```

- [ ] **Step 2: Verify**

```bash
./scripts/check-docker-workflow-cache.sh
./scripts/check-ci-wave3.sh || true  # expect fail: missing ci-wave3-guard
```

Expected: docker-cache exit 0; wave3 exit 1 on missing `check-ci-wave3.sh` in ci.yml.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/docker.yml
git commit -m "ci: build GHCR server and web images in parallel"
```

---

### Task 3: Wire `ci-wave3-guard` + docs

**Files:**
- Modify: `.github/workflows/ci.yml` (append guard job)
- Modify: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Modify: `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md`
- Test: `./scripts/check-ci-wave3.sh` → PASS

- [ ] **Step 1: Append to `ci.yml` after `ci-wave2-guard`:**

```yaml
  ci-wave3-guard:
    name: CI Wave 3 guard
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Assert CI Wave 3 wiring
        run: ./scripts/check-ci-wave3.sh
```

Also update the top comment to mention Wave 3 if present.

- [ ] **Step 2: Replace the `docker.yml` paragraph under Release and CI with:**

```markdown
**`docker.yml`** (push of `v*` tags): parallel jobs `docker-server` and `docker-web`
build/push GHCR images tagged `${GITHUB_REF_NAME#v}`; `docker-visibility` runs after
both (`needs: [docker-server, docker-web]`, `continue-on-error`) to mark packages
public. `GITHUB_TOKEN` permissions: `contents: read`, `packages: write`,
`actions: write`. Each build imports/exports Buildx layers via GitHub Actions cache
(`cache-from` / `cache-to` `type=gha`, `mode=max`) with per-image scopes
`mtgfr-server` and `mtgfr-web`. Dockerfile `--mount=type=cache` Cargo mounts are not
persisted across jobs. Guards: `scripts/check-docker-workflow-cache.sh`,
`scripts/check-ci-wave3.sh`.
```

Update the `ci.yml` bullet to include `ci-wave3-guard` in the always-on guards list.

- [ ] **Step 3: Set roadmap status to:**

```markdown
**Status:** Wave 3 implemented — parallel Docker jobs (absorbed into [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); further redesign remains evidence-gated)
```

- [ ] **Step 4: Final verification**

```bash
./scripts/check-ci-wave1.sh
./scripts/check-ci-wave2.sh
./scripts/check-ci-wave3.sh
./scripts/check-docker-workflow-cache.sh
```

Expected: all exit 0.

- [ ] **Step 5: Commit**

```bash
git add \
  .github/workflows/ci.yml \
  docs/superpowers/specs/2026-07-20-production-topology-and-operations.md \
  docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md
git commit -m "docs: record parallel Docker Wave 3 and wire CI guard"
```

(If preferred, split into `ci:` + `docs:` commits — one commit is fine when both land together.)

---

## Plan self-review

| Spec Wave 3 candidate | Decision |
|---|---|
| Parallel Docker jobs | Task 2 — in scope |
| Revisit pass-marker / path filters / release cascade | Out of scope (no evidence) |
| Larger runners / alternate caches | Out of scope (parallel Docker addresses release wall-clock) |

No TBD. Cache scopes and tag trigger unchanged.
