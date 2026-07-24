# Nested Effect Families Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Regroup the flat `Effect` enum into family wrappers with inner mode enums, mirrored in card TOML as `type` + `mode`, via a hard-cut behavior-identical bang across the whole pool.

**Architecture:** Top-level `Effect` becomes `Damage(DamageEffect)`, `Draw(DrawEffect)`, … plus structural `Sequence` / `Conditional` / `ChooseOne`. Each family enum is `#[serde(tag = "mode")]`. A checked mapping table drives a TOML migrator. `Game::run` / `execute_effect` dispatch on the outer family, then family modules match modes. Query helpers (`target`, `label`, …) forward into family methods.

**Tech Stack:** Rust (`crates/engine`, `crates/cards`), serde + toml, `cargo nextest`, Python 3 migrator script (stdlib only), existing `card-dsl` feature.

**Spec:** [docs/superpowers/specs/2026-07-23-nested-effect-families-design.md](../specs/2026-07-23-nested-effect-families-design.md)

## Global Constraints

- Behavior-identical: no intentional rules / fidelity changes.
- Hard cut: no serde aliases for old flat `type` strings.
- Mapping table in the design spec is authoritative for `(old_type → family, mode)`.
- Every current `Effect` leaf appears in exactly one family (or structural).
- `CardDef` / `Effect` stay `Copy`; keep `cfg_attr(feature = "card-dsl", …)` patterns.
- Exhaustive matches remain — two-level, not registries/traits.
- Angular commits; branch `cursor/nested-effect-families-design-11d9` (rename to `cursor/nested-effect-families-11d9` only if continuing implementation on a fresh branch — prefer continuing this branch).
- TDD where a seam exists (migrator, deserialize fixture); the cutover itself is one green window after types + Rust sites + TOMLs land together.
- Update DSL_REFERENCE + card-dsl-and-card-pool spec examples in the same change.
- Mark design spec **Status:** Done when implementation merges.

---

## File map

| File | Responsibility |
|------|----------------|
| `docs/superpowers/specs/2026-07-23-nested-effect-families-design.md` | Normative mapping + shape (already written) |
| `crates/engine/src/types/effect/mod.rs` (new) | Top-level `Effect` + re-exports; replaces monolithic `effect.rs` |
| `crates/engine/src/types/effect/{damage,draw,life,destroy,control,counters,mana,mill,pump,reveal,token,zone,copy,dig,choice,static_effect,misc}.rs` (new) | Family enums + inherent `target` / helpers |
| `crates/engine/src/types/mod.rs` | `mod effect;` path update if needed |
| `crates/engine/src/resolution/mint.rs` | Outer family dispatch into typed `mint_*` |
| `crates/engine/src/resolution/{damage,draw,…}.rs` | Match on family enums, not flat `Effect::DealDamage` |
| `crates/engine/src/effects.rs` | `Game::run` outer family match |
| `crates/engine/src/label.rs` | Forward or family `label` methods |
| `crates/engine/src/{pending,cast,triggers,playable,…}.rs` | Nested pattern matches |
| `crates/engine/tests/game.rs` | `Effect::…` literals → nested |
| `crates/cards/data/**/*.toml` | Hard-cut `type`/`mode` rewrite |
| `scripts/migrate_effect_toml.py` (new) | Mapping-driven TOML rewriter |
| `scripts/effect_type_map.json` (new) | Machine-readable `old_type → {family, mode}` from the spec |
| `.agents/skills/card-dsl/DSL_REFERENCE.md` | Document nested authoring |
| `docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md` | Nested examples |

---

### Task 1: Machine-readable mapping + migrator (tested, no engine change)

**Files:**
- Create: `scripts/effect_type_map.json`
- Create: `scripts/migrate_effect_toml.py`
- Create: `scripts/test_migrate_effect_toml.py`
- Test: run the Python tests only

