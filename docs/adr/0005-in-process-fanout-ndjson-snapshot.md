# 0005 — In-process fan-out and snapshot-then-deltas

Status: **Accepted**; transport superseded by [0018](0018-effect-generated-client-and-sse-stream.md) (SSE, was NDJSON); extended by [0030](0030-table-instance-affinity-for-drain-rolls.md) (two-instance affinity for drain rolls).

## Decision

- `tokio::broadcast` per table; canonical events redacted per viewer at subscribe edge (`schema::redact`).
- Single-instance registry behind `std::sync::Mutex`; lock never held across `.await`.
- Stream opens with redacted snapshot at current `seq`; later frames are deltas with full `VisibleState`.
- Setup emits no events — opening board arrives as snapshot, not replay.

## Consequences

- No Redis. Client folds deltas in place (0006); reconnect re-snapshots on seq gap.
- Scale-out path: swap broadcast channel for Redis behind same publish/redact seam.
