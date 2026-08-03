# lobby-and-live-game Specification

## Purpose

Gather 2–4 players in a pre-game lobby, seed an authoritative in-memory Commander table on the newest API pod, route live traffic by `table_id` to the owning pod, and push visibility-filtered game updates while rolling deploys drain without killing in-progress games.

## Requirements

### Requirement: Persistence split

Pre-game lobby state SHALL live only on the Nitro BFF (`edh-web`) against Postgres `mtgfr_web` (Drizzle). Live games SHALL live only in the API pod's in-memory `Registry`; the system SHALL NOT persist live game state across API process restart. The BFF SHALL own seat claim, ready-up, host start, and `table_routes` (`table_id` → `pod_dns`).

#### Scenario: Lobby survives without an API table

- **WHEN** players create and join a lobby before Start
- **THEN** seats and ready flags persist in `mtgfr_web` and no API registry entry exists yet

#### Scenario: API restart loses in-memory games

- **WHEN** an API pod process exits while tables are in the registry
- **THEN** those games are gone and clients must start a new game; the system does not resume them from disk

### Requirement: BFF lobby HTTP surface

Lobby and meta HTTP SHALL use one Nitro route file per operation with `export default defineHandler(...)`. Required table identifiers SHALL appear in path params, not request bodies. The shipped routes SHALL include:

| Method + path | Role |
|---|---|
| `POST /api/tables/v1` | Create lobby |
| `GET /api/tables/{table}/lobby/v1` | Lobby snapshot |
| `POST /api/tables/{table}/join/v1` | Join (`{ deck_id }` body) |
| `POST /api/tables/{table}/ready/v1` | Ready (`{ ready }` body) |
| `POST /api/tables/{table}/start/v1` | Host start (empty/`{}` body) |
| `DELETE /api/tables/{table}/route/v1` | Clear table route |
| `GET /api/meta/health/v1`, `GET /api/meta/version/v1`, `GET /api/meta/coverage/v1` | Meta |

`/api/rpc/[...path]` SHALL remain the catch-all for Effect-RPC style API forwarding. Unknown or missing lobbies SHALL return HTTP 404 with a structured lobby view error code `UnknownTable` (not a generic unreachable body).

#### Scenario: Join uses path table id

- **WHEN** a signed-in player joins with `POST /api/tables/{table}/join/v1` and body `{ deck_id }`
- **THEN** the BFF binds the seat using the path `table` param and does not require `table_id` in the body

#### Scenario: Stale table link is UnknownTable

- **WHEN** a client requests lobby state for a table id that is absent or expired
- **THEN** the BFF responds 404 with lobby error code `UnknownTable`

### Requirement: Effect-native lobby store on the BFF

Lobby store operations SHALL be Effect programs that `yield*` a `WebDb` service over `drizzle-orm/effect-postgres` and `@effect/sql-pg`. `withLobbyAuth` SHALL authenticate, annotate the request span, sweep idle rows, provide `WebDbLive`, and accept an Effect body returning `Response`. The remaining Promise-edge caller for in-game dial resolution MAY use `runWebDb` around `lookupTableRoute`. Store mutations SHALL NOT wrap all work in a single SQL transaction: create SHALL retry on unique table-code violation; join SHALL re-read and reconcile seat races; start SHALL delete a freshly written `table_routes` row if marking started fails.

#### Scenario: Auth helper runs Effect store work

- **WHEN** a lobby route handler invokes `withLobbyAuth` with an Effect body
- **THEN** the body can `yield*` lobby-store operations under `WebDb` without a pg-proxy Promise bridge

### Requirement: Lobby lifecycle

