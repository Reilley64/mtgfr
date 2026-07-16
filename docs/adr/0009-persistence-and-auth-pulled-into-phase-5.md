# 0009 — Auth and persistence before pool growth

Status: **Accepted** (historical sequencing)

## Decision

- Phase 5 = deck builder + auth + Postgres persistence; pool/DSL growth deferred.
- Full Commander legality enforced even on tiny pool (basics fill to 99).

## Consequences

- Decks persist per account. Pool growth became 0012 → 0014 open-ended fidelity.
