# 0029 — Turn yield

Status: **Accepted**. Depends on [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md), [0027](0027-stack-chrome-next-pass-and-yield.md). Extended by [0037](0037-end-turn.md) (same flag while active = End Turn).

## Context

**Stack yield** only covers the current stack. Players also want Arena-style “auto-pass until my turn” without re-arming on every new stack. Server **auto-pass** (0007) only skips helpless seats — not a standing preference while you *could* act.

## Decision

- Per-seat **turn yield** flag on the table (alongside stack `yields`). While set, `auto_advance` treats that seat like a stack-yielded seat (`turn_yield || stack_yield || !has_meaningful_action`).
- Clears when that seat becomes the **active player** at Untap (start of their turn).
- Clears when that seat is **attacked** (`AttackerDeclared` / `TokenEnteredAttacking` naming them as `defender`) so they can respond and declare blockers — the toggle turns off for that seat only, not for bystanders.
- Clears on any **player-initiated** intent for that seat (cast, activate, manual Pass/Next, etc.). Auto `PassPriority` from auto-advance or the stack-hold timer does not clear it.
- Does **not** clear when the stack empties (unlike stack yield).
- Independent of stack yield. UI: Arena-style toggle on the **priority context bar**; `POST /turn-yield/v1`; stamped as `VisibleState.turn_yielded` for the viewer.

Related (engine / ADR 0007, not turn-yield chrome): after attackers are declared, each **defending** seat's Declare Attackers priority treats empty-stack instants as meaningful so auto-pass stops only when that seat can actually respond — helpless defenders (yielded or not) auto-pass through to blockers.

## Consequences

Three distinct skip policies: helpless **auto-pass** (0007), **stack yield** (this stack), **turn yield** (until my turn / until I act / until I'm attacked). Being attacked clears turn yield; the engine attack-response window (empty-stack instants meaningful for defenders) is separate and applies to every attacked seat. Engine stays intent-only — yield flags are server chrome.
