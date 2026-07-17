# 0003 — Continuous effects / characteristics (P/T layers partial)

Status: **Accepted** (body updated 2026-07 for landed `PtLayer`; full CR 613 still deferred)

## Decision

- Call sites (combat, SBAs, damage) stay on **effective characteristics** queries (`Game::power` /
  `toughness` / keywords) — they do not know about layers.
- **P/T recompute** is no longer a flat additive sum. Internally, `characteristics` gathers ordered
  [`PtLayer`] entries (7b base-set, then 7c deltas: counters, pumps, anthems, attachment grants) and
  applies them via `apply_pt_layers`. The old ADR phrasing “base + counters + pumps + anthems
  (additive, on demand)” is **superseded for P/T** by that internal layer list.
- **Keywords** remain largely set-union of base ∪ granted. Full CR 613 (timestamps, dependency,
  type-changing / ability-removing order beyond the pool’s ponytail limits) is still deferred —
  grow when a card provably needs it (see `docs/FIDELITY_BACKLOG.md`).

## Consequences

- Layer-sensitive P/T interactions that only need 7b-then-7c (set-base vs counters/pumps) are in
  scope and already exercised.
- Type-changing continuous effects, lose-all-abilities ordering, and full timestamp/dependency
  still wrong or approximated until backlog items land; do not pretend the engine is CR 613-complete.
- Filename (`…-additive-…-no-layers`) is historical; prefer this Decision over the title.

[`PtLayer`]: ../../crates/engine/src/characteristics.rs
