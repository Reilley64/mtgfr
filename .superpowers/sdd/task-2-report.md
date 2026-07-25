# Task 2 Report: selectedDeckId from route + not-found normalization + host redirect

## Status

DONE

## Summary

- Added `normalizeAppRoute(route, path)` in `client/app/routes.ts`.
- Normalized init and `UrlChanged` routes so non-integer Play/Table deck IDs become `NotFoundRoute({ path })`.
- Bound lobby `selectedDeckId` from `parseDeckIdParam(route.deckId)` for Play/Table route entry.
- Removed `deckFromCurrentPath` and stopped using `?deck=` for host redirect.
- Changed lobby host/join deck selection to use only `model.selectedDeckId`; no first-deck fallback.
- Changed shell Play nav to `routePath(HomeRoute())`.
- Left deck list tile href behavior unchanged for Task 5 ownership.

## TDD Evidence

Red run:

```text
cd client && bun test app/routes.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/update.test.ts
16 pass, 4 fail
Expected failures: NotFound normalization missing, route deck not selected, host redirect still had ?deck=, lobby host fell back to first deck.
```

Green/final run:

```text
cd client && bun test app/routes.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/update.test.ts app/shell/surfaces.test.ts
30 pass, 0 fail
```

Additional verification:

```text
cd client && bun run typecheck
tsc --noEmit passed

cd client && bun run lint
biome check passed; existing schema-version info only (2.5.3 config vs 2.5.5 CLI)

git diff --check
passed
```

## Self-review

- Scope matches the brief and avoids Task 4/5/6 work.
- `parseDeckIdParam` is the single source for route deck integer normalization.
- Redirect path uses `routePath(TableRoute({ deckId: String(selectedDeckId), table }))` with no query string.
- Route switch touched in `update.ts` now has explicit Login/NotFound cases plus a `never` default.
- No inline imports, no new dependencies, no generated artifacts committed.

## Concerns

- None. The lint command exits 0 but prints the existing Biome schema-version info.
