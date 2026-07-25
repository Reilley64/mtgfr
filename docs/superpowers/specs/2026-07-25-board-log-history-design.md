# Board Log History Design

## Goal

Make the existing board log useful as a short history tool without adding chat, server replay, or localized event narration. The feature exposes the full client fold buffer that already exists (`GameFoldState.log`, capped at 200 lines) while keeping the default board HUD compact.

## Chosen Approach

- Keep the log as a board HTML overlay fed by `GameFoldState.log`.
- Add viewer-local board state:
  - `logExpanded` toggles between the last 30 lines and the full fold buffer.
  - `logCopied` records successful clipboard copy feedback.
  - Failure feedback stays local to the same toolbar so players can see clipboard denial without affecting game state.
- Add board messages for toggle, copy request, and copy completion.
- Implement clipboard writes as a Foldkit `Command` using `navigator.clipboard.writeText`, matching the lobby copy command pattern.

## UI

The toolbar sits above `board-log` and contains:

- `board-log-expand`: `Expand` in collapsed mode, `Collapse` in expanded mode.
- `board-log-copy`: `Copy`, `Copied`, or `Copy failed` depending on local feedback state.

Collapsed mode keeps the current `max-h-[150px]` recent-log panel. Expanded mode uses `max-h-[min(40vh,420px)]` and renders every retained fold line.

## Scope Boundaries

Copying exports only the current client fold buffer text joined by newline. It does not add player chat, durable replay, server history fetches, or MessageRef localization for existing English fold descriptions.

## Tests

Scene tests cover collapsed slicing, expand reveal, copy control presence, and empty-log absence. A board update test covers the full-buffer payload emitted by `LogCopyRequested`.
