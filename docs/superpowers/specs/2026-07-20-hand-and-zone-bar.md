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
- Hand tiles fan with Arena-forward resting geometry (`HAND_FACE_W` 208, `HAND_BAR_PEEK` 92, `HAND_VISIBLE_H` 178, derived `HAND_BAR_H` 218 — pip-row 24 + bar bottom padding 16 are already implied by that height), hover raise, and cost pips above the card face.
- Resting cost pips show the card's printed cast cost, not cycle or hand-ability costs. Multi-legal-mode tiles omit Cycle/Discard captions; a sole legal `cycle` or `activate_hand_ability` keeps that caption.
- Hovering a bar tile elevates that tile's root above all other action-bar tiles (`[z-index:var(--hand-z)]` resting + `hover:[z-index:50]` on the slot; resting z is not inline). Discard-selected (and the same chrome for pending hand puts / face-down cast) raises and rings Llanowar but does **not** get hover bring-to-front (`hover:[z-index:50]` is omitted while selected); legal unselected choices ring Island blue; non-choices stay off that target chrome.
- A release above `HAND_BAR_H - HAND_PLAY_SLACK_PX` commits the drop (`HAND_PLAY_SLACK_PX` is 96); releasing below snaps back.
- Activating a hand tile with exactly one legal mode runs the existing play/cost/target pipeline immediately. With two or more legal modes, activation clears other local action sessions, seeds a stack flight, parks the card in local `playModePick` state, and opens docked `play-mode-aim` until `PlayModeChosen` continues the selected action through the same cost/target pipeline or Cancel restores the card.
- Design: [`2026-07-26-hand-play-mode-chooser-design.md`](2026-07-26-hand-play-mode-chooser-design.md).
- While `playModePick` is open, snapshot and delta sync reconcile the parked modes against current legal actions. Pruned modes disappear; exactly one remaining mode auto-continues through the same play/cost/target pipeline; zero remaining modes cancel the session, return the card to hand, and submit no intent.
- `hiddenId`, `hiddenIds`, and flight ownership suppress tiles while a staged play or flight owns the card.
- Playable hand/command tiles get the playable border from `barZoneAura(zone, playable)`.
- Unplayable hand/command tiles stay full brightness: no `brightness-[0.55]` or equivalent veil.
- Resting faces use `--shadow-hand` (`shadow-hand`). The drag source fades with `opacity-25` and loses playable aura. The drag **ghost** is painted on the Mount flight / screen-motion layer (`DragGhost` via `screen-motion.ts`), not as HTML — shared lift shadow and zone playable strokes, continuous with the flight that seeds on release. Idle hits use `cursor-grab` when playable and `cursor-not-allowed` otherwise; an active drag sets `cursor-grabbing` on the document element.
- Graveyard/exile bar tiles appear only for actions and use their zone outline colors when playable.
- Graveyard-section actions include casts from that zone (flashback/escape/retrace), encore, and activated abilities whose source is in the graveyard (`functions_in_graveyard` — Teacher's Pest's `{B}{G}` self-return). Wire `ActionView.section` is `"graveyard"` for those activates so `bySection` buckets them here rather than the battlefield radial.
- Hand and priority controls render only for active seated players, not spectators or eliminated players.

## Implementation Decisions

- The bar is DOM, not canvas, so real buttons, keyboard activation, and drag data attributes stay available.
- `slotInert` is reserved for staged/in-flight cards; it is not a visual dimming signal for unplayable cards.
- `cardArt(h, opts)` is used for DOM faces and accepts optional `style` for precise tile sizing.
- Alt-inspect hover metadata is attached to every face-up bar tile, playable or not.
- Resting bar spacing is hand-tuned Arena-forward constants (not a single global scale factor). Hit height, raise translate, sticky inspect band, and drag play threshold derive from those constants.
- Canvas drag ghost strokes must use the dragged tile's **zone** (not hard-coded hand) when dragging command/gy/exile. Resting bar tiles still use `barZoneAura` CSS.

## Testing Decisions

- Scene/unit tests cover the hand bar, command/hand playable borders, unplayable no-dim behavior, drag-source opacity fade (no HTML `hand-drag-ghost`), spectator suppression, and a graveyard-section `activate` tile (Teacher's Pest–style self-return).
- Schema snapshot tests lock `ActionView.section == "graveyard"` for a `functions_in_graveyard` activate.
- Interaction checks should drag above and below the play threshold and assert commit versus cancel outcomes.
- Scene tests cover multi-mode hand activation entering `playModePick`, local-session exclusivity, `PlayModeChosen` continuation, the single-mode auto path, stale legality prune/cancel behavior, stale `PlayModeChosen` without intent, and Cancel restoring the parked hand card.
- Geometry lock in `handBarHit.test.ts` asserts face/peek/visible/`HAND_BAR_H` targets so a silent regress to the old dense values fails.
- `hand.test.ts` locks hover elevate on `hand-tile-{id}` and asserts discard-selected omits `hover:[z-index:50]` (no bring-to-front while selected).

## Out of Scope

- Showing non-action graveyard/exile inventory in the bar.
- Reintroducing unplayable hand darkening under another class name.
- Moving the hand bar into the canvas layer.

## Further Notes

- Zone pile expansion is handled separately by `PileOverlay`.
- Flights suppress duplicate hand and resting battlefield faces through `hideCardIds`, `flightOwnedIds`, and `handHidden`. Stack faces hide only for `kind: "stack"` flights (see stack spec).
- The play-mode behavior follows the local chooser design in [hand-play-mode-chooser-design](2026-07-26-hand-play-mode-chooser-design.md).
