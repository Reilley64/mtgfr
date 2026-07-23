# Canvas flight paint performance

**Status:** Draft (awaiting user review)  
**Date:** 2026-07-23  
**PR:** [#74](https://github.com/Reilley64/mtgfr/pull/74)  
**Related:** [`docs/client-canvas-map.md`](../../client-canvas-map.md), [`docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md`](2026-07-20-client-game-board-and-interaction.md)

## Problem

HTML chrome (hand bar, overlays) feels fine. The board **canvas** feels laggy/jittery **while cards are flying** — including on a sparse board with a single flight. Idle pointer move was not the reported pain; mid/late permanent count is not required to reproduce.

## Root cause (current Foldkit path)

While any flight is alive:

1. Flight Mount rAF enqueues `TickedFrame`
2. `updateBoard` runs `stepFlights` and returns a new board model
3. Foldkit **rebuilds the full vector** `Canvas.view` (`sceneShapes`, including every card)
4. `publishBitmapFrame` **full-clears and repaints both** bitmap layers (resting permanents + flights)

So every animation frame pays for a full board update + dual full-canvas blit, even when only flight screen poses changed. That matches “sparse board, one flight still stutters.”

## Decision

**Approach 1 (flight-local rAF), with a thin dirty gate (approach 2 insurance).**

Do **not** pursue WebGL / OffscreenCanvas / Pixi in this pass (board specs keep dual-surface canvas as the architecture).

### Animation loop

- The **flight Mount** owns rAF while `flights.length > 0`.
- Each frame: advance flight poses locally (same easing / τ / reduced-motion rules as today’s pure `stepFlights`) → paint the **flight layer only**.
- Mid-flight ticks **must not** drive a full `updateBoard` → view → `publishBitmapFrame` cycle.
- On **spawn, retarget, or settle**: one normal board model update so `hideCardIds`, ownership, and resting paint stay correct.

### Paint gating & model sync

- **Resting bitmap** repaints only when non-flight frame inputs change (layout, camera, selection, combat staging, targets, payment preview, hide sets after settle). Mid-flight pose ticks do not republish that layer.
- **Vector `Canvas.view`** is not rebuilt on flight ticks.
- **Spawns / retargets** still originate from game sync → board model. The Mount does not invent zone moves; it animates poses the model already authorized.
- **`hideCardIds`:** resting face stays hidden for the whole flight; cleared on settle via the settle model update (existing product semantics).
- **Reduced motion:** snap-to-end; Mount short-circuits the glide the same way `stepFlights` does today.

### Optional follow-up (not required for first win)

- Stop building redundant **vector card chrome** in `sceneShapes` (bitmap already covers resting faces). Useful, but secondary if flights no longer rebuild the view every frame.

## Out of scope

- Pointer-move publish throttling / idle cursor cost
- Wiring `withBoardDensity` / hover-raise / tap tweens
- WebGL, workers, dirty-rect systems beyond “don’t repaint resting on flight ticks”
- Changing flight visual design (shadow, timing curve) except as needed to keep parity with current motion

## Success

- A single cast/resolve flight on a sparse board feels smooth (no full-board hitch per frame).
- No double-drawn card (resting + flight); settle lands on the resting pose; multi-flight still works.
- HTML chrome behavior unchanged.
- Regression: resting layer paint count does not increase on mid-flight ticks; settle still clears hide and shows the resting card.

## Tests

- Keep pure unit coverage for flight step/settle math (`motion/flights`).
- Mount/frame tests: while only poses advance, resting-layer render is not invoked; flight layer is.
- Existing hide/settle Scene or board tests still pass (or extend if gaps).

## Hotspot files

| File | Role |
|------|------|
| `client/app/board/bitmap/mount.ts` | Frame bus, rAF, split layers — primary change site |
| `client/app/board/motion/flights.ts` | Pure step/spawn — reuse from Mount-local clock |
| `client/app/board/submodel.ts` | Stop mid-flight `TickedFrame` storm; settle/spawn updates only |
| `client/app/board/view.ts` | `publishBitmapFrame` / composition — gate resting publishes |
| `client/app/board/canvas/scene.ts` | Optional later: drop covered vector card shapes |
