# 0004 — Resumable engine with explicit pending choice

Status: **Accepted**

## Decision

- `Game.pending_choice: Option<PendingChoice>` — plain data, no callbacks.
- While pending, only the matching answer intent from the awaited player is legal.
- 0–1 items auto-resolve; ≥2 raises a choice.

## Consequences

- Uniform intent interface for tests, server, and client. Paused games are fully serializable.
- New prompts = new `PendingChoice` variant + resolver arm (extended in 0006).
