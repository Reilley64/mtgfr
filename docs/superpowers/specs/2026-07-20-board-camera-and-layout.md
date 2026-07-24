# Board Camera and Layout

**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/geometry/camera.ts`, `client/app/board/geometry/interaction.ts`, `client/app/board/geometry/layout.ts`, `client/app/board/geometry/hit-test.ts`, `client/app/board/geometry/density.ts`, `client/app/board/submodel.ts`

---

## Problem Statement

The board needs one coordinate model that can support pan, zoom, layout, pointer hits, combat drops, targeting, Mount bitmap paint, and HTML overlays without divergent math. Crowded Commander battlefields also need packing and fan behavior that keeps seats readable without changing game state.

## Solution

`camera.ts` defines the pure screen/world transform. `layout.ts` converts the visible game snapshot into `RenderCard` records in world coordinates. `interaction.ts` handles camera fitting and pointer transitions. `hit-test.ts` maps pointer coordinates back to logical cards. `density.ts` defines the intended overlays for row packing, cluster fans, and hover raise.

The layer stack and the paint-vs-hit ownership rules are documented in [`docs/client-canvas-map.md`](../../client-canvas-map.md).

## User Stories

- As a player, I can pan and zoom the table while cards, arrows, and overlays stay aligned.
- As a player joining or resizing a game, the camera frames the table between HUD and hand areas.
- As a player on a crowded board, cards stay within their seat band and the object under the pointer is the one selected.
- As a player inspecting or targeting, the topmost card under the pointer wins.

## Behavior

### Camera

The camera is `{ panX, panY, zoom }` and follows:

```text
screen = world * zoom + pan
```

`worldToScreen` and `screenToWorld` are pure and do not read DOM state. `zoomAt(cam, sx, sy, factor)` preserves the world point under the screen coordinate while zooming. Zoom is bounded to the board limits. Panning changes `panX` and `panY` and sets `cameraUserMoved` so automatic fitting does not fight the player on later game syncs.

### fitCamera

`fitCamera(viewport, playerCount, reservedBottom)` frames the table for the active player count and available viewport. It accounts for HUD and hand space and caps zoom so the table starts readable rather than over-magnified. The board re-fits on cold load and relevant viewport/player-count changes until the user pans or zooms.

### RenderCard layout

`layout(state, viewer)` returns a flat `RenderCard[]` for visible objects. Each record carries world-space position and size, zone, owner/controller, seat, tapped rotation state, face status, print/card ids, combat/chrome fields, attachment info, and cluster membership.

Cards use 96 x 134 world units. Seat bands are arranged as a four-seat table from the viewer perspective: viewer at the bottom, opponents around the top and sides, with top seats oriented toward the viewer. Fewer than four seats leave unused bands empty. Zone columns for command, graveyard, exile, and library live at the left edge of each seat band, and battlefield mana anchoring is derived from the same geometry.

### Hit testing

Pointer events arrive in screen coordinates and are tested through the shared camera. Hits resolve against the logical `RenderCard` layout, not against tweened or in-flight paint poses. When multiple cards overlap, the topmost card in the resolved layout order wins. Avatar hits use the same camera transform and seat positions.

### Density, packing, and clusters

The intended density overlay is `withBoardDensity`, with top-order lifting handled by `withHoverRaise`:

- Row packing compresses horizontal spacing per battlefield row when a row exceeds its normal slot count.
- Packed rows stay inside the seat band; seats do not widen and cards do not spill into neighboring bands.
- Identical indistinguishable permanents may collapse into one cluster face with a member count.
- Hover or long-press fans cluster members in an MTGA-style arc.
- A selected fanned member keeps the fan open until deselected.
- Hover raise lifts the hovered card and its attachment stack above peers for paint and hit testing.

These transforms are presentation overlays only. They do not change object identity, game zones, controller, or engine state.

### Attachments and tapped cards

Attachments remain associated with their host for layout and hover raise. Tapped cards rotate through the render data so paint and hits can agree on footprint. In-flight cards use flight poses for paint only; their source/destination ownership is handled by flight state.

## Implementation Decisions

- Keep camera math pure and shared by Canvas, Mount, and HTML projection code.
- Use `RenderCard` as the board layout contract; do not read object positions from DOM.
- Resolve hits from logical layout and topmost order, not animation poses.
- Treat density, hover raise, packing, and cluster fans as layout overlays rather than engine facts.
- Keep `fitCamera` in geometry code so tests can exercise it without rendering.

## Testing Decisions

- Camera tests cover world/screen round trips, pan, zoom-at invariants, and fit behavior.
- Layout tests cover seat placement, zone columns, tapped rotation fields, attachments, and player-count variation.
- Hit-test tests cover overlapped/tapped cards, topmost resolution, and avatar hits.
- Density tests cover row packing, cluster fan poses, clamping to seat bands, and hover raise ordering.
- Interaction tests cover pan-vs-click thresholds and camera user-moved behavior.
- Board sync tests cover that a user-panned camera is preserved across later game syncs (actions/deltas must not re-fit).

## Out of Scope

- Multi-touch pinch zoom.
- Persisting per-user camera across sessions.
- Changing engine object ordering to support visual packing.
- Reflowing the board for portrait orientation.

## Further Notes

- Sibling specs: [`2026-07-20-board-composition.md`](2026-07-20-board-composition.md), [`2026-07-20-battlefield.md`](2026-07-20-battlefield.md), [`2026-07-20-flights.md`](2026-07-20-flights.md).
