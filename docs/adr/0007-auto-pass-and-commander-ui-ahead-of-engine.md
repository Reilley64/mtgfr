# 0007 — Server auto-pass and Commander-shaped UI

Status: **Accepted**; extended by [0020](0020-engine-computed-action-lists-with-ids.md).

## Decision

- `has_meaningful_action(player)` — land, affordable legal spell, non-mana activated, combat declaration (not bare tap-for-mana). Checks untapped-land mana, not just pool.
- Server loops `PassPriority` after each intent while holder has no meaningful action and no pending choice (bounded 256).
- Client renders 4-seat Commander layout; engine supports variable player count (0008).

## Consequences

- Engine stays intent-only; server automates tedious passes. `can_act` on wire for UI emphasis.
- Extended: same enumeration becomes per-player action lists (0020).