**Interfaces:**
- Consumes: mapping tables from the design spec
- Produces:
  - JSON: `{ "deal_damage": { "family": "damage", "mode": "target" }, … }` covering every non-structural leaf
  - CLI: `python3 scripts/migrate_effect_toml.py path/to/file.toml` rewrites in place; `--check` exits 1 if any `[[abilities.effects]]` (or nested step tables) still use a mapped flat `type` without `mode`
  - Structural types `sequence` / `conditional` / `choose_one` are left as `type` only; their nested `steps` / `options` / `then` effect tables are rewritten recursively

- [ ] **Step 1: Write the failing test**

Create `scripts/test_migrate_effect_toml.py`:

```python
import json
import tempfile
import textwrap
import unittest
from pathlib import Path

from migrate_effect_toml import load_map, migrate_text

ROOT = Path(__file__).resolve().parent


class MigrateEffectToml(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.mapping = load_map(ROOT / "effect_type_map.json")

    def test_map_covers_deal_damage(self):
        self.assertEqual(
            self.mapping["deal_damage"],
            {"family": "damage", "mode": "target"},
        )

    def test_rewrites_flat_effect_table(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "deal_damage"
            amount = 3
            target = "any"
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "damage"', out)
        self.assertIn('mode = "target"', out)
        self.assertNotIn('type = "deal_damage"', out)
        self.assertIn("amount = 3", out)

    def test_leaves_structural_sequence_type(self):
        src = textwrap.dedent(
            """\
            [[abilities.effects]]
            type = "sequence"

            [[abilities.effects.steps]]
            type = "draw_cards"
            count = 1
            """
        )
        out = migrate_text(src, self.mapping)
        self.assertIn('type = "sequence"', out)
        self.assertIn('type = "draw"', out)
        self.assertIn('mode = "cards"', out)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /workspace/scripts && python3 test_migrate_effect_toml.py -v`  
Expected: FAIL (import / missing map)

- [ ] **Step 3: Implement mapping JSON + migrator**

1. Transcribe every row from the design spec’s mapping tables into `scripts/effect_type_map.json`.
2. Implement `scripts/migrate_effect_toml.py`:
   - Parse TOML with `tomllib` (3.11+) or a minimal line-oriented rewriter that only rewrites effect tables (prefer `tomllib` load → walk tables → dump with a TOML writer, or use `tomlkit` if already available — **do not add a new Python dep**; if no writer exists, use a careful text transform that replaces `type = "old"` lines inside effect tables and inserts `mode = "…"` on the next line).
   - Recommended approach without new deps: regex/line walk scoped to tables whose path ends in `effects`, `steps`, `options`, or `then` (array-of-tables), replacing known flat types.
   - Recurse into nested step arrays.
   - `--check` mode: scan all `crates/cards/data/**/*.toml` and exit non-zero if any mapped old type remains as a bare `type = "…"`.

Include a completeness self-check in the test file:

```python
def test_map_has_at_least_230_entries(self):
    self.assertGreaterEqual(len(self.mapping), 230)
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /workspace/scripts && python3 test_migrate_effect_toml.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scripts/effect_type_map.json scripts/migrate_effect_toml.py scripts/test_migrate_effect_toml.py
git commit -m "build: add Effect type→family/mode TOML migrator"
```

---

### Task 2: Nested `Effect` types (damage + draw first, compile with stubs)

**Files:**
- Create: `crates/engine/src/types/effect/mod.rs`
- Create: `crates/engine/src/types/effect/damage.rs`
- Create: `crates/engine/src/types/effect/draw.rs`
- Modify: `crates/engine/src/types/mod.rs` (point `effect` at the new module dir; remove old `effect.rs` only after all families moved — this task moves **all** families in one go to avoid a half-flat enum)

