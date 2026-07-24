# Task 2-5 report: nested Effect family hard-cut

## Status

DONE

## Scope delivered

- Nested `Effect` family wrappers landed under `crates/engine/src/types/effect/`.
- Two-level dispatch landed in resolution and `Game::run`.
- Flat `Effect::OldVariant` Rust matches were updated across engine/cards tests.
- Card/token TOMLs were hard-cut to `type` + `mode`.
- Migrator/checking script was strengthened to cover real pool shapes discovered during cutover.

## TDD evidence

### Red

1. The task-added fixture `cards::tests::nested_damage_target_deserializes` existed to prove the new nested TOML shape.
2. While finishing the cutover, `cargo nextest run --profile ci -p cards nested_damage_target_deserializes` initially failed in `crates/cards/src/lib.rs` because the refactor left malformed nested `matches!` assertions and missing family enum imports in test code.
3. After the syntax/import repair, the same focused test became the regression check for the new nested damage shape.

### Green

- `cargo nextest run --profile ci -p cards nested_damage_target_deserializes`
  - Result: PASS, 1 test run / 1 passed.

## Migration notes

The original migrator was not sufficient for the full hard cut. During verification I extended it to:

- accept a directory path like `python3 scripts/migrate_effect_toml.py crates/cards/data`
- rewrite inline singleton effect objects such as `then = { type = "draw_cards", ... }`
- rewrite inline nested arrays under additional keys such as `effects` and `on_expiry`
- rewrite old `choose_one` payload keys from `modes` to `options`
- recognize single-table effect sections such as `[alternative_cost.rider]`
- cover the legacy `set_attached_base_p_t` spelling in `effect_type_map.json`

## Verification commands and results

1. `python3 scripts/migrate_effect_toml.py crates/cards/data`
   - Result: rewrote the full pool, then later rewrote residual holdouts after migrator fixes.
2. `python3 scripts/migrate_effect_toml.py --check crates/cards/data`
   - Result: exit 0 after migrator fixes.
3. Spot checks:
   - `crates/cards/data/shock.toml` now uses `type = "damage"` + `mode = "target"`.
   - `crates/cards/data/breath_of_darigaaz.toml` now uses `damage/each_creature` and `damage/each_player`.
   - `crates/cards/data/hydra_omnivore.toml` now uses `damage/each_other_opponent`.
4. `cargo nextest run --profile ci -p cards -p engine`
   - Final result: PASS, 1951 tests run / 1951 passed.
5. `cargo fmt`
   - Result: exit 0.
6. `cargo nextest run --profile ci -p cards -p engine`
   - Final post-format verification: PASS, 1951 tests run / 1951 passed.

## Behavior/fidelity notes

- No intentional rules changes were introduced.
- Fixes after the first broad migration were vocabulary-shape fixes only:
  - residual flat TOML spellings
  - `choose_one.options` field cutover
  - inline test fixtures updated to the nested DSL

## Concerns

- The cutover touched a very large diff surface by design.
- The strengthened migrator is now materially more trustworthy than the Task 1 baseline, but the large pool rewrite still merits normal review attention on the generated TOML churn.
