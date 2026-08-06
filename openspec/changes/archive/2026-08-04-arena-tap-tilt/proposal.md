# Show a tapped permanent as a tilted, darkened square

## Why

The board turned a tapped permanent a quarter turn. That read on a tall card, but the Arena
permanent is square: a 90° turn leaves the same silhouette in the same footprint, so a tapped
permanent looked untapped. Arena tilts the tile a few degrees and darkens it instead — legible at
four-seat zoom, and unmistakable next to an upright neighbour.

## What Changes

- A tapped permanent tilts a few degrees off square rather than turning a quarter turn.
- A tapped permanent paints a black veil over its face; the veil fades in with the tap animation.
- Hit testing drops the sideways footprint it kept for the quarter turn — every rotation the board
  draws leaves a card centred on its upright rect.

## Capabilities

- **New Capabilities**: none
- **Modified Capabilities**: `game-board`

## Impact

- Client board: the tap angle and veil in `bitmap/paint-cards.ts`, the matching Scene rotation in
  `canvas/scene.ts`, and the footprint in `geometry/hit-test.ts`.

## Non-goals

- The tap glyph, auto-tap preview, and the tap animation's timing are unchanged.
