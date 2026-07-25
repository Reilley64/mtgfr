# CI Improvement Wave 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Wave 1 of the CI roadmap — cancel superseded PR runs, path-skip terraform, and wire cheap missing gates (`engine-cr-index-check`, `client-mana-oracle-check`, docker-cache guard) into CI.

**Architecture:** Keep reusable `verify-jobs.yml` and the verify → semantic-release → `v*` → Docker cascade. Extend `ci.yml` with concurrency cancel, a `dorny/paths-filter` changes job gating terraform, and always-on cheap guard jobs. Fold CR-index and mana-oracle checks into `just server-check` / `just client-check` so local and CI share one path; extend server pass-marker `hashFiles` for CR index inputs. Guard Wave 1 wiring with `scripts/check-ci-wave1.sh`.

**Tech Stack:** GitHub Actions, `dorny/paths-filter@v3`, just, bash, existing `scripts/check-docker-workflow-cache.sh` / `scripts/gen_cr_index.py`

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md` (Wave 1 only)
- Surface spec to amend when shipping: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`
- Do not change `verify-and-release.yml` concurrency (`cancel-in-progress: false` stays)
- Do not redesign pass-markers beyond adding hash inputs / bumping key version to `v2`
- Do not implement Wave 2 (Postgres-on-skip, hash narrowing, Vitest JUnit, Node 20 cleanup) or Wave 3
- Do not add Playwright / live-game verify to Actions
- Angular commit subjects (`ci:`, `docs:`, `test:`)
- Do not index the design file in `docs/superpowers/specs/README.md`

## File map

| File | Role |
|---|---|
| `scripts/check-ci-wave1.sh` | Assert Wave 1 wiring in `ci.yml`, `verify-jobs.yml`, `justfile` |
| `.github/workflows/ci.yml` | Concurrency cancel; changes job; terraform `if:`; docker-cache guard (Task 2); `ci-wave1-guard` (Task 3) |
| `.github/workflows/verify-jobs.yml` | Pass-marker `v2` + CR index hash inputs; step name comments |
| `justfile` | `server-check` / `client-check` / `check` gain CR-index + mana-oracle |
| `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md` | Document shipped Wave 1 behavior |
| `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md` | Status → Wave 1 implemented |

---

### Task 1: Failing Wave 1 guard script

**Files:**
- Create: `scripts/check-ci-wave1.sh`
- Test: `scripts/check-ci-wave1.sh`

**Interfaces:**
- Consumes: text of `.github/workflows/ci.yml`, `.github/workflows/verify-jobs.yml`, `justfile`
- Produces: exit 0 only when Wave 1 concurrency, terraform gate, guard jobs, just recipes, and server hash inputs are present; also invokes `./scripts/check-docker-workflow-cache.sh`

- [ ] **Step 1: Write the failing check script**

Create `scripts/check-ci-wave1.sh`:

```bash
#!/usr/bin/env bash
# Assert CI Wave 1 wiring (concurrency cancel, terraform path-skip, cheap gates).
set -euo pipefail

ci=".github/workflows/ci.yml"
verify=".github/workflows/verify-jobs.yml"
justfile="justfile"

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

need_file "$ci"
need_file "$verify"
need_file "$justfile"
need_file "scripts/check-docker-workflow-cache.sh"

# PR concurrency cancel
need_grep "$ci" 'concurrency:' "ci concurrency"
need_grep "$ci" 'group:[[:space:]]*ci-\$\{\{[[:space:]]*github\.ref[[:space:]]*\}\}' "ci concurrency group"
need_grep "$ci" 'cancel-in-progress:[[:space:]]*true' "ci cancel-in-progress"

# paths-filter changes job + terraform gate
need_grep "$ci" 'dorny/paths-filter@v3' "paths-filter action"
need_grep "$ci" "iac/\*\*" "iac path filter"
need_grep "$ci" '\.github/workflows/ci\.yml' "ci.yml path filter"
need_grep "$ci" 'needs:[[:space:]]*changes' "terraform needs changes"
need_grep "$ci" "needs\.changes\.outputs\.iac[[:space:]]*==[[:space:]]*'true'" "terraform if iac"

# Always-on cheap guards
need_grep "$ci" 'check-docker-workflow-cache\.sh' "docker cache guard step"
need_grep "$ci" 'check-ci-wave1\.sh' "wave1 guard step"

# just recipes
need_grep "$justfile" '^server-check:.*engine-cr-index-check' "server-check includes engine-cr-index-check"
need_grep "$justfile" '^client-check:.*client-mana-oracle-check' "client-check includes client-mana-oracle-check"
need_grep "$justfile" '^check:.*engine-cr-index-check' "check includes engine-cr-index-check"
need_grep "$justfile" '^check:.*client-mana-oracle-check' "check includes client-mana-oracle-check"

# server pass-marker includes CR index inputs + v2 key
need_grep "$verify" 'verify-server-v2-' "server pass marker v2"
need_grep "$verify" "docs/CR_INDEX\.md" "server hashFiles CR_INDEX"
need_grep "$verify" "scripts/gen_cr_index\.py" "server hashFiles gen_cr_index"

./scripts/check-docker-workflow-cache.sh

echo "ok: CI Wave 1 wiring present"
```