**Interfaces:**
- Consumes: design shape
- Produces:
  - `pub enum Effect { Damage(DamageEffect), Draw(DrawEffect), …, Sequence { … }, Conditional { … }, ChooseOne { … } }`
  - Each family: `#[derive(Clone, Copy, Debug, PartialEq, Eq)]` + `#[cfg_attr(feature = "card-dsl", derive(serde::Deserialize))]` + `#[cfg_attr(feature = "card-dsl", serde(tag = "mode", rename_all = "snake_case"))]`
  - `DamageEffect::Target { amount, target, count, divided }` (fields from today’s `DealDamage`)
  - `DrawEffect::Cards { count }` (from `DrawCards`)
  - `impl Effect { pub(crate) fn target(self) -> TargetSpec { … } }` forwarding to family methods

**Note:** This task will not leave `cargo check` green by itself. Bundle Tasks 2–5 in one agent session / one green checkpoint if needed; commits may be WIP only if the branch policy allows — prefer a single green commit at the end of Task 5.

- [ ] **Step 1: Write a deserialize fixture test (fails until nest + TOML exist)**

Add to `crates/cards/src/lib.rs` tests (or new `crates/cards/tests/effect_nest.rs` if the crate supports integration tests):

```rust
#[test]
fn nested_damage_target_deserializes() {
    let toml = r#"
name = "Fixture Bolt"
id = "00000000-0000-0000-0000-000000000001"
default_print = "00000000-0000-0000-0000-000000000002"
oracle = "Fixture deals 3 damage to any target."

[cost]
red = 1

[kind]
type = "instant"

[[abilities]]
timing = "spell"

[[abilities.effects]]
type = "damage"
mode = "target"
amount = 3
target = "any"
"#;
    let def: engine::CardDef = toml::from_str(toml).expect("nested damage parses");
    assert!(matches!(
        def.abilities[0].effects[0],
        engine::Effect::Damage(engine::DamageEffect::Target { amount: engine::Amount::Fixed(3), .. })
    ));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo nextest run --profile ci -p cards nested_damage_target_deserializes`  
Expected: FAIL (unknown variant `damage` / no `DamageEffect`)

- [ ] **Step 3: Split `types/effect` and nest every family**

1. Move supporting types that today live above `enum Effect` in `effect.rs` (`Amount`, `TargetSpec`, …) carefully — either keep them in `effect/mod.rs` or leave non-Effect types in place if they already live in sibling modules (`types/filter.rs`, etc.). **Do not move unrelated enums.**
2. Create one file per family; cut leaf variants into the family enum; update field docs to point at the new paths.
3. Top-level `Effect` uses `serde(tag = "type")` with variants named `damage`, `draw`, … Serde externally tags the family; the inner value carries `mode`.
4. For `Sequence` / `Conditional` / `ChooseOne`, keep today’s field shapes and `static_slice` / `static_effect` helpers.
5. Implement `DamageEffect::target`, `DrawEffect::target`, … and `Effect::target` as a match on families.
6. Re-export family enums from `types/effect/mod.rs` and `types/mod.rs` / `lib.rs` as needed so tests can write `engine::DamageEffect`.

- [ ] **Step 4: Do not expect full green yet** — proceed immediately to Task 3 in the same working tree.

---

### Task 3: Update resolution + `Game::run` to two-level dispatch

**Files:**
- Modify: `crates/engine/src/resolution/mint.rs`
- Modify: `crates/engine/src/resolution/damage.rs` (and every other family mint/pause/choreo file that matches `Effect::…`)
- Modify: `crates/engine/src/effects.rs` (`Game::run` and any remaining flat arms)
- Modify: `crates/engine/src/resolution/mod.rs` / pause peels as needed

**Interfaces:**
- Consumes: nested `Effect`
- Produces:
  - `execute_effect` matches `Effect::Damage(d) => self.mint_damage(d, …)` etc.
  - `mint_damage(&self, effect: DamageEffect, …) -> Vec<Event>` matching on `DamageEffect::…`
  - `Game::run` matches outer family, delegates to `run_damage` / existing pause peels updated to take family enums or `Effect::Dig(d)`

