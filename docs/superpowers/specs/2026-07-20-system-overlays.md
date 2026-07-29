# System Overlays
**Status:** Current (as of 2026-07-28)
**Module:** `client/app/board/html/overlays.ts`, `client/app/board/html/result-overlay.ts`, `client/app/board/html/concede.ts`, `client/app/board/html/pile-overlay.ts`, `client/app/board/html/seen-hands.ts`, `client/app/board/view.ts`

## Problem Statement

The board needs system-level overlays for game results, concede confirmation, pile expansion, and reconnect state without interfering with the core hand, prompt, HUD, and inspect layers.

## Solution

Compose system overlays in `boardOverlays` as DOM layers above the board surfaces. The result overlay and the concede confirmation are `Dialog` submodels rendered through `modalDialog` / `confirmDialog` (see [ui-component-layer](2026-07-28-ui-component-layer.md)), so open/close, focus trap, Escape, backdrop click, and scroll lock come from `@foldkit/ui`. `PileOverlay` owns its own backdrop and controls. The inspect dock stays above the non-modal overlays.

## User Stories

- As an eliminated player, I can acknowledge the result and keep watching.
- As a player, I must confirm before conceding.
- As a player, I can expand graveyard or exile piles to inspect their cards.
- As a player who looked at an opponent's hand, I can reopen what I saw.
- As a player on a disconnected stream, I see reconnect status.

## Behavior

- The result overlay is raised once, by `raiseResultDialog` on the fold that first reports a win, loss, elimination, or game-over outcome for the viewer. Dismissing it keeps it down: `resultRaised` latches when it is raised, so later folds do not put it back up.
- Result actions are Stay on the board (Keep watching when the game continues without the viewer) and Back to your decks. Staying takes Dialog's close path, the same one as Escape and the backdrop; leaving dispatches `LeaveGame`.
- Both modals render their `<dialog>` at all times — a closed `<dialog>` is what Dialog opens — and carry `pointer-events-auto` so their controls receive clicks under the board overlays `pointer-events-none` layer.
- Concede is a top-right button for active seated players.
- Concede confirmation submits a real `concede` intent only after confirmation; Cancel, Escape, and the backdrop all dismiss it without one.
- `PileOverlay` opens for non-battlefield zone piles, shows an art grid, and closes by backdrop, Close, or Escape. When `selectableIds` is set, thumbs carry `data-selectable` / `data-selected` and paint Island blue / Priority Gold rings via Tailwind `data-[selected=…]` utilities. Its heading names the zone and count (`pile-overlay-title`).
- `seenHandsView` renders one `seen-hand-<seat>` chip per opponent whose hand cards this viewer's snapshot itemized — the looked-at hands of Glasses of Urza (CR 701.20), which are otherwise invisible because every other read of an opponent's hand is `hand_count`. Each chip reads `<name>'s hand (<count>)` and opens that hand in `PileOverlay`. The strip is absent when nothing has been looked at.
- Reconnect banner appears fixed top-center when the stream is disconnected. A transient disconnect says `Connection lost — reconnecting…`. Terminal stream failures use specific copy: 401 says the session expired and asks the player to sign in again; 404 says the table is no longer available. The banner keeps `data-testid="board-reconnecting"` for all reconnect states and carries `role="alert"` so the state change announces.
- Inspect renders above pile, HUD, and prompts. The result and concede modals sit above inspect: they are native `<dialog>` elements, which Dialog layers over the page while open.

## Implementation Decisions

- System overlays remain DOM, not canvas.
- Concede is game action chrome, not navigation.
- Prompt modals and the mulligan overlay stay hand-rolled rather than moving to `Dialog`. `Dialog` bundles Escape and backdrop-click close into the frame with no way to drop them, and a pending choice that can be dismissed leaves the player unable to answer it. See [prompts-and-pending-choices](2026-07-20-prompts-and-pending-choices.md).
- Pile overlay uses `cardArt(h, opts)` for card thumbnails and falls back to card names when art is unavailable. Selectable thumbs follow the AGENTS.md `data-selected` / `data-selectable` Tailwind pattern.
- Escape priority dismisses inspect, the activation menu, stack expansion, and then local action/pile state.

## Testing Decisions

- Scene tests cover result overlay actions, the concede confirmation's Cancel/Confirm, pile overlay contents/close, the looked-at-hand chip and its pile heading (present and absent), and reconnect banner copy for transient and terminal stream states. Modal Scene tests seed an open `Dialog.Model` (`Dialog.init({ id, isOpen: true })`).
- Result overlay Scene coverage asserts `pointer-events-auto` so Stay/Leave remain hittable under the board overlays root.
- Board update tests cover `ConcedeConfirmed` submitting a `concede` intent, dismissal submitting none, and the result overlay coming up once and staying down after it is dismissed.
- App update tests cover `LeaveGame` redirecting home.
- Layer tests should preserve inspect above all system overlays.

## Out of Scope

- Moving the pile overlay onto `Dialog`; it is a zone inspector with its own selection chrome, not a confirm.
- Portrait reflow of the board; portrait CSS landscape rotate (no dialog) lives at the app root (see [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Showing private hidden pile cards to non-owners.

## Further Notes

- The authoritative board layer stack lives in `docs/client-canvas-map.md`.
