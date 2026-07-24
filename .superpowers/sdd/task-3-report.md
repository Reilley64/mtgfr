# Task 3 report: Tile grid + search UI

## Summary

- Replaced deck-list rows with a searchable tile grid in `client/app/shell/decks/list/view.ts`.
- Added `deck-list-search` and `deck-tile-${deck.id}` Scene coverage.
- Tile links use `/play?deck={id}`.
- Removed always-visible Play/Edit/Delete row actions from the deck tile surface.
- Kept the existing delete confirmation dialog and commander hover preview Mount behavior.

## TDD evidence

### RED

Command:

```bash
cd /workspace/client
bunx vitest run app/shell/surfaces.test.ts app/shell/decks/list/story.test.ts
```

Result:

- Exit code: `1`
- Expected failures:
  - `deck-list-search` was missing.
  - `deck-tile-1[href="/play?deck=1"]` was missing.

### GREEN

Command:

```bash
cd /workspace/client
bunx vitest run app/shell/surfaces.test.ts app/shell/decks/list/story.test.ts
```

Result:

- Exit code: `0`
- `2` files passed.
- `12` tests passed.

## Full verification

`just client-check` could not run because `just` is not installed in this cloud image (`just: command not found`).

Equivalent command run from `/workspace/client`:

```bash
bun run gen:tokens:check && bun run gen && bun run format && bun run lint && bun run typecheck && bun run test
```

Result:

- Exit code: `0`
- Design tokens check passed.
- Codegen completed.
- Biome format completed.
- Biome lint completed with one existing schema-version info message.
- TypeScript typecheck passed.
- Vitest passed: `83` files, `862` tests.

## Notes

- The current Foldkit Scene typings rejected `Story.message(...)` as a `Scene.scene` step even though it ran at runtime. The search UI story uses `Scene.type(...)` on `[data-testid="deck-list-search"]` instead, which exercises the real `OnInput` path and filters rendered tiles.
- `visibleDecks` now accepts the minimal readonly commander shape it reads, so schema-derived readonly app state typechecks cleanly.
