# Task 4 report: Seat-claim lobby UI

## Status

Implemented the Foldkit lobby options card and commander-damage switch above seats. The host can toggle it; watchers/non-hosts see a disabled switch. No board commander-damage paint or living specs were changed.

## Changes

- Added `RequestedLobbyCommanderDamage` to lobby messages.
- Added `SetLobbyOptions` and host-only update handling around `setTableOptions`.
- Routed the new lobby message through the app-level exhaustive update switch.
- Rendered `lobby-commander-damage` and `lobby-commander-damage-switch` with the requested copy and reused `turnYieldChrome` rocker classes.
- Broadened `NotHost` copy to `Only the host can change that.`
- Added Scene coverage for host chrome and disabled watcher switch.
- Added update coverage for host dispatch and non-host no-op.

## TDD / verification

- Red: `cd client && bun test app/shell/surfaces.test.ts app/shell/lobby/update.test.ts` failed for missing `lobby-commander-damage` chrome and missing `SetLobbyOptions`.
- Green: `cd client && bun test app/shell/surfaces.test.ts app/shell/lobby/update.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/story.test.ts` -> 40 pass / 0 fail.
- Targeted quality: `cd client && bunx biome check --formatter-enabled=false app/update.ts app/shell/lobby/messages.ts app/shell/lobby/update.ts app/shell/lobby/update.test.ts app/shell/lobby/view.ts app/shell/surfaces.test.ts && bunx tsc --noEmit && bun test app/shell/surfaces.test.ts app/shell/lobby/update.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/story.test.ts` -> pass.

## Self-review

- Placement matches the brief: after table-code / clipboard fallback and before seats.
- Copy/testids match the brief exactly.
- Host gating is enforced in both UI disabled state and update no-op.
- The switch is disabled while submitting or after start, matching the brief snippet.

## Concern

- `just client-check` currently fails before lint/type/test at `bun run gen:tokens:check` because generated design token outputs are stale. I restored unrelated formatter churn and did not include token changes in this task.
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

## Review follow-up: reduced-motion publish path

- Fixed the Important review finding in `client/app/board/bitmap/mount.ts` by collapsing publish-time `exitFx` through the same reduced-motion preference the Mount already uses for rAF ticks, so the animated layer never paints a one-frame ExitFx flash.
- Added a mount regression in `client/app/board/bitmap/mount.test.ts` that stubs reduced motion, publishes a frame with `exitFx`, asserts the animated frame/state are cleared immediately, confirms no rAF is needed, and checks that an immediate sync payload is surfaced.
- Test evidence:
  - `cd /workspace/client && bun test app/board/bitmap/paint-exit-fx.test.ts app/board/bitmap/mount.test.ts`
  - Result: `25 pass, 0 fail`
