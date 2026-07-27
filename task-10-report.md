# Task 10 re-review report

## 2026-07-27

- Status: fixed the non-effect schema escapes from the re-review finding.
- Closed schema fields:
  - `CostToml.additional` now projects as `AdditionalCost`.
  - `CostToml.reduce_own_generic` now projects as `Amount | null`.
  - `KindToml.also` now projects as `TypeSet`.
  - `KindToml.produces` now projects as `LandProduces | null`.
- Generated artifacts:
  - `just cards-schema`
  - `just cards-dsl-ref`
  - `just engine-cr-index` after `server-check` reported the generated CR index was stale.
- Regression coverage:
  - `crates/cards/tests/schema_validate.rs` rejects invalid `cost.additional.pay_life`,
    `cost.reduce_own_generic`, `kind.also`, and `kind.produces` values.
- Verification:
  - `cargo nextest run --profile ci -p cards rejects_untyped_non_effect_schema_escape_fields`
  - `just cards-schema-check`
  - `just cards-dsl-ref-check`
  - `cargo nextest run --profile ci -p cards`
  - `just cards-toml-validate crates/cards/data/*.toml`
  - `just cards-toml-validate --token crates/cards/data/tokens/*.toml`
  - `just server-check`
- Remaining WAVE_C:
  - Per-effect payload typing is still deferred: `EffectTomlSchema.payload` remains the only open
    TOML-surface `serde_json::Value`.
- Remaining `serde_json::Value` notes:
  - `crates/engine/src/toml_surface/card.rs`: `EffectTomlSchema.payload`, intentionally open WAVE_C.
  - Other `serde_json::Value` uses are schema-building/parsing helpers, not open DSL field escapes.
