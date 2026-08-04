# Close the Arena tile's border, keep the zone piles printed

## Why

Two gaps in the square permanent tile, both found on the live board:

- The tile drew only the frame's top strip, so its border ran across the top and part-way down
  the sides, then stopped — the art bled to the bottom edge and the tile read as unfinished.
- The zone column (library, graveyard, exile, commander) takes its card size from the battlefield
  card, so those piles went square with it and started drawing the Arena face. A pile is a stack
  of cards, not a permanent; it should still look like a card.

## What Changes

- The rendered square's frame borders all four edges, sourced from the frame asset's matching
  edges so the ring keeps the card's own colour.
- Zone-column piles keep the printed card's proportions and keep painting the printed card image.

## Capabilities

- **New Capabilities**: none
- **Modified Capabilities**: `game-board`

## Impact

- Client board: `card-render/frame.ts` slot geometry, `geometry/layout.ts` zone-column size, and
  the battlefield-only guard in `bitmap/paint-cards.ts`.

## Non-goals

- Tokens keep their art-only face and arched top; the arch is a paint-time clip a border ring
  would have to follow.
