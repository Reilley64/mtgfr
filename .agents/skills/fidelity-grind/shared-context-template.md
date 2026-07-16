# Shared context — fidelity-grind wave (read this first, every agent)

You are implementing ONE increment from the engine fidelity backlog for **mtgfr**, a
Rust MTG Commander engine.

**Work in the isolated git worktree at `{{WORKTREE}}` (branch `{{BRANCH}}`) — `cd` there
before any command.** Do NOT touch the main checkout (it has unrelated concurrent work).
All paths below are relative to the worktree root.

## What the project is (one paragraph)

A browser-based 4-player Commander game. North star: support *any* card, built
**faithfully**. Method: grow the engine + card DSL *from real cards*, TDD,
smallest-increment-first, **flag-don't-force**. When a card needs something the DSL can't
express, you record it as an `approximates` note rather than contort the card. The whole
point of these increments is to turn existing approximations into faithful models and
delete/trim their notes.

## The backlog (authoritative task source)

`docs/FIDELITY_BACKLOG.md` — your brief names ONE increment by number and heading. Read
that section; the *Sketch* line is the intended design. Verify "landed" claims against
the code, not the prose.

## Where things live

`docs/agent-navigation.md` maps the layout — grep to confirm, module boundaries move.
Fixed points:

- `crates/engine/src/types/` — `CardDef`, `Effect`, `Trigger`, filters, amounts, costs,
  stack types, with the serde derives (feature `card-dsl`). Card enums use `&'static [T]`
  (leaked slices), NOT `Vec`, because **`CardDef: Copy`**. A `Vec` field breaks the build.
- `crates/engine/src/de.rs` — the TOML→CardDef deserializer. `deny_unknown_fields` is on,
  so any new TOML key requires a matching struct field/enum variant.
- `crates/engine/src/effects.rs` — resolves `Effect` arms; `triggers.rs` — trigger
  placement/firing; `pending/` — pending-choice pause/answer machinery.
- `crates/engine/tests/game.rs` — the ONLY engine test file; tests build `CardDef` structs
  directly and drive the public API. This is your TDD surface.
- `crates/cards/data/*.toml` — the card pool. Faithful cards have no `approximates`.
- `crates/engine/src/label.rs` — exhaustive `match Effect` for ability labels; every new
  variant needs an arm.
- `crates/schema/` mirrors the wire protocol with exhaustive matches — a new `Event` needs
  a `VisibleEvent` + `redact_for` arm; a new `PendingChoice`/`Intent` needs its projection.
  Hands/libraries are private; drop hidden fields in the `VisibleEvent`.

## Non-negotiable constraints

1. **TDD.** Failing engine test FIRST, then code. Use the `tdd` skill. Every bug you find
   and fix gets a regression test in the same change, at the lowest layer that catches it.
2. **Fidelity.** Use the `card-dsl` skill. Model behavior after the real CR; name
   types/tests after the real concept. Shortcuts get a `// ponytail:` comment naming the
   rule approximated and the upgrade path.
3. **Keep the engine pure** — no I/O, no wall-clock, no `rand`. Determinism is required.
4. **Style:** guard-return-first; readability over cleverness.
5. **`CardDef: Copy`** — new card-data fields are `&'static [T]` / `Option<..>` / scalars.
6. **Only touch what your increment needs.** No unrelated refactors; no premature
   generalization of the DSL.
7. **Faithful cards lose their note.** Delete `approximates` when fully faithful, or trim
   to the precise residual gap.
8. **No "faithful" comments — silence means faithful.** The only comments allowed:
   `approximates`-paired modeling notes, `ponytail:` gap notes, and oracle quotes (rule 9).
   Delete any `# Faithful:`-style prose on cards you touch.
9. **Oracle quotes everywhere.** The file header is the bare verbatim oracle text (no
   `Oracle:` prefix). Every `[[abilities]]` block (incl. `[[back.abilities]]`) carries the
   exact oracle sentence(s) it implements directly above; every `[[abilities.effects]]`
   block carries its oracle CLAUSE. Verbatim quotes only, no commentary. Comment lines wrap
   at 120 characters.
10. **Effects always use the `[[abilities.effects]]` array form** — the singular `effect =`
    sugar does not parse.
11. **DSL changes land in `.agents/skills/card-dsl/DSL_REFERENCE.md`** in the same change —
    every field, tag, timing, filter axis. No new surface ships undocumented.

## Definition of done (every increment)

- New/changed behavior covered by at least one new test named after the MTG concept and card.
- The named example cards are faithful (or trimmed with a precise residual note — say which
  in your return summary).
- `cargo test` GREEN, `cargo clippy --all-targets` no new warnings, `cargo fmt` applied.
  **Do not finish on a red or dirty tree** — the next agent inherits it. Do not commit.
- Return a terse summary: cards now faithful, cards still approximated and why, files
  touched, follow-ups noticed.

## You are one stage in a sequence

Agents run one after another on the SAME working tree. Assume prior increments already
landed (their enum arms/fields exist). Do not revert or "clean up" their uncommitted work.
If your increment collides with something present, adapt to it.
