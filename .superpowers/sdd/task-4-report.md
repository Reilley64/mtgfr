# Task 4 Report: Right-click context menu

## Status

Done.

## TDD evidence

Red:

- Added Scene tests for the owned deck menu and delete confirmation path before production code.
- `cd client && bunx vitest run app/shell/decks/list/story.test.ts`
- Result: failed as expected because `BindDeckListContextMenu` / menu UI did not exist yet.

Green:

- Implemented `BindDeckListContextMenu`, attached it to deck tiles, rendered the Edit/Delete menu overlay, and updated Scene mount lifecycles.
- `cd client && bunx vitest run app/shell/decks/list/story.test.ts`
- Result: 4 tests passed.

## Verification

- `cd client && bun run lint`
  - Passed. Biome reported the existing schema-version info for `biome.json`.
- `cd client && bun run typecheck`
  - Passed.
- `cd client && bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts app/shell/decks/list/update.search.test.ts app/shell/decks/list/visible.test.ts`
  - Passed: 4 files, 25 tests.

## Notes

- `just` was unavailable in this container, so client scripts were used directly.
- The delete confirm Scene clicks `deck-list-menu-delete` directly; this is stricter than dispatching the message and keeps the Scene test type-safe.
