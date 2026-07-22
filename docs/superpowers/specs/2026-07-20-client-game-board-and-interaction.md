# Client Game Board and Interaction

**Status:** Current (as of 2026-07-22)
**Module:** `client/app/board/` — `canvas/` (vector felt/avatars/arrows/scene), `bitmap/` (Mount card art + flights), `html/` (hand, stack, prompts, chrome), `geometry/` (camera, hit-test, layout, density, interaction), `action/` (session, targeting, execution), `motion/flights.ts`, plus `submodel.ts` / `view.ts` / `messages.ts`

---

## Problem Statement

A 4-player Commander (MTG) game requires a shared table where every player sees the battlefield from their own perspective, can drag spells from their hand, declare attackers and blockers, respond during opponent turns, and receive instant feedback when the game advances. The board must carry hundreds of permanents per seat without visual collision, support continuous card-play animation (client-game-board-and-interaction spec), synthesize audio attention cues without shipped audio files (client-game-board-and-interaction spec), and remain readable under the Landscape Rule (DESIGN.md).

A pure DOM approach collapses under the paint cost of rendering 300+ permanents per frame. A pure retained-scene graph (Pixi, Konva) imposes a large dependency and opaque z-order between the canvas board and the HTML hand/stack overlays. The dual-surface architecture — canvas for the battlefield, HTML for hand/stack — was chosen deliberately and is an invariant.

---

## Solution

The board is a **dual-surface** Foldkit submodel (`client/app/board/`): a full-screen Foldkit **Canvas** paints vector battlefield layers (felt, seats, avatars, arrows, scene) while a Foldkit **Mount** bitmap layer paints card art and flights; thin **HTML** overlays handle hand tiles, the stack pile, the mana tray, priority chrome, and the inspect dock. A single camera transform (pan + zoom) is the shared source of truth for both surfaces so they stay aligned under scroll and resize.

The living architecture map for this module is **[docs/client-canvas-map.md](../client-canvas-map.md)**. The module map, invariants, and **authoritative board layer stack** (bottom→top paint/DOM order) there are the source of truth for z-index and paint order. This spec documents the behavior visible to players and the decisions behind it; the canvas map documents the code structure.

---

## User Stories

- As a player, I see my seat at the bottom of the table and my opponents above me, matching where they would physically sit.
- As a player, I drag a card from my hand over the threshold to play it; the card flies from my hand to the stack without a disjointed snap.
- As a player, I can click a battlefield permanent to select it and open a radial menu of its activated abilities, then select one to cast.
- As a player with priority during a non-empty stack, I see a "Resolve card" button and a "Resolve stack" arm control, not a generic "Next".
- As an active player who wants to stop clicking Next, I press "End Turn" and the game advances through my remaining phases while opponents retain their response windows.
- As a player, I Alt-click (or Alt-hold) a face-up card to pin it in the inspect dock on the left, with a modifier ledger for battlefield permanents.
- As a player on a crowded board, packed cards lift when I hover them; identical indistinguishable permanents collapse into a cluster face+count and fan on hover.
- As a player, I hear a soft synthesized ping when I gain priority, a warmer chord when my turn begins, and light table-feel cues (land, spell, resolve, damage) as the game advances.
- As a spectator, I see the board read-only with no hand bar or action controls.
- As an eliminated player, I keep the canvas and can watch the rest of the game; my hand and action controls are removed.

---

## Behavior

### Camera and coordinate model

`lib/camera.ts` defines the camera as `{ panX, panY, zoom }` with a pure forward transform:

```
screen = world * zoom + pan
```

`worldToScreen` and `screenToWorld` are pure functions (no DOM). `zoomAt(cam, sx, sy, factor)` solves the pan so the world point under `(sx, sy)` stays fixed — standard scroll-zoom behavior. Min zoom 0.2, max zoom 5. `fitCamera` frames the whole table between the turn banner HUD and the hand bar, capping at `zoom ≤ 1.35`. A `userMoved` guard in `TableSurface` prevents re-fitting after the player has panned.

### Logical layout (`layout.ts`)

`layout(state, viewer)` converts a `VisibleState` into a flat `RenderCard[]` array: one entry per visible object, carrying world-space `(x, y, w, h)`, zone, seat, rotation (tapped = 90°), kind, and cluster membership. Seat bands follow a 2×2 quadrant arrangement: viewer bottom-left, front directly above, side beside, diagonal for the fourth. Top-row seats are flipped (rotated 180°) to face down across the table. Fewer than four seats leave the later quadrant positions empty. Zone columns (commander, exile, library, graveyard) sit on the left edge of each seat band; mana trays anchor outside the band under the zone column. Card world units: 96 × 134 (`CARD_W` × `CARD_H`).

