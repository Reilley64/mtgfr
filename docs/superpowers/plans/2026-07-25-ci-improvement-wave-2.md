# CI Improvement Wave 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Wave 2 of the CI roadmap — skip Postgres on server pass-marker hits, tighten client skip hashes (buf-only codegen), Vitest JUnit/summary parity, and clear Node 20 Action deprecation warnings.

**Architecture:** Split server verify into a lightweight cache-gate job (no services) plus a full verify job that only runs on miss (with Postgres). Narrow client `hashFiles` to proto/client inputs and drop unused Rust setup from the client job. Emit Vitest JUnit under CI and wire `test-summary` like nextest. Bump first-party Actions to Node-24 majors. Guard with `scripts/check-ci-wave2.sh`.

**Tech Stack:** GitHub Actions (`actions/cache` lookup-only + save), Vitest reporters, bash guards, existing `just` recipes

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-20-ci-and-release.md` (Wave 2 only)
- Surface spec to amend: `docs/superpowers/specs/2026-07-20-ci-and-release.md`
- Do not redesign semantic-release / Docker cascade (Wave 3)
- Keep Wave 1 guards (`check-ci-wave1.sh`, concurrency, terraform path-skip) working
- Do not add Playwright / live-game verify to Actions
- Client codegen is `buf generate` via Bun — no Cargo/Rust required for `just client-check`
- Angular commit subjects (`ci:`, `docs:`, `test:`, `fix:`)
- Do not index the roadmap design in `docs/superpowers/specs/README.md`

## File map

| File | Role |
|---|---|
| `.github/workflows/verify-jobs.yml` | Split server gate/full jobs; tighten client hash + drop Rust; Vitest JUnit steps; Action pin bumps |
| `.github/workflows/ci.yml` | Action pin bumps; add `ci-wave2-guard` job |
| `.github/workflows/verify-and-release.yml` | Action pin bumps only |
| `.github/workflows/docker.yml` | Action pin bumps only |
| `client/vitest.config.ts` | JUnit reporter when `CI` is set |
| `scripts/check-ci-wave2.sh` | Assert Wave 2 wiring |
| `docs/superpowers/specs/2026-07-20-ci-and-release.md` | Document shipped Wave 2 behavior |
| `docs/superpowers/specs/2026-07-20-ci-and-release.md` | Status → Wave 2 implemented |

---

### Task 1: Failing Wave 2 guard script

**Files:**
- Create: `scripts/check-ci-wave2.sh`
- Test: `scripts/check-ci-wave2.sh`

**Interfaces:**
- Consumes: `.github/workflows/verify-jobs.yml`, `.github/workflows/ci.yml`, `client/vitest.config.ts`
- Produces: exit 0 only when Wave 2 patterns are present; also runs `./scripts/check-ci-wave1.sh`

- [ ] **Step 1: Write the failing check script**

Create `scripts/check-ci-wave2.sh`:

```bash
#!/usr/bin/env bash
# Assert CI Wave 2 wiring (server cache gate, client hash tighten, vitest junit, Node-24 actions).
set -euo pipefail

verify=".github/workflows/verify-jobs.yml"
ci=".github/workflows/ci.yml"
vitest="client/vitest.config.ts"

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

need_file "$verify"
need_file "$ci"
need_file "$vitest"

# Server split: gate job without postgres service; full job has postgres + if miss
need_grep "$verify" 'verify-server-gate:' "server gate job"
need_grep "$verify" 'lookup-only:[[:space:]]*true' "cache lookup-only"
need_grep "$verify" 'needs:[[:space:]]*verify-server-gate' "full server needs gate"
need_grep "$verify" "needs\.verify-server-gate\.outputs\.cache-hit[[:space:]]*!=[[:space:]]*'true'" "full server if miss"
need_grep "$verify" 'image:[[:space:]]*postgres:16' "postgres only on full job"

# Gate job must not declare services:
if awk '/^  verify-server-gate:/,/^  [a-z]/{print}' "$verify" | grep -q '^    services:'; then
  echo "verify-server-gate must not declare services:" >&2
  exit 1
