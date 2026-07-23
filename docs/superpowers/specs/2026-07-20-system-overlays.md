# System Overlays
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/overlays.ts`, `client/app/board/html/result-overlay.ts`, `client/app/board/html/concede.ts`, `client/app/board/html/pile-overlay.ts`, `client/app/board/view.ts`

## Problem Statement

The board needs system-level overlays for game results, concede confirmation, pile expansion, reconnect state, and portrait gating without interfering with the core hand, prompt, HUD, and inspect layers.

## Solution

Compose system overlays in `boardOverlays` as DOM layers above the board surfaces. `ResultOverlay`, concede `ConfirmDialog`, and `PileOverlay` each own their own backdrop and controls. The inspect dock remains topmost when pinned.

## User Stories

- As an eliminated player, I can acknowledge the result and keep watching.
- As a player, I must confirm before conceding.
- As a player, I can expand graveyard or exile piles to inspect their cards.
- As a player on a disconnected stream, I see reconnect status.

## Behavior

- `ResultOverlay` appears for win, loss, elimination, or game-over outcomes until dismissed.
- Result actions are Watch/Stay on the board and Back to your decks.
- Concede is a top-right button for active seated players.
- Concede confirmation submits a real `concede` intent only after confirmation.
- `PileOverlay` opens for non-battlefield zone piles, shows an art grid, and closes by backdrop, Close, or Escape.
- Reconnect banner appears fixed top-center when the stream is disconnected.
- A portrait gate may exist as a system modal under the Landscape Rule; it stays below inspect when inspect is pinned.
- Inspect renders above result, concede, pile, HUD, and prompts.

## Implementation Decisions

- System overlays remain DOM, not canvas.
- Concede is game action chrome, not navigation.
- Pile overlay uses `cardArt(h, opts)` for card thumbnails and falls back to card names when art is unavailable.
- Escape priority dismisses inspect, radial, stack expansion, and then local action/pile state.

## Testing Decisions

- Scene tests cover result overlay actions, concede confirm/cancel, pile overlay contents/close, and reconnect banner.
- Board update tests cover `ConcedeConfirmed` submitting a `concede` intent.
- Layer tests should preserve inspect above all system overlays.

## Out of Scope

- Replacing result/concede/pile with a unified modal framework.
- Portrait reflow of the board; portrait gate is a rotate prompt.
- Showing private hidden pile cards to non-owners.

## Further Notes

- The authoritative board layer stack lives in `docs/client-canvas-map.md`.
