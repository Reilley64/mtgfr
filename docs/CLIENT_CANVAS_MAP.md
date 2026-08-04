# Agent navigation — client canvas board

How agents find where battlefield paint, hits, flights, and DOM overlays live.
This is a **code map**, not a design system doc — tokens stay in [`DESIGN.md`](../DESIGN.md).

The client is a Foldkit SPA hosted on Nitro; the board is one submodel with three
surfaces (Canvas vector, Mount bitmap, HTML overlays). Living board requirements:
[`game-board`](../openspec/specs/game-board/spec.md).

## How to find a concern

1. **Paint (pixels):** `client/app/board/canvas/scene.ts` builds the vector `BoardScene`; `felt.ts` / `avatars.ts` / `arrows.ts` are dumb paint helpers under `client/app/board/canvas/`.
2. **Bitmaps (card art):** `client/app/board/bitmap/mount.ts` — Foldkit `Mount` regions blit card faces + flights on top of the canvas via the shared `ImageCache`.
3. **Hits / camera:** `client/app/board/geometry/{camera,hit-test,layout,interaction}.ts` — pure geometry; used by the board submodel + `action/` planners. `client/app/board/engagement.ts` supplies the committed-permanent set that layout splits out of clusters.
4. **Screen motion / flights:** `client/app/board/motion/flights.ts`, `exit-fx.ts`, `screen-motion.ts` — canvas-owned drag ghosts, in-flight cards, and battlefield exit FX share the Mount flight layer; resting hand/stack stay HTML.

   Flight / ExitFx animation is Mount-local rAF: mid-flight ticks paint only the
   flight canvas. Drag ghosts republish with Foldkit pointer updates (no rAF
   required for drag alone). Resting bitmap republishes when layout/chrome/hide
   sets change, not on every pose tick. Model receives `FlightsSynced` when
   flying or `ExitFx` membership changes.

5. **Board submodel:** `client/app/board/submodel.ts` composes canvas, bitmap, motion, action-session, and HTML overlays. `view.ts` is the composition root.
6. **HTML chrome:** `client/app/board/html/` — `stack.ts`, `turn-chrome.ts`, `priority-bar.ts`, `discoverability.ts`, `overlays.ts`, `hand.ts`, `mana-tray.ts`, `actions.ts`, `log-panel.ts`, `prompts.ts`, `activation-menu.ts`, `inspect.ts`.

## Module → responsibility map

| Module | Role |
|--------|------|
| `app/board/geometry/camera.ts` | Camera SoT: `screen = world * zoom + pan` |
| `app/board/geometry/hit-test.ts` | Screen→world card/avatar hits (tapped/fan footprints) |
| `app/board/geometry/layout.ts` | Seat bands, card size, zone columns, attach layout, row packing, clusters ([game-board](../openspec/specs/game-board/spec.md)) |
| `app/board/engagement.ts` | Committed permanents that split out of a cluster (attackers, blockers, targets) |
| `app/board/geometry/interaction.ts` | Pointer FSM reducers + `fitCamera` |
| `app/board/geometry/combat-staging.ts` | Combat pointer resolution |
| `app/board/canvas/scene.ts` | Plain `BoardScene` builder + dumb `paintBoardScene` |
| `app/board/canvas/{felt,avatars,arrows}.ts` | Dumb canvas paint helpers |
| `app/board/bitmap/mount.ts` | Foldkit `Mount` regions for card faces |
| `app/board/bitmap/paint-cards.ts` / `paint-flights.ts` | Bitmap draw routines using `ImageCache` |
| `app/board/motion/flights.ts` | Flight spawn/step; `hideCardIds` / `flightOwnedIds` |
| `app/board/motion/exit-fx.ts` | Battlefield destroy/exile FX step + particle budgeting |
| `app/board/motion/screen-motion.ts` | Drag ghost pose; screen-motion paint ownership helpers |
| `app/board/bitmap/paint-screen-motion.ts` | Single flight-layer paint: drag + flights + ExitFx |
| `app/board/action/session.ts` | Play / target / combat staging session state |
| `app/board/action/{execution,targeting,modal,chrome}.ts` | Pure action planners |
| `app/board/bitmap/paint-exit-fx.ts` | Paint battlefield exit FX on the flight canvas |
| `app/board/submodel.ts` | Board `Model`/`update` composition |
| `app/board/view.ts` | Board composition root (canvas + Mount + HTML overlays) |
| `app/board/html/stack.ts` | Stack DOM (pile / strip / full) |
| `app/board/html/turn-chrome.ts` | Turn/priority chrome |
| `app/board/html/log-panel.ts` | Game log HUD (`board-log`; last 30 fold lines) |
| `lib/image-cache.ts` | Art decode cache (shared canvas + bitmap) |
| `lib/wire/types.ts` | Wire shapes (snake_case) |

