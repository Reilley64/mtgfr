# 0006 — Client delta fold and choice framework

Status: **Accepted**; refines [0005](0005-in-process-fanout-ndjson-snapshot.md).

## Decision

- `DeltaEnvelope` carries events + viewer's full post-apply `VisibleState` — no mid-stream snapshot refetch.
- Render assembly in wire layer (`schema::snapshot`), not fat engine events. `Effect::label()` lives in engine (`label.rs`).
- General `PendingChoice` variants (target, may, pay cost, assign combat damage) with fixture-driven tests.
- Multi-block damage chosen at block declaration; stored in combat state until damage step.

## Consequences

- Log and stack panel fold from the same deltas as the board. `GET /snapshot` only for connect/reconnect/gap.
