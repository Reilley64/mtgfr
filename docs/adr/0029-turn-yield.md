# 0029 — Turn yield

Status: **Accepted**. Depends on [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md), [0027](0027-stack-chrome-next-pass-and-yield.md).

## Context

**Stack yield** only covers the current stack. Players also want Arena-style “auto-pass until my turn” without re-arming on every new stack. Server **auto-pass** (0007) only skips helpless seats — not a standing preference while you *could* act.

## Decision

- Per-seat **turn yield** flag on the table (alongside stack `yields`). While set, `auto_advance` treats that seat like a stack-yielded seat (`turn_yield || stack_yield || !has_meaningful_action`).
- Clears when that seat becomes the **active player** at Untap (start of their turn).
- Clears when that seat is **attacked** (`AttackerDeclared` / `TokenEnteredAttacking` naming them as `defender`) so they can respond and declare blockers — the toggle turns off for that seat only, not for bystanders.
- After that seat is attacked, empty-stack instants become ADR 0007 **meaningful** for their Declare Attackers priority (engine attack-response window), so `auto_advance` stops only when they can actually respond — helpless defenders auto-pass through to blockers.
- Clears on any **player-initiated** intent for that seat (cast, activate, manual Pass/Next, etc.). Auto `PassPriority` from auto-advance or the stack-hold timer does not clear it.
- Does **not** clear when the stack empties (unlike stack yield).
- Independent of stack yield. UI: Arena-style toggle on the **priority context bar**; `POST /turn-yield/v1`; stamped as `VisibleState.turn_yielded` for the viewer.

## Consequences

Three distinct skip policies: helpless **auto-pass** (0007), **stack yield** (this stack), **turn yield** (until my turn / until I act / until I'm attacked). Being attacked clears turn yield and opens an engine attack-response window where empty-stack instants count as meaningful — helpless defenders still auto-pass. Engine stays intent-only — all three are server chrome.