fi

# Client hash no longer includes crates/** or Cargo.*
if grep -E "verify-client-v[0-9]+-.*hashFiles\([^)]*crates/\*\*" "$verify"; then
  echo "client hashFiles still includes crates/**" >&2
  exit 1
fi
need_grep "$verify" 'verify-client-v3-' "client pass marker v3"
need_grep "$verify" "hashFiles\('client/\*\*', 'proto/\*\*'" "client hashFiles proto+client"

# Client job must not install Rust toolchain
if awk '/^  verify-client:/,/^  [a-z]/{print}' "$verify" | grep -q 'dtolnay/rust-toolchain'; then
  echo "verify-client must not use dtolnay/rust-toolchain" >&2
  exit 1
fi

# Vitest JUnit
need_grep "$vitest" 'junit' "vitest junit reporter"
need_grep "$verify" 'client-junit|junit\.xml' "client junit path in workflow"
need_grep "$verify" 'test-summary/action@v2' "test-summary for client"

# Node-24 action majors (checkout@v5+, cache@v5+, setup-node@v5+, upload-artifact@v5+)
need_grep "$ci" 'actions/checkout@v[5-9]' "ci checkout major >=5"
need_grep "$verify" 'actions/checkout@v[5-9]' "verify checkout major >=5"
need_grep "$verify" 'actions/cache@v[5-9]' "verify cache major >=5"
need_grep "$ci" 'actions/setup-node@v[5-9]' "ci setup-node major >=5"
need_grep "$verify" 'actions/upload-artifact@v[5-9]' "upload-artifact major >=5"

# Wave 2 guard job
need_grep "$ci" 'check-ci-wave2\.sh' "wave2 guard step"

./scripts/check-ci-wave1.sh

echo "ok: CI Wave 2 wiring present"
```

```bash
chmod +x scripts/check-ci-wave2.sh
```

- [ ] **Step 2: Run check — expect FAIL**

```bash
./scripts/check-ci-wave2.sh
```

Expected: exit 1 (server gate / client hash / etc. missing).

- [ ] **Step 3: Commit**

```bash
git add scripts/check-ci-wave2.sh
git commit -m "test: add CI Wave 2 wiring guard script"
```

---

### Task 2: Split server verify — gate job + Postgres only on miss

**Files:**
- Modify: `.github/workflows/verify-jobs.yml` (server jobs only in this task; leave client for Task 3)
- Test: local YAML structure grep; `./scripts/check-ci-wave2.sh` still fails on client items

**Interfaces:**
- Consumes: existing server pass-marker key `verify-server-v2-…` (keep key; only split jobs)
- Produces:
  - `verify-server-gate` — checkout + `actions/cache` `lookup-only: true`, output `cache-hit`
  - `verify-server` — `needs: verify-server-gate`, `if: needs.verify-server-gate.outputs.cache-hit != 'true'`, `services.postgres`, full `just server-check`, write marker + cache save

- [ ] **Step 1: Replace the `verify-server` job block in `verify-jobs.yml` with the following two jobs**

Keep `on:` / `env:` unchanged. Replace from `jobs:` through the end of the old `verify-server` job (before `verify-client`) with:

```yaml
jobs:
  verify-server-gate:
    name: Verify (server) cache
    runs-on: ubuntu-latest
    outputs:
      cache-hit: ${{ steps.pass.outputs.cache-hit }}
    steps:
      - uses: actions/checkout@v5

      - uses: actions/cache@v5
        id: pass
        with:
          path: .ci-pass
          key: verify-server-v2-${{ hashFiles('crates/**', 'proto/**', 'Cargo.toml', 'Cargo.lock', 'Toasty.toml', 'toasty/**', '.config/nextest.toml', 'justfile', '.github/workflows/verify-jobs.yml', 'docs/CR_INDEX.md', 'scripts/gen_cr_index.py') }}
          lookup-only: true

      - name: Skip — server hash already verified
        if: steps.pass.outputs.cache-hit == 'true'
        run: echo "Pass marker hit for server content hash; full verify job will be skipped."

  verify-server:
    name: Verify (server)
    needs: verify-server-gate
    if: needs.verify-server-gate.outputs.cache-hit != 'true'
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: mtgfr
          POSTGRES_PASSWORD: mtgfr
          POSTGRES_DB: mtgfr
        ports:
          - 5432:5432
        options: >-
          --health-cmd "pg_isready -U mtgfr -d mtgfr"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 10

    steps:
      - uses: actions/checkout@v5

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - uses: extractions/setup-just@v3

      - uses: taiki-e/install-action@v2
        with:
          tool: nextest

      - name: Install protoc
        run: sudo apt-get update && sudo apt-get install -y --no-install-recommends protobuf-compiler

      - name: just server-check (CR index, then fmt + clippy + migrate + nextest)
        run: just server-check

      - name: Upload Rust JUnit report
        if: ${{ !cancelled() && hashFiles('target/nextest/ci/junit.xml') != '' }}
        uses: actions/upload-artifact@v5
        with:
          name: rust-junit
          path: target/nextest/ci/junit.xml

      - name: Rust test summary
        if: ${{ !cancelled() && hashFiles('target/nextest/ci/junit.xml') != '' }}
        uses: test-summary/action@v2
        with:
          paths: target/nextest/ci/junit.xml

      - name: Write pass marker
        run: mkdir -p .ci-pass && echo ok > .ci-pass/marker

      - uses: actions/cache/save@v5
        with:
          path: .ci-pass
          key: verify-server-v2-${{ hashFiles('crates/**', 'proto/**', 'Cargo.toml', 'Cargo.lock', 'Toasty.toml', 'toasty/**', '.config/nextest.toml', 'justfile', '.github/workflows/verify-jobs.yml', 'docs/CR_INDEX.md', 'scripts/gen_cr_index.py') }}
