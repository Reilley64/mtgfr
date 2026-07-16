# 0013 — Spectator projection via optional viewer

Status: **Partial** — durable tables superseded by [0021](0021-live-games-in-memory-only.md); spectator projection **current**.

## Decision (current)

- `snapshot`/`redact` over `Option<PlayerId>`: `Some(seat)` = player view, `None` = spectator (all hands/libraries hidden).
- `VisibleState.viewer` uses `SPECTATOR_VIEWER` (255) for seatless watchers.

## Decision (superseded)

- ~~Intent-log replay (`SavedGame`/`SavedIntent`) for restart-resume.~~ Deleted in 0021.

## Consequences

- Eliminated players and signed-in non-seated users get public projection, not 403.
