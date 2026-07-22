# Task 10 Report: Arena playable borders + zone outlines

**Date:** 2026-07-22  
**Status:** DONE_WITH_CONCERNS

## Summary

- Added shared board chrome colors and action-derived battlefield playable ids.
- Removed default controller/seat strokes from resting cards; tap-only mana lands stay selectable without a playable border.
- Kept commander gold and layered playable outline support for commander permanents.
- Added playable hand borders plus graveyard purple / exile green bar outlines.
- Updated `DESIGN.md`, CSS tokens, and the board legend for the new outline language.

## TDD evidence

- RED: `cd client && bunx vitest run app/board/html/hand.test.ts app/board/bitmap/paint-cards.test.ts app/board/bitmap/mount.test.ts app/board/canvas/scene.test.ts` failed for missing hand, bitmap, and scene outline behavior.
- GREEN: same focused command passed: **4 files, 27 tests**.

## Verification

- `just client-check` passed through gen, format, lint, typecheck, and all client tests before cleanup: **66 files, 580 tests**.
- After reverting unrelated formatter churn: `cd client && bun run lint && bun run typecheck && bun run test` passed: **66 files, 580 tests**.

## Concerns

- `just client-format` rewrites unrelated pre-existing files in this checkout; I reverted that formatter-only churn and verified without the write-format step.
- No browser/live visual pass was run in this subagent; coverage is automated hand/bitmap/canvas/client tests.