```

Leave `verify-client` as-is for this commit (Task 3 rewrites it). Temporarily use checkout@v5 / cache@v5 / upload-artifact@v5 only on the new server jobs; Task 4 finishes remaining pins.

- [ ] **Step 2: Sanity-check**

```bash
grep -n 'verify-server-gate\|lookup-only\|needs: verify-server-gate\|postgres:16' .github/workflows/verify-jobs.yml
awk '/^  verify-server-gate:/,/^  verify-server:/{print}' .github/workflows/verify-jobs.yml | grep -c services || true
```

Expected: gate + lookup-only + needs + postgres present; gate section has 0 `services:` lines.

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/verify-jobs.yml
git commit -m "ci: skip Postgres service on server pass-marker hit"
```

---

### Task 3: Tighten client skip hash + drop Rust + Vitest JUnit

**Files:**
- Modify: `.github/workflows/verify-jobs.yml` (`verify-client` job)
- Modify: `client/vitest.config.ts`
- Test: `cd client && CI=true bun run test` produces junit; guard still fails until Task 5 adds wave2 job (or until pins complete — Task 4)

**Interfaces:**
- Consumes: buf-only `bun run gen` (no Cargo)
- Produces: `verify-client-v3` hash over `client/**`, `proto/**`, `.bun-version`, `justfile`, `verify-jobs.yml` only; Vitest JUnit at `client/junit.xml`; upload + test-summary steps

**Rationale (document in production-topology in Task 5):** Client wire codegen is `@bufbuild` / Effect-gRPC via `buf generate`. Engine crate edits that do not change `proto/**` cannot break `client-check`, so hashing `crates/**` / `Cargo.*` was over-invalidation.

- [ ] **Step 1: Update `client/vitest.config.ts`**