```bash
chmod +x scripts/check-ci-wave1.sh
```

- [ ] **Step 2: Run check — expect FAIL**

```bash
./scripts/check-ci-wave1.sh
```

Expected: exit 1 with at least one “missing pattern” message (concurrency and/or just recipes and/or hashFiles).

- [ ] **Step 3: Commit**

```bash
git add scripts/check-ci-wave1.sh
git commit -m "test: add CI Wave 1 wiring guard script"
```

---

### Task 2: `ci.yml` — concurrency, path-skip terraform, docker-cache guard

**Files:**
- Modify: `.github/workflows/ci.yml`
- Test: local grep + `./scripts/check-docker-workflow-cache.sh`  
  Do **not** add the `ci-wave1-guard` job yet — `check-ci-wave1.sh` still fails until Task 3 finishes justfile/hash wiring; adding a self-guard job here would red the PR mid-way.

**Interfaces:**
- Consumes: `dorny/paths-filter@v3`; outputs `changes.outputs.iac` as `'true'` / `'false'`
- Produces: cancelled superseded PR runs; terraform skipped when `iac` filter false; always-on docker-cache guard job

- [ ] **Step 1: Replace `.github/workflows/ci.yml` with**

```yaml
name: CI

# Deploy PRD §GitHub Actions — PR verify via reusable verify-jobs.yml; terraform validates iac/.
# Mirrors verify in verify-and-release.yml; no release steps here.
# Wave 1: cancel superseded PR runs; path-skip terraform; cheap always-on guards.

on:
  pull_request:
    branches:
      - main
      - master

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

jobs:
  changes:
    name: Detect changes
    runs-on: ubuntu-latest
    outputs:
      iac: ${{ steps.filter.outputs.iac }}
    steps:
      - uses: actions/checkout@v4

      - uses: dorny/paths-filter@v3
        id: filter
        with:
          filters: |
            iac:
              - 'iac/**'
              - '.github/workflows/ci.yml'

  commitlint:
    name: Commitlint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - uses: actions/setup-node@v4
        with:
          node-version-file: .node-version
          cache: npm

      - run: npm clean-install

      - name: Lint PR commits
        run: npx commitlint --from "origin/${{ github.base_ref }}" --to "${{ github.event.pull_request.head.sha }}" --verbose

  verify:
    name: Verify
    uses: ./.github/workflows/verify-jobs.yml

  terraform:
    name: Terraform (iac/)
    needs: changes
    if: needs.changes.outputs.iac == 'true'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: hashicorp/setup-terraform@v3
        with:
          terraform_wrapper: false

      - name: terraform fmt -check
        working-directory: iac
        run: terraform fmt -check -recursive

      - name: terraform init (no backend — this only validates syntax/config, no state access)
        working-directory: iac
        run: terraform init -backend=false

      - name: terraform validate
        working-directory: iac
        run: terraform validate

  docker-cache-guard:
    name: Docker workflow cache guard
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Assert docker.yml Buildx GHA cache wiring
        run: ./scripts/check-docker-workflow-cache.sh
```

