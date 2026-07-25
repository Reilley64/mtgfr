# Board Log Panel
**Status:** Current (as of 2026-07-25)
**Module:** `client/app/board/html/log-panel.ts`, `client/app/board/html/overlays.ts`, `client/app/game/fold.ts`

## Problem Statement

Players need a short, readable history of recent game events on the board without a full chat surface or a second rules engine. Server auto-actions and the viewer's own draws should be visually distinct from ordinary fold lines.

## Solution

Compose a fixed DOM log panel above the hand bar (left column) from `GameFoldState.log`. Deltas append `LogLine` entries in `applyDeltaPure`; the panel shows the last 30 lines in a Hud surface and hides entirely when the log is empty.

## User Stories

- As a player, I can skim recent game events without leaving the board.
- As a player, I can tell auto-submitted or auto-draw lines apart from ordinary event lines.
- As a spectator, I see the same public fold narration the seated viewers see (no private hand/library text).

## Behavior

- `logPanelView(log)` returns `null` when `log.length === 0`.
- Otherwise it renders `data-testid="board-log"` with `role="log"` and `aria-live="polite"`.
- Only the last 30 lines are painted (`LOG_VISIBLE = 30`); older lines remain in fold state up to the fold cap.
- `GameFoldState.log` keeps at most the last 200 lines (`applyDeltaPure` slices `-200` when appending).
- Lines with `auto: true` show an **AUTO** chip (`bg-auto-moss`) plus the line text in snow-mint caption styling.
- Ordinary lines use mist caption text with no chip.
- Panel geometry: fixed bottom-left above the hand bar (`--b: HAND_BAR_H + 10px`), `max-h-[150px]`, scrollable, `rounded-hud` / `bg-forest-hud` / `shadow-hud`.
- Composition: `boardOverlays` includes `logPanelView(log)` with other HUD overlays (below inspect; see `docs/client-canvas-map.md`).

### Log line sources (`client/app/game/fold.ts`)

- For each delta event, `describe(event, state)` may produce a text line (`event-fold`).
- The viewer's own `card_drawn` events become auto lines: `Drew {card}` or `Drew a card`.
- Each `delta.auto_actions` `MessageRef` becomes an auto line via `formatMessage`.
- Snapshots do not clear or rewrite the log; only deltas append.

## Implementation Decisions

- Narration stays client-side fold text (`describe` / `formatMessage`); the panel does not invent engine rules text.
- The panel is DOM HUD, not canvas, so overflow scroll and `aria-live` work natively.
- Auto styling is driven only by `LogLine.auto`, not by parsing the text string.

## Testing Decisions

- Scene tests assert `board-log` presence with an AUTO chip and ordinary line text when the fold log is non-empty.
- Scene tests assert `board-log` is absent when the log is empty.
- Fold unit tests cover append / slice behavior for log lines where present under `client/app/game/`.

## Out of Scope

- Chat, whispers, or player-authored messages.
- Full replay or export of the 200-line fold buffer.
- Localizing fold `describe()` English strings (MessageRef covers `auto_actions` and other keyed labels; event-fold narration is separate).

## Further Notes

- Wire stream batches carry `VisibleEvent`s “for the game log” (wire-protocol-and-visibility); this spec owns the client surface that paints them.
- Hand-bar height (`HAND_BAR_H`) is defined in the hand-and-zone-bar spec; the log panel sits `10px` above that bar.
