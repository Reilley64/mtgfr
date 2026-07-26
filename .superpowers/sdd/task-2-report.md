# Task 2 Report: BFF Scryfall sets + per-set oracle counts

## Status: DONE

## Summary

Implemented the brief’s BFF-side Scryfall cache changes:

- `client/lib/scryfall-oracle-total.ts` now parses the oracle bulk JSONL once per refresh and caches both the global total and `Record<setCode, count>` derived from each row’s `set`.
- Added `getCachedOracleTotalBySet()` while keeping `getCachedOracleTotal()` and the existing TTL/SWR/inflight behavior intact.
- Added `client/lib/scryfall-sets.ts` with a 24h in-memory cache for `GET https://api.scryfall.com/sets`, filtering out rows with `card_count <= 0`, mapping to `{ code, name, releasedAt, cardCount }`, and reusing the same User-Agent.

## TDD Evidence

### RED

Ran the required targeted Vitest command before implementation:

```bash
$ cd client && bun run test -- lib/scryfall-oracle-total.test.ts lib/scryfall-sets.test.ts
FAIL lib/scryfall-sets.test.ts
  Error: Cannot find module './scryfall-sets'
FAIL lib/scryfall-oracle-total.test.ts
  TypeError: getCachedOracleTotalBySet is not a function
```

Expected failures: missing module and missing export/new behavior.

### GREEN

After implementation:

```bash
$ cd client && bun run test -- lib/scryfall-oracle-total.test.ts lib/scryfall-sets.test.ts
Test Files  2 passed (2)
Tests       9 passed (9)
```

Additional static verification:

```bash
$ cd client && bun run typecheck
$ tsc --noEmit   # exit 0
```

## Files

| File | Action |
|---|---|
| `client/lib/scryfall-oracle-total.ts` | Modified |
| `client/lib/scryfall-oracle-total.test.ts` | Modified |
| `client/lib/scryfall-sets.ts` | Created |
| `client/lib/scryfall-sets.test.ts` | Created |

## Behavior covered by tests

1. Oracle bulk refresh caches global total and per-set totals from the same download.
2. Blank JSONL lines are skipped and non-string `set` values do not affect the by-set map.
3. Oracle cache still serves stale data after TTL expiry when refresh fails.
4. Oracle `ensure*` refresh remains fire-and-forget and populates the cache.
5. Sets cache returns `null` before the first successful refresh.
6. Sets refresh filters out `card_count === 0` rows and sends the required User-Agent.
7. Sets cache serves stale rows after TTL expiry when refresh fails.
8. Sets `ensure*` refresh remains fire-and-forget and populates the cache.

## Commit

- Pending local commit at report-write time; updated after commit below.

## Concerns

None blocking. This task intentionally stops at reusable cache primitives; BFF route wiring remains later work.
