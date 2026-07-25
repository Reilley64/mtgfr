# Win #3 report: board log history

## Status

Implemented usable board game history on `cursor/board-log-history-a822`.

## Changes

- Added collapsed/expanded board log state on `BoardModel`.
- Added board log toolbar controls: `board-log-expand`, `board-log-copy`, and `board-log-toolbar`.
- Collapsed log paints the last 30 lines at `max-h-[150px]`.
- Expanded log paints the retained fold buffer at `max-h-[min(40vh,420px)]`.
- Added `CopyBoardLog` Foldkit command using `navigator.clipboard.writeText` through Effect.
- Copy uses every line in `GameFoldState.log`, joined by newline.
- Added local `Copied` and `Copy failed` feedback.
- Updated the board log panel spec and added the short board log history design doc.

## Verification

- RED: `cd /workspace/client && bun run test app/board/scene.test.ts -t "log panel"`
  - Failed on missing `board-log-expand` and `board-log-toolbar`.
- GREEN/final: `cd /workspace/client && bun run lint && bun run typecheck && bun run test app/board/scene.test.ts app/board/html/surfaces.test.ts`
  - Biome lint: 277 files checked, no fixes applied.
  - TypeScript: `tsc --noEmit` passed.
  - Vitest: 2 files passed, 157 tests passed.
