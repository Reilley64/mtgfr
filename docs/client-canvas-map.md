# Agent navigation — client canvas board

How agents find where battlefield paint, hits, flights, and DOM overlays live.
This is a **code map**, not a design system doc — tokens stay in [`DESIGN.md`](../DESIGN.md).

## How to find a concern

1. **Paint (pixels):** `client/src/lib/boardDraw.ts` orchestration → felt / cards / avatars / arrows modules; scene builder in `boardScene.ts` when present.
2. **Hits / camera:** `client/src/lib/camera.ts`, `hitTest.ts`, `boardDensity.ts`; composed by `controllers/tableSurface.ts`.
3. **Flights (client-game-board-and-interaction spec):** `controllers/cardFlights.ts` + `lib/cardFlight.ts` — canvas owns in-flight cards; resting hand/stack stay DOM.
4. **Wiring root:** `components/organisms/board.tsx` — controllers, pointer Match, paint loop, composition only.
5. **DOM chrome:** sibling organisms (`stack-overlay`, `turn-chrome`, `priority-context-bar`, `board-discoverability`, `board-overlays`) + molecules (`hand`, mana tray, prompts).

## Module → responsibility map

| Module | Role |
|--------|------|
| `lib/camera.ts` | Camera SoT: `screen = world * zoom + pan` |
| `lib/hitTest.ts` | Screen→world card/avatar hits (tapped/fan footprints) |
| `lib/boardDensity.ts` | Row packing / hover-raise / clusters ([client board](superpowers/specs/2026-07-20-client-game-board-and-interaction.md)) |
| `lib/stackLayout.ts` | Shared stack geometry (DOM overlay + aim origins) |
| `lib/boardDraw.ts` | `DrawCtx` + paint orchestration (re-exports stack helpers) |
| `lib/boardFelt.ts` / `boardCardPaint.ts` / `boardAvatarPaint.ts` / `boardArrows.ts` | Dumb paint helpers |
| `lib/boardScene.ts` | Plain `BoardScene` builder + dumb `paintBoardScene` |
| `lib/tween.ts` / `lib/cardFlight.ts` / `lib/imageCache.ts` | Paint-only motion + art decode |
| `lib/interaction.ts` | Pointer FSM reducers + `fitCamera` |
| `controllers/tableSurface.ts` | Camera + hits + `drawnCards` facade (`SurfaceEffect`) |
| `controllers/tableEntrances.ts` | Pure entrance seeding for zone piles |
| `controllers/cardFlights.ts` | Flight spawn/step; `hideCardIds` / `flightOwnedIds` |
| `controllers/action-session.tsx` / `combatStaging.ts` | Play / target / combat staging |
| `components/organisms/board.tsx` | Composition root only |
| `components/organisms/stack-overlay.tsx` | Stack DOM (pile / strip / full) |
| `layout.ts` | Seat bands, card size, zone columns |

## Invariants (do not break)

1. **Hits use logical layout**, never tweened/`drawnCards` paint positions. TableSurface documents this; keep it.
2. **Paint order:** felt → seats → resting cards → avatars → arrows → flights on top.
3. **Flight ownership:** while a canvas flight owns an id, suppress duplicate stack CSS entrances and hide the resting face (`hideCardIds` / `flightOwnedIds`).
4. **Hand/stack rest as DOM;** battlefield + zone piles + flights are canvas. Do not merge into one scene graph.
5. **Canvas colors** are hex literals (see DESIGN.md); keep the legend swatches in sync when changing badge/outline colors.

## Related docs

| Doc | Use for |
|-----|---------|
| [Client board spec](superpowers/specs/2026-07-20-client-game-board-and-interaction.md) | Packing, flights, chrome, audio, inspect |
| [`DESIGN.md`](../DESIGN.md) | Tokens; canvas hex exemptions |
| [`agent-navigation.md`](agent-navigation.md) | Engine CR lookup (server-side) |

## Non-goals

- No Pixi / Konva / fabric / WebGL migration from this map.
- No unified DOM+canvas retained graph — dual surface is intentional.
- Decision history lives in the feature specs; do not duplicate them here.
