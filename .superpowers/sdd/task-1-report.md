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

## Review fix: stable param tokens

- Replaced `debug_param` / `debug_token` with explicit token helpers for filters, destinations, keywords, counters, colors, scopes, targets, and timing params used by `Effect::message`.
- Machine params now use stable snake_case tokens such as `first_strike`, `library_top`, `basic_land`, and `permanent_creature_you_control`.
- Added `message_params_use_snake_case_machine_tokens` to pin keyword, destination, card-filter, and permanent-filter tokens.
- Removed the stale unrelated route-report content.

### Review-fix TDD evidence

Red run:

```text
cargo nextest run --profile ci -p engine message_params_use_snake_case_machine_tokens
```

Result: failed as expected before the helper replacement.

```text
assertion `left == right` failed
  left: "firststrike"
 right: "first_strike"
```

Green/focused run:

```text
cargo nextest run --profile ci -p engine message_params_use_snake_case_machine_tokens
```

Result:

```text
Summary [   0.007s] 1 test run: 1 passed, 1938 skipped
```

Requested focused run:

```text
cargo nextest run --profile ci -p engine message_refs_are_stable reject_messages_use_reject_namespace
```

Result:

```text
Summary [   0.006s] 2 tests run: 2 passed, 1937 skipped
```

Broader engine run:

```text
cargo nextest run --profile ci -p engine
```

Result:

```text
Summary [  80.785s] 1939 tests run: 1939 passed, 0 skipped
```
