# Elo Leaderboard Design

**Status:** Design note (as of 2026-07-25)
**Module:** `crates/server` (auth/db + game session elimination hook), proto `Ratings` (or equivalent),
BFF `/api/rpc/ratings/*`, `client/app/shell` (`/leaderboard` + home teaser on `/`)

This document is design input for a public Elo leaderboard. Implementation must also update the
living module specs it touches (accounts, shell routes, deck list, lobby/live-game) in the same
change — this file does not replace those surface specs.

---

## Problem Statement

Players want public bragging rights: a skill rating that rises and falls from real multiplayer
tables, visible as a full leaderboard and a small home teaser. Today accounts persist and the
engine emits `PlayerLost`, but games are in-memory only and no rating or match history is stored.

---

## Goals

- Public leaderboard across the whole player base (not friends-only, not matchmaking).
- Every multiplayer game contributes: on each elimination, every still-active account seat records
  a win over the eliminated account seat; updates persist immediately.
- Every account appears on the board from signup with a default rating.
- Sort: higher rating first; ties broken by who reached that rating value first.

---

## Non-goals (v1)

- Ranked matchmaking or lobby skill gates.
- Deck-scoped or commander-scoped ratings.
- Glicko / TrueSkill / uncertainty display.
- In-board seat badges or mid-game rating chrome.
- Durable outbox / exactly-once cross-pod rating delivery.
- Rating seasons or soft resets.
- Guest / unbound seats affecting ratings.

---

## Solution summary

Classic Elo (`K = 32`, starting rating `1000`) with **pairwise updates on each elimination**.
The API pod applies updates when `Event::PlayerLost` appears after an intent; Postgres `mtgfr`
is the source of truth; the BFF exposes a leaderboard RPC to the Foldkit shell (`/leaderboard`
plus a top-N teaser on the deck list home).

---

## Architecture

| Concern | Owner |
|---------|--------|
| Rating columns on `users` | Toasty migration + `User` model (`mtgfr`) |
| Elo math | Pure function in `crates/server` (no I/O) |
| Elimination hook | `TableSession` apply path after engine events |
| Leaderboard reads | gRPC on API → BFF Effect RPC → SPA |
| UI | Auth-gated `/leaderboard`; top-5 teaser on `/` |

Writes never go through the BFF. Rating write failures must not fail game apply or the stream.

---

## Data model

Extend `users`:

| Column | Type | Default / rules |
|--------|------|-----------------|
| `rating` | `i32` | `1000` at signup |
| `rating_set_at` | `i64` (Unix seconds) | account-created time at signup; set to `now` whenever the stored integer `rating` changes |

No separate `ratings` table in v1. Optional counters (`rated_wins`, etc.) are deferred.

**Leaderboard sort:** `ORDER BY rating DESC, rating_set_at ASC, id ASC` (final `id` for total stability).

---

## Elo math

Constants:

- `K = 32`
- `E_self = 1 / (1 + 10^((R_opp − R_self) / 400))`

On each elimination of account user `L` with still-active account winners `W1…Wn`:

1. **Snapshot** current ratings for `L` and all winners.
2. For each winner `W`, compute one virtual match vs `L` using snapshot ratings:
   - Winner score `1`: `Δ_w = K * (1 − E_w)`
   - Loser score `0`: `Δ_l += K * (0 − E_l)` (sum loser deltas across all winners in the batch)
3. **Apply** new integer ratings: compute in `f64`, then round to nearest `i32` (half away from
   zero, i.e. `f64::round` semantics).
4. For each user whose integer rating changed, set `rating_set_at = now`.

Snapshot-then-apply makes multi-winner batches order-independent.

**Skip** any pair where either seat has `user_id == None`.

---

## Write path

1. After a table drive accepts, clone the emitted events, seat `user_id` snapshot, and post-apply
   `Game` under the registry lock; persist ratings only after unlock. This runs from the player
   intent path (`game_loop::with_seated_drive`) and scheduled stack resolution
   (`session::schedule_stack_resolution`).