### Hit testing

`lib/hitTest.ts` maps a screen coordinate to a `RenderCard` using the **density-overlaid logical layout** (not tweened/drawn positions). `withHoverRaise` and `withBoardDensity` are applied before hit tests so packed and fanned cards resolve correctly. Tweened positions belong exclusively to the paint path. This separation is a hard invariant (see canvas-map invariants).

### Density, packing, and clusters (client-game-board-and-interaction spec)

`lib/boardDensity.ts` provides two transforms layered over logical layout:

- **Row packing:** when a battlefield row exceeds its nominal slot count, horizontal spacing is independently compressed per row until cards stay inside the seat band. No seat widening, no spill.
- **Permanent clusters:** when packing alone is insufficient, identical indistinguishable groups (same card, no attachments) collapse into a cluster face + member count. Hover or long-press (400 ms) fans the members in an MTGA-style arc (`FAN_STEP = CARD_W * 0.45`; up to ±12° tilt). A selected fanned member stays raised with the fan open until deselected.
- **Hover raise:** any hovered distinct card (and its attachment stack) floats to the top of the paint and hit-test list.

### Canvas paint pipeline (`lib/boardDraw.ts`)

The `draw(ctx, scene, arrowAnim)` call is the single paint orchestration entry point. Paint order (invariant): felt → seats → resting cards → avatars → arrows → flights on top. Sub-helpers — `boardFelt`, `boardCardPaint`, `boardAvatarPaint`, `boardArrows`, `boardPaintPrims` — are pure functions of their inputs (no canvas globals). The canvas paint loop lives in `onMount`'s `createEffect` tracking `tick()`, `size()`, camera, drawnCards, and the full game scene; it fires on any change. Arrow draw-on animations (`arrowAnim`) request follow-up frames via `requestAnimationFrame` until settled.

Canvas colors are hex literals (exempt from Tailwind tokens); the legend in DESIGN.md must stay in sync when badge/outline colors change.

### Canvas flight layer — play motion (client-game-board-and-interaction spec, `controllers/playMotion.ts`)

When a player commits a play (hand drop or radial), `PlayMotion.spawnFromHand` spawns a **canvas flight**: a single actor carrying screen-space position + scale that interpolates from commit to destination over ~150–200 ms exponential ease. The flight owns the card id until settle:

- The DOM hand ghost and the resting canvas face are suppressed (`hideCardIds`, `handHidden`).
- On delta arrival the flight is **retargeted** to the permanent's or stack object's layout position — no second `ENTER_RISE` card appears.
- Stack resolve / leave-stack: a flight spawns from `stackAimOrigin` at stack scale toward the BF/GY layout position.
- `prefers-reduced-motion`: flights snap to target immediately.
- Opponent plays enter from the avatar position when no hand ghost exists.

Flights paint **above** resting cards in the draw order. `flightOwnedIds` is exposed to `TableSurface` and `StackOverlay` so competing entrance seeds and CSS stack-in deltas are suppressed while a flight owns the id.

### DOM hand bar (`components/molecules/hand.tsx`)

The hand bar is a DOM overlay anchored at the bottom of the viewport. Each card face is a full MTGA-sized tile (`HAND_CARD_W` from `HAND_FACE_W`), offset into a dense fan (up to ±10° tilt, center rises). Cast-cost pips sit above each face. A zone-group ordering matches Arena: command → hand → graveyard → exile, separated by aura color gaps (no section captions). Dragging a card above the play threshold — clearance measured as `HAND_BAR_H - HAND_PLAY_SLACK_PX` above the hand bar — constitutes a drop; releasing below snaps back. `hiddenId` suppresses the face while a flight owns it. The hand suppresses entirely for spectators and eliminated players.

### Stack overlay (`components/organisms/stack-overlay.tsx`)

