# Wave 1 Task 6 Report: Parent `UrlChanged` / cold-load via `inform*` only

## Summary

Implemented the route-entry refactor so the app parent now enters shell surfaces only through feature `informRouteChanged` helpers. `UrlChanged` and post-`ReceivedMe` cold-load now share that same child-owned path, and the home route clears transient deck-list UI through the deck-list child instead of calling `loadDeckList` directly.

## What changed

### Parent route entry

- Reduced `client/app/update.ts` route entry to parent-owned auth gating plus per-surface helper calls.
- Added focused parent helpers:
  - `enterDeckListRoute(...)`
  - `enterDeckBuilderRoute(...)`
  - `enterLeaderboardRoute(...)`
  - `enterLobbyRoute(...)`
- Each helper now:
  - calls the child `informRouteChanged(...)`
  - stores the returned child model
  - maps child commands back through the existing `Got*Message` wrapper
- `ReceivedMe` still funnels through `routeEntry(...)`, so cold load now uses the same feature-inform path as `UrlChanged`.

### Deck list route inform

- Added `client/app/shell/decks/list/inform.ts`.
- Added `ChangedDeckListRoute` to `client/app/shell/decks/list/messages.ts`.
- Updated `client/app/shell/decks/list/update.ts` so route entry is child-owned and now clears:
  - `accountMenuOpen`
  - `contextMenu`
  - `confirmingDeleteId`
  - `error`
  - then starts `FetchDecks`
- Kept `RequestedDecksRefresh` / `loadDeckList(...)` behavior intact for in-surface refresh flows.

### Deck builder route inform

- Added `client/app/shell/decks/builder/inform.ts`.
- Added `ChangedBuilderRoute` to `client/app/shell/decks/builder/messages.ts`.
- Updated `client/app/shell/decks/builder/update.ts` so route entry delegates to `enterBuilder(editingId)` from inside the child reducer.

### Leaderboard route inform

- Added `client/app/shell/leaderboard/inform.ts`.
- Added `ChangedLeaderboardRoute` to `client/app/shell/leaderboard/messages.ts`.
- Updated `client/app/shell/leaderboard/update.ts` so route entry delegates to `loadLeaderboard(...)` from inside the child reducer.

### Public child entrypoints

- Exported the new `inform.ts` helpers from:
  - `client/app/shell/decks/list/index.ts`
  - `client/app/shell/decks/builder/index.ts`
  - `client/app/shell/leaderboard/index.ts`

### Tests

- Extended `client/app/routes.test.ts` with regression coverage that proves:
  - `UrlChanged` to `/` clears transient deck-list UI before load
  - cold-load `ReceivedMe` on `/` clears the same transient deck-list UI through the same route-entry path
- Existing leaderboard route-entry coverage now exercises the new leaderboard `informRouteChanged(...)` path.

### Specs updated

- Updated `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md` to describe route entry and cold-load via per-surface `informRouteChanged`.
- Updated `docs/superpowers/specs/2026-07-20-deck-list-and-builder.md` to describe deck-surface route entry as child-owned.

## Verification

### Red phase

Started with the home-route regression tests and confirmed the pre-refactor failure:

- `bun test app/routes.test.ts`

Observed failure:

- `confirmingDeleteId` stayed `7` after both `UrlChanged` and cold-load `ReceivedMe`, proving the parent was bypassing child-owned route entry.

### Green phase

Re-ran the focused route suite after the refactor:

- `bun test app/routes.test.ts`

Result:

- 1 test file passed
- 18 tests passed

### Full client verification

Ran the requested full client verification:

- `just client-check`

Result:

- format passed
- lint passed
- typecheck passed (`tsc --noEmit`)
- 111 test files passed
- 1140 tests passed

## Notes

- `just client-check` initially failed on Biome export ordering and `RpcClient` typing in the new `inform.ts` shims; both were fixed and the rerun passed cleanly.
- No unrelated format-only diffs remained in the final tree.
- This task intentionally changes internal routing ownership, not the visible shell product behavior, except for correctly clearing stale deck-list transient UI on route entry.

## Review follow-up: builder + lobby route-entry regressions

Added the missing focused regressions that drive the parent route-entry path directly:

- `client/app/shell/decks/builder/story.test.ts`
  - `UrlChanged` to `/decks/abc` resets stale builder state through the parent update and emits `SearchDeckBuilderCards` plus `LoadDeckForBuilder`
- `client/app/shell/lobby/entry.test.ts`
  - cold-load `ReceivedMe` on `/play/9/XYZ789` resets stale lobby entry state through the parent update and sets `tableId` / `selectedDeckId` from the route

### Follow-up focused tests

- `cd client && bun test app/shell/decks/builder/story.test.ts app/shell/lobby/entry.test.ts`

Result:

- 39 tests passed
- 0 tests failed

### Follow-up full client verification

- `just client-check`

Result:

- format passed
- lint passed
- typecheck passed (`tsc --noEmit`)
- 111 test files passed
- 1142 tests passed
