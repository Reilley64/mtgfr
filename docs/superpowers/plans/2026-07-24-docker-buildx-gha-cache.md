# Docker Buildx GHA Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist Buildx layer cache across `v*` GHCR builds so Docker Build summaries stop reporting near-zero cache reuse for unchanged stages.

**Architecture:** Add GitHub Actions cache export/import (`type=gha`, `mode=max`) to both `docker/build-push-action` steps in `docker.yml`, with separate scopes per image and `actions: write` so the restricted job token can write cache. Document current behavior on the production-topology surface spec. Guard with a small shell assertion script.

**Tech Stack:** GitHub Actions, Docker Buildx / `docker/build-push-action@v6`, bash

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-24-docker-buildx-gha-cache-design.md`
- Surface spec to amend: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Cache backend: `type=gha` only (not registry, not inline)
- `mode=max` required (multi-stage builder layers)
- Scopes: `mtgfr-server` and `mtgfr-web` only
- Do not change Dockerfiles, image tags, or tag-only triggers
- Do not add `buildkit-cache-dance`
- Angular commit subjects (`ci:`, `docs:`, `test:`)
- Do not index the design file in `docs/superpowers/specs/README.md`

## File map

| File | Role |
|---|---|
| `.github/workflows/docker.yml` | `actions: write` + `cache-from` / `cache-to` on both builds |
| `scripts/check-docker-workflow-cache.sh` | Asserts required cache wiring is present |
| `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md` | Document GHA layer cache as current behavior |
| `docs/superpowers/specs/2026-07-24-docker-buildx-gha-cache-design.md` | Status → Implemented after landing |

---

### Task 1: Failing check + wire `docker.yml` cache

**Files:**
- Create: `scripts/check-docker-workflow-cache.sh`
- Modify: `.github/workflows/docker.yml`
- Test: `scripts/check-docker-workflow-cache.sh`

**Interfaces:**
- Consumes: `.github/workflows/docker.yml` text
- Produces: exit 0 only when both builds use scoped GHA `mode=max` cache and workflow has `actions: write`

- [ ] **Step 1: Write the failing check script**

Create `scripts/check-docker-workflow-cache.sh`:

```bash
#!/usr/bin/env bash
# Assert docker.yml persists Buildx layers via type=gha (mode=max) per image scope.
set -euo pipefail

wf=".github/workflows/docker.yml"

if [[ ! -f "$wf" ]]; then
  echo "missing $wf" >&2
  exit 1
fi

need() {
  local pat=$1
  if ! grep -qE "$pat" "$wf"; then
    echo "docker.yml missing required pattern: $pat" >&2
    exit 1
  fi
}

need 'actions:[[:space:]]*write'
need 'cache-from:[[:space:]]*type=gha,scope=mtgfr-server'
need 'cache-to:[[:space:]]*type=gha,mode=max,scope=mtgfr-server'
need 'cache-from:[[:space:]]*type=gha,scope=mtgfr-web'
need 'cache-to:[[:space:]]*type=gha,mode=max,scope=mtgfr-web'

echo "ok: docker.yml Buildx GHA cache wiring present"
```

```bash
chmod +x scripts/check-docker-workflow-cache.sh
```

- [ ] **Step 2: Run check — expect FAIL**

```bash
./scripts/check-docker-workflow-cache.sh
```

Expected: exit 1 with messages about missing `actions: write` and/or `cache-from` / `cache-to` patterns.

- [ ] **Step 3: Update `.github/workflows/docker.yml`**

Replace the `permissions` block and both build-push steps so the file’s job section matches:

```yaml
permissions:
  contents: read
  packages: write
  actions: write

env:
  REGISTRY: ghcr.io

jobs:
  docker:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Image names (GHCR requires lowercase)
        id: image
        run: |
          owner=$(echo '${{ github.repository_owner }}' | tr '[:upper:]' '[:lower:]')
          echo "server=ghcr.io/${owner}/mtgfr-server" >> "$GITHUB_OUTPUT"
          echo "web=ghcr.io/${owner}/mtgfr-web" >> "$GITHUB_OUTPUT"
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
```

Keep the existing “Make GHCR packages public” step unchanged after the web build.

- [ ] **Step 4: Run check — expect PASS**

```bash
./scripts/check-docker-workflow-cache.sh
```

Expected: `ok: docker.yml Buildx GHA cache wiring present` and exit 0.

- [ ] **Step 5: Commit**

```bash
git add scripts/check-docker-workflow-cache.sh .github/workflows/docker.yml
git commit -m "ci: persist Buildx layer cache for GHCR release builds"
```

---

### Task 2: Amend production-topology surface spec

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Modify: `docs/superpowers/specs/2026-07-24-docker-buildx-gha-cache-design.md` (status only)
- Test: `scripts/check-docker-workflow-cache.sh` (still pass); manual read of amended paragraphs

**Interfaces:**
- Consumes: Task 1 workflow behavior
- Produces: surface-spec text describing GHA scopes / `mode=max` / `actions: write` as current behavior

- [ ] **Step 1: Update the `docker.yml` bullet under Release and CI pipeline**

Replace:

```markdown
**`docker.yml`** (push of `v*` tags): builds and pushes both GHCR images tagged with
`${GITHUB_REF_NAME#v}`. `GITHUB_TOKEN` with `packages: write` permission.
```

with:

```markdown
**`docker.yml`** (push of `v*` tags): builds and pushes both GHCR images tagged with
`${GITHUB_REF_NAME#v}`. `GITHUB_TOKEN` permissions: `contents: read`, `packages: write`,
`actions: write`. Each `docker/build-push-action` step imports/exports Buildx layers via
GitHub Actions cache (`cache-from` / `cache-to` `type=gha`, `mode=max`) with per-image
scopes `mtgfr-server` and `mtgfr-web`. Dockerfile `--mount=type=cache` Cargo mounts are
not persisted across jobs. Guard: `scripts/check-docker-workflow-cache.sh`.
```

- [ ] **Step 2: Add an Implementation Decision bullet**

In the `## Implementation Decisions` list, add:

```markdown
- **Buildx GHA layer cache for release images** (this spec): `docker.yml` uses
  `type=gha,mode=max` with scopes `mtgfr-server` / `mtgfr-web` so multi-stage builder
  layers survive across `v*` tag builds on ephemeral runners. Requires `actions: write`.
  Cache mounts (Cargo registry/`target`) are out of scope without cache-dance.
```

- [ ] **Step 3: Mark design Implemented**

In `docs/superpowers/specs/2026-07-24-docker-buildx-gha-cache-design.md`, change:

```markdown
**Status:** Design (amends [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); durable behavior lands in that surface spec)
```

to:

```markdown
**Status:** Implemented (absorbed into [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); not a separate indexed feature surface)
```

- [ ] **Step 4: Re-run cache check**

```bash
./scripts/check-docker-workflow-cache.sh
```

Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add \
  docs/superpowers/specs/2026-07-20-production-topology-and-operations.md \
  docs/superpowers/specs/2026-07-24-docker-buildx-gha-cache-design.md
git commit -m "docs: document Buildx GHA cache on release images"
```

---

## Plan self-review

1. **Spec coverage:** Goals (GHA cache, scopes, surface-spec update, check script) → Tasks 1–2. Non-goals (registry, cache-dance, Dockerfile ENV reorder, PR builds) → not tasked.
2. **Placeholders:** None.
3. **Consistency:** Scopes / permission / script path match design and both tasks.
