# 0021 — Live games in-memory; stable action ids

Status: **Accepted**; supersedes durable-tables half of [0013](0013-durable-tables-via-replay-and-spectator-projection.md); amends [0020](0020-engine-computed-action-lists-with-ids.md).

## Decision

- `refresh_actions()` keeps id for surviving `(player, kind)` entries; mints new ids only for new actions.
- Delete `SavedGame`/`SavedIntent`/persist module. Registry is sole home of live games — lost on restart.
- DB holds users, sessions, decks only.

## Consequences

- Tap-then-cast and any client holding an id across intents works. `UnknownAction` = genuinely stale action.
- Spectator projection from 0013 unchanged.
