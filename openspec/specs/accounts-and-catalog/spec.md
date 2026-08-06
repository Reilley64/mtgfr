# accounts-and-catalog Specification

## Purpose

Persistent player accounts, Commander deck ownership and legality, leaderboard ratings, and a searchable card catalog so players can build decks and bring them to a table without shipping the full engine registry to the browser.

## Requirements

### Requirement: Auth sessions

The system SHALL authenticate players with email and password using Argon2id password hashes stored in Postgres `mtgfr`. Signup and login SHALL create or reuse a `Session` whose token is a random hex string (not a JWT) with a fixed TTL of 30 days from creation. The Nitro BFF SHALL terminate the browser `session` cookie (`HttpOnly`, `SameSite=Lax`, host-only; `Secure` when `COOKIE_SECURE=true`) and SHALL forward the raw token to the API as gRPC metadata `x-session-token` on protected calls. Session resolution SHALL delete expired session rows lazily on the next auth attempt with that token. `Auth.Logout` SHALL delete the session row and the BFF SHALL clear the cookie. `Auth.GetMe` SHALL return `Me { id, email, username }` only for the authenticated user; email SHALL remain auth-private and SHALL NOT appear on lobby or game seat payloads.

#### Scenario: Signup creates user, session, and starting rating

- **WHEN** a client calls `Auth.Signup` with a unique email, unique username, and password
- **THEN** the server creates a `User` with Argon2id-hashed password, seeds `rating = 1000` and `rating_set_at` to the current Unix seconds, creates a `Session`, and returns `AuthSession` (token + `Me`), and the BFF sets the `session` cookie

#### Scenario: Login returns a usable session

- **WHEN** a client calls `Auth.Login` with a valid email and password
- **THEN** the server verifies the Argon2id hash, creates or reuses a `Session`, and returns `AuthSession`

#### Scenario: Expired session is rejected and swept

- **WHEN** a protected call presents a session token older than 30 days
- **THEN** the server deletes that session row and returns unauthorized, and the BFF does not treat the caller as signed in

#### Scenario: GetMe is private to the session owner

- **WHEN** an authenticated client calls `Auth.GetMe`
- **THEN** the response includes that user's `id`, `email`, and `username` and no other account's email

### Requirement: Account Gravatar chrome

Account-facing surfaces SHALL derive the signed-in user's Gravatar face from `Me.email` (`trim().toLowerCase()` → SHA-256 hex → `https://www.gravatar.com/avatar/{hash}?s=64&d=404`). Public lobby and game seat payloads SHALL carry only `gravatar_hash`, never email. The system SHALL NOT provide in-app avatar upload, crop, or moderation; account chrome MAY link out to Gravatar for changes.

#### Scenario: Public seats never expose email

- **WHEN** a lobby or live-game projection includes a seated player
- **THEN** the payload includes at most a public `gravatar_hash` and does not include that player's email

### Requirement: Deck CRUD and ownership

The `Decks` gRPC service SHALL be auth-gated. A user-owned deck SHALL persist `(name, commander, commander_print, cards)` where `cards` is a JSON blob of `DeckCardEntry` values (`id`, `count`, `print`) read and written as a whole. Print (Scryfall card UUID) SHALL be required on every line and on the commander. `Decks.List` SHALL return the authed user's DB decks plus all precon summaries. `Decks.Get` SHALL resolve negative ids to precon fixtures and positive ids to Postgres rows owned by the authed user. `Decks.Delete` SHALL refuse precon (negative) ids and SHALL delete only rows owned by the authed user. Create and update SHALL run Commander legality validation before persisting; on failure the RPC SHALL return every legality problem joined by newline and SHALL NOT partially save.

#### Scenario: Create rejects an illegal deck with every problem

- **WHEN** an authenticated user saves a deck that violates multiple legality rules
- **THEN** the server returns an error listing all problems and writes no deck row

#### Scenario: List interleaves owned decks and precons

- **WHEN** an authenticated user calls `Decks.List`
- **THEN** the response includes that user's decks and every precon summary with its fixed negative id

#### Scenario: Delete refuses a precon id

- **WHEN** an authenticated user calls `Decks.Delete` with a negative precon id
- **THEN** the server refuses the delete and the precon fixture remains available

### Requirement: Precon virtual decks

The server SHALL offer ten compile-time precon decks with fixed ids `-1` through `-10`, loaded from `crates/server/fixtures/decks/*.json` via `include_str!`. Precons SHALL NOT be DB rows and SHALL NOT be editable or deletable. `is_precon(id)` SHALL be true for any `id < 0`. Negative ids SHALL never collide with Postgres autoincrement deck ids.

#### Scenario: Every user can take a precon without seeding

