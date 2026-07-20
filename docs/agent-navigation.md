# Agent navigation — engine CR lookup

How agents find where Comprehensive Rules concepts live in this repo.
This is **not** official CR text; it maps *our* citations and approximations to code.

## How to find a rule

1. Open [`CR_INDEX.md`](CR_INDEX.md) for `CR NNN…` → `path:line` hits (source + tests).
2. Or run `rg 'CR 601.2c' crates/engine`.
3. Read the module `//!` header for chapter ownership (coarse map below).
4. When behavior spans modules, follow [`pipeline.rs`](../crates/engine/src/pipeline.rs): SBA → trigger enqueue → APNAP placement → refresh actions.

## Module → CR chapter map

| Module | Primary CR / role |
|--------|-------------------|
| `pipeline` | CR 704 (SBA), CR 603 (place), CR 608 (priority rounds) |
| `apply` | Event apply + CR 704 SBA body |
| `triggers` | CR 603.* |
| `effects` / `resolution` | CR 608 — `Game::run` sole Effect→board verb; `resolution/{draw,damage,life,destroy,counters,mill,…}` mint families; pause via `pending` |
| `cast` | CR 601, 602, 118; alt costs (flashback, escape) |
| `playable` | CR 601 timing/zone gates, CR 307 (sorcery speed) |
| `combat` | CR 506–511, CR 702.* combat keywords, CR 701.38 |
| `priority` | Turn structure / TBAs / cleanup; CR 605 mana planning |
| `characteristics` (+ cache) | Effective P/T/keywords; internal `PtLayer` 7b/7c for P/T; CR 614 slices; full CR 613 still deferred ([engine-core](superpowers/specs/2026-07-20-engine-core-and-event-model.md)) |
| `pending` | Targets / modes / assignment; CR 601.2c, CR 608 pauses (`raise`/`answer`/`forced`) |
| `query` | Meaningful actions ([turn-priority](superpowers/specs/2026-07-20-turn-priority-and-stack.md)); CR 605 carve-outs |
| `zones` | Zone membership, draw/shuffle, mana-pool helpers |
| `core` | Game construction; object identity; CR 800.4a elimination |
| `spawn` | Test/lobby minting; commander tax (CR 903) |
| `types` | Cross-cutting vocabulary — use `CR_INDEX.md` |
| `lib` | Engine entry; points at CONTEXT / FIDELITY / this index |
| `state` | Goad (CR 701.38), delayed triggers (CR 603.7), exile links, until-EOT control (CR 720) |
| `de` / `label` | DSL deserialize / UI labels — little CR ownership |

## Related docs

| Doc | Use for |
|-----|---------|
| [`CONTEXT.md`](../CONTEXT.md) | Ubiquitous language / glossary |
| [`docs/fidelity/`](fidelity/) | Per-deck fidelity reports + increments backlogs |
| [Feature specs](superpowers/specs/) | Module behavior source of truth |
| [`.agents/skills/card-dsl/`](../.agents/skills/card-dsl/) | Card authoring / DSL |

## Maintaining citations

When landing fidelity work:

1. Cite the rule on the arm, type doc, or test (`CR 704.5m`, not bare prose).
2. Keep the module `//!` header honest about primary chapters.
3. Regenerate: `just engine-cr-index` (or rely on `.cursor/hooks.json`: dirty on engine `.rs` edit for *this* conversation — Task/subagent edits attribute to the parent — then regen on that conversation's agent `stop`).
4. Check: `just engine-cr-index-check`. Include `docs/CR_INDEX.md` in the commit when it changes.

Letter-slash forms in comments (`CR 704.5m/n`, `CR 608.2b/c`) are expanded by the generator into sibling rule entries.

## Non-goals

- No engine graph DB or embedding RAG — the corpus is small; grep + this index is enough.
- Official Comprehensive Rules meaning is external; do not treat our cites as the full rulebook.
- `ponytail:`, `approximates`, and fidelity backlog deferrals mean **cited ≠ fully faithful**.