- [ ] **Step 1: Rewrite `mint.rs` outer match**

```rust
match effect {
    Effect::Damage(d) => self.mint_damage(d, controller, source, target, x),
    Effect::Draw(d) => self.mint_draw(d, controller, source, target, x),
    // … one arm per family …
    Effect::Static(_) => Vec::new(),
    Effect::Sequence { .. } | Effect::Conditional { .. } | Effect::ChooseOne { .. }
    | Effect::Dig(_) | Effect::Choice(_) | Effect::Copy(_) | Effect::Misc(_)
        // pausing / composite — only via Game::run
        => unreachable!("pausing/composite effect reached execute_effect"),
}
```

Adjust the unreachable set to match today’s pause-vs-mint split (some `Misc`/`Choice`/`Dig` modes mint; keep the same sets as current `mint.rs` comments — move modes that today mint into the mint path, modes that pause into `run`).

- [ ] **Step 2: Inside each `resolution/*.rs` family file, change**

```rust
Effect::DealDamage { amount, divided, .. } => { … }
```

to

```rust
DamageEffect::Target { amount, divided, .. } => { … }
```

and rename `mint_damage_family` → `mint_damage` (same for other families).

- [ ] **Step 3: Rewrite `Game::run` outer match** to family wrappers; keep pause peel calls, updating signatures to accept `DigEffect` / `ChoiceEffect` where that simplifies peels.

- [ ] **Step 4: `cargo check -p engine` until the resolution/effects layer compiles** (pending/cast may still fail — Task 4).

---

### Task 4: Update remaining Rust pattern matches + helpers

**Files:**
- Modify: `crates/engine/src/label.rs`
- Modify: `crates/engine/src/pending/**/*.rs`
- Modify: `crates/engine/src/cast.rs`, `playable.rs`, `triggers.rs`, `query.rs`, `characteristics.rs`, and any other `Effect::DealDamage`-style matches (`rg 'Effect::[A-Z]' crates/engine`)
- Modify: `crates/engine/tests/game.rs` and other engine tests
- Modify: `crates/cards/src/lib.rs` tests that construct/assert effects
- Modify: `crates/server` / `crates/schema` only if they match on `Effect` (usually they should not)

**Interfaces:**
- Consumes: nested `Effect`
- Produces: compiling workspace with old TOMLs still broken until Task 5

- [ ] **Step 1: Find all flat matches**

Run: `rg -n 'Effect::(DealDamage|DrawCards|DamageEach|GainLife|DestroyTarget|AddMana|Sequence|Conditional)' crates --glob '*.rs'`

- [ ] **Step 2: Update each site** to `Effect::Damage(DamageEffect::Target { .. })` etc. Prefer `matches!` / `if let` that bind through one nest.

- [ ] **Step 3: Move `label` arms** onto family methods or nest under `Effect::Damage(d) => d.label()`.

- [ ] **Step 4: Update contextualize / fill helpers** in `types/effect` that rewrite effect trees (today’s `fill_*` functions matching flat variants).

- [ ] **Step 5: `cargo check --workspace`**  
Expected: engine/tests compile; `cards` may fail load until TOMLs migrate.

---

### Task 5: Hard-cut migrate all card TOMLs + pass deserialize tests

**Files:**
- Modify: `crates/cards/data/**/*.toml` (and `data/tokens/**/*.toml` if they embed effects)
- Modify: fixture test from Task 2 (should now pass)
- Test: cards load + nested_damage test

- [ ] **Step 1: Run migrator across the pool**

```bash
python3 scripts/migrate_effect_toml.py crates/cards/data
python3 scripts/migrate_effect_toml.py --check crates/cards/data
```

Expected: `--check` exits 0.

- [ ] **Step 2: Spot-check a few cards**

