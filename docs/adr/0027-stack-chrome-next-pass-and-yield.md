# 0027 — Stack chrome: Next, Pass, and yield

Status: **Accepted** (amended for priority context bar + one-shot stack yield). Depends on [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md), [0026](0026-helpless-stack-hold-dwell.md). Extended by [0029](0029-turn-yield.md).

## Context

With a physical stack pile and server stack-hold, the old primary **Next** button (empty-stack turn advance) and a single "don't care" yield control were easy to confuse. Players need a one-shot pass when they *could* respond, without committing to yield the whole stack. Pass/yield chrome also belonged under the pile while **Next** lived bottom-right — two homes for the same priority job.

## Decision

- All priority-advance controls live in the **priority context bar**: always bottom-right; paint order above the **Stack view**, below prompt forms.
- **Empty stack:** primary control is **Next** (`pass_priority` / combat confirm as today).
- **Non-empty stack:** hide **Next**. While this seat holds priority and has a meaningful action, show **Pass** (one `pass_priority`) and **stack yield** (arm once for the rest of this stack). Space/Enter mirrors **Pass** in that window.
- **Stack yield** is one-shot from chrome: press to arm, then the control disables until the server clears it when the stack empties. No "Cancel auto-pass" on the bar.
- **Cannot act** on the stack (helpless / not your priority): hide Pass and the stack-yield arm (unless already armed — then show disabled). Resolution uses hold + helpless dwell (0026) and server auto-pass (0007).
- **Instant-priority focus** (battlefield dimming) is client presentation only: dim when you can act in an instant-speed window; keep the board bright on empty-stack main and declare attackers/blockers. Spectators never get that dimming.

## Consequences

- Yield stays a per-stack opt-out (no in-chrome revoke); Pass stays a normal priority pass — two distinct intents/routes.
- Hand playability still comes from the projected action list (sorcery-speed cards dim when they have no cast action), independent of battlefield dimming.
- Standing "skip until my turn" is **turn yield** (0029), not stack yield.