The stack renders as a right-edge DOM overlay. Three presentation modes driven by viewport height and object count: **pile** (peeking strip from the right edge), **expanded strip** (full names visible), and **full** (multi-row grid). The `stackPeekFor` helper calculates peek height from available vertical space minus `STACK_VERTICAL_RESERVED`. The "helpless dwell" timer (turn-priority-and-stack spec) pauses stack-hold when hovering — `onDwell` wires to the server `SetStackDwell` intent. CSS translate deltas (`entranceDeltas`) drive play-in animations for opponent and non-flight stack entrances. The staged card ghost appears in the pile when arrow-aiming (suppressed in expand/full modes). `hideFaceIds` hides resting faces for ids owned by canvas flights.

### Targeting and the staging pipeline (`controllers/action-session.tsx`, `lib/targeting.ts`)

When a player selects a spell or ability requiring a target, the action session enters **arrow-aiming mode**:

- A draw-on arrow animates from the staged card's stack origin toward the cursor.
- Valid targets are highlighted (Mountain Red for player targets, Island Blue for object targets per DESIGN.md).
- Clicking a valid target calls `session.aim(target)`, completing the `take_action { id, target }` submission.
- Stack-expand and tile-picker modals suspend arrow drawing (but staged still blocks Pass/Space/yield).
- `session.cancel()` aborts local pre-submit state only — pending engine choices are handled by `PromptHost`.

### Combat staging (`controllers/combatStaging.ts`)

During Declare Attackers step, the player drags a creature to an opponent's life-orb avatar to stage an attack. DOM life-orb hit targets (z=16, `pointer-events-auto` only when `lifeOrbInteractive()`) overlay the canvas orbs so Playwright can target them. During Declare Blockers, a drag from a blocker to a declared attacker stages a block. `handleCombatDrop` resolves drops into `WireAttack[]` / `WireBlock[]` changes purely. The primary action button (Confirm Attackers / Confirm Blockers) submits the declaration. `cancelAttacker` / `cancelBlocker` remove staged declarations before submission. Required attacks (goad) are merged with staged ones via `mergeRequiredAttacks`.

### Priority chrome (turn-priority-and-stack spec, turn-priority-and-stack spec, turn-priority-and-stack spec)

`PriorityContextBar` sits bottom-right, above the stack overlay, below prompt forms. It reads `StackChrome` from `boardChromeFromState` (shared memo — no divergent staged/mana recomputation). Controls:

| Condition | Controls shown |
|---|---|
| Empty stack, your priority | **Next** (primary) |
| Non-empty stack, your priority, meaningful action | **Resolve card** (primary) + **Resolve stack** arm |
| Non-empty stack, helpless | Nothing (dwell/hold times out) |
| Active player | **End Turn** toggle (turn-priority-and-stack spec) |
| Non-active player | **Until my turn** rocker (turn-priority-and-stack spec) |

Space bar mirrors the primary action. Enter mirrors End Turn / turn-yield toggle. `yielded` reflects the server flag — no client mirror to drift. Stack yield is one-shot: arm, then disabled until the stack empties. Turn yield clears on own-intent, on being attacked, or at Untap as active player.

**Instant-priority focus** (battlefield dimming, turn-priority-and-stack spec): client-only presentation. When you can act in a non-empty-stack window, non-usable permanents take a black veil; legal activates and untapped mana sources stay bright. Empty-stack main and declare attackers/blockers stay fully lit. Spectators are never dimmed.

### Activation radial

Selecting your battlefield permanent opens a **continuous SVG donut** of legal options
(`radialOptions`: tap-for-mana + battlefield activates). Wedges commit on **pointer-up**
on the same wedge (slide-off cancels; outside/hole dismisses). Screen center + zoom are
frozen while open. Empty option lists do not show a hollow ring.

### Inspect dock (client-game-board-and-interaction spec, `lib/inspect.ts`, `lib/deck-builder/card-hover-preview.ts`)

Board inspect uses the shared card-preview **`dock`** mode (same module as deck-builder hover; `mode: "follow" | "dock"`). Alt-down over a face-up card pins it with a full-board dim **backdrop** (modal: board/HUD clicks blocked). `InspectPin` carries `{ name, prepared, objectId?, cardId?, print? }`. Layout:

- **Left:** card art with DFC flip for prepared permanents (opens on the play face — back when `prepared`).
- **Right:** oracle text with inline mana pips; for battlefield permanents, **modifier ledger / effects** in that column.
- **Topmost** in the board layer stack (layer 10) — above prompts, HUD, and system modals while pinned.

