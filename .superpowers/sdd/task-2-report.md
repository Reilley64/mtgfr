# Task 2 Report: Split server verify — gate job + Postgres only on miss

## Status

COMPLETE

## Branch

`cursor/ci-improvement-wave-2-5be3`

## Files modified

| File | Action |
|------|--------|
| `.github/workflows/verify-jobs.yml` | Split server verify into gate + full job; moved Postgres service to miss-only job |

## Step 1: Workflow change

Replaced the single `verify-server` job with:

- `verify-server-gate`
  - `actions/checkout@v5`
  - `actions/cache@v5`
  - same `verify-server-v2-...` key
  - `lookup-only: true`
  - exposes `cache-hit` output
  - contains no `services:` block and no `postgres`
- `verify-server`
  - `needs: verify-server-gate`
  - `if: needs.verify-server-gate.outputs.cache-hit != 'true'`
  - owns the `postgres:16` service
  - runs the existing server toolchain + `just server-check`
  - writes the pass marker and saves it with `actions/cache/save@v5`

Left `verify-client` unchanged for this task.

## Step 2: Sanity-check

```bash
rg -n "verify-server-gate:|lookup-only|needs: verify-server-gate|postgres:16" .github/workflows/verify-jobs.yml
```

Result:

```text
13:  verify-server-gate:
26:          lookup-only: true
34:    needs: verify-server-gate
39:        image: postgres:16
```

Body-level checks:

```bash
gate_body=$(awk '/^  verify-server-gate:/{grab=1; next} grab && /^  verify-server:/{exit} grab {print}' .github/workflows/verify-jobs.yml)
server_body=$(awk '/^  verify-server:/{grab=1; next} grab && /^  verify-client:/{exit} grab {print}' .github/workflows/verify-jobs.yml)
```

Results:

```text
gate_clean=yes
server_postgres=yes
```

This confirms the tightened guard requirements:

- gate job id exists
- gate cache step is `lookup-only: true`
- gate body has no `services:` and no `postgres`
- full server job needs gate
- full server job keeps `postgres:16`

## Step 3: Shared Wave 2 guard status

```bash
./scripts/check-ci-wave2.sh
echo "wave2_exit=$?"
```

Result:

```text
          key: verify-client-v2-${{ hashFiles('client/**', 'proto/**', 'crates/**', 'Cargo.toml', 'Cargo.lock', '.bun-version', 'justfile', '.github/workflows/verify-jobs.yml') }}
client hashFiles still includes crates/**
wave2_exit=1
```

This is expected for Task 2 because `verify-client` is intentionally unchanged and later tasks own the client-side Wave 2 fixes.

## Step 4: Commit

Commit message:

```text
ci: skip Postgres service on server pass-marker hit
```
