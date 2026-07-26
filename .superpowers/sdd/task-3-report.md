# Task 3 Report: Extend BFF `meta/version/v1` and upstream parse

## Status: DONE

## Summary

Implemented the Task 3 wiring for pool-coverage meta:
- `client/lib/api-upstream-auth.ts` now exports `parseLiveStatus()` and `fetchApiMeta()`, statically imports Task 2’s oracle-total cache helpers, fire-and-forgets `ensureOracleTotalRefresh()`, and falls back to cached `oracleTotal` on upstream failures.
- `client/server/routes/api/[...path].ts` now serves `GET /api/meta/version/v1` as `{ version, faithful_count?, oracle_total? }`.
- `client/lib/lobby/client.ts` now exports `apiMeta()` and keeps `apiVersion()` as a thin wrapper.
- `client/app/fetch-api-version.ts` now reads `apiMeta()` so the existing shell fetch path stays aligned with the richer response.
- Updated the living specs for `/health/live` and shell boot meta fetching.

## TDD Evidence

### RED

Added `client/lib/api-upstream-auth.test.ts` and extended `client/lib/lobby/client.test.ts`.

```bash
$ cd /workspace/client && bun test lib/api-upstream-auth.test.ts lib/lobby/client.test.ts
SyntaxError: Export named 'fetchApiMeta' not found
SyntaxError: Export named 'apiMeta' not found
```

Expected failure: the new exports and wiring were not implemented yet.

### GREEN

```bash
$ cd /workspace/client && bun run test -- lib/api-upstream-auth.test.ts lib/lobby/client.test.ts lib/scryfall-oracle-total.test.ts
Test Files  3 passed (3)
Tests      14 passed (14)
```

## Verification

```bash
$ cd /workspace/client && bun x biome check lib/api-upstream-auth.ts lib/api-upstream-auth.test.ts lib/lobby/client.ts lib/lobby/client.test.ts app/fetch-api-version.ts server/routes/api/'[...path].ts' --formatter-enabled=false
# exit 0

$ cd /workspace/client && bun run typecheck
$ tsc --noEmit
# exit 0
```

## Files

- `client/lib/api-upstream-auth.ts`
- `client/lib/api-upstream-auth.test.ts`
- `client/server/routes/api/[...path].ts`
- `client/lib/lobby/client.ts`
- `client/lib/lobby/client.test.ts`
- `client/app/fetch-api-version.ts`
- `docs/superpowers/specs/2026-07-20-lobby-table-routing-and-live-game.md`
- `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md`

## Self-review

- Kept the BFF handler non-blocking for Scryfall refresh by calling `ensureOracleTotalRefresh()` without awaiting it.
- Preserved the existing `apiVersion()` contract for current callers while adding `apiMeta()` for later coverage-badge work.
- Chose nullable fallback fields instead of synthetic zeros so callers can distinguish “missing telemetry” from real counts.

## Concerns

- The brief’s `bun test ...` command uses Bun’s built-in runner, which lacks Vitest helpers used by this repo (`vi.stubGlobal`, `vi.mocked`). The correct project command is `bun run test -- ...`, which I used for green verification.
