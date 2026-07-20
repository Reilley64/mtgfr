# Accounts, Decks, and Catalog

**Status:** Current (as of 2026-07-20)
**Module:** `crates/server/src/auth.rs`, `crates/server/src/db.rs`, `crates/server/src/decks.rs`,
`crates/server/src/decks_api.rs`, `crates/server/src/legality.rs`, `crates/server/src/precons.rs`,
`crates/server/src/catalog_search.rs`, `proto/mtgfr/v1/catalog.proto`, `proto/mtgfr/v1/mtgfr.proto`

---

## Problem Statement

Players need persistent identities and persistent deck lists to bring to a Commander table.
Before each game, a player selects one of their own decks for the lobby seat they claim. The
server must enforce Commander legality — format rules about commander choice, deck size, singleton
constraint, and color identity — at save time and at game-start time, returning every problem at
once so a human can fix them. Additionally, the client needs a browsable, searchable pool of cards
to build decks from, without shipping the entire engine registry to the browser on every session.

---

## Solution

Three subsystems — auth sessions, deck persistence, and card catalog — compose around a shared
Postgres database (`mtgfr`), accessed via Toasty ORM models (`User`, `Session`, `Deck`).

**Auth** is email + password with Argon2id hashing. Signup and login produce an HttpOnly cookie
(`session`) on the browser via the BFF; the session token also travels as gRPC metadata
(`x-session-token`) from the BFF to tonic on every protected call. Session TTL is 30 days;
expired sessions are lazily swept on the next auth attempt with that token.

**Decks** are fully user-owned data: `(name, commander, commander_print, cards)` where `cards`
is a JSON blob of `Vec<DeckCardEntry>` (`id`, `count`, `print`). Print (Printing UUID) is
required on every line and on the commander; decks are always read and written as a whole.
`legality::validate` runs on every create/update and on game start, returning every problem as a
list so the deck builder can display all errors simultaneously.

**Precon virtual decks** (-1 through -8) are static fixtures baked into the server binary at
compile time via `include_str!` (`crates/server/fixtures/decks/*.json`). They are not DB rows
and cannot be edited or deleted. Negative ids can never collide with the Postgres autoincrement
positive ids of user decks. Every user sees precons in their deck list alongside their own decks.

**Card catalog** is a Postgres projection of the engine's `cards::registry()`, populated on
server boot into the `catalog_cards` table (DDL managed by Toasty migrations; data refreshed by
`catalog_search::project()` truncate + reinsert). Each row holds a lowercased `search_blob`
haystack (name + kind + subtypes + set + colors + keywords + Scryfall oracle-tag slugs) and the
card's full wire JSON. `GET /cards/search` (proto `Cards.Search`) runs a tokenized `LIKE` query
against `search_blob`; `Cards.Lookup` fetches specific cards by id for deck hydration on load.
Neither endpoint requires authentication.

---

## User Stories

- As a **new user**, I sign up with email + password + username; the server hashes my password
  with Argon2id, creates a `User` and a `Session`, and the BFF sets the `session` cookie.
- As a **returning user**, I log in; the existing session (or a fresh one) is set as a cookie.
  My session is valid for 30 days; if it expires, the next request gets a 401 and the BFF
  redirects me to log in.
- As a **deck builder**, I create a new deck, pick a commander, add 99 cards, choose art
  (Printings) for each card and the commander, then save. If my deck is not legal, the server
  returns every problem at once: wrong commander type, wrong count, singleton violation, off-color
  cards, missing prints, unknown card ids.
- As a **deck builder**, I browse the card catalog in the deck builder — searching by name,
  type, color, keyword, subtype, set, or Scryfall oracle-tag slug. Results are capped at 200 per
  query; I paginate by adjusting `offset`.
- As a **deck builder**, I open an existing deck; the client calls `Cards.Lookup` with all card
  ids in the deck to hydrate names, stats, and art without fetching the full catalog.
- As a **player**, I own precon decks (ids -1 through -8) automatically — no signup action
  needed. I can take a precon to a lobby seat without ever building a custom deck.
- As a **player**, I take my custom or precon deck to a lobby seat; the lobby validates that the
  deck belongs to my account (or is a precon) before letting me ready up.

---

## Behavior

### Auth (`Auth` gRPC service)

| RPC | Auth required | Behavior |
|-----|--------------|----------|
| `Signup` | No | Validate email + username uniqueness; Argon2id hash password; create `User` + `Session`; return `AuthSession` (token + `Me`). BFF sets `Set-Cookie: session=<token>; HttpOnly; SameSite=Lax [; Secure]`. |
| `Login` | No | Verify password hash; create or reuse `Session`; return `AuthSession`. |
| `Logout` | Yes (cookie) | Delete the session row; BFF clears the cookie. |
| `GetMe` | Yes (cookie) | Resolve session token → `User`; return `Me {id, email, username}`. |

