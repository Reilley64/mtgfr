---
name: card-dsl
description: Author or edit a card definition TOML in crates/cards/data/, or judge whether a card is faithfully expressible in the engine. Use for card fidelity review, adding a new Secrets of Strixhaven card, or deciding whether to flag a gap.
---

# Card-Definition TOML DSL

One TOML file per card in `crates/cards/data/*.toml`, parsed as `engine::CardToml`
and folded into `engine::CardDef`.
Token profiles live in `crates/cards/data/tokens/*.toml` and are referenced from creating cards
by Scryfall oracle id (`token = "<id>"` on `create_token` — no inline token tables).

**Full field reference: [`DSL_REFERENCE.md`](DSL_REFERENCE.md)** (in this skill folder). Read
it before writing or editing a card TOML. It is generated from the TOML surface; keep §0
process discipline here in `SKILL.md`. **Source of truth for shapes** is `crates/engine/src/toml_surface/`,
`crates/cards/src/types/effect/`, and `crates/cards/src/de.rs` — if the reference and code
disagree, the code wins. Engine gaps for a deck live in that deck's
`docs/fidelity/<slug>-increments.md` (created by the `fidelity-grind` skill) — flag with
`approximates` / `# ponytail:` on the card rather than contorting the model.

## Schema and validation

- Generated schemas live at `crates/cards/schema/card.schema.json` and
  `crates/cards/schema/token.schema.json`.
- Validate touched deckable cards with `just cards-toml-validate crates/cards/data/<card>.toml`.
- Validate token profiles with
  `just cards-toml-validate --token crates/cards/data/tokens/<token>.toml`.
- After changing TOML surface types or schema annotations, run `just cards-schema` and
  `just cards-dsl-ref`; use `just cards-schema-check` and `just cards-dsl-ref-check` to prove
  the committed generated files are fresh.
- Schema validation catches structural TOML mistakes. Rust deserialize through
  `CardToml -> CardDef` remains the authority for what actually loads.

## Non-negotiable discipline

- **Oracle text first.** Every card file opens with a comment holding the verbatim current
  Scryfall Oracle text (bare quote, no `Oracle:` prefix), above `name`. Vanilla cards (basics,
  French-vanilla creatures) still get the line, even if it's just keywords or empty. Comment
  lines wrap at 120 characters.
- **Faithful by default.** Model what the card actually does; don't reach for an approximation
  out of convenience.
- **When you must trim/approximate:** set the machine-readable `approximates` field (what the
  catalog and audits read) *and* leave a `# ponytail:` comment next to the divergence naming the
  rule approximated (for humans). Both, not either — comment alone doesn't count.
- **Flag, don't force-script.** If a card needs something the DSL can't express, don't contort
  the model to fake it — flag the gap in that deck's `docs/fidelity/<slug>-increments.md` and
  see [`card-dsl-and-card-pool` Out of Scope](docs/superpowers/specs/2026-07-20-card-dsl-and-card-pool.md#out-of-scope)
  for deliberate DSL limits.
- **Test-first when behavior changes.** New or changed card behavior that needs engine
  support goes through the **`test-driven-development`** skill (failing test in
  `crates/engine/tests/game.rs` before the TOML / `Effect` arm). Pure TOML authoring against
  an already-expressible DSL surface still wants a regression test when the card is
  non-trivial.
- Card identity is the `name` field, not the filename; filename is arbitrary.
