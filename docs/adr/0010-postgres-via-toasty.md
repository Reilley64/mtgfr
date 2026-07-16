# 0010 — Postgres via Toasty ORM

Status: **Accepted**; game tables removed per [0021](0021-live-games-in-memory-only.md).

## Decision

- Postgres via Toasty (`db.rs` seam); models `User`, `Session`, `Deck` in server only.
- `Deck.cards` = JSON blob of `Vec<DeckCardEntry>` (always read/written whole).
- `push_schema()` at boot for dev; tests use in-memory SQLite.

## Consequences

- `DATABASE_URL` required to start server. Swap ORM = one file change.
- Only users, sessions, decks persist — not live games (0021).
