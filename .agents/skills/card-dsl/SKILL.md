---
name: card-dsl
description: Author or edit a card definition TOML in crates/cards/data/, or judge whether a card is faithfully expressible in the engine. Use for card fidelity review, adding a new Secrets of Strixhaven card, or deciding whether to flag a gap.
---

# Card-Definition TOML DSL

One TOML file per card in `crates/cards/data/*.toml`, deserialized into `engine::CardDef`.
Token profiles live in `crates/cards/data/tokens/*.toml` and are referenced from creating cards
by Scryfall oracle id (`token = "<id>"` on `create_token` — no inline token tables).

**Full field reference: [`DSL_REFERENCE.md`](DSL_REFERENCE.md)** (in this skill folder). Read
it before writing or editing a card TOML. **Source of truth for shapes** is
`crates/engine/src/types.rs` and `crates/engine/src/de.rs` — if the reference and code
disagree, the code wins. Engine gaps for a deck live in that deck's
`docs/fidelity/<slug>-increments.md` (created by the `fidelity-grind` skill) — flag with
`approximates` / `# ponytail:` on the card rather than contorting the model.

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
- **Flag, don't force-script.** If a card needs something the DSL can't express (see reference
  §10, "Unsupported"), don't contort the model to fake it — flag it as a gap instead.
- **Test-first when behavior changes.** New or changed card behavior that needs engine
  support goes through the **`test-driven-development`** skill (failing test in
  `crates/engine/tests/game.rs` before the TOML / `Effect` arm). Pure TOML authoring against
  an already-expressible DSL surface still wants a regression test when the card is
  non-trivial.
- Card identity is the `name` field, not the filename; filename is arbitrary.
