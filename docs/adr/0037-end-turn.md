# 0037 — End Turn (Arena pass-the-turn)

Status: **Accepted**. Depends on [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md), [0029](0029-turn-yield.md).

## Context

Players need Arena-style **End Turn**: finish the rest of your turn without mashing **Next**, while opponents still get priority windows to cast instants / activate abilities. If an opponent acts, End Turn cancels so you can respond.

[0029](0029-turn-yield.md) already arms the same per-seat `turn_yield` flag for “auto-pass until my turn” when you are *not* active. End Turn is that flag while you *are* active, with two extra clear/stop rules.

ADR 0007 deliberately omits empty-stack instants from `has_meaningful_action` (so the table does not halt for every removal in hand). That would make End Turn skip opponent response windows — so while the active seat is end-turning, auto-advance also stops for other seats with [`Game::has_empty_stack_instant_play`].

## Decision

- **Arm:** active seat sets `turn_yield` (existing `SetTurnYield` / `turn_yielded`). UI label **End Turn** (toggle off cancels).
- **Advance:** `auto_advance` skips the end-turning seat; other seats still play. Empty-stack instant-speed casts on other seats stop the walk (ADR 0007 carve-out above).
- **Clear** (in addition to 0029’s own-intent / attacked / Untap-as-active rules):
  - On `Cleanup` for the ending active seat (so a discard-to-hand-size pause cannot leave End Turn armed into the next turn).
  - When another seat submits a non-`PassPriority` intent: clear the **active** seat’s yield so they can respond.
  - End Turn response windows override the responder’s until-my-turn for that priority seat only.
- **Not** a new wire RPC or engine intent — server chrome only, same as 0029.
- Unconditional pass-turn (Arena Shift+Enter) is out of scope.

## Consequences

Four skip policies: helpless auto-pass (0007), stack yield (0027), turn yield until my turn (0029), End Turn (this). Client shows **End Turn** on your turn and the until-my-turn rocker otherwise; both stamp `turn_yielded`.
