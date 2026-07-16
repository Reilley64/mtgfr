# 0003 — Additive continuous effects, no CR 613 layers

Status: **Accepted**

## Decision

- Effective P/T = base + counters + pumps + anthems (additive, on demand).
- Effective keywords = base ∪ granted. No layer ordering or timestamps.

## Consequences

- Wrong only when layer-sensitive interactions matter (set-base P/T vs counters, type changes).
- Build CR 613 when a card provably needs it; call sites (combat, SBAs) stay on "effective characteristics" queries.
