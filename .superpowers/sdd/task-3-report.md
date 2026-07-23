# Task 3 Report: Mount-local rAF + resting paint gate

## Status

DONE

## Summary

Refactored `client/app/board/bitmap/mount.ts` so publish-time frame decisions are pure (`applyPublishedFrame`), flight stepping is local (`tickFlightClock`), resting paints are gated by `restingPaintChanged`, and mount streams emit `ArtLoaded` + `FlightsSynced` instead of per-rAF `TickedFrame`.

## TDD

1. **RED:** Added `pose-only flight tick does not request resting paint` to `client/app/board/bitmap/mount.test.ts`; `bunx vitest run app/board/bitmap/mount.test.ts -t "pose-only flight tick"` failed with `TypeError: applyPublishedFrame is not a function`.
2. **GREEN:** Implemented the exported clock helpers, preserved live poses on publish, and rewired the flight-layer rAF loop to step and paint locally.
3. **VERIFY:** `bunx vitest run app/board/bitmap/flight-frame.test.ts app/board/bitmap/mount.test.ts app/board/story.test.ts` passed; `just client-lint` passed; `just client-typecheck` passed after fixing an unrelated fixture type in `client/app/board/html/hand.test.ts`.

## Commits

- `perf(client): step and paint flights without full-board frames`

## Concerns

- `client/app/board/chrome.ts` picked up a small optional-chain lint cleanup while unblocking verification.
