# Task 1 Report: Play routes require deckId path param

## Status

DONE

## Summary

- Added `client/app/deck-id.ts` with `parseDeckIdParam(raw: string): number | null`.
- Added `deckCardViewTransitionName(deckId: number): string`.
- Updated `PlayRoute` to require `{ deckId: string }` and build `/play/:deckId`.
- Updated `TableRoute` to require `{ deckId: string, table: string }` and build `/play/:deckId/:table`.
- Ordered `tableRouter` before `playRouter` so `/play/:deckId/:table` parses as `TableRoute`.
- Updated existing `PlayRoute()` and `TableRoute({ table })` call sites to pass `deckId` without implementing later lobby binding or view-transition behavior.

## TDD Evidence

Red run:

```text
cd client && bun test app/deck-id.test.ts app/routes.test.ts
```

Expected failures were observed before production changes:

- `client/app/deck-id.test.ts` could not import missing `./deck-id`.
- `/play/7` parsed as the old `TableRoute`.
- Bare `/play` still parsed as the old `PlayRoute`.
- `routePath(PlayRoute({ deckId: "7" }))` still returned `/play`.

Green run:

```text
cd client && bun test app/deck-id.test.ts app/routes.test.ts app/smoke.test.ts
```

Result: 18 pass, 0 fail.

Final focused verification:

```text
cd client && bun test app/deck-id.test.ts app/routes.test.ts app/smoke.test.ts app/shell/lobby/entry.test.ts app/shell/lobby/story.test.ts app/shell/surfaces.test.ts app/game/story.test.ts
```

Result: 39 pass, 0 fail.

Compile verification:

```text
cd client && bun run typecheck
```

Result: exit 0.

## Self-Review

- Scope matches Task 1: route shapes, `parseDeckIdParam`, and `deckCardViewTransitionName`.
- Did not add lobby UI, selectedDeckId path binding, route normalization, or CSS view transitions.
- Kept existing query-string selected-deck behavior intact where tests already covered it by adding the required path segment and preserving `?deck=`.
- Left non-integer `/play/:deckId` normalization for Task 2, per the brief note.
- No feature spec update was needed because this task is a narrow route/helper change in an implementation plan sequence.

## Concerns

- The top-level nav `Play` link now uses `PlayRoute({ deckId: "0" })` as a temporary constructor value because the final lobby UI deck binding is explicitly deferred to later tasks.