```typescript
/// <reference types="vitest/config" />

import { configDefaults, defineConfig } from "vitest/config";

const ci = process.env.CI === "true" || process.env.CI === "1";

export default defineConfig({
  resolve: {
    // Match vite.config.ts — path aliases come from tsconfig.json (`~/*`).
    // Requires root vite@8 (see package.json overrides); vitest may otherwise nest vite@6.
    tsconfigPaths: true,
  },
  test: {
    environment: "node",
    exclude: [...configDefaults.exclude, ".output/**"],
    ...(ci
      ? {
          reporters: ["default", "junit"],
          outputFile: {
            junit: "./junit.xml",
          },
        }
      : {}),
  },
});
```

- [ ] **Step 2: Replace the entire `verify-client` job with**

```yaml
  verify-client:
    name: Verify (client)
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v5

      - uses: actions/cache@v5
        id: pass
        with:
          path: .ci-pass
          key: verify-client-v3-${{ hashFiles('client/**', 'proto/**', '.bun-version', 'justfile', '.github/workflows/verify-jobs.yml') }}

      - name: Skip — client hash already verified
        if: steps.pass.outputs.cache-hit == 'true'
        run: echo "Pass marker hit for client content hash; skipping setup and checks."

      # Pin — `latest` resolves via GitHub API and fails when API is degraded.
      - uses: oven-sh/setup-bun@v2
        if: steps.pass.outputs.cache-hit != 'true'
        with:
          bun-version-file: .bun-version

      - uses: extractions/setup-just@v3
        if: steps.pass.outputs.cache-hit != 'true'

      - name: Install client dependencies
        if: steps.pass.outputs.cache-hit != 'true'
        run: cd client && bun install --frozen-lockfile

      - name: just client-check (tokens + mana-oracle + codegen + format + lint + typecheck + vitest)
        if: steps.pass.outputs.cache-hit != 'true'
        run: just client-check

      - name: Upload client JUnit report
        if: ${{ steps.pass.outputs.cache-hit != 'true' && !cancelled() && hashFiles('client/junit.xml') != '' }}
        uses: actions/upload-artifact@v5
        with:
          name: client-junit
          path: client/junit.xml

      - name: Client test summary
        if: ${{ steps.pass.outputs.cache-hit != 'true' && !cancelled() && hashFiles('client/junit.xml') != '' }}
        uses: test-summary/action@v2
        with:
          paths: client/junit.xml

      - name: Write pass marker
        if: steps.pass.outputs.cache-hit != 'true'
        run: mkdir -p .ci-pass && echo ok > .ci-pass/marker
```

- [ ] **Step 3: Verify Vitest JUnit locally**

```bash
cd client && CI=true bun run test
test -f client/junit.xml || test -f junit.xml
# from repo root after cd client, file is client/junit.xml if run from client:
ls -la junit.xml
cd ..
# ensure junit.xml is gitignored if created under client/
grep -n 'junit' client/.gitignore || echo 'add junit.xml to client/.gitignore'
```

If `client/.gitignore` lacks `junit.xml`, append:

```
junit.xml
```

- [ ] **Step 4: Commit**

```bash
git add client/vitest.config.ts .github/workflows/verify-jobs.yml client/.gitignore
git commit -m "ci: tighten client pass-marker and publish Vitest JUnit"
```

---

### Task 4: Bump Actions to Node-24 majors + enable wave2 guard job

**Files:**
- Modify: `.github/workflows/ci.yml`, `verify-and-release.yml`, `docker.yml` (and any remaining v4 pins in `verify-jobs.yml`)
- Modify: `.github/workflows/ci.yml` — append `ci-wave2-guard` job
- Test: `./scripts/check-ci-wave2.sh` → PASS

**Interfaces:**
- Pins: `actions/checkout@v5`, `actions/cache@v5` (and `actions/cache/save@v5`), `actions/setup-node@v5`, `actions/upload-artifact@v5` everywhere in `.github/workflows/`
- Do not bump third-party actions whose majors are unknown-safe (`dorny/paths-filter@v3`, `extractions/setup-just@v3`, `oven-sh/setup-bun@v2`, `hashicorp/setup-terraform@v3`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `taiki-e/install-action@v2`, `test-summary/action@v2`, `docker/*`) unless already required

- [ ] **Step 1: Replace all `actions/checkout@v4` → `@v5`, `actions/setup-node@v4` → `@v5` across workflow YAMLs; ensure `verify-jobs` / `ci` match guard patterns**

- [ ] **Step 2: Append to `ci.yml` after `ci-wave1-guard`:**

```yaml
  ci-wave2-guard:
    name: CI Wave 2 guard
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Assert CI Wave 2 wiring
        run: ./scripts/check-ci-wave2.sh
```

- [ ] **Step 3: Run**

```bash
./scripts/check-ci-wave2.sh
./scripts/check-ci-wave1.sh
```

Expected: both exit 0.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/
git commit -m "ci: bump Actions to Node 24 majors and add Wave 2 guard"
```

---

### Task 5: Docs — production-topology + roadmap status

**Files:**
- Modify: `docs/superpowers/specs/2026-07-20-ci-and-release.md`
- Modify: `docs/superpowers/specs/2026-07-20-ci-and-release.md`
- Test: `./scripts/check-ci-wave2.sh`

- [ ] **Step 1: Replace the `ci.yml` / `verify-jobs.yml` bullets under Release and CI with:**

```markdown
**`ci.yml`** (PRs): `concurrency` group `ci-${{ github.ref }}` with
`cancel-in-progress: true` so superseded pushes cancel. Jobs: `changes`
(`dorny/paths-filter` for `iac/**` + `.github/workflows/ci.yml`), commitlint,
`verify-jobs.yml`, terraform (only when `changes.outputs.iac == 'true'`),
always-on `docker-cache-guard`, `ci-wave1-guard`, and `ci-wave2-guard`.

**`verify-jobs.yml`** (reusable):
- `verify-server-gate`: checkout + pass-marker `lookup-only` cache
  (`verify-server-v2-*`, includes `docs/CR_INDEX.md` + `scripts/gen_cr_index.py`).
  No Postgres service.
- `verify-server`: runs only on cache miss; Postgres 16 + `just server-check`
  (`engine-cr-index-check` before fmt, then clippy + migrate + nextest); writes
  pass marker via `actions/cache/save`. Uploads nextest JUnit + test summary.
- `verify-client`: Bun-only `just client-check` (tokens + mana-oracle + buf
  codegen + format + lint + typecheck + vitest). Pass marker
  `verify-client-v3-*` hashes `client/**`, `proto/**`, `.bun-version`,
  `justfile`, and this workflow — not `crates/**` (wire codegen does not
  compile Rust). On miss: Vitest JUnit (`client/junit.xml`) upload + test
  summary. No Rust toolchain on this job.
```

- [ ] **Step 2: Set roadmap design status to:**

```markdown
**Status:** Wave 2 implemented (absorbed into [ci-and-release](2026-07-20-ci-and-release.md); Wave 3 remains evidence-gated roadmap)
```

- [ ] **Step 3: Final verification**

```bash
./scripts/check-ci-wave1.sh
./scripts/check-ci-wave2.sh
./scripts/check-docker-workflow-cache.sh
```

- [ ] **Step 4: Commit**

```bash
git add \
  docs/superpowers/specs/2026-07-20-ci-and-release.md \
  docs/superpowers/specs/2026-07-20-ci-and-release.md
git commit -m "docs: record CI Wave 2 job split, client hash, and Vitest JUnit"
```

---

## Plan self-review

| Spec Wave 2 requirement | Task |
|---|---|
| No Postgres on pass-marker hit | Task 2 |
| Document + tighten client hashFiles | Task 3 + Task 5 |
| Vitest JUnit / summary parity | Task 3 |
| Node 20 Action deprecation cleanup | Task 4 |
| Optional fail-fast / step names | Deferred (YAGNI) — step names already clarified in Task 2/3 |
| Wave 1 still green | Task 1/4 invoke `check-ci-wave1.sh` |
| Wave 3 out of scope | Global Constraints |

**Note:** `actions/cache@v5` + `lookup-only` / `cache/save@v5` are the supported split. Do not use empty `services.*.image` expressions keyed off step outputs (services evaluate before steps).