Session resolution flows:
1. BFF reads `session` cookie from browser request.
2. BFF passes `x-session-token: <value>` as gRPC metadata to the API pod.
3. tonic handler calls `auth::resolve_session_token(db, token)` → `User` or 401.
4. Expired sessions are deleted lazily on resolution failure (not by a background sweep).

Password is argon2id PHC format, stored in `Session.token` is a random hex token (not a JWT).

### Deck CRUD (`Decks` gRPC service)

All five Decks RPCs are auth-gated. `DeckSummary` is the list view (id, name, commander name,
commander print). `DeckDetail` is the full view (id, name, commander, commander\_print, cards).

**Create / Update:** `SaveDeckRequest` → `legality::validate` → Postgres insert or update.
If validation fails, the gRPC call returns an error containing all legality problems joined by
newline; no partial saves.

**List:** Returns `DeckList` with both DB-backed decks (owned by the authed user) and the eight
precon summaries. Precons appear in the list with their fixed negative ids; the client can
display them like any deck.

**Get:** Resolves negative id → precon fixture, positive id → Postgres row (guarded to the
authed user's owned decks). Returns `DeckDetail`.

**Delete:** Refuses negative id (precons are immutable). Deletes the Postgres row if it belongs
to the authed user.

### Commander legality (`legality::validate`)

Validates the full deck at save time and at game start. All problems are collected before
returning; the caller gets the complete list. Invariants checked:

1. `commander` exists in `cards::registry()` as a legendary creature or legendary planeswalker.
2. `commander_print` is a non-empty valid Printing UUID (8-4-4-4-12 hex).
3. Total card count equals exactly 99 (`DECK_SIZE`).
4. Each card id exists in `cards::registry()`.
5. Each card's `print` is a non-empty valid Printing UUID.
6. Singleton constraint: no card appears more than once, except cards with the basic-land
   supertype (`is_basic`).
7. Color identity constraint: every card's color identity is a subset of the commander's color
   identity. Color identity is the union of all color symbols in cost and rules text.

### Precon virtual decks (`precons.rs`)

Eight precons with ids `-1` through `-8` are loaded from `fixtures/decks/*.json` via
`include_str!` at compile time. Each fixture records `commander`, `commander_print`, and `cards`.
Precon names are:

| ID | Name |
|----|------|
| -1 | Silverquill Influence |
| -2 | Prismari Performance |
| -3 | Witherbloom Witchcraft |
| -4 | Lorehold Legacies |
| -5 | Quantum Quandrix |
| -6...-8 | Additional fidelity-grind decks |

`is_precon(id)` returns `true` for `id < 0`. Edit and delete of a precon id returns a 422.
Precon decklists are the same source of truth as the Phase 5.5 legality fixtures
(`fixtures/decks/*.json`, generated from `docs/decklists/*.md`).

### Card catalog (`catalog_search.rs`, `Cards` gRPC service)

On server boot, `catalog_search::project()` truncates `catalog_cards` and reinserts one row per
card in `cards::registry()`. The `search_blob` for each card is a lowercased concatenation of:
name, card type (creature/instant/sorcery/enchantment/artifact/planeswalker/land), set code,
color identity words (white/blue/black/red/green or "colorless"), "legendary" if legendary,
printed subtypes, keywords, and Scryfall oracle-tag slugs (both hyphenated and space-separated
forms).

`Cards.Search` tokenizes the query `q` on whitespace and runs an AND of `LIKE '%<token>%'`
against `search_blob`, with `limit` capped at 200 and `offset` for pagination. Bind placeholders
are dialect-aware (`$1` for Postgres, `?1` for sqlite in tests) via the Toasty raw-SQL escape
hatch.

`Cards.Catalog` returns all cards (full catalog dump, for eventual offline/local use — not the
primary search path).

`Cards.Lookup` accepts a list of card ids and returns their `CatalogCard` rows — the fast path
for hydrating a saved deck without fetching the full catalog.

### Card and Printing identity (accounts-decks-and-catalog spec)

- **Card id** = Scryfall oracle id. The canonical rules identity. Used in deck lines, `CatalogCard.id`,
  `ObjectView.card_id`, and `PendingChoiceView` `ChoiceItem` labels.
- **Printing** = Scryfall card UUID. Art preference only. Required on every deck line and the
  commander. `default_print` on `CatalogCard` is the Scryfall-preferred print from `/cards/named`.
  Precon fixtures stamp explicit Archidekt/SoC Printing UUIDs.
- The engine is print-agnostic. Art resolution (`imageUrlByPrint()`) is CDN-only by Printing UUID;
  missing art is a broken image (no Scryfall image host fallback).

---

## Implementation Decisions

- **Toasty ORM for Postgres** (accounts-decks-and-catalog spec): models `User`, `Session`, `Deck` in
  `crates/server/src/db.rs`. `Deck.cards` is a JSON blob (`Vec<DeckCardEntry>`) — always read
  and written as a whole; no per-card relational queries. `push_schema()` is dev / SQLite-test
  only; production runs Toasty migrations (`just migrate`).
- **Session cookie + `x-session-token` gRPC metadata** (accounts-decks-and-catalog spec): the BFF terminates the
  cookie, passing the raw token as metadata. This means no cookie crosses the same-origin
  boundary; only the BFF knows how to set/clear it. Cookie is `HttpOnly`, `SameSite=Lax`,
  optionally `Secure` (`COOKIE_SECURE=true` in prod), host-only (no `Domain` attribute in prod).
- **Commander validation on every save** (accounts-decks-and-catalog spec): `legality::validate` runs at `Create` and
  `Update`, not deferred to game start. Game start re-validates as a safety check. All problems
  returned at once — not fail-fast — so the deck builder UI can display the complete error list.
- **Precon negative-id convention** (`precons.rs`): avoids any DB migration when new precons are
  added. New precons are committed as fixture JSON files; no user rows need seeding.
- **Catalog as a Postgres projection** (accounts-decks-and-catalog spec): `cards::registry()` (the engine's in-process
  compile-time card pool) is projected into `catalog_cards` on each boot. This means the catalog
  schema tracks the binary — no drift between engine behavior and what the deck builder shows.
  `otags` (Scryfall oracle-tag slugs) are backfilled via `tooling/backfill-otags.mjs` and folded
  into `search_blob` for thematic search (e.g. "ramp", "draw", "removal").
- **Print required everywhere** (accounts-decks-and-catalog spec): a deck line without a print is a legality error.
  This ensures every game object can be rendered with art; the client never needs a fallback-to-
  oracle-text path for art failures in-game.
- **Legendary planeswalkers as commanders** (`legality.rs`): `can_command = def.legendary &&
  matches!(def.kind, Creature | Planeswalker)`. This covers commanders with the rule text "can
  be your commander" (planeswalkers) without a separate flag.

---

## Testing Decisions

- `tests/deck_legality.rs` validates all five `soc` precon decks pass `legality::validate` —
  this is the canonical fidelity bar for the initial faithful deck scope.
- `crates/server/src/legality.rs` contains inline unit tests covering: missing commander in pool,
  wrong commander type, count too low/high, singleton violation, basic-land exemption, off-color
  card, missing/invalid print UUID.
- `catalog_search.rs` tests use sqlite (Toasty test driver) to exercise `project()`, `search()`,
  and `lookup()` without a live Postgres instance, verifying placeholder dialect branching.
- gRPC service-level tests in `crates/server/src/grpc/tests.rs` cover `Auth.Signup`, `Auth.Login`,
  `Decks.Create`, `Decks.List` (including precon interleaving), and `Decks.Delete` with auth
  enforcement.

---

## Out of Scope

- **Deck sharing between users**: decks are strictly user-owned; no public/shared deck concept.
- **Deck import from EDHREC, Moxfield, or Archidekt URLs**: the deck builder is manual + search
  only. Precon decklists are source-of-truth fixtures, not imports.
- **Per-card printing CDN management**: the art CDN (`VITE_CARD_CDN`) is a separate service;
  this module only stores and validates the Printing UUID, not the image bytes.
- **Full-text search ranking or fuzzy matching**: the current search is AND-of-LIKE tokens. No
  relevance scoring, no trigram index, no Scryfall-style operator syntax.
- **Session refresh / sliding expiry**: sessions are 30 days from creation, not from last use.
  Logout deletes the session row; no "remember me" concept beyond the TTL.
- **Multi-commander decks** (Partner, Friends Forever): not currently modeled in `legality.rs`.
  A deck with two commanders would fail "exactly one legendary creature" check.

---

## Further Notes

- `schema::color_identity` in `legality.rs` computes the color identity from a `CardDef` —
  the union of all WUBRG symbols in mana cost and rules text. This matches the MTG comprehensive
  rules definition (CR 903.4) within the engine's simplified color model.
- The `catalog_search::project()` call on boot means a server restart always reflects the current
  engine card pool — no stale catalog entries survive a binary update. The truncate+reinsert is
  cheap for the current pool size (~493 cards).
- `CatalogCard.approximates` is the one-line fidelity note on how a card's engine behavior
  differs from its printed rules. The deck builder displays this to inform players of known gaps.
- `CatalogCard.oracle` carries the printed rules text for deck builder hover/inspect; it is
  absent for vanilla cards and for cards whose oracle text hasn't been recorded in the card TOML.
- Precon fixture JSON files are also the source of truth for `tests/deck_legality.rs` via
  `include_str!` — changing a precon's list automatically runs it through the legality validator
  on the next `cargo nextest run`.
