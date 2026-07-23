## Destroy/Exile/Sacrifice effect family split

### What changed

- Split `DestroyEffect` so top-level `Effect` now carries three families: `Destroy(DestroyEffect)`, `Exile(ExileEffect)`, and `Sacrifice(SacrificeEffect)`.
- Slimmed `DestroyEffect` to destroy-only modes and created new `crates/engine/src/types/effect/exile.rs` and `crates/engine/src/types/effect/sacrifice.rs`.
- Updated engine minting, labels, target helpers, contextual fill helpers, runtime dispatch, and call sites to use the new top-level families.
- Updated `scripts/effect_type_map.json` and extended `scripts/migrate_effect_toml.py` so nested `type` + `mode` rewrites move old destroy-backed exile/sacrifice rows onto the new families with shortened mode names.
- Re-ran the migrator across `crates/cards/data`, updating affected card TOMLs to the hard-cut authoring syntax.
- Updated the authored syntax docs in `docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md`, `docs/superpowers/specs/2026-07-23-nested-effect-families-design.md`, and `.agents/skills/card-dsl/DSL_REFERENCE.md`.
- Regenerated `docs/CR_INDEX.md`.

### Verification

- `just engine-cr-index`
- `python3 scripts/test_migrate_effect_toml.py`
- `python3 scripts/migrate_effect_toml.py --check crates/cards/data`
- `cargo nextest run --profile ci -p cards -p engine`

### Results

- `scripts/test_migrate_effect_toml.py`: 11 tests passed.
- `scripts/migrate_effect_toml.py --check crates/cards/data`: passed with no unmigrated files.
- `cargo nextest run --profile ci -p cards -p engine`: 1952 tests passed, 0 skipped.

### Concerns

- No known functional regressions from verification.
- This is intended to be a behavior-preserving refactor plus authoring hard cut, so downstream diffs should mostly be enum family renames and TOML family/mode rewrites.
