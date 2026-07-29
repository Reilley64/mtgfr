# Board Camera and Layout

**Status:** Current (as of 2026-07-25)
**Module:** `client/app/board/geometry/camera.ts`, `client/app/board/geometry/interaction.ts`, `client/app/board/geometry/layout.ts`, `client/app/board/geometry/hit-test.ts`, `client/app/board/geometry/density.ts`, `client/app/board/html/camera-gesture-mount.ts`, `client/app/board/submodel.ts`

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

`worldToScreen` and `screenToWorld` are pure and do not read DOM state. `zoomAt(cam, sx, sy, factor)` preserves the world point under the screen coordinate while zooming. Zoom is bounded to the board limits. Panning, wheel zoom, and two-finger pinch zoom all set `cameraUserMoved` so automatic fitting does not fight the player on later game syncs.

Wheel and pinch gestures are translated by the board camera gesture Mount into `BoardCameraZoomed({ x, y, factor })`, where `x` and `y` are board-internal screen coordinates in the same space as canvas pointer handlers. The mount prevents native wheel/touch zoom only while the gesture starts over the live board rectangle.

### Board viewport

`board.viewport` is the size of the board in CSS pixels. The board is `fixed inset-0`, so it is the window's inner size: `initialBoardModel` measures the window on entry (falling back to 1440 x 900 where there is no window), and a persistent `resize` subscription feeds `BoardViewportResized({ width, height })` back into the board.

Every canvas layer sizes its backing store from this value while CSS stretches the element to fill the window, so a viewport that does not track the window renders the whole board through a scaled bitmap — blurry, aspect-distorted, and magnified relative to the HTML hand bar and overlays, which are laid out in real CSS pixels.

`board.dpr` is `window.devicePixelRatio` clamped to 3, measured alongside the viewport and refreshed by the same resize message. All three canvas layers multiply their backing store by it: the Mount bitmap and flight layers set `ctx.setTransform(dpr, …)` in `prepareLayerCtx`, and the Foldkit scene canvas wraps `sceneShapes` in a `Canvas.Group({ scale: { x: dpr, y: dpr } })` and divides its pointer coordinates back by the DPR, since Foldkit hands pointer events back in backing-store space. Without this the felt, seat bands, avatars, and arrows paint at 1x and are visibly soft on any retina display.

The bitmap and flight canvases are painted imperatively but their `width`/`height` attributes still come from the view, so both must state the same device-pixel size: the view multiplies the viewport by `board.dpr`, and `prepareLayerCtx` reads the DPR off the published frame rather than re-reading `window.devicePixelRatio`. If the two disagree, the vdom patch resizes the backing store out from under the Mount and the layer drops to 1x until something repaints it. The DPR is part of the resting paint snapshot, so changing it forces a repaint of the cleared canvas.

### fitCamera

`fitCamera(viewport, playerCount, reservedBottom)` frames the table for the active player count and available viewport; `reservedBottom` is the live `handMetrics(viewport).barH`, so the framing follows the bar as it rescales. It accounts for HUD and hand space and caps zoom so the table starts readable rather than over-magnified. The board re-fits on cold load, on player-count change, and on viewport resize, until the user pans or zooms. A resize re-fit remaps in-flight cards for the zoom change the same way a player-count re-fit does.

### RenderCard layout

`layout(state, viewer)` returns a flat `RenderCard[]` for visible objects. Each record carries world-space position and size, zone, owner/controller, seat, tapped rotation state, face status, print/card ids, combat/chrome fields, attachment info, and cluster membership.

Cards use 96 x 134 world units. Seat bands are arranged as a four-seat table from the viewer perspective: viewer at the bottom, opponents around the top and sides, with top seats oriented toward the viewer. Fewer than four seats leave unused bands empty. `seatSlot(seat, viewer, count)` is the shared viewer-relative slot (viewer = slot 0) behind both `seatCell` and the first-player reveal grid; a viewer that sits at no seat — the spectator sentinel id — anchors on seat 0, so a spectator sees plain seat order across the four quadrants rather than every band folding onto one cell. Zone columns for command, graveyard, exile, and library live at the left edge of each seat band, and battlefield mana anchoring is derived from the same geometry.

Seat avatars use a 40 world-unit radius. Their label gutter is part of `boardBounds`: hand count sits toward the battlefield, while life, username, and commander-damage labels sit on the outer side of the circle. Top-row flipped seats mirror those offsets, so `fitCamera` reserves room on the side where labels actually paint instead of only the circular face.

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
- Keep camera persistence out of browser storage; the camera resets to fit on cold board entry until the user pans or zooms.

## Testing Decisions

- Camera tests cover world/screen round trips, pan, zoom-at invariants, and fit behavior.
- Layout tests cover seat placement, zone columns, tapped rotation fields, attachments, avatar label bounds, player-count variation, and that a spectator viewer still resolves four distinct quadrants.
- Hit-test tests cover overlapped/tapped cards, topmost resolution, and avatar hits.
- Density tests cover row packing, cluster fan poses, clamping to seat bands, and hover raise ordering.
- Interaction tests cover pan-vs-click thresholds and camera user-moved behavior.
- Gesture mount tests cover wheel factors, pinch factors, and client-to-board coordinate conversion.
- Board sync tests cover that a user-panned or user-zoomed camera is preserved across later game syncs (actions/deltas must not re-fit).
- Viewport tests cover that the board starts at the measured window size, that a resize updates the viewport so the canvas backing store matches its CSS box, that a resize re-fits the camera, and that a camera the player moved survives a resize.
- A board Scene test locks that the scene canvas backing store is the viewport multiplied by the DPR.

## Out of Scope

- Persisting per-user camera across sessions.
- Changing engine object ordering to support visual packing.
- Reflowing the board for portrait orientation.

## Further Notes

- Sibling specs: [`2026-07-20-board-composition.md`](2026-07-20-board-composition.md), [`2026-07-20-battlefield.md`](2026-07-20-battlefield.md), [`2026-07-20-flights.md`](2026-07-20-flights.md).
