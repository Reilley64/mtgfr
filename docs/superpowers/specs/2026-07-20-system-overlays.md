# System Overlays
**Status:** Current (as of 2026-07-26)
**Module:** `client/app/board/html/overlays.ts`, `client/app/board/html/result-overlay.ts`, `client/app/board/html/concede.ts`, `client/app/board/html/pile-overlay.ts`, `client/app/board/view.ts`

## Problem Statement

The board needs system-level overlays for game results, concede confirmation, pile expansion, and reconnect state without interfering with the core hand, prompt, HUD, and inspect layers.

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
- Result overlay root uses `pointer-events-auto` so Stay/Leave receive clicks under the board overlays `pointer-events-none` layer.
- Concede is a top-right button for active seated players.
- Concede confirmation submits a real `concede` intent only after confirmation.
- `PileOverlay` opens for non-battlefield zone piles, shows an art grid, and closes by backdrop, Close, or Escape. When `selectableIds` is set, thumbs carry `data-selectable` / `data-selected` and paint Island blue / Priority Gold rings via Tailwind `data-[selected=…]` utilities.
- Reconnect banner appears fixed top-center when the stream is disconnected. A transient disconnect says `Connection lost — reconnecting…`. Terminal stream failures use specific copy: 401 says the session expired and asks the player to sign in again; 404 says the table is no longer available. The banner keeps `data-testid="board-reconnecting"` for all reconnect states.
- Inspect renders above result, concede, pile, HUD, and prompts.

## Implementation Decisions

- System overlays remain DOM, not canvas.
- Concede is game action chrome, not navigation.
- Pile overlay uses `cardArt(h, opts)` for card thumbnails and falls back to card names when art is unavailable. Selectable thumbs follow the AGENTS.md `data-selected` / `data-selectable` Tailwind pattern.
- Escape priority dismisses inspect, the activation menu, stack expansion, and then local action/pile state.

## Testing Decisions

- Scene tests cover result overlay actions, concede confirm/cancel, pile overlay contents/close, and reconnect banner copy for transient and terminal stream states.
- Result overlay Scene coverage asserts `pointer-events-auto` so Stay/Leave remain hittable under the board overlays root.
- Board update tests cover `ConcedeConfirmed` submitting a `concede` intent.
- App update tests cover `LeaveGame` redirecting home.
- Layer tests should preserve inspect above all system overlays.

## Out of Scope

- Replacing result/concede/pile with a unified modal framework.
- Portrait reflow of the board; portrait CSS landscape rotate (no dialog) lives at the app root (see [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Showing private hidden pile cards to non-owners.

## Further Notes

- The authoritative board layer stack lives in `docs/client-canvas-map.md`.
