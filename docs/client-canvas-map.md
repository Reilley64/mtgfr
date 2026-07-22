# Agent navigation — client canvas board

How agents find where battlefield paint, hits, flights, and DOM overlays live.
This is a **code map**, not a design system doc — tokens stay in [`DESIGN.md`](../DESIGN.md).

The client is a Foldkit SPA hosted on Nitro; the board is one submodel with three
surfaces (Canvas vector, Mount bitmap, HTML overlays). See the [Foldkit migration
design](superpowers/specs/2026-07-21-foldkit-client-migration-design.md) for the
overall module split.

## How to find a concern

1. **Paint (pixels):** `client/app/board/canvas/scene.ts` builds the vector `BoardScene`; `felt.ts` / `avatars.ts` / `arrows.ts` are dumb paint helpers under `client/app/board/canvas/`.
2. **Bitmaps (card art):** `client/app/board/bitmap/mount.ts` — Foldkit `Mount` regions blit card faces + flights on top of the canvas via the shared `ImageCache`.
3. **Hits / camera:** `client/app/board/geometry/{camera,hit-test,density,layout,interaction}.ts` — pure geometry; used by the board submodel + `action/` planners.
4. **Flights:** `client/app/board/motion/flights.ts` — canvas-owned in-flight cards; resting hand/stack stay HTML.
5. **Board submodel:** `client/app/board/submodel.ts` composes canvas, bitmap, motion, action-session, and HTML overlays. `view.ts` is the composition root.
6. **HTML chrome:** `client/app/board/html/` — `stack.ts`, `turn-chrome.ts`, `priority-bar.ts`, `discoverability.ts`, `overlays.ts`, `hand.ts`, `mana-tray.ts`, `actions.ts`, `prompts.ts`, `activation-radial.ts`, `inspect.ts`.

## Module → responsibility map

| Module | Role |
|--------|------|
| `app/board/geometry/camera.ts` | Camera SoT: `screen = world * zoom + pan` |
| `app/board/geometry/hit-test.ts` | Screen→world card/avatar hits (tapped/fan footprints) |
| `app/board/geometry/density.ts` | Row packing / hover-raise / clusters ([client board](superpowers/specs/2026-07-20-client-game-board-and-interaction.md)) |
| `app/board/geometry/layout.ts` | Seat bands, card size, zone columns, attach layout |
| `app/board/geometry/interaction.ts` | Pointer FSM reducers + `fitCamera` |
| `app/board/geometry/combat-staging.ts` | Combat pointer resolution |
| `app/board/canvas/scene.ts` | Plain `BoardScene` builder + dumb `paintBoardScene` |
| `app/board/canvas/{felt,avatars,arrows}.ts` | Dumb canvas paint helpers |
| `app/board/bitmap/mount.ts` | Foldkit `Mount` regions for card faces |
| `app/board/bitmap/paint-cards.ts` / `paint-flights.ts` | Bitmap draw routines using `ImageCache` |
| `app/board/motion/flights.ts` | Flight spawn/step; `hideCardIds` / `flightOwnedIds` |
| `app/board/action/session.ts` | Play / target / combat staging session state |
| `app/board/action/{execution,targeting,modal,chrome}.ts` | Pure action planners |
| `app/board/submodel.ts` | Board `Model`/`update` composition |
| `app/board/view.ts` | Board composition root (canvas + Mount + HTML overlays) |
| `app/board/html/stack.ts` | Stack DOM (pile / strip / full) |
| `app/board/html/turn-chrome.ts` | Turn/priority chrome |
| `lib/image-cache.ts` | Art decode cache (shared canvas + bitmap) |
| `lib/wire/types.ts` | Wire shapes (snake_case) |

## Invariants (do not break)

1. **Hits use logical layout**, never tweened/`drawnCards` paint positions.
2. **Paint order:** felt → seats → resting cards (canvas + Mount bitmap) → avatars → arrows → flights on top.
3. **Flight ownership:** while a flight owns an id, suppress duplicate HTML entrances and hide the resting face (`hideCardIds` / `flightOwnedIds`).
4. **Hand/stack rest as HTML;** battlefield + zone piles + flights are canvas/Mount. Do not merge into one scene graph.
5. **Canvas colors** are hex literals (see DESIGN.md); keep the legend swatches in sync when changing badge/outline colors.

## Related docs

| Doc | Use for |
|-----|---------|
| [Client board spec](superpowers/specs/2026-07-20-client-game-board-and-interaction.md) | Packing, flights, chrome, audio, inspect |
| [Foldkit migration design](superpowers/specs/2026-07-21-foldkit-client-migration-design.md) | Board submodel split, Mount escape hatch |
| [`DESIGN.md`](../DESIGN.md) | Tokens; canvas hex exemptions |
| [`agent-navigation.md`](agent-navigation.md) | Engine CR lookup (server-side) |

## Non-goals

- No Pixi / Konva / fabric / WebGL migration from this map.
- No unified DOM+canvas retained graph — dual surface is intentional.
- Decision history lives in the feature specs; do not duplicate them here.
