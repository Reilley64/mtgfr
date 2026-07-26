# Task 3 report — Sync: spawn ExitFx, suppress glide, hide faces

## Status

Completed.

## What changed

- Added `BoardModel.exitFx` and `BoardModel.lastBattlefieldPoses` in `client/app/board/submodel.ts`.
- Wired battlefield-exit sync to:
  - spawn `ExitFx` from a live flight pose or cached battlefield pose,
  - suppress generic zone-move and from-stack glides for battlefield exits,
  - include active exit-FX ids in `hideCardIds`,
  - refresh cached battlefield poses from the current battlefield layout.
- Extended `FlightsSynced` in `client/app/board/messages.ts` to carry `exitFx`.
- Updated `applyFlightsSynced` plus the mount sync payload in `client/app/board/bitmap/mount.ts` to accept `exitFx` now and send `[]` until Task 4 owns the clock.
- Added focused regression coverage in `client/app/board/exit-fx-sync.test.ts`.
- Extended `client/app/board/story.test.ts` to cover `FlightsSynced` exit-FX membership.
- Updated `docs/superpowers/specs/2026-07-20-flights.md` for battlefield-exit sync behavior.

## Verification

- Red phase: `cd client && bun test app/board/exit-fx-sync.test.ts`
  - Failed as expected because `BoardModel` had no `exitFx`.
- Green verification: `cd client && bun test app/board/exit-fx-sync.test.ts app/board/story.test.ts`
  - 11 passing tests, 0 failures.
- Typecheck: `just client-typecheck`
  - Passed.

## Self-review

- Kept changes scoped to sync/message plumbing only; no bitmap paint/tick work added.
- Preserved existing flight ownership semantics; `ownedIds` still tracks flights, while `hideCardIds` now unions flights and exit FX.
- Chose cached screen-space battlefield poses so exit FX can spawn even after the permanent has already left the battlefield layout.

## Concern

- `client/app/board/bitmap/mount.test.ts` currently fails in this environment because `vi.stubGlobal` / `vi.unstubAllGlobals` are unavailable in the active Vitest runtime. I did not change that test file; the task’s focused sync suites and client typecheck still pass.
