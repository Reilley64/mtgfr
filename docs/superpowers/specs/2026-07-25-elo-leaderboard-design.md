# Elo Leaderboard Design

**Status:** Design input (superseded by living module specs as of 2026-07-25). Do not treat this
file as the source of truth for current behavior.

**Shipped behavior lives in:**

- [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md) — `User.rating` /
  `rating_set_at`, signup seeding, `Ratings.GetLeaderboard`, BFF
  `GET /api/rpc/ratings/leaderboard`
- [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md) —
  post-unlock `PlayerLost` → `ratings::persist_player_lost` (best-effort, non-fatal)
- [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) — auth-gated `/leaderboard` route
- [deck-list-and-builder](2026-07-20-deck-list-and-builder.md) — home top-5 teaser on `/`
- [wire-protocol-and-visibility](2026-07-20-wire-protocol-and-visibility.md) — `Ratings`
  gRPC surface and paging rules

---

## Problem Statement (historical)

Before this feature shipped, accounts persisted and the engine emitted `PlayerLost`, but games were
in-memory only and no rating or match history was stored. Players wanted a public skill board that
rose and fell from real multiplayer eliminations.

---

## Design intent (v1)

- **Scope:** Public bragging-rights leaderboard across all accounts; not matchmaking, not
  deck-scoped ratings, not in-board seat badges.
- **Math:** Classic Elo (`K = 32`, starting rating `1000`); on each elimination, every still-active
  account seat records a win over the eliminated account seat; snapshot-then-apply for multi-winner
  batches.
- **Persistence:** `users.rating` + `users.rating_set_at`; sort `rating DESC`, `rating_set_at ASC`,
  `id ASC`. No separate ratings table or match-history table in v1.
- **Write path:** API pod applies updates after accepted intent apply, outside the registry lock;
  skip seats without `user_id`; rating write failures must not fail game apply or the stream.
- **Read path:** `Ratings.GetLeaderboard` on gRPC; BFF Effect RPC to the Foldkit shell.
- **UI:** Auth-gated `/leaderboard` plus a top-5 teaser on the deck list home.

**Explicit non-goals:** ranked matchmaking, Glicko/TrueSkill, durable outbox / exactly-once
cross-pod delivery, rating seasons, guest-seat ratings, `GetMe.rating` polish.

---

## Implementation touchpoints (reference)

`crates/server/src/elo.rs`, `crates/server/src/ratings.rs`, `crates/server/src/game_loop.rs`,
`crates/server/src/session.rs`, `crates/server/src/grpc/ratings_svc.rs`, Toasty migration on
`users`, `client/app/shell` leaderboard submodel, deck-list home teaser.
