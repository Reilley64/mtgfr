# Unified screen motion (design input)

**Status:** Design input for implementation (2026-07-27)
**Living surfaces to update at ship:** [`2026-07-20-flights.md`](2026-07-20-flights.md), [`2026-07-20-hand-and-zone-bar.md`](2026-07-20-hand-and-zone-bar.md), [`docs/client-canvas-map.md`](../../client-canvas-map.md)

---

## Problem

Three parallel “card on screen” paths exist today:

1. **Hand drag ghost** — HTML overlay in `hand.ts` (`hand-drag-ghost`), fixed CSS size, zone aura classes, mana pips.
2. **CardFlight** — Mount flight canvas via `motion/flights.ts` + `paint-flights.ts`.
3. **ExitFx** — Sibling orbit on the same Mount canvas via `motion/exit-fx.ts` + `paint-exit-fx.ts`.

They already share lift-shadow tokens and the flight layer for (2)+(3), but drag lives on HTML. That split causes continuous-handoff fragility (drag → seed flight), duplicate paint recipes, and Scene tests that assert an HTML ghost that is not the in-flight visual language.

## Goal

One **screen-motion paint ownership** on the Mount flight layer: dragging, flying, and exiting are phases of the same visual system. Pointer capture and Foldkit drag messages stay on the hand bar; only the **ghost paint** moves to the flight canvas so release→`seedDropFromHand` continues the same face at the same screen pose.

## Non-goals

- Collapsing `BoardModel.flights` / `exitFx` / `handDrag` into a single Map in this change (metadata stays split; paint unifies first).
- WebGL / Pixi / workers / dirty-rects.
- Changing play-threshold, cost/target pipeline, or ExitFx choreography semantics.
- Keeping mana-cost pips on the drag ghost (flights do not show them; continuity favors flight paint).

## Approaches considered

1. **Facade + canvas drag paint (chosen).** Keep pure step modules; add `DragGhost` + `paintScreenMotion`; wire `dragGhost` on `BitmapFrame`; remove HTML ghost.
2. **Single `ScreenMotion` Map in the board model.** Cleaner long-term; large `submodel.ts` churn and test rewrite — defer until paint ownership is proven.
3. **HTML ghost stays; only share helpers.** Does not deliver the unified layer the product asked for.

## Design

### Types

`motion/screen-motion.ts` owns the drag pose type and conversion:

- `DragGhost`: `{ print, name, x, y, scale, zone }` — screen center pose; `scale` is `handFlightScale(zoom)` so release seeds match.
- `dragGhostFromHandDrag(handDrag, zoom): DragGhost`.

Flights and ExitFx remain their existing types; the facade documents them as flying / exiting phases.

### Paint

`bitmap/paint-screen-motion.ts` (or `paintFlightLayer` rewritten to call helpers) paints, in order:

1. Active `DragGhost` (if any) via flight-card paint + zone playable stroke (mint ring; command/gy/exile outer outline from `chrome.ts` colors).
2. All `CardFlight`s.
3. All `ExitFx`.

Shared lift shadow stays in `lift-shadow.ts` / `paint-flights.ts`.

### Frame / Mount

`BitmapFrame.dragGhost?: DragGhost | null`. Pose-only drag moves republish the frame; `applyPublishedFrame` treats drag-ghost identity/pose changes as flight-layer paint (not resting bitmap). rAF still runs only while flights or ExitFx need stepping; drag alone does not need the clock.

### Hand bar

`hand.ts` keeps source fade / lose-aura while `handDrag != null`, and stops rendering `handDragGhost`. `MountHandBarDrag` unchanged.

### Visibility

Unchanged triad: `hideCardIds` / `handHidden` / `ownedIds`. Drag does not add object-id hide (source fades via `handDrag.action.object` opacity path already).

## Testing

- Pure: `dragGhostFromHandDrag` scale/pose; paint tests for drag lift shadow + zone strokes.
- Unit: hand view has **no** `hand-drag-ghost`; source still `opacity-25` and loses playable ring.
- Scene: active drag still shows faded source; assert no HTML ghost; flight layer present.
- Existing flight / ExitFx / handoff suites stay green.

## Success criteria

- Drag → play seed uses one continuous canvas face (no HTML ghost then canvas flight).
- Destroy/exile ExitFx and flights still share that layer.
- Living flights + hand-bar specs describe canvas drag ownership.