A player SHALL create or join a lobby, claim a seat bound to their account, and pick a deck they own or a precon. The host SHALL be the first joiner. Start SHALL require at least two claimed seats and every claimed seat ready. On Start the BFF SHALL call `Tables.Seed` on Service `edh-api` (newest active API pod only), then write `table_routes` and mark the lobby started. Idle lobbies with no activity for 30 minutes SHALL be swept by the BFF. `table_routes` rows SHALL expire after 24 hours and SHALL be swept. Table codes SHALL be six characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ`, regenerated until they contain at least one letter, generated on the BFF.

#### Scenario: Host cannot start until ready quorum

- **WHEN** fewer than two seats are claimed or any claimed seat is not ready
- **THEN** Start does not seed a table

#### Scenario: Start seeds then records pod affinity

- **WHEN** the host starts a ready lobby
- **THEN** the BFF receives `SeedResponse { table_id, pod_dns, version }`, writes `table_routes`, and marks the lobby started

### Requirement: Client play route phases

Browser play URLs SHALL distinguish deck pick, pregame, and in-game phases. Pregame seated lobby SHALL use `/play/{deck_id}/{table_id}`; after start the client SHALL navigate to `/play/{table_id}` only. Server and BFF live routing SHALL key solely on `table_id` once the game is live. Deck choice SHALL remain on seats in `mtgfr_web` and in the seed payload, not in the in-game URL.

#### Scenario: Start strips deck from the path

- **WHEN** lobby poll reports the table started
- **THEN** the client navigates to `/play/{table_id}` while keeping the same `table_id` for routing and stream subscribe

### Requirement: Tables.Seed

`Tables.Seed` SHALL reject with 503 while `AppState.draining` is true. It SHALL require 2..=4 seats, a host who is among those seats, and a `table_id` not already in the registry. Deck resolution and legality re-validation SHALL run outside the registry lock (precon negative ids → fixtures; positive ids → Postgres guarded to `seat.user_id`). Entropy SHALL come from configured `settings.master_seed` / `MTGFR_MASTER_SEED` (64 hex chars, `beacon_round = 0`) or from drand (`https://drand.cloudflare.com/public/latest`, retrying `https://api.drand.sh/public/latest`). If beacon entropy is unavailable and no fixed seed is configured, Seed SHALL return 503 with no partial table and SHALL NOT fall back to `OsRng`. Under the lock the server SHALL build the table, record seed and `beacon_round`, copy seat username and public `gravatar_hash`, fill per-seat prints, seed the game, and `try_insert`. Seeded games SHALL deal BO1-smoothed opening hands and enter mulligans without calling `begin_first_turn()` until all living seats keep.

#### Scenario: Draining pod refuses new seeds

- **WHEN** `Tables.Seed` is called while the pod is draining
- **THEN** the RPC returns 503 and the registry gains no table

#### Scenario: Beacon failure fails closed

- **WHEN** no fixed master seed is configured and drand randomness cannot be fetched or parsed
- **THEN** Seed returns 503 and creates no registry entry

### Requirement: In-game pod routing

All in-game requests SHALL carry `table_id` in the URL path. The BFF SHALL look up `table_routes` in `mtgfr_web`, then dial gRPC at `{pod_dns}:50051` via headless Service `edh-api-headless` with `publishNotReadyAddresses=true`. There SHALL be no affinity cookie; `table_id` is the sole routing key. Seed and non-game API traffic SHALL use ClusterIP Service `edh-api` (newest instance only). Health probes SHALL remain HTTP on `:8080`.

#### Scenario: Mid-roll game stays on Terminating pod

- **WHEN** a table was seeded on pod A and a newer Deployment becomes the `edh-api` selector target
- **THEN** in-game dials for that `table_id` still reach pod A through headless DNS while new seeds go to the newest pod

#### Scenario: Missing route is unknown

- **WHEN** the BFF cannot find a `table_routes` row for a live `table_id`
- **THEN** it treats the table as unknown (404 / `UnknownTable`) rather than guessing a pod

### Requirement: In-memory registry and table

Each API process SHALL hold one `Registry` (`Mutex<HashMap<table_id, Table>>`). Each `Table` SHALL hold the `engine::Game`, 32-byte master seed and `beacon_round`, monotonic `seq`, per-table `tokio::broadcast` fan-out, `ChromeState`, per-seat username/`gravatar_hash`, per-seat prints map, and `quiet_since` for abandon eviction. Registry mutex poison SHALL be recovered so other tables remain usable. `active_table_count()` SHALL count tables with a seeded game.

#### Scenario: Duplicate seed id is rejected

- **WHEN** `Tables.Seed` retries an already-inserted `table_id` on the same pod
- **THEN** insert fails and no second table overwrites the first

### Requirement: Game stream

