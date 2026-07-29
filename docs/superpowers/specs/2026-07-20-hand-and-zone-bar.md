# Hand and Zone Bar
**Status:** Current (as of 2026-07-27)
**Module:** `client/app/board/html/hand.ts`, `client/app/board/html/hand-drag-mount.ts`, `client/app/board/html/actions.ts`, `client/app/board/geometry/handBarHit.ts`, `client/app/board/motion/flights.ts`, `client/app/board/motion/screen-motion.ts`, `client/app/board/submodel.ts`

## Problem Statement

Players need a bottom bar that keeps their private hand usable, keeps command-zone actions visible, and exposes graveyard/exile actions without crowding the battlefield. Spectators and eliminated players must not see or interact with a hand.

## Solution

Render a fixed DOM hand bar at the bottom of the board. It groups tiles in Arena order: command, hand, graveyard, exile. Command and hand show owned visible cards; graveyard and exile show playable actions from those zones. The bar owns drag-to-play hit geometry and hand/command playable borders. `actions.ts` owns `barZoneAura` and action grouping helpers shared by the bar.

## User Stories

- As a player, I can see my command card, hand, and playable graveyard/exile options in one bottom bar.
- As a player, I can drag a playable hand tile upward to play it.
- As a player, I can tell castable cards by their playable border.
- As a spectator, I do not see another player’s hand or action controls.

## Behavior

- One DOM tile per hand card; multiple legal hand-section actions on the same object do not mint extra tiles.
- Hand tiles fan with Arena-forward resting geometry, hover raise, and cost pips above the card face. The geometry is drawn against a 1440 x 900 window (`HAND_FACE_W` 208, `HAND_BAR_PEEK` 92, `HAND_VISIBLE_H` 178, derived bar height 218 — pip-row 24 + bar bottom padding 16 are already implied by that height).
- The bar is a constant fraction of the window, not a fixed pixel size. `handMetrics(viewport)` returns every bar length in CSS px for the live window, scaled by `handUiScale` — `min(width/1440, height/900)` clamped to `[0.75, 1.5]`. Face, peek, visible height, pip row, pip glyph, bar height, sticky inspect band, play slack, and the hand-flight/drag-ghost scale all derive from it, so a 208px face that reads on a laptop becomes a ~312px face on a 2560 x 1440 desktop rather than a thumbnail.
- Overlays anchored above the bar (prompts, log panel, priority bar, discoverability) read the inherited `--hand-bar-h` custom property set on the board root, so they follow the bar when it rescales instead of baking its height.
- Resting cost pips show the card's printed cast cost, not cycle or hand-ability costs. Multi-legal-mode tiles omit Cycle/Discard captions; a sole legal `cycle` or `activate_hand_ability` keeps that caption. Pips render through the shared `pipChip` (`board/html/pip-chip.ts`) — opaque plate + mana-font glyph sized by `--sz` / `--fsz` / `--plate` variables — also used by the activation menu and color-pick prompts.
- Hovering a bar tile elevates that tile's root above all other action-bar tiles (`[z-index:var(--hand-z)]` resting + `hover:[z-index:50]` on the slot; resting z is not inline). Discard / hand-put pick chrome uses `group/hand-tile` with `data-selected` / `data-selectable` on the tile root: Tailwind `group-data-[selected=true]/hand-tile:…` raises and rings Llanowar; unselected legal choices use `group-data-[selected=false]/hand-tile:group-data-[selectable=true]/hand-tile:…` Island blue. Selection alone does not elevate z; hover still brings a selected tile to the front. Non-choices omit those data attrs and stay off target chrome.
- A release above `barH - playSlack` commits the drop (play slack is 96 at the design window and scales with it); releasing below snaps back.
- Activating a hand tile with exactly one legal mode runs the existing play/cost/target pipeline immediately. With two or more legal modes, activation clears other local action sessions, seeds a stack flight, parks the card in local `playModePick` state, and opens docked `play-mode-aim` until `PlayModeChosen` continues the selected action through the same cost/target pipeline or Cancel restores the card.
- Design: [`2026-07-26-hand-play-mode-chooser-design.md`](2026-07-26-hand-play-mode-chooser-design.md).
- While `playModePick` is open, snapshot and delta sync reconcile the parked modes against current legal actions. Pruned modes disappear; exactly one remaining mode auto-continues through the same play/cost/target pipeline; zero remaining modes cancel the session, return the card to hand, and submit no intent.
- `hiddenId`, `hiddenIds`, and flight ownership suppress tiles while a staged play or flight owns the card.
- Playable hand/command tiles get the playable border from `barZoneAura(zone, playable)`.
- Unplayable hand/command tiles stay full brightness: no `brightness-[0.55]` or equivalent veil.
- Resting faces use `--shadow-hand` (`shadow-hand`). The drag source fades with `opacity-25` and loses playable aura. The drag **ghost** is painted on the Mount flight / screen-motion layer (`DragGhost` via `screen-motion.ts`), not as HTML — shared lift shadow and zone playable strokes, continuous with the flight that seeds on release. Pointer `clientX`/`clientY` are mapped into board logical space (`clientToHandDragPoint`) so the canvas ghost stays under the cursor when the board CSS box is stretched. Idle hits use `cursor-grab` when playable and `cursor-not-allowed` otherwise; an active drag sets `cursor-grabbing` on the document element.
- Graveyard/exile bar tiles appear only for actions and use their zone outline colors when playable.
- Graveyard-section actions include casts from that zone (flashback/escape/retrace), encore, and activated abilities whose source is in the graveyard (`functions_in_graveyard` — Teacher's Pest's `{B}{G}` self-return). Wire `ActionView.section` is `"graveyard"` for those activates so `bySection` buckets them here rather than the battlefield radial.
- Hand and priority controls render only for active seated players, not spectators or eliminated players.

