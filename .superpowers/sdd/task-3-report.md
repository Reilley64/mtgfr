## Task 3 report

Status: implemented BFF/DB lobby commander damage option.

Changes:
- Added `lobbies.commander_damage_enabled` via `0003_lobby_commander_damage.sql`, Drizzle journal, and schema column.
- Added `LobbySnapshot.commanderDamageEnabled`, `toLobbyView().commander_damage_enabled`, `loadLobby` projection, and `setCommanderDamageEnabled(db, tableId, userId, enabled)`.
- Added `POST /api/tables/options/v1` and changed table start seeding to use `snap.commanderDamageEnabled`.
- Added `LobbyView` Effect schema field, `setTableOptions`, and updated typed lobby fixtures.
- Fixed an existing `StreamFrame["state"]` union-index typecheck blocker and removed the now-unused `StreamFrame` test import.

TDD:
- Added the requested store projection and DB mutation tests before implementation.
- Red run: `cd client && bun test lib/lobby-store.test.ts` failed on missing `setCommanderDamageEnabled` export.

Verification:
- `just client-migrate`
- `cd client && bun test lib/lobby-store.test.ts` -> 7 pass, 0 fail.
- `just client-typecheck`
- `just client-test` -> 108 files passed, 1086 tests passed.
- `just client-lint` -> exit 0; reports 5 existing warning-level non-null assertions.
- `git diff --check`

Concerns:
- `just client-lint` still prints pre-existing warning-level non-null assertions in unrelated files.
# Task 3 report — Sync: spawn ExitFx, suppress glide, hide faces

## Status

Completed.

## What changed

- Added `BoardModel.exitFx` and `BoardModel.lastBattlefieldPoses` in `client/app/board/submodel.ts`.
- Wired battlefield-exit sync to:
  - spawn `ExitFx` from a live flight pose or cached battlefield pose,
  - resolve rebased battlefield exits through `zoneMoves` so the new id can reuse the prior id's flight pose or cached battlefield pose,
  - suppress generic zone-move and from-stack glides for battlefield exits,
  - include active exit-FX ids in `hideCardIds`,
  - refresh cached battlefield poses from the current battlefield layout.
- Extended `FlightsSynced` in `client/app/board/messages.ts` to carry `exitFx`.
- Updated `applyFlightsSynced` plus the mount sync payload in `client/app/board/bitmap/mount.ts` to preserve the published frame's current `exitFx` membership instead of forcing `[]` during flight-clock syncs.
- Added focused regression coverage in `client/app/board/exit-fx-sync.test.ts`.
- Extended `client/app/board/story.test.ts` to cover `FlightsSynced` exit-FX membership and `hideCardIds = flying ∪ exitFx ids`.
- Added a focused mount regression in `client/app/board/bitmap/mount.test.ts` for preserving active `exitFx` in `FlightsSynced` payloads.
- Updated `docs/superpowers/specs/2026-07-20-flights.md` for battlefield-exit sync behavior.

## Verification

- Red phase: `cd /workspace/client && bun test app/board/exit-fx-sync.test.ts app/board/story.test.ts app/board/bitmap/mount.test.ts`
  - Failed as expected on the new rebind-id exit regression and on the focused mount payload regression before the sync fixes landed.
- Green verification: `cd /workspace/client && bun test app/board/exit-fx-sync.test.ts app/board/story.test.ts`
  - 13 passing tests, 0 failures.
- Focused mount regression: `cd /workspace/client && bun test app/board/bitmap/mount.test.ts -t "preserves active exit FX in sync payloads while flights settle"`
  - 1 passing test, 0 failures.
- Typecheck: `just client-typecheck`
  - Passed.

## Self-review

- Kept changes scoped to sync/message plumbing only; no bitmap paint/tick work added.
- Preserved existing flight ownership semantics; `ownedIds` still tracks flights, while `hideCardIds` now unions flights and exit FX.
- Chose cached screen-space battlefield poses so exit FX can spawn even after the permanent has already left the battlefield layout, including provenance rebases from prior battlefield ids.