- **WHEN** any authenticated user lists decks or selects a precon id for a lobby seat
- **THEN** the precon resolves from the baked fixture without a per-user DB row

### Requirement: Commander legality

`legality::validate` SHALL collect every problem before returning and SHALL run on every deck create/update and again at game start. Validation SHALL enforce: commander exists in `cards::registry()` as a legendary creature or legendary planeswalker; `commander_print` is a non-empty valid Printing UUID; main-deck card count equals exactly 99; every card id exists in the registry; every card `print` is a non-empty valid Printing UUID; singleton except basic lands (`is_basic`); every card's color identity is a subset of the commander's color identity (union of color symbols in cost and rules text).

#### Scenario: Legendary planeswalker may command

- **WHEN** a deck names a legendary planeswalker as commander with an otherwise legal 99-card list
- **THEN** legality validation accepts the commander type

#### Scenario: Basic lands may repeat; other cards may not

- **WHEN** a deck lists two copies of a non-basic card or an off-identity card
- **THEN** legality validation reports the singleton or color-identity problem among any other problems

### Requirement: Leaderboard ratings

`Ratings.GetLeaderboard` SHALL be auth-gated via `x-session-token`. Request `limit == 0` SHALL default to `50`; any higher value SHALL be capped at `100`. The server SHALL order users by `rating DESC`, then `rating_set_at ASC`, then `id ASC`, and SHALL return 1-based global `rank` values plus `total` before paging. The BFF SHALL expose `GET /api/rpc/ratings/leaderboard?limit=&offset=` (GET-only); invalid non-`u32` query values SHALL return HTTP 400 before gRPC. Signup SHALL seed `rating = 1000` so new accounts appear on the leaderboard immediately.

#### Scenario: Paged ranks are global

- **WHEN** a client requests the leaderboard with `offset = 25` and a valid limit
- **THEN** the first entry's `rank` is `26` and entries follow the global sort order

#### Scenario: Invalid paging is rejected at the BFF

- **WHEN** a client calls the leaderboard route with a non-`u32` `limit` or `offset`
- **THEN** the BFF returns HTTP 400 and does not call gRPC

### Requirement: Card catalog projection and search

On API boot the server SHALL truncate and reinsert `catalog_cards` from `cards::registry()` so the catalog tracks the binary. Each row SHALL store a lowercased `search_blob` (name, kind, subtypes, every `sets` code, colors, keywords, Scryfall oracle-tag slugs in hyphenated and space-separated forms) plus the card's full wire JSON. `Cards.Search` SHALL tokenize `q` on whitespace, AND `LIKE '%token%'` against `search_blob`, cap `limit` at 200, and honor `offset`. `Cards.Lookup` SHALL return `CatalogCard` rows for a list of ids. `Cards.Catalog` MAY return the full dump. Catalog search and lookup SHALL NOT require authentication. `CatalogCard.sets` SHALL list every Scryfall set code with a printing; legacy field `set` SHALL remain on the wire for compatibility and MAY be emitted empty.

#### Scenario: Search is tokenized and capped

- **WHEN** an unauthenticated client searches with multiple whitespace-separated tokens and `limit` above 200
- **THEN** results match all tokens in `search_blob` and the effective limit is at most 200

#### Scenario: Lookup hydrates a saved deck

- **WHEN** a client calls `Cards.Lookup` with the card ids from a saved deck
- **THEN** the response includes those cards' catalog rows without requiring a full catalog fetch

### Requirement: Card and printing identity

Card id SHALL be the Scryfall oracle id (rules identity) used in deck lines, `CatalogCard.id`, and related projections. Printing SHALL be the Scryfall card UUID (art preference only) and SHALL be required on every deck line and commander. The engine SHALL remain print-agnostic. Client art URL construction and CDN vs Scryfall origin rules SHALL live in the deck-builder capability; this capability only requires that prints are stored and validated.

#### Scenario: Deck line without print is illegal

- **WHEN** a save or game-start validation sees an empty or invalid print UUID on a line or commander
- **THEN** legality validation includes that print problem in the returned list

### Requirement: Persistence boundary

Accounts, sessions, decks, ratings fields on `User`, and `catalog_cards` SHALL live in Postgres `mtgfr` via Toasty models and migrations. `push_schema()` SHALL be limited to development and SQLite tests; production SHALL apply Toasty migrations before serving. Deck sharing between users, external deck-URL import, fuzzy/full-text ranking beyond AND-of-LIKE, sliding session expiry, and multi-commander (Partner) decks SHALL remain out of this capability's contract.

#### Scenario: Production uses migrations not push_schema

- **WHEN** the API serves production traffic against Postgres `mtgfr`
- **THEN** schema comes from applied Toasty migrations and the process does not rely on `push_schema()` to create account or deck tables
