# Task 2 Report: Promote CostToml with JsonSchema + fold into Cost

## Summary

- Added `engine::toml_surface::CostToml` and `XPips` in `crates/engine/src/toml_surface/cost.rs`.
- Added `card-schema = ["card-dsl", "dep:schemars", "dep:serde_json"]` to `crates/engine/Cargo.toml`.
- Switched `CardToml.cost` to `CostToml` and fold it into `Cost` in `From<CardToml> for CardDef`.
- Changed `Deserialize for Cost` to deserialize `CostToml`, validate hybrid pips, then fold via `From<CostToml> for Cost`.
- Added `JsonSchema` support for `CostToml`/`XPips` and `Color` under `card-schema`.

## TDD Evidence

### RED 1

- Added the brief's required test: `cost_toml_schema_lists_pip_keys`.
- Command: `cargo nextest run --profile ci -p engine --features card-schema cost_toml_schema`
- Result: failed because `engine::toml_surface::CostToml` did not exist.

### GREEN 1

- Implemented `CostToml`, schema feature/dependencies, export, and `Cost` folding.
- Command: `cargo nextest run --profile ci -p engine --features card-schema cost_toml_schema`
- Result: passed.

### RED 2

- Self-review found that `CardToml.cost: CostToml` could bypass the old `Cost` hybrid validation.
- Added regression: `card_toml_rejects_a_mono_hybrid_pair`.
- Command: `cargo nextest run --profile ci -p engine --features card-schema card_toml_rejects_a_mono_hybrid_pair`
- Result: failed because `CardDef` accepted `hybrid = [["red", "red"]]`.

### GREEN 2

- Added `CostToml::validate_hybrid` and `deserialize_cost_toml`; reused validation from `Deserialize for Cost`.
- Command: `cargo nextest run --profile ci -p engine --features card-schema card_toml_rejects_a_mono_hybrid_pair`
- Result: passed.

## Final Verification

- `cargo fmt`
- `cargo nextest run --profile ci -p engine --features card-schema cost_toml` — passed, 1 test.
- `cargo nextest run --profile ci -p cards` — passed, 16 tests.
- `cargo nextest run --profile ci -p engine --features card-schema card_toml_rejects_a_mono_hybrid_pair` — passed, 1 test.
- `git diff --check` — passed.

## Self-Review

- Confirmed lockfile churn is limited to `schemars` and its transitive dependencies.
- Preserved direct `Cost` deserialization behavior and restored top-level `CardToml` validation for mono hybrid pairs.
- `additional` and `reduce_own_generic` use broad `serde_json::Value` schema placeholders to avoid pulling the entire runtime DSL graph into this task's small cost-surface schema.

## Concerns

- `CostToml` schema exposes broad shapes for `additional` and `reduce_own_generic`; a later schema task can tighten those with dedicated TOML surface types.