Open `crates/cards/data/shock.toml`, `breath_of_darigaaz.toml`, `hydra_omnivore.toml` — confirm `type`/`mode` and fields preserved.

- [ ] **Step 3: Run cards + engine tests**

```bash
cargo nextest run --profile ci -p cards -p engine
```

Expected: PASS (fix any migrator edge cases — nested `steps`, `choose_one.options`, kicked amounts).

- [ ] **Step 4: Commit cutover**

```bash
git add crates/engine crates/cards scripts docs/superpowers/specs/2026-07-23-nested-effect-families-design.md
git commit -m "refactor(engine): nest Effect into family/mode enums

Hard-cut card TOML to type + mode. Behavior-identical vocabulary reshape
per nested-effect-families design."
```

---

### Task 6: Docs + authoring reference

**Files:**
- Modify: `.agents/skills/card-dsl/DSL_REFERENCE.md` (effect authoring section — show `type`/`mode`; update examples that used flat types)
- Modify: `docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md` (Lightning Bolt-style example)
- Modify: `docs/superpowers/specs/2026-07-23-nested-effect-families-design.md` — Status → **Done**
- Modify: `docs/superpowers/specs/2026-07-20-choices-actions-and-resolution.md` only if it cites flat effect type strings

- [ ] **Step 1: Update DSL_REFERENCE** with a short “Effect families” section:

```markdown
## Effect families

Effects use adjacent tags:

```toml
[[abilities.effects]]
type = "damage"          # family
mode = "target"          # mode within family
amount = 3
target = "any"
```

Structural composers (`sequence`, `conditional`, `choose_one`) have no `mode`.
See `docs/superpowers/specs/2026-07-23-nested-effect-families-design.md` for the full map.
```

- [ ] **Step 2: Update card-dsl-and-card-pool example** to nested damage.

- [ ] **Step 3: Commit**

```bash
git add .agents/skills/card-dsl/DSL_REFERENCE.md docs/superpowers/specs
git commit -m "docs: document nested Effect type/mode authoring"
```

---

### Task 7: Full verification

**Files:** none (verify only)

- [ ] **Step 1: Format + lint + test server side**

```bash
just server-check
```

Expected: PASS

- [ ] **Step 2: Migrator check still clean**

```bash
python3 scripts/migrate_effect_toml.py --check crates/cards/data
```

Expected: exit 0

- [ ] **Step 3: Confirm no flat leftovers in Rust**

```bash
rg -n 'Effect::DealDamage\b|Effect::DrawCards\b|Effect::DamageEach' crates --glob '*.rs'
```

Expected: no matches (except comments/docs if any — clean those too)

- [ ] **Step 4: Push and update PR**

```bash
git push -u origin HEAD
```

Update PR body to note implementation complete; title may become `refactor(engine): nest Effect into family/mode enums` when ready for review (semantic-release: `refactor:` does not bump version — acceptable for this reshape).

---

## Spec coverage checklist

| Spec requirement | Task |
|---|---|
| Family wrappers + structural top-level | Task 2 |
| TOML `type` + `mode` adjacent tags | Tasks 2, 5 |
| Hard cut / no aliases | Tasks 1, 5 |
| Complete mapping | Task 1 (`effect_type_map.json`) + design spec |
| Two-level dispatch | Task 3 |
| Forwarding helpers | Tasks 2–4 |
| `types/effect/*.rs` split | Task 2 |
| Migrate all TOMLs | Task 5 |
| DSL_REFERENCE + card-dsl spec | Task 6 |
| Behavior-identical / tests green | Tasks 5, 7 |
| Non-goals (no strategy, no Amount nest) | respected throughout |

## Plan self-review

- No TBD placeholders in tasks.
- Migrator tested before cutover; deserialize fixture gates the nest.
- Cutover Tasks 2–5 are one green window — agents must not merge half-nested.
- Mapping JSON must stay in lockstep with the design spec tables.
