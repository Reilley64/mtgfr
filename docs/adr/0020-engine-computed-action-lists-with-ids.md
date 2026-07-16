# 0020 — Engine-computed action lists, taken by id

Status: **Accepted**; extends [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md); amended by [0021](0021-live-games-in-memory-only.md), [0022](0022-payment-settles-engine-side-with-auto-tap.md).

## Decision

- After every `submit()`, `refresh_actions()` builds each player's `Vec<LegalAction>` from `meaningful_actions`.
- `Intent::TakeAction { id, … }` resolves stored action; classic intents remain for tests/choices/combat.
- `VisibleState.actions` filtered to viewer's seat. Empty while `pending_choice` is set.
- Server `auto_advance` submits forced single-answer choices + empty priority passes; `auto_actions` on wire for AUTO-marked game-log lines.

## Consequences

- Client renders sections (`hand`, `command`, `graveyard`, …), not rules. New play zones = engine-only work.
- Stable ids (0021). Payment settled engine-side (0022). Commander canvas click and combat still use legacy intents.
