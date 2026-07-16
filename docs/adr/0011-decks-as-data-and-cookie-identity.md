# 0011 — Decks as data; session cookie identity

Status: **Accepted**; replaces ADR 0008 token stand-in.

## Decision

- Deck = persisted `(name, commander, cards)` owned by user. `DeckChoice` enum deleted.
- HttpOnly session cookie + `AuthUser` extractor; `IntentEnvelope.token` removed.
- `JoinRequest.deck_id` resolved to caller's deck; `legendary` on `CardDef` for commander eligibility.
- `legality::validate` on save and game start.

## Consequences

- `start_game`: collect ids under lock → load/validate decks (no lock) → seed under lock.
- Client uses `credentials: "include"` on all requests including SSE.