Modifier ledger: continuous mods re-derived live from the snapshot, timed/stateful mods recorded by `source_name`. Contributions grouped by source card def name; each name is an underlined link that pushes that def onto the inspect **history stack**. Back control pops. Marked damage is out of scope.

Releasing Alt or pressing Esc dismisses. Space is blocked while the dock is open.

### Turn chrome and phase track (`components/organisms/turn-chrome.tsx`)

`TurnBanner` renders a fixed phase-track HUD (five phase bands: Beginning, Main 1, Combat, Main 2, End) with the active player's name and a priority watch indicator. The current phase band is highlighted; multi-step phases show the specific step name below the band when it differs (e.g. "Declare Attackers"). Attention audio cues (client-game-board-and-interaction spec) fire here: `playAttentionYourTurn` when the active player flag flips to you; `playAttentionPriority` when priority arrives at you (your-turn cue wins when both arrive in the same update). Watchers and eliminated seats are skipped for attention cues.

### Table audio (client-game-board-and-interaction spec, `lib/tableAudio.ts`)

All audio is synthesized via Web Audio API — no shipped files, no VO, no music. The shared `AudioContext` is **unlocked on lobby Ready-up** (user gesture requirement). Table-feel cues (`playTableFeelLand`, `playTableFeelStack`, `playTableFeelResolve`, `playTableFeelDamage`) fire once per kind per delta batch from `store.ts`'s `lastTableFeelBatch`. Attention cues (`playAttentionPriority`, `playAttentionYourTurn`) fire from the turn chrome. Mute preference stored in `localStorage` under `mtgfr.sound`; default on. Muted or suspended contexts no-op silently. One cue per kind per delta prevents audio spam on fast sequences.

### Discoverability and legend (`components/organisms/board-discoverability.tsx`)

A coaching strip (`HintStrip`) appears at game start explaining the interaction grammar (drag, Alt, Space/Esc, badge/dot meanings). It auto-hides after 12 s, hides on explicit dismiss (persisted to `localStorage` as `mtgfr.hintDismissed`), and hides on the first real hand drag-drop. A `?` button opens the `LegendPanel` explaining badge colors, outline meanings, and keyboard shortcuts. Sound toggle appears beside the `?` button (visible to all, including spectators).

### Accessibility

A `sr-only` `aria-live="polite"` region carries `boardStatusSummary(game.state, viewer)` — a spoken-word summary of the current board state for screen readers. The canvas itself has no ARIA role (it is an unlabeled pointer surface). DOM life-orb buttons carry `aria-label` with player name and life total. The `hitQuiet` prop on Ghost controls inflates hit targets to ≥44×44 px for coarse pointers.

### Reconnect banner

`game-reconnecting` div appears fixed top-center (z-40) in `reconnect-rust` background when the stream is disconnected. The stream health is read from `connectedAtom`.

### Result overlay and concede

`ResultOverlay` appears on win/loss. "Watch" dismisses it (the eliminated player keeps the read-only board). "Leave" navigates back to `/`. Concede goes through a `ConfirmDialog` before submitting the `concede` intent — concede is a real game action (CR 104.3a), not navigation.

### Pile expand

Any non-battlefield zone pile can be expanded into a `PileOverlay` by clicking it. Escape or the close button dismisses.

### Spectator mode

`viewer === SPECTATOR_VIEWER` (255) removes the hand bar and all action affordances. A fixed "Spectating" badge appears. The board renders read-only. The server rejects any intent from spectators.

---

## Implementation Decisions

- **Dual surface is intentional.** Resting hand and stack are DOM; battlefield + zone piles + flights are canvas. Do not merge into one scene graph. See canvas-map invariants.
- **Hits use logical layout, never tweened positions.** `withBoardDensity` applies to both hit and paint paths so hover-raise and fans are consistent, but tweened/drawn positions from `drawnCards()` are paint-only.
- **Flights suppress competing entrances.** `flightOwnedIds` feeds both `TableSurface` (skips competing seeds) and `StackOverlay` (hides resting faces). `hideCardIds` suppresses canvas resting faces. `handHidden` suppresses DOM hand ghosts.
- **`drawnCards()` vs `cards()`**: `drawnCards()` carries tween and density overlays (used by paint and `byId` lookup); `cards()` is the logical layout (used by hit testing and flight targeting).
- **The camera is a Solid signal** (`createSignal`) inside `TableSurface` so paint and hit-test effects track it reactively.
- **Payment is engine-side.** The client does not plan land taps; it previews the `auto_tap` field from the action hover but submits only action id + target. `settle_payment` auto-taps on the server (choices-actions-and-resolution spec).
- **Stack yield is one-shot, no revoke.** Resolve card is a normal `pass_priority`; Resolve stack arms the server flag until the stack empties. No in-chrome cancel.
- **End Turn reuses turn yield.** turn-priority-and-stack spec is not a new intent — it sets `turn_yield` while the player is active. Same `SetTurnYield` wire call, same `turn_yielded` stamp on `VisibleState`.
- **Canvas colors bypass Tailwind.** Hex literals in `Board.tsx` / `layout.ts` are exempt from `global.css` tokens; the DESIGN.md legend is the sync point.
- **`selectedId` clears when the selected object leaves the battlefield.** The `createEffect` guards against stale selection across zone changes.
- **Image preload on board mount.** `preloadDecksIntoCache` runs once per board mount, warming all seated decks' art into `sharedImageCache` before the first delta arrives.

