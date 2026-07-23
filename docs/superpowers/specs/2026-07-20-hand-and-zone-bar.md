# Hand and Zone Bar
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/hand.ts`, `client/app/board/html/hand-drag-mount.ts`, `client/app/board/geometry/handBarHit.ts`, `client/app/board/motion/flights.ts`

## Problem Statement

Players need a bottom bar that keeps their private hand usable, keeps command-zone actions visible, and exposes graveyard/exile actions without crowding the battlefield. Spectators and eliminated players must not see or interact with a hand.

## Solution

Render a fixed DOM hand bar at the bottom of the board. It groups tiles in Arena order: command, hand, graveyard, exile. Command and hand show owned visible cards; graveyard and exile show playable actions from those zones. The bar owns drag-to-play hit geometry and hand/command playable borders.

## User Stories

- As a player, I can see my command card, hand, and playable graveyard/exile options in one bottom bar.
- As a player, I can drag a playable hand tile upward to play it.
- As a player, I can tell castable cards by their playable border.
- As a spectator, I do not see another player’s hand or action controls.

## Behavior

- Hand tiles fan with dense overlap, hover raise, and cost pips above the card face.
- A release above `HAND_BAR_H - HAND_PLAY_SLACK_PX` commits the drop; releasing below snaps back.
- `hiddenId`, `hiddenIds`, and flight ownership suppress tiles while a staged play or flight owns the card.
- Playable hand/command tiles get the playable border from `barZoneAura(zone, playable)`.
- Unplayable hand/command tiles stay full brightness: no `brightness-[0.55]` or equivalent veil.
- The drag source fades with `opacity-25` while the drag ghost carries the face.
- Graveyard/exile bar tiles appear only for actions and use their zone outline colors when playable.
- Hand and priority controls render only for active seated players, not spectators or eliminated players.

## Implementation Decisions

- The bar is DOM, not canvas, so real buttons, keyboard activation, and drag data attributes stay available.
- `slotInert` is reserved for staged/in-flight cards; it is not a visual dimming signal for unplayable cards.
- `cardArt(h, opts)` is used for DOM faces and accepts optional `style` for precise tile sizing.
- Alt-inspect hover metadata is attached to every face-up bar tile, playable or not.

## Testing Decisions

- Scene/unit tests cover the hand bar, command/hand playable borders, unplayable no-dim behavior, drag-source opacity fade, and spectator suppression.
- Interaction checks should drag above and below the play threshold and assert commit versus cancel outcomes.

## Out of Scope

- Showing non-action graveyard/exile inventory in the bar.
- Reintroducing unplayable hand darkening under another class name.
- Moving the hand bar into the canvas layer.

## Further Notes

- Zone pile expansion is handled separately by `PileOverlay`.
- Flights suppress duplicate hand/stack/resting faces through `hideCardIds`, `flightOwnedIds`, and `handHidden`.
