# Task 4 Report: Server Ack + forced auto_actions as MessageRef

## Status

Done.

## TDD evidence

Red:

- Added server tests for forced auto-action keys, engine reject keys, server-only reject keys in `MessageKey::all()`, and nested MessageRef proto mapping.
- `cargo nextest run --profile ci -p server a_forced_single_legal_target_choice_auto_resolves_without_a_client_intent engine_rejects_are_returned_as_message_refs delta_collapses_divided_damage_into_object_amount_rows stack_yield_rejects_disable_once_armed server_message_keys_are_in_the_closed_catalog`
- Result: failed to compile as expected because server/session/gRPC still used strings and `pb::Ack.reason`.

Green:

- Wired `schema::MessageRef` through `ApplyResult`, `DwellResult`, `PublishedDelta`, `Ack.reject_reason`, `stream::frame_for`, and gRPC DTO-to-proto mapping.
- Mapped engine rejects through `engine::reject_message`.
- Added forced auto-action keys and server-only reject keys to `engine::MessageKey::all()`.
- `cargo nextest run --profile ci -p server a_forced_single_legal_target_choice_auto_resolves_without_a_client_intent engine_rejects_are_returned_as_message_refs delta_collapses_divided_damage_into_object_amount_rows stack_yield_rejects_disable_once_armed server_message_keys_are_in_the_closed_catalog`
- Result: 5 tests passed.

## Verification

- `cargo fmt`
  - Passed.
- `cargo nextest run --profile ci -p server`
  - Passed: 164 tests.

## Notes

- `CatalogCard.summary` remains a proto string, so the server mapper joins summary message keys at that boundary to keep `-p server` compiling while Task 4's Ack/delta MessageRef proto fields are wired.
