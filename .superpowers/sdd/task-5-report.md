# Task 5 Report: Restore battlefield permanent chrome on bitmap layer

## Status

**DONE**

## RED evidence

- `cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/bitmap/paint-cards.test.ts`
- Failed as expected: `paintBitmapLayer > paints battlefield permanent chrome on the resting layer without under-card labels` did not include `text:2/2`, proving the resting bitmap layer was still art-only.

## GREEN evidence

- `cd client && bunx vitest run app/board/bitmap/mount.test.ts app/board/bitmap/paint-cards.test.ts` — **2 files / 12 tests passed**.
- `cd client && bun run typecheck` — **passed**.
- `cd client && bunx biome check --formatter-enabled=false app/board/bitmap/mount.ts app/board/bitmap/mount.test.ts` — **2 files checked, no fixes applied**.
- `just client-check` — **64 files / 562 tests passed**; Biome reported existing `noNonNullAssertion` warnings in `app/board/inspect-pile-concede.test.ts`.

## Changes

| File | Change |
|------|--------|
| `client/app/board/bitmap/mount.ts` | Resting permanents now call `paintCard`; auto-tap preview and target highlight still paint afterward. |
| `client/app/board/bitmap/mount.test.ts` | Adds RED/GREEN coverage for creature P/T, planeswalker loyalty, summoning-sick chip, counters, and no under-card name labels. |

## Self-review

- `paintCard` only paints the card face fallback name inside the card when art is missing; the regression keeps cached-art resting cards from adding under-card captions.
- No split was needed in `paint-cards.ts`; overlay order stayed in `mount.ts`.
