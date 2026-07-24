# Task 3 report: Remove deck-list hover preview

## Summary

- Removed the Your decks cursor-follow hover preview surface.
- Deleted `client/app/shell/decks/list/hover.ts`.
- Removed deck-list hover messages, submodel state, update handlers, view mount, and preview render.
- Preserved Play href, search, ordering, context menu, delete confirmation, and builder hover preview behavior.
- Updated the stale deck-list tile chooser spec line that still said to keep the list hover preview.

## TDD evidence

### RED

Command:

```bash
cd /workspace/client
bunx vitest run app/shell/decks/list/story.test.ts
```

Result:

- Exit code: `1`
- Expected failures:
  - `deck list does not render a hover preview` failed on unresolved `BindDeckListCommanderHover`.
  - The other deck-list story scenes also failed on unresolved `BindDeckListCommanderHover` mounts after the tests stopped resolving hover.

### GREEN

Command:

```bash
cd /workspace/client
bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts app/smoke.test.ts
```

Result:

- Exit code: `0`
- `3` files passed.
- `22` tests passed.

## Full verification

```bash
cd /workspace/client
just client-check
```

Result:

- Exit code: `0`
- Design token check passed.
- Codegen completed.
- Biome lint completed with one schema-version info message.
- TypeScript typecheck passed.
- Vitest passed: `85` files, `887` tests.
- `bunx vitest run app/shell/decks/list/story.test.ts app/shell/surfaces.test.ts app/smoke.test.ts` was rerun after restoring unrelated formatter churn and passed: `3` files, `22` tests.

## Notes

- The builder hover preview remains wired through `BindBuilderCardPointer`, `MovedBuilderHover`, `ClearedBuilderHover`, and `builder-hover-preview`.
- `just client-check` ran `biome format --write`, which formatted two unrelated files; those incidental changes were restored before final affected-suite verification.