- [ ] **Step 2: Sanity-check YAML keys locally**

```bash
grep -E 'concurrency:|cancel-in-progress:|dorny/paths-filter|needs\.changes\.outputs\.iac|check-docker-workflow-cache' .github/workflows/ci.yml
./scripts/check-docker-workflow-cache.sh
```

Expected: grep hits for concurrency, paths-filter, terraform `if`, docker-cache script; docker-cache script exits 0. `./scripts/check-ci-wave1.sh` still exits 1 (justfile / hashFiles / `ci-wave1-guard` not done).

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: cancel superseded PR runs and path-skip terraform"
```

---

### Task 3: Fold cheap checks into just + pass-marker hashes + enable wave1 guard job

**Files:**
- Modify: `justfile` (recipes `server-check`, `client-check`, `check`)
- Modify: `.github/workflows/verify-jobs.yml` (server `hashFiles` + key `v2`; step display names)
- Modify: `.github/workflows/ci.yml` (append `ci-wave1-guard` job)
- Test: `scripts/check-ci-wave1.sh`; `just engine-cr-index-check`; `just client-mana-oracle-check`

**Interfaces:**
- Consumes: existing `engine-cr-index-check` and `client-mana-oracle-check` recipes; Task 2 `ci.yml`
- Produces: `server-check` / `client-check` / `check` always run those gates; server pass-marker invalidates when `docs/CR_INDEX.md` or `scripts/gen_cr_index.py` change; PR CI always runs `./scripts/check-ci-wave1.sh`

- [ ] **Step 1: Update `justfile` recipes**

Replace the three recipes so they match exactly:

```just
[doc("Server CI check (fmt + clippy + CR index + migrate + nextest)")]
server-check: server-format server-lint engine-cr-index-check
    cargo run -p server -- migration apply
    just server-test

[doc("Client CI check (tokens + mana-oracle + codegen + format + lint + typecheck + vitest)")]
client-check: client-tokens-check client-mana-oracle-check server-codegen client-format client-lint client-typecheck client-test

[doc("Run all checks")]
check: client-tokens-check client-mana-oracle-check engine-cr-index-check server-codegen format lint typecheck test
```

- [ ] **Step 2: Update pass-markers and step names in `.github/workflows/verify-jobs.yml`**

Change the server cache step `key` to:

```yaml
          key: verify-server-v2-${{ hashFiles('crates/**', 'proto/**', 'Cargo.toml', 'Cargo.lock', 'Toasty.toml', 'toasty/**', '.config/nextest.toml', 'justfile', '.github/workflows/verify-jobs.yml', 'docs/CR_INDEX.md', 'scripts/gen_cr_index.py') }}