`Game.Stream` SHALL require auth. A seated user SHALL receive a redaction for their seat; a signed-in non-seated user SHALL receive the public spectator projection (`viewer` absent / spectator path). Subscribe SHALL send a `SnapshotFrame` at the current `seq`, then forward redacted `DeltaEnvelope`s from the broadcast channel, including periodic heartbeat frames to survive Cloudflare Tunnel idle timeouts. Reconnect SHALL re-open the stream and receive a fresh snapshot; the server SHALL NOT gap-replay missed deltas. Unauthenticated clients SHALL NOT open a stream. Hands and libraries SHALL remain private under server-side visibility filtering.

#### Scenario: Reconnect re-snapshots

- **WHEN** a seated client disconnects and re-subscribes to the same `table_id`
- **THEN** the server sends a new snapshot at the current `seq` and continues deltas from there

#### Scenario: Non-seated signed-in watcher is spectator

- **WHEN** an authenticated user with no seat opens `Game.Stream`
- **THEN** they receive the public spectator projection without hand or library card identities

### Requirement: Intent submit and unlock-tail ratings

`Game.SubmitIntent` SHALL require a seated authenticated user. Submit SHALL apply the engine intent, run the settle loop, publish a `DeltaEnvelope`, and return `Ack { accepted, reason }` without embedding deltas in the ack. After an accepted apply, unlock-tail work SHALL append the local TOON action log (when configured) and best-effort persist Elo updates for `Event::PlayerLost` without failing the already-accepted action on DB errors. While mulliganing, clients SHALL submit `KeepHand` or `Mulligan`; the server SHALL NOT begin turn one until mulligans finish.

#### Scenario: Accepted intent fans out on the stream

- **WHEN** a seated player submits a valid intent
- **THEN** subscribers receive a redacted delta and the submit RPC returns an ack without the delta payload

#### Scenario: Rating write failure does not reject the play

- **WHEN** Elo persistence fails after an accepted apply that eliminated a player
- **THEN** the ack and stream fan-out still succeed and the failure is logged as a warning

### Requirement: Priority chrome settle

After every intent, `TableSession::submit` SHALL run a bounded settle loop (at most 256 auto-passes) that auto-passes priority holders with no meaningful action and no pending choice, arms/clears uncontested stack-hold, and respects per-seat stack yield (`SetYield`), turn yield / End Turn (`SetTurnYield`), and stack dwell (`SetStackDwell`). Hold-countdown ticks MAY advance `broadcast_seq` without bumping game `seq`. Auto-actions SHALL be reported on `DeltaEnvelope.auto_actions`.

#### Scenario: Stack yield clears when the stack empties

- **WHEN** a seat has stack yield armed and the stack becomes empty
- **THEN** that seat's stack yield flag clears

### Requirement: SIGTERM drain and health

On SIGTERM the API SHALL set `draining=true`, refuse new seeds with 503, continue serving existing tables, evict tables with no stream subscribers for ≥60 seconds (`ABANDONED_TABLE_GRACE`), and exit when `active_table_count() == 0` or when `terminationGracePeriodSeconds` elapses. The first no-subscriber sweep SHALL arm `quiet_since` from now (not seed time) so briefly disconnected watched games get a full grace. `GET /health/live` SHALL return 200 with version and faithful card counts; `GET /health/ready` SHALL remain 200 while draining; `GET /health/drain` SHALL report `{ active_tables, draining }` for operators and SHALL NOT be exposed on the public tunnel.

#### Scenario: Ready stays up while draining

- **WHEN** a pod is draining with active tables
- **THEN** `/health/ready` still returns 200 so Kubernetes does not prematurely cut owned in-game traffic

#### Scenario: Abandoned ghost tables free the drain

- **WHEN** a draining pod has a table with no subscribers for more than 60 seconds after grace armed
- **THEN** `evict_abandoned` removes it so drain can complete

### Requirement: Seat face privacy on seed

The BFF SHALL derive and store lobby `gravatar_hash` from the authenticated user's email and SHALL forward only that hash in `Tables.Seed`. API table chrome and streams SHALL never receive or expose seat email.

#### Scenario: Seed seats carry hash only

- **WHEN** the BFF calls `Tables.Seed`
- **THEN** each `SeedSeat` includes username and `gravatar_hash` and does not include email