## Invariants (do not break)

1. **Hits use logical layout**, never tweened/`drawnCards` paint positions.
2. **Board layer stack (authoritative):** bottom → top paint/DOM order is fixed below. New board visuals must declare which layer they join; no ad-hoc `z-*` without updating this map.

   **Bottom → top:**

   Card/avatar paint order matches `mount.ts`: felt → seats → resting cards → avatars → arrows → flights.

   | # | Layer | Surface | Contents |
   |---|--------|---------|----------|
   | 1 | Felt / seats | Canvas vector | Table, seat bands |
   | 2 | Zone furniture | Canvas / world DOM | Library, command zone, **battlefield in-play mana** (left under your seat), GY, exile |
   | 3 | Resting battlefield permanents + avatars | Mount bitmap + Canvas vector (+ card chrome) | Battlefield faces paint first; avatar/life paint follows resting cards |
   | 4 | Arrows | Canvas | Committed attack/block, **declare-attackers drag aim**, spell aim — always above resting permanents |
   | 5 | Hand / stack / spell mana | HTML (`z-20`) | Resting hand & stack; **spell/payment mana tray** (same layer as hand, above hand cards); legend panel floats one notch up (`z-21`) |
   | 6 | Flights | Mount / motion (`z-30`) | In-flight play cards and battlefield exit FX — **above** hand and stack |
   | 7 | Combat / life hit targets | HTML | Interactive orbs when needed (paint stays in layer 2; hits here) |
   | 8 | Prompts / choice UI | HTML | `pending_choice` and related; centered `*-modal` / pile overlays (`z-40` / `z-29`); prompt frames, waiting chips, and the activation menu float at hand-adjacent `z-30` (panel `z-31` inside its own backdrop's stacking context); mulligan overlay is `z-40` and the one-shot first-player reveal sits above it at `z-50` |
   | 9 | Turn HUD / simple prompt bar | HTML | Phase track; idle Next / End Turn (`z-25`); **simple** prompt answer chrome in `priority-context-bar` at `z-45` so it stays above prompt/pile backdrops; the reconnect banner pins top at `z-40` |
   | 10 | Inspect dock | HTML (`z-100`) | Mode `dock` + backdrop — **topmost** |

   **Layer rules:**

   1. **Avatar paint** follows resting battlefield cards in layer 3; **clear bands** packing must not cover it. **Orb hits** stay in layer 7.
   2. **Two mana surfaces:** battlefield in-play mana (layer 2) vs spell/payment mana tray on the hand layer (5).
   3. No resting permanent paint or DOM card face may sit above layer 4 while combat/spell arrows are active. Declare-drag arrows and **declared stack→target arrows** use the **same Mount arrow layer** as committed combat arrows (not the Foldkit Canvas under resting art).
   4. Flights and battlefield exit FX paint above hand/stack (layer 6 over 5).
   5. Prompts (8) above combat/life hits (7). Simple-prompt primary actions live in layer 9 above prompt/pile dimmers; rich modals omit the priority bar.
   6. Inspect (10) above everything else on the board, including system modals, while pinned.
   7. Under-card name labels are forbidden on resting permanents (not a separate layer — deleted).

3. **Flight ownership:** while a flight or active `ExitFx` owns an id, suppress duplicate HTML entrances and hide the resting face (`hideCardIds` / `flightOwnedIds`).
4. **Hand/stack rest as HTML;** battlefield + zone piles + flights are canvas/Mount. Do not merge into one scene graph.
5. **Canvas colors** are hex literals (see DESIGN.md); keep the legend swatches in sync when changing badge/outline colors.

## Related docs

| Doc | Use for |
|-----|---------|
| [`game-board` OpenSpec](../openspec/specs/game-board/spec.md) | Living board requirements (composition, zones, flights, chrome) |
| [`DESIGN.md`](../DESIGN.md) | Tokens; canvas hex exemptions |
| [`AGENT_NAVIGATION.md`](AGENT_NAVIGATION.md) | Engine CR lookup (server-side) |

## Non-goals

- No Pixi / Konva / fabric / WebGL rewrite from this map.
- No unified DOM+canvas retained graph — dual surface is intentional.
- Decision history for past waves lives in git history; do not duplicate it here.
