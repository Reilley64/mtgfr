# 0008 — Multiplayer combat, elimination, lobby

Status: **Accepted**; identity superseded by [0011](0011-decks-as-data-and-cookie-identity.md).

## Decision

- `Game::with_players(n, seed)` — variable seats; `next_player` skips eliminated.
- Per-attacker targets: `DeclareAttackers` carries `(attacker, defending_player)`. Blockers declared per defender (APNAP). Players only, no planeswalkers.
- `PlayerLost` tombstones owned objects (CR 800.4a); sole survivor wins.
- In-process table registry; lobby with seat claim, deck pick, ready-up, host start.

## Consequences

- 4-seat Commander with split attacks and elimination covered in `tests/game.rs`.
- Lobby polled via `GET /tables/lobby`; seat identity now session cookie (0011).
