# Task 3 Report: Schema DTOs + projection hard cut

Status: DONE_WITH_CONCERNS

## Changes

- Added schema-owned `MessageRef` / `MessageParam` DTOs with proto-shaped serde fields.
- Added a single engine-to-schema message mapper in `crates/schema/src/message.rs`.
- Replaced in-scope schema labels with message refs:
  - `StackObjectView.label`
  - `ActionView.label`
  - `ModeView.label`
  - pending-choice `label` / `labels`
  - `DeltaEnvelope.auto_actions`
- Changed catalog `summary` from joined English prose to `Vec<MessageRef>` so catalog tests still assert keyword/effect meaning.
- Kept `ChoiceItem.label` as `String` for card/seat identity labels.

## TDD evidence

- RED: `cargo nextest run --profile ci -p schema pay_cost_projects_the_paid_effect_label`
  - Failed on the old `String` label shape and remaining `.label()` projection calls.
- GREEN: `cargo nextest run --profile ci -p schema`
  - 76 tests passed.
- Final verification after formatting: `cargo fmt && cargo nextest run --profile ci -p schema`
  - 76 tests passed.

## Concerns / handoff

- Server/client are expected to remain broken until Tasks 4-6 update gRPC mapping and client formatting.
- Schema uses owned DTO-only keys for action chrome and catalog keywords (`action.*`, `card.name`, `keyword.*`, `choice.option`) because engine `MessageKey` is closed.
- `choice.option` carries vote option text as a `name` param for now; a later engine-owned vote option message key would make that stricter.
