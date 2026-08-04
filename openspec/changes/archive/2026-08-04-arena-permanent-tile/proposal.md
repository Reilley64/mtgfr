# Arena-style permanent tile

## Why

A battlefield of nine-tall printed card images reads as a wall of text nobody can parse at a
glance: the mana cost, type line, rules text, flavour, and collector line are all still there,
all illegible at board zoom, and the art — the only part a player actually recognises — is a
narrow band in the middle. Commander is four seats; at 1440×900 a printed card is 57px wide.

## What Changes

- Resting battlefield permanents render as a **square art-first tile**: a rendered card face
  (real frame assets, the card's own name and art drawn into the frame's slots), not a crop of
  the printed image.
- The resting footprint becomes square; a card in motion (drag ghost, flight) keeps the
  card-shaped footprint, so the square is what a permanent settles into.
- A token draws no name and an arched top; a legendary permanent draws the legend crown.
- The tile deliberately omits the printed mana cost (the hand's pip tray owns cost) and the
  printed P/T plate (the live P/T badge already paints over the tile).
- Object views on the wire carry the token, legendary, and colour facts the face needs. Legendary
  and colours are face-down redacted; the token flag is not, since a card back looks the same
  either way.

## Capabilities

- **New Capabilities**: none
- **Modified Capabilities**: `game-board`, `wire-protocol`

## Impact

- Client board: world-space card geometry, the Mount bitmap layer's card paint, and a card-face
  render cache keyed on the drawn facts.
- Wire: three additive `ObjectView` fields; no breaking change.
- Card inspect, the hand bar, the stack overlay, and the deck builder keep the printed image.

## Non-goals

- The hand-bar and stack card faces — a later slice.
- Borderless / showcase / retro / textless treatments, and non-`normal` layouts (saga,
  planeswalker, split, adventure, battle, DFC backs).