---

## Testing Decisions

- `lib/camera.test.ts` — pure unit tests for `worldToScreen`, `screenToWorld`, `zoomAt`, `panBy`.
- `lib/hitTest.test.ts` — unit tests for card/avatar hit resolution under tapped/fanned footprints.
- `lib/boardDensity.test.ts` — row packing and cluster fan pose.
- `lib/boardScene.test.ts` — `BoardScene` builder + `paintBoardScene` dumb paint (no canvas).
- `lib/boardDraw.stackAim.test.ts` — stack aim origin geometry.
- `lib/interaction.test.ts` — pointer FSM: pan vs click vs combat drag.
- `lib/stackLayout.test.ts` — stack geometry helpers.
- `controllers/tableSurface.test.ts` — `SurfaceEffect` resolution from pointer sequences.
- `controllers/action-session.test.ts`, `controllers/actionExecution.test.ts` — cast pipeline.
- `controllers/combatStaging.test.ts` — attacker/blocker staging and drop resolution.
- `controllers/playMotion.test.ts` — flight lifecycle: spawn, retarget, absorb, GC.
- `lib/tableAudio.test.ts` — synthesized cue logic without `AudioContext` (reset helper).
- End-to-end board interaction is validated by the `verify` skill (live two-player game via Playwright — see `.agents/skills/verify/SKILL.md`).

---

## Out of Scope

- Multi-touch pinch-to-zoom (pointer events only; `onWheel` for zoom).
- WebGL / Pixi / Konva migration.
- Unified DOM+canvas retained scene graph.
- Per-card unique sound effects or music.
- Accessibility reflow of the board for portrait phones (portrait gate is a rotate prompt, never a reflow — DESIGN.md Landscape Rule).
- Full CR 613 layer ordering for displayed power/toughness (engine-core-and-event-model spec approximation; flagged in the deck increments under `docs/fidelity/`).
- Unconditional pass-turn (Arena Shift+Enter) — out of scope per turn-priority-and-stack spec.

---

## Further Notes

- **Living map:** [docs/client-canvas-map.md](../client-canvas-map.md) is the authoritative module→responsibility table and invariants list. Update it when adding modules or changing ownership.
- **DESIGN.md tokens:** canvas hex exemption applies. When a badge or outline color changes in `boardCardPaint.ts`, update the DESIGN.md legend swatches.
- **Forge reference:** for tricky combat, stack, and priority interactions, consult [Forge's card scripts and rules implementation](https://github.com/Card-Forge/forge).
- **client-game-board-and-interaction spec is superseded.** Segmented DOM/canvas play-motion legs (0033) were replaced by continuous canvas flights (0035). Do not re-introduce multi-leg play motion.
- **Mana tray** is a world-anchored DOM overlay, not part of the canvas paint — `ManaTray` receives projected screen positions from `projectManaTrays(...)` and renders CSS-positioned elements above the canvas.
- **`data-testid` markers** on `bf-card-{id}` (pointer-events none) and `life-orb-{seat}` give Playwright real screen coordinates to aim combat/targeting gestures.
- **Client stack is Foldkit + Nitro.** Historical Implementation Decisions may still mention the pre-cutover SolidStart tree; live board code is the Foldkit submodel (`client/app/board/`) with Canvas + Mount + HTML overlays. See the [Foldkit migration design](2026-07-21-foldkit-client-migration-design.md) (archive) and [`docs/client-canvas-map.md`](../client-canvas-map.md).
