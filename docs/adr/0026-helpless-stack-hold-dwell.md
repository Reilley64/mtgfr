# 0026 — Helpless stack-hold dwell

Status: **Accepted**. Depends on [0007](0007-auto-pass-and-commander-ui-ahead-of-engine.md).

## Context

Uncontested stack objects sit for `STACK_HOLD` (2s) so the table can read them. Hovering a stack card while you cannot respond should buy a little more time without letting a parked cursor freeze the game.

## Decision

- While a stack-hold is active, a seat with **no meaningful action** may set a **helpless dwell** by hovering the stack (`POST /stack-dwell/v1`).
- Any active helpless dwell postpones resolution until the dwell ends or the hard cap (`STACK_HOLD + 3s` from hold start) is hit.
- Seats that still have meaningful actions cannot dwell-pause (`accepted: false`, `NotHelpless`).
- The visible state carries `stack_hold_remaining_ms` for the client countdown. Dwell toggles fan out a same-game-`seq` hold tick (`broadcast_seq` advances) so every seat's countdown reseeds; stamped `0` always clears the stream countdown (never resurrect a prior hold when the stack is still non-empty).

## Consequences

- Stack resolution is no longer a single fire-and-forget sleep; the hold task polls.
- Client hover becomes a server-side timing input (capped).
- Hold ticks must not bump game `seq`, or the hold timer would treat itself as stale.
- Stack Pass / Auto-pass chrome when the seat *can* act is separate (ADR 0027); dwell is only for helpless seats.