# Task 4 Report: Paint + Mount clock for exit FX

## Status

Complete.

## Scope delivered

- Added `client/app/board/bitmap/paint-exit-fx.ts` with distinct destroy vs exile bitmap paint:
  - destroy: lifted card face, ash/burn veil, warm ember particles
  - exile: lifted card face, squash-to-center void, teal shard particles
- Added `client/app/board/bitmap/paint-exit-fx.test.ts`
- Extended `client/app/board/bitmap/mount.ts` to:
  - track `liveExitFx` alongside `liveFlights`
  - preserve in-progress exit FX across republishes
  - step exit FX in `tickFlightClock` via `stepExitFx`
  - keep rAF alive while exit FX remain active
  - publish stepped `exitFx` in `FlightsSynced`
  - drop completed FX from the frame/sync payload
  - paint exit FX on the flight layer after flights
  - preload exit-FX card art
- Extended `client/app/board/bitmap/mount.test.ts` for flight-layer paint, rAF gating, stepped sync, and completion cleanup
- Added `mergeExitFxPoses` in `client/app/board/bitmap/flight-frame.ts`
- Verified `client/app/board/view.ts` already publishes `exitFx`, so no view-path change was needed

## Verification

Passed:

- `cd /workspace/client && bun run test -- app/board/bitmap/paint-exit-fx.test.ts app/board/bitmap/mount.test.ts app/board/exit-fx-sync.test.ts app/board/story.test.ts`
- `cd /workspace/client && bun run typecheck`
- `cd /workspace/client && bunx biome check --formatter-enabled=false app/board/bitmap/paint-exit-fx.ts app/board/bitmap/paint-exit-fx.test.ts app/board/bitmap/mount.ts app/board/bitmap/mount.test.ts app/board/bitmap/flight-frame.ts`

## Self-review

No blocking issues found.

Non-blocking note:

- `tickFlightClock` now emits `FlightsSynced` while exit FX progress changes so the board model stays in lockstep with the short-lived 550ms animation. That is intentional, but it does mean a brief burst of sync messages during active exit FX.
