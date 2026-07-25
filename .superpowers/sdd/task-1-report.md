# Task 1 report: Playable border follows ghost + idle cursors

## Scope

Implemented Task 1 on branch `cursor/hand-drag-chrome-b23c`.

Included:

- `HandDragStarted.zone` message payload support
- `HandDragState.zone` submodel storage
- hand-bar drag payload zone inference from `data-bar-zone` / action section
- playable border moved off the drag source and onto the hand-drag ghost
- idle hit-strip cursors changed to `cursor-grab` for playable and `cursor-not-allowed` for unplayable

Explicitly not included:

- Task 2 grabbing cursor behavior
- Task 3 shadow work

## Files changed

- `client/app/board/messages.ts`
- `client/app/board/submodel.ts`
- `client/app/board/html/hand-drag-mount.ts`
- `client/app/board/html/hand.ts`
- `client/app/board/html/hand.test.ts`
- `client/app/board/html/hand-drag-mount.test.ts`

## Original Task 1 evidence

### Red

```bash
cd client && bunx vitest run app/board/html/hand.test.ts -t "drag chrome|not-allowed"
```

Observed failures:

- drag source still contained `ring-playable-border`
- unplayable hit strip still rendered `cursor-default`

### Green

```bash
cd client && bunx vitest run app/board/html/hand.test.ts app/board/hand-drag.test.ts app/board/html/surfaces.test.ts
```

Result: 3 files passed, 100 tests passed.

## Review fix append: hand drag zone coverage

### Finding addressed

`readHandDragPayload` zone resolution (`data-bar-zone` -> `action.section` -> `"hand"`) was untested, and the schema boundary still rejected non-hand zones even though the parser resolved them.

### Root cause

`readHandDragPayload` already computed the correct zone priority, but `HandDragStarted` used `S.Literal("hand", "command", "graveyard", "exile")`. In this codebase, multi-literal schemas must use `S.Union([S.Literal(...), ...])`, so `command` and `graveyard` payloads failed validation before the ghost ever received them.

### Red

Added focused tests first:

- `client/app/board/html/hand-drag-mount.test.ts`
- `client/app/board/html/hand.test.ts` command ghost aura assertion

Ran:

```bash
cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/html/hand.test.ts -t "readHandDragPayload|command-zone drag ghost"
```

Result: 2 failures in `hand-drag-mount.test.ts` with schema errors:

- `Expected "hand" | undefined, got "command"`
- `Expected "hand" | undefined, got "graveyard"`

### Green

Changed `client/app/board/messages.ts` to define `HandBarZone` as:

```ts
S.Union([S.Literal("hand"), S.Literal("command"), S.Literal("graveyard"), S.Literal("exile")])
```

This lets `HandDragStarted` carry the resolved bar zone for command / graveyard / exile ghosts.

### Verification

Focused rerun:

```bash
cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/html/hand.test.ts -t "readHandDragPayload|command-zone drag ghost"
```

Result: 2 files passed, 4 tests passed, 12 skipped.

Covering suites:

```bash
cd client && bunx vitest run app/board/html/hand-drag-mount.test.ts app/board/html/hand.test.ts app/board/hand-drag.test.ts
```

Result: 3 files passed, 24 tests passed.

Targeted static checks:

```bash
cd client && bunx tsc --noEmit -p tsconfig.json && bunx biome check app/board/messages.ts app/board/html/hand-drag-mount.test.ts app/board/html/hand.test.ts
```

Result: passed, no Biome fixes required.