```

Bump the client key version for consistency when `justfile` check meaning changes (hash already includes `justfile` + `client/**`):

```yaml
          key: verify-client-v2-${{ hashFiles('client/**', 'proto/**', 'crates/**', 'Cargo.toml', 'Cargo.lock', '.bun-version', 'justfile', '.github/workflows/verify-jobs.yml') }}
```

Update the server check step name/comment to mention CR index:

```yaml
      - name: just server-check (fmt + clippy + CR index + migrate + nextest)
        if: steps.pass.outputs.cache-hit != 'true'
        run: just server-check
```

Update the client check step name:

```yaml
      - name: just client-check (tokens + mana-oracle + codegen + format + lint + typecheck + vitest)
        if: steps.pass.outputs.cache-hit != 'true'
        run: just client-check
```

- [ ] **Step 3: Append `ci-wave1-guard` to `.github/workflows/ci.yml`**

After the `docker-cache-guard` job, add:

```yaml
  ci-wave1-guard:
    name: CI Wave 1 guard
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Assert CI Wave 1 wiring
        run: ./scripts/check-ci-wave1.sh
```

- [ ] **Step 4: Run guards and local cheap checks — expect PASS**

```bash
./scripts/check-ci-wave1.sh
just engine-cr-index-check
just client-mana-oracle-check
```

Expected:
- `ok: CI Wave 1 wiring present`
- `ok: docker.yml Buildx GHA cache wiring present` (via the wave1 script)
- CR index check exit 0
- mana-oracle check exit 0

- [ ] **Step 5: Commit**

```bash
git add justfile .github/workflows/verify-jobs.yml .github/workflows/ci.yml
git commit -m "ci: gate server/client checks on CR index and mana-oracle"
```

---

### Task 4: Docs — production-topology + design status

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md` (Release and CI pipeline)
- Modify: `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md` (status header)
- Test: manual read of amended paragraphs; `./scripts/check-ci-wave1.sh`

**Interfaces:**
- Consumes: shipped Wave 1 behavior from Tasks 2–3
- Produces: surface spec matches reality; roadmap marks Wave 1 implemented

- [ ] **Step 1: Replace the `ci.yml` / `verify-jobs.yml` bullets under “Release and CI pipeline”**

In `docs/superpowers/specs/2026-07-20-production-topology-and-operations.md`, replace the paragraphs that currently begin with **`ci.yml`** and **`verify-jobs.yml`** with:

```markdown
**`ci.yml`** (PRs): `concurrency` group `ci-${{ github.ref }}` with
`cancel-in-progress: true` so superseded pushes cancel. Jobs: `changes`
(`dorny/paths-filter` for `iac/**` + `.github/workflows/ci.yml`), commitlint,
`verify-jobs.yml`, terraform (only when `changes.outputs.iac == 'true'`),
always-on `docker-cache-guard` (`scripts/check-docker-workflow-cache.sh`) and
`ci-wave1-guard` (`scripts/check-ci-wave1.sh`).

**`verify-jobs.yml`** (reusable): two parallel jobs:
- `verify-server`: `just server-check` (fmt + clippy + `engine-cr-index-check` +
  migrate + nextest) — needs Rust + Postgres.
- `verify-client`: `just client-check` (tokens + `client-mana-oracle-check` +
  proto codegen + format + lint + typecheck + vitest) — needs Bun (+ Rust for
  codegen).
- Content-hash skip: each job caches a pass marker keyed by `hashFiles` of its
  side's inputs (`verify-server-v2-*` / `verify-client-v2-*`); server hash
  includes `docs/CR_INDEX.md` and `scripts/gen_cr_index.py`. PRs restore markers
  from `main` (client-only PR skips the server job and vice versa).
```

Leave the **`verify-and-release.yml`** and **`docker.yml`** paragraphs as they are (release concurrency stays non-cancelling; docker cache already documented).

- [ ] **Step 2: Update roadmap design status**

At the top of `docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md`, set:

```markdown
**Status:** Wave 1 implemented (absorbed into [production-topology-and-operations](2026-07-20-production-topology-and-operations.md); Waves 2–3 remain roadmap)
```

- [ ] **Step 3: Final verification**

```bash
./scripts/check-ci-wave1.sh
./scripts/check-docker-workflow-cache.sh
just engine-cr-index-check
just client-mana-oracle-check
```

Expected: all exit 0.

On the PR after push: confirm a second push cancels the prior CI run; confirm terraform is skipped when the PR does not touch `iac/**` or `ci.yml` (this Wave 1 PR touches `ci.yml`, so terraform **will** run on this PR — that is correct fail-closed behavior).

- [ ] **Step 4: Commit**

```bash
git add \
  docs/superpowers/specs/2026-07-20-production-topology-and-operations.md \
  docs/superpowers/specs/2026-07-25-ci-improvement-roadmap-design.md
git commit -m "docs: record CI Wave 1 concurrency, path-skip, and cheap gates"
```

---

## Plan self-review

| Spec Wave 1 requirement | Task |
|---|---|
| PR concurrency cancel | Task 2 |
| Path-skip terraform (`iac/**` + `ci.yml`) | Task 2 |
| `engine-cr-index-check` in `server-check` + hashFiles | Task 3 |
| `client-mana-oracle-check` in `client-check` | Task 3 |
| Docker cache guard in CI | Task 2 |
| Static guard script + `ci-wave1-guard` job | Task 1 + Task 3 |
| Production-topology + design status | Task 4 |
| Waves 2–3 out of scope | Global Constraints |

No TBD/TODO placeholders. Pass-marker key names (`verify-server-v2-` / `verify-client-v2-`) match across Task 1 guard and Task 3 implementation.
