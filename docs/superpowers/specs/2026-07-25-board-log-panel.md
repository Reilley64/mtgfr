# Board Log Panel
**Status:** Current (as of 2026-07-25)
**Module:** `client/app/board/html/log-panel.ts`, `client/app/board/html/overlays.ts`, `client/app/game/fold.ts`

## Problem Statement

Players need a readable history of recent game events on the board without a full chat surface or a second rules engine. The default view stays compact during play, but players can expand or copy the full client fold buffer when they need older context. Server auto-actions and the viewer's own draws should be visually distinct from ordinary fold lines.

## Solution

Compose a fixed DOM log panel above the hand bar (left column) from `GameFoldState.log`. Deltas append `LogLine` entries in `applyDeltaPure`; the panel hides entirely when the log is empty. With entries present, the panel shows a toolbar for expand/collapse and copy, paints the last 30 lines by default, and can expand to the full in-memory fold buffer.

## User Stories

- As a player, I can skim recent game events without leaving the board.
- As a player, I can expand the log to inspect older entries still retained in the fold buffer.
- As a player, I can copy the full fold buffer for sharing or debugging.
- As a player, I can tell auto-submitted or auto-draw lines apart from ordinary event lines.
- As a spectator, I see the same public fold narration the seated viewers see (no private hand/library text).

## Behavior

- `logPanelView(board, log)` returns `null` when `log.length === 0`.
- Otherwise it renders `data-testid="board-log-toolbar"` plus `data-testid="board-log"` with `role="log"` and `aria-live="polite"`.
- Collapsed mode paints only the last 30 lines (`LOG_VISIBLE = 30`); older lines remain in fold state up to the fold cap.
- Expanded mode paints every line currently retained in `GameFoldState.log`.
- `board-log-expand` toggles `BoardModel.logExpanded`; toggling clears copy feedback.
- `board-log-copy` emits `CopyBoardLog` with all `log` line text joined by `\n`, not only the visible slice.
- Copy feedback is local board UI state: successful copies show `Copied`; failed clipboard writes show `Copy failed`; a new copy attempt clears prior feedback until completion.
- `GameFoldState.log` keeps at most the last 200 lines (`applyDeltaPure` slices `-200` when appending).
- Lines with `auto: true` show an **AUTO** chip (`bg-auto-moss`) plus the line text in snow-mint caption styling.
- Ordinary lines use mist caption text with no chip.
- Panel geometry: fixed bottom-left above the hand bar (`--b: HAND_BAR_H + 10px`), scrollable, `rounded-hud` / `bg-forest-hud` / `shadow-hud`. Collapsed height is `max-h-[150px]`; expanded height is `max-h-[min(40vh,420px)]`.
- Composition: `boardOverlays` includes `logPanelView(board, log)` with other HUD overlays (below inspect; see `docs/client-canvas-map.md`).

### Log line sources (`client/app/game/fold.ts`)

- For each delta event, `describe(event, state)` may produce a text line (`event-fold`).
- The viewer's own `card_drawn` events become auto lines: `Drew {card}` or `Drew a card`.
- Each `delta.auto_actions` `MessageRef` becomes an auto line via `formatMessage`.
- Snapshots do not clear or rewrite the log; only deltas append.

## Implementation Decisions

- Narration stays client-side fold text (`describe` / `formatMessage`); the panel does not invent engine rules text.
- The panel is DOM HUD, not canvas, so overflow scroll and `aria-live` work natively.
- Auto styling is driven only by `LogLine.auto`, not by parsing the text string.
- Expansion and copy feedback live on `BoardModel` (`logExpanded`, `logCopied`, plus local failure feedback) because they are viewer-local board UI state.
- Clipboard access is a Foldkit `Command` (`CopyBoardLog`) built with `navigator.clipboard.writeText` through Effect, mirroring lobby table-code copy.

## Testing Decisions

- Scene tests assert `board-log` presence with an AUTO chip and ordinary line text when the fold log is non-empty.
- Scene tests assert collapsed mode omits the 31st-oldest line while keeping the newest line.
- Scene tests click `board-log-expand` and assert an older retained fold line becomes visible.
- Scene tests assert `board-log-copy` and `board-log-toolbar` render when the log is non-empty.
- Board update tests assert `LogCopyRequested` emits `CopyBoardLog` with every retained line joined by newline.
- Scene tests assert `board-log`, toolbar, and copy controls are absent when the log is empty.
- Fold unit tests cover append / slice behavior for log lines where present under `client/app/game/`.

## Out of Scope

- Chat, whispers, or player-authored messages.
- Server-side replay, chat transcript export, or persisted history beyond the client fold buffer.
- Localizing fold `describe()` English strings (MessageRef covers `auto_actions` and other keyed labels; event-fold narration is separate).

## Further Notes

- Wire stream batches carry `VisibleEvent`s “for the game log” (wire-protocol-and-visibility); this spec owns the client surface that paints them.
- Hand-bar height (`HAND_BAR_H`) is defined in the hand-and-zone-bar spec; the log panel sits `10px` above that bar.
