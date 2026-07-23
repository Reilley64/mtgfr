# Task 2 Report: `FlightsSynced` message + submodel apply

**Status:** DONE  
**Branch:** `cursor/foldkit-migration-design-1ef0`

## Scope

Implemented the Task 2 board-model sync path for canvas flights without per-frame `TickedFrame` stepping:

- added `FlightsSynced({ now, flights })` to the board message schema
- forwarded the new tag through app-level message exports and `update.ts`
- added `applyFlightsSynced()` in `client/app/board/submodel.ts`
- demoted `TickedFrame` to cleanup-only behavior when no flights remain
- replaced the old “tick advances flights” coverage with `FlightsSynced` + no-step regression tests

## TDD Evidence

### RED — failing tests first

Command:

```bash
cd client && bunx vitest run app/board/story.test.ts
```

Output:

```text
❯ app/board/story.test.ts (6 tests | 3 failed)
  × FlightsSynced stores still-flying poses and hides the source card
  × FlightsSynced clears hidden cards when flights disappear
  × ticked frame does not step in-flight positions

FAIL  app/board/story.test.ts > FlightsSynced stores still-flying poses and hides the source card
TypeError: FlightsSynced is not a function

FAIL  app/board/story.test.ts > ticked frame does not step in-flight positions
AssertionError: expected 19.21132032700088 to be +0
```

Why this is the right RED:

- `FlightsSynced` did not exist yet
- `TickedFrame` still stepped live flights every frame, violating the brief

### GREEN — minimal implementation passes

Same command after implementation:

```text
Test Files  1 passed (1)
     Tests  6 passed (6)
```

## Implementation Summary

### `client/app/board/messages.ts`

- Added `FlightPhase`, `FlightKind`, and `CardFlight` schemas.
- Added `FlightsSynced = m("FlightsSynced", { now, flights })`.
- Registered `FlightsSynced` in the board `Message` union.

### `client/app/board/submodel.ts`

- Added `applyFlightsSynced(model, flightsIn, now)`.
- Keeps only `phase === "flying"` flights in `model.flights`.
- Rebuilds `hideCardIds` and `ownedIds` from the synced flying set.
- Updates `lastFlightFrame` from `now` when flights remain; clears it when none remain.
- Restores `handHidden` for source ids that disappeared from the synced flight set, and also clears it immediately for explicit settled entries.
- Changed `TickedFrame` handling to:
  - no-op while flights are still present
  - clear stale `hideCardIds`, `ownedIds`, and `lastFlightFrame` once flights are already gone

### `client/app/update.ts` and `client/app/messages.ts`

- Forwarded the new `FlightsSynced` tag through the top-level app message plumbing.

### `client/app/board/story.test.ts`

Added or updated regression coverage for:

1. `FlightsSynced` storing live poses and hiding the source hand card
2. `FlightsSynced` clearing hidden state when flights disappear
3. `TickedFrame` preserving in-flight positions
4. `TickedFrame` clearing stale hide and owned state after flights are gone

## Verification

### Focused regression suites

Command:

```bash
cd client && bunx vitest run app/board/story.test.ts app/board/hand-drag.test.ts app/board/bitmap/flight-frame.test.ts app/board/bitmap/mount.test.ts
```

Output:

```text
Test Files  4 passed (4)
     Tests  31 passed (31)
```

### Lint on touched files

Command:

```bash
cd client && bunx biome check app/board/messages.ts app/board/submodel.ts app/board/story.test.ts app/messages.ts app/update.ts
```

Output:

```text
Checked 5 files in 31ms. No fixes applied.
```

### Broader repo-preferred client check

Command:

```bash
just client-check
```

Result: **blocked by pre-existing unrelated issues**, not by this task:

- `client/app/board/chrome.ts` — existing biome optional-chain warning
- `client/app/board/html/hand.test.ts` — existing type mismatch for `WireKind`

The task files themselves passed focused lint and focused runtime verification.

## Files Changed

- `client/app/board/messages.ts`
- `client/app/board/submodel.ts`
- `client/app/board/story.test.ts`
- `client/app/messages.ts`
- `client/app/update.ts`

## Concerns

None for Task 2 itself. Task 3 still needs to teach the Mount to emit `FlightsSynced` instead of relying on per-frame `TickedFrame` stepping.
