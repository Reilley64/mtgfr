# 0012 — Faithful soc precon pool

Status: **Accepted**; reframed by [0014](0014-any-card-faithful-scope-reversal.md) (proving ground, not ceiling).

## Decision

- Grow engine + pool to faithfully cover five `soc` Commander decks (~389 unique cards).
- Delete 19-card placeholder pool. Build subsystems when target cards need them (TDD, ADR 0002).

## Consequences

- Frozen decklists in `decklists/*.md` are the first fidelity gate. Scope now open-ended per 0014.
