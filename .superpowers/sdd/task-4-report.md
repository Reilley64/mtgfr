# Task 4 Report

Status: Complete. `schemars::schema_for!(CardToml)` compiles under `card-schema`, and the schema text includes `"name"` and `damage`.

Commit: `feat(engine): jsonschema for cardtoml surface`

Tests:
- RED: `cargo test -p engine --features card-schema card_toml_schema_includes_name_and_damage_effect` failed with missing `CardToml: JsonSchema`.
- GREEN: `cargo test -p engine --features card-schema card_toml_schema_includes_name_and_damage_effect`
- VERIFY: `cargo test -p engine --features card-schema --test de`
- VERIFY: `cargo clippy -p engine --features card-schema --tests -- -D warnings`
- VERIFY: `cargo fmt --check && git diff --check && git status --short`

Concerns / WAVE_C escapes:
- Effect payloads use a schema catch-all after the authored `type` tag.
- Ability schema still treats activation sacrifice, `Amount`, `CounterKind`, `Condition`, `PermanentFilter`, and `SpellFilter` as opaque schema values.
- CardToml schema still treats `Condition`, `PermanentFilter`, `AlternativeCost`, `SacrificeCost`, `CumulativeUpkeepCost`, `EscapeCost`, and `EnterAsCopy` as opaque schema values.
- No CR comments moved; `just engine-cr-index` was not needed.
