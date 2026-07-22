# Task 4 Report: Pending-choice prompts only for the awaited seat

## Status

**DONE**

## RED evidence

- `cd client && bunx vitest run app/board/scene.test.ts app/board/html/surfaces.test.ts`
- Failed as expected: `may_yes_no prompt mounts only for the awaited seat` saw `prompt-yes` for viewer `1`.

## GREEN evidence

- `cd client && bunx vitest run app/board/scene.test.ts app/board/html/surfaces.test.ts` — **41 passed**.
- `cd client && bunx biome format --write app/board/html/overlays.ts app/board/html/prompts.ts app/board/scene.test.ts` — **No fixes applied**.
- `cd client && bun run lint` — exit `0`; reports existing `noNonNullAssertion` warnings in `app/board/inspect-pile-concede.test.ts`.
- `cd client && bun run typecheck` — **passed**.
- `cd client && bun run test` — **64 files / 561 tests passed**.

## What changed

- `promptsView` now renders shared engine `pending_choice` formulators only when the viewer is a seated active player and `pending_choice.player === state.viewer`.
- Client-local prompts (`xPrompt`, modal cast, cost picks, staged target picks) still return before the pending-choice gate.
- `overlays.ts` renames the outer active-player gate to `seatedViewer` to avoid confusing it with the awaited pending-choice seat.

## Self-review

- Wire state still carries `pending_choice` for every viewer; only the interactive DOM is gated.
- The regression test covers awaited player, non-awaited seated player, and spectator viewer.
