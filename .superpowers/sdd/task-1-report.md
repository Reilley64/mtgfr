# Task 1 report: Engine `MessageRef` + `Effect::message`

## Status

DONE

## Commit

- `b4e5106 feat(engine): replace Effect::label with MessageRef keys`

## Implementation summary

- Added `crates/engine/src/message.rs` with:
  - `MessageKey` closed constant set via `message_keys!`.
  - `MessageParam`, `MessageParamValue`, and `MessageRef`.
  - `Effect::message(self) -> MessageRef`.
  - `reject_message(Reject) -> MessageRef`.
  - `amount_param(name, Amount) -> MessageParam`.
- Deleted `crates/engine/src/label.rs`.
- Replaced engine `.label()` call sites with `.message()`.
- Changed `ModeInfo.label` from `String` to `MessageRef`.
- Updated engine tests that asserted label strings to assert message keys/params.

## TDD evidence

### RED

Command:

```bash
cargo nextest run --profile ci -p engine message_refs_are_stable reject_messages_use_reject_namespace
```

Result: failed for the expected missing API/types after adding the brief's tests first.

Key output:

```text
error[E0599]: no method named `message` found for enum `effect::Effect` in the current scope
error[E0425]: cannot find function `reject_message` in this scope
error[E0433]: cannot find type `MessageParamValue` in this scope
error: could not compile `engine` (lib test) due to 7 previous errors
```

### GREEN: focused tests

Command:

```bash
cargo nextest run --profile ci -p engine message_refs_are_stable reject_messages_use_reject_namespace
```

Result:

```text
Summary [   0.006s] 2 tests run: 2 passed, 1936 skipped
```

## Final verification

Commands:

```bash
cargo fmt -p engine
cargo nextest run --profile ci -p engine
```

Result:

```text
Summary [  81.216s] 1938 tests run: 1938 passed, 0 skipped
```

Additional checks:

```bash
git diff --check
rg "Effect::label|\.label\(|mod label" crates/engine
```

Results:

- `git diff --check` exited 0.
- `rg` found no remaining engine `.label()` / `Effect::label` / `mod label` references.

## Self-review notes

- `Effect::message` is exhaustive and uses no `_` catch-all arm.
- `Sequence`, `ChooseOne`, and `Conditional` carry child `MessageRef`s instead of joined English.
- Reject keys use the `reject.<snake_variant>` namespace.
- Amount params encode `Fixed` as `Int` and non-fixed amounts as `AmountToken` snake ids.
- During review, corrected `ZoneEffect::MassReturnFromGraveyard` to use its own `effect.zone_mass_return_from_graveyard` key.

## Concerns

None.
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
