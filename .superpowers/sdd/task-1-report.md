# Task 1 Report: Proto + schema DTOs + gRPC map

**Status:** DONE_WITH_CONCERNS
**Branch:** `cursor/proxy-card-art-cd62`
**Base:** `693bd3b`

## What implemented

- Added expand-only proto fields with the exact task-brief field numbers:
  - `DeckCardEntry.proxy_art_url = 4`
  - `DeckDetail.commander_proxy_art_url = 6`
  - `SaveDeckRequest.commander_proxy_art_url = 5`
  - `ObjectView.proxy_art_url = 28`
  - `StackObjectView.proxy_art_url = 11`
  - `ChoiceItem.proxy_art_url = 5`
- Added the matching schema DTO fields in `crates/schema/src/dto.rs` with:
  - `#[serde(default, skip_serializing_if = "String::is_empty")]`
  - empty string as the absent/legacy value
- Updated gRPC mapping:
  - catalog deck entry/detail/save request mapping copies the new fields
  - stream object/stack/choice-item mapping copies the new fields
- Fixed all affected Rust literal sites to provide `String::new()` for the new optional fields where Task 1 does not yet source real values.
- Ran `just server-codegen` so the generated wire clients match the proto expansion.

## TDD evidence

### RED

Command:

```bash
cargo nextest run --profile ci deck_card_entry_round_trips_proxy_art_url
```

Observed failure before implementation:

```text
error[E0560]: struct `dto::DeckCardEntry` has no field named `proxy_art_url`
error[E0609]: no field `proxy_art_url` on type `dto::DeckCardEntry`
```

This was the expected missing-field failure from the brief.

### GREEN

Command:

```bash
cargo nextest run --profile ci deck_card_entry_round_trips_proxy_art_url
```

Observed pass after implementation:

```text
PASS [   0.004s] (1/1) schema dto::tests::deck_card_entry_round_trips_proxy_art_url
Summary [   0.007s] 1 test run: 1 passed, 2392 skipped
```

## Verification commands and results

### Codegen

```bash
just server-codegen
```

Result: passed; `cd client && bun run gen` completed successfully.

### Focused tests

```bash
cargo nextest run --profile ci deck_card_entry_round_trips_proxy_art_url
cargo nextest run --profile ci decks_round_trip_create_list_get_update_delete
cargo nextest run --profile ci rich_snapshot_preserves_choice_actions_and_oneof_kinds
```

Result:

- `schema dto::tests::deck_card_entry_round_trips_proxy_art_url` passed
- `server grpc::tests::decks_round_trip_create_list_get_update_delete` passed
- `server grpc::map::stream::tests::rich_snapshot_preserves_choice_actions_and_oneof_kinds` passed

### Compile check

```bash
cargo check -p schema -p server
```

Result:

```text
Checking schema v0.1.0 (/workspace/crates/schema)
Checking server v0.1.0 (/workspace/crates/server)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.32s
```

## Files changed

- `proto/mtgfr/v1/catalog.proto`
- `proto/mtgfr/v1/stream.proto`
- `crates/schema/src/dto.rs`
- `crates/schema/src/projection/choice.rs`
- `crates/schema/src/snapshot.rs`
- `crates/schema/src/answer_protocol.rs`
- `crates/server/src/grpc/map/catalog.rs`
- `crates/server/src/grpc/map/stream.rs`
- `crates/server/src/grpc/tests.rs`
- `crates/server/src/test_support.rs`
- `crates/server/src/legality.rs`
- `crates/server/src/decks_api.rs`
- `crates/server/src/precons.rs`

## Self-review

- Field numbers match the task brief exactly.
- The wire change is expand-only and uses empty-string-as-absent consistently.
- The new DTO fields are backward-compatible for legacy JSON via `serde(default)`.
- Catalog deck JSON now round-trips `DeckCardEntry.proxy_art_url` and still deserializes old rows without the field.
- Stream/catalog mappers copy the new fields where Task 1 owns the mapping layer.
- Non-Task-1 behavior is intentionally unchanged: projection/literal sites use `String::new()` until later tasks add real sourcing.

## Concerns

- `commander_proxy_art_url` is present on the wire/DTO contract, but Task 1 intentionally does not persist it yet; DB-backed `DeckDetail` still returns an empty string for that field until later tasks add storage.
- I did not update living feature specs in this task because Task 1 lands only partial contract scaffolding, not the full persisted/visible behavior those specs would describe.

## Review follow-up: non-empty gRPC map assertions

- Extended `crates/server/src/grpc/map/catalog.rs` tests so non-empty `proxy_art_url` / `commander_proxy_art_url` are asserted across:
  - `DeckCardEntry -> pb::DeckCardEntry`
  - `DeckDetail -> pb::DeckDetail`
  - `pb::SaveDeckRequest -> SaveDeckRequest`
- Extended `crates/server/src/grpc/map/stream.rs`'s rich snapshot test to assert non-empty `proxy_art_url` survives for:
  - `ObjectView -> pb::ObjectView`
  - `StackObjectView -> pb::StackObjectView`
  - `ChoiceItem -> pb::ChoiceItem`

### Covering tests rerun

```bash
cargo nextest run --profile ci proxy_art_url
cargo nextest run --profile ci rich_snapshot_preserves_choice_actions_and_oneof_kinds
```

Result:

- `schema dto::tests::deck_card_entry_round_trips_proxy_art_url` passed
- `server grpc::map::catalog::tests::deck_card_entry_to_pb_preserves_proxy_art_url` passed
- `server grpc::map::catalog::tests::save_deck_request_from_pb_preserves_card_and_commander_proxy_art_urls` passed
- `server grpc::map::catalog::tests::deck_detail_to_pb_preserves_card_and_commander_proxy_art_urls` passed
- `server grpc::map::stream::tests::rich_snapshot_preserves_choice_actions_and_oneof_kinds` passed