2. Resolve loser seat → `user_id`; if missing, skip.
3. For a batch with multiple `PlayerLost` events, reconstruct players active at batch start as
   every player either not lost in the post-apply game or listed in the batch's losses.
4. Process losses in event order. For each loser, winners are the current alive set minus the
   loser; after the update, remove that loser before the next batch event.
5. If no winners with accounts, skip.
6. Load ratings; run pure `apply_elimination`; persist changed rows.
7. **Failure policy:** log + metric; do not fail the intent, settle loop, or broadcast. Best-effort
   single retry is optional; no durable outbox in v1.
8. **Idempotency:** apply once per emitted `PlayerLost` in that apply’s event list (engine does not
   re-emit). Table is pinned to one pod; no cross-pod double-apply.
9. **Concurrency:** concurrent tables updating the same user may race; last-write-wins is acceptable
   for v1 bragging rights.

Abandoned tables and pod death do not reverse already-committed elimination updates.

---

## Read path / wire

New gRPC surface (name flexible; e.g. `Ratings`):

- `GetLeaderboard { limit, offset }` → `{ entries: [{ user_id, username, rating, rank }], total }`
  - `rank` is 1-based in global sort order.
  - Default page size suitable for UI (e.g. 50); cap max limit (e.g. 100).
- Signup / user creation initializes `rating` and `rating_set_at`.
- Extending `GetMe` with `rating` is optional v1 polish (enables “your rating” near the teaser);
  not required to ship the board.

BFF: same-origin Effect RPC, e.g. `/api/rpc/ratings/leaderboard`, auth-gated like other shell
surfaces. Required ids stay in path params if any resource routes are added later; list query uses
optional `limit` / `offset` query or RPC fields only.

---

## Client UI

### `/leaderboard`

- Auth-gated Foldkit shell route.
- Ranked list: rank, username, rating.
- Paging or top page + load more.
- Link from home teaser.
- Scene tests with `data-testid`s (AGENTS.md surface rule).

### Home teaser (`/`)

- Top 5 entries + control linking to `/leaderboard`.
- One job: show who’s ahead; not a stats dashboard or card grid.
- Lives beside/above the existing deck list composition without turning home into a second product.

### Deferred

- Mid-game seat rating badges.
- Profile pages dedicated to rating history.

---

## Testing

| Layer | Coverage |
|-------|----------|
| Pure Elo unit | 2p / 3p / 4p elimination sequences; skip missing `user_id`; unchanged rating leaves `rating_set_at`; batch order independence |
| Server integration | `ratings.rs` covers ordered multi-loss batches; gRPC seeded table → concede → DB ratings move; stream/apply still succeeds if rating DB write fails |
| Leaderboard RPC | Sort `rating DESC, rating_set_at ASC`; default-1000 users included; paging |
| Client Scene | `/leaderboard` rows; home teaser top N + navigation |
| Specs in same change | Update accounts, shell-routes, deck-list, lobby/live-game living specs to match shipped behavior |

---

## Implementation touchpoints

- `toasty/migrations/*` + `crates/server/src/db.rs` (`User`)
- Pure module e.g. `crates/server/src/elo.rs`
- Hook in `crates/server/src/ratings.rs`, `crates/server/src/game_loop.rs`, and
  `crates/server/src/session.rs` (post-apply events, persisted after registry unlock)
- Proto + tonic handler + BFF RPC route
- `client/app/routes.ts` + shell leaderboard submodel/view
- Deck list home teaser in `client/app/shell/decks/**`
- Scene tests in `client/app/shell/surfaces.test.ts` (and focused tests as needed)

---

## Out of scope / further notes

- Engine CR / pure engine crates stay free of rating I/O.
- No match-history table in v1 (only current rating + `rating_set_at`).
- Farming and smurfs are accepted risk for a bragging-rights board; revisit only if abuse appears.
- If rating write loss becomes visible in production, prefer a small outbox over complicating the
  apply path.