## Implementation Decisions

- The bar is DOM, not canvas, so real buttons, keyboard activation, and drag data attributes stay available.
- Hand-pick selection chrome is Tailwind-first: the tile root is `group/hand-tile` and carries `data-selected` / `data-selectable`; raise, hit height, and Llanowar / Island rings are `group-hover` / `group-data-*` utilities rather than JS class ternaries. Art chrome is attribute-driven the same way: the tile root always carries `data-playable` and `data-drag-source`, and hover-brighten (`group-hover/hand-tile:group-data-[playable=true]/hand-tile:brightness-110`) plus drag-source fade (`group-data-[drag-source=true]/hand-tile:opacity-25`) are variant tokens on the face — never bare classes a ternary swaps in. Discard-cost hit targets are named controls: `role="button"`, tab focus, and an aria-label (`<name> (discard)`) — a discard pick is not a playable action, so it never borrows the playable label.
- `slotInert` is reserved for staged/in-flight cards; it is not a visual dimming signal for unplayable cards.
- `cardArt(h, opts)` is used for DOM faces and accepts optional `style` for precise tile sizing.
- Alt-inspect hover metadata is attached to every face-up bar tile, playable or not.
- Resting bar spacing is hand-tuned Arena-forward constants at the design window; one clamped viewport scale multiplies them for the live window. Hit height, raise translate, sticky inspect band, and drag play threshold all derive from `handMetrics(viewport)`, so they cannot drift apart from the painted faces.
- Canvas drag ghost strokes must use the dragged tile's **zone** (not hard-coded hand) when dragging command/gy/exile. Resting bar tiles still use `barZoneAura` CSS.

## Testing Decisions

- Scene/unit tests cover the hand bar, command/hand playable borders, unplayable no-dim behavior, drag-source opacity fade (no HTML `hand-drag-ghost`), spectator suppression, and a graveyard-section `activate` tile (Teacher's Pest–style self-return).
- Schema snapshot tests lock `ActionView.section == "graveyard"` for a `functions_in_graveyard` activate.
- Interaction checks should drag above and below the play threshold and assert commit versus cancel outcomes.
- Scene tests cover multi-mode hand activation entering `playModePick`, local-session exclusivity, `PlayModeChosen` continuation, the single-mode auto path, stale legality prune/cancel behavior, stale `PlayModeChosen` without intent, and Cancel restoring the parked hand card.
- Geometry lock in `handBarHit.test.ts` asserts face/peek/visible/`HAND_BAR_H` targets at the design window so a silent regress to the old dense values fails.
- `hand-scale.test.ts` covers that the design window still yields the design sizes, that a 2560 x 1440 desktop grows the faces past a physical card's width, that a small laptop shrinks them, that both clamp ends hold, and that every derived length stays in step with the face.
- `hand.test.ts` locks hover elevate on `hand-tile-{id}`, asserts discard-selected does not add selection z elevate (`hover:[z-index:50]` remains for hover+selected), and locks `data-selected` / `group-data-[selected=true]/hand-tile:ring-llanowar` pick chrome. It also locks the art chrome contract: `data-playable` / `data-drag-source` on the tile root, the hover-brighten and drag-fade variant tokens on the face, and no bare ternary `opacity-25`.

## Out of Scope

- Showing non-action graveyard/exile inventory in the bar.
- Reintroducing unplayable hand darkening under another class name.
- Moving the hand bar into the canvas layer.

## Further Notes

- Zone pile expansion is handled separately by `PileOverlay`.
- Flights suppress duplicate hand and resting battlefield faces through `hideCardIds`, `flightOwnedIds`, and `handHidden`. Stack faces hide only for `kind: "stack"` flights (see stack spec).
- The play-mode behavior follows the local chooser design in [hand-play-mode-chooser-design](2026-07-26-hand-play-mode-chooser-design.md).
