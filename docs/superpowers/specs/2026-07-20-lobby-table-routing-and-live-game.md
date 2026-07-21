# Lobby, Table Routing, and Live Game

**Status:** Current (as of 2026-07-21)
**Module:** `crates/server/src/table.rs`, `crates/server/src/lobby.rs`, `crates/server/src/session.rs`,
`crates/server/src/game_loop.rs`, `crates/server/src/stream.rs`, `crates/server/src/chrome.rs`,
`crates/server/src/health.rs`, `crates/server/src/main.rs`,
BFF `client/src/server/` (SolidStart lobby + `table_routes`)

---

## Problem Statement

A Commander game requires 2–4 players to gather before the game starts. Each player must choose
a deck, and the host must trigger the start. Once started, all players need a live, push-based
channel to receive game state updates without polling. The game state is authoritative and pure —
it must not be interrupted by a rolling deploy, and game progress must survive brief client
reconnects. Simultaneously, a rolling deploy may leave multiple API binary generations alive;
in-flight games must remain on the pod that owns them (in-memory), while new games land on the
newest pod only.

---

## Solution

The system separates pre-game lobby from live-game concerns across two persistence boundaries:

- **Pre-game lobby** lives on the SolidStart BFF (`edh-web`) against Postgres `mtgfr_web`
  (Drizzle), entirely outside the API pod.
- **Live game** lives in the API pod's in-memory `Registry` (lobby-table-routing-and-live-game spec). No game state is
  persisted; the game is lost if the pod restarts.

**Lobby flow (BFF-owned):**

1. User creates or joins a lobby on `mtgfr_web` (SolidStart Effect RPC, Drizzle/Postgres).
2. Each user claims a seat (bound to their account via session cookie) and picks a deck.
3. Each user toggles ready. The host (first to join) sees the Start button when ≥2 seats are
   claimed and all claimed seats are ready.
4. The host clicks Start. The BFF calls `Tables.Seed` (gRPC, Service `edh-api`) on the
   **newest** active API pod. The seed request carries seat order, user ids, and deck ids.
5. `Tables.Seed` resolves and validates all decks, fetches a drand beacon master seed (or a configured fixed seed in dev/test), deals opening hands, enters the mulligan phase, inserts the
   `Table` into the in-memory `Registry`, and returns `SeedResponse { table_id, pod_dns, version }`.
6. BFF writes `table_routes (table_id → pod_dns)` to `mtgfr_web`. Clients are redirected to
   `/play/{table_id}`.

**In-game routing (lobby-table-routing-and-live-game spec):**

All in-game requests carry `table_id` in the URL path. The BFF looks up `table_routes` in
`mtgfr_web` → `pod_dns` → proxies to `http://{pod_dns}:8080` (headless Service
`edh-api-headless`, `publishNotReadyAddresses=true`). Terminating pods remain reachable on the
headless Service for the duration of their drain. No affinity cookie; `table_id` in the path is
the sole routing key.

**In-memory Registry and Table (lobby-table-routing-and-live-game spec):**

One `Registry` per API pod process (a `std::sync::Mutex<HashMap<String, Table>>`). Table ids
are random 128-bit hex strings (unguessable). Each `Table` holds:
- The `engine::Game` (pure, deterministic state machine).
- The 32-byte master seed and `beacon_round` used to derive every engine random operation (`beacon_round = 0` for configured/test seeds).
- A monotonic `seq` (event counter / stream resume watermark).
- A `tokio::broadcast` channel per table for fan-out to all subscribers.
- `ChromeState` (yield flags, stack-hold, dwell — see below).
- Per-seat `prints` map (Card id → Printing UUID, for `ObjectView` art).
- `quiet_since` (for abandoned-table eviction during drain).

**Stream fan-out (lobby-table-routing-and-live-game spec):**

`tokio::broadcast` per table. The subscribe edge (`crates/server/src/stream.rs`) attaches, reads
the current snapshot at the current `seq`, sends it as a `SnapshotFrame`, then loops receiving
`DeltaEnvelope`s from the broadcast channel and redacting them per viewer before forwarding.
Reconnect re-opens the stream from the current `seq` — no gap replay; reconnect re-snapshots.

**Chrome settle (turn-priority-and-stack spec / turn-priority-and-stack spec / 0029 / 0037):**

After every intent, `TableSession::apply` applies the engine action, then runs a settle loop
(`settle_after_apply`) that:
1. Loops `PassPriority` while the current priority holder has no meaningful action
   (`has_meaningful_action`) and no pending choice, up to 256 auto-passes.
2. Checks stack-hold arm/clear conditions (uncontested stack → arm hold; contested or resolved →
   clear hold).
3. Respects per-seat stack yield (arm via `SetYield`; clear on stack empty) and turn yield (arm
   via `SetTurnYield`; clear on active-player turn start, on intentional action, or on attack
   targeting that seat).
4. Reports auto-actions in `DeltaEnvelope.auto_actions` so the client can label forced plays.

**SIGTERM drain (lobby-table-routing-and-live-game spec):**

On SIGTERM, the server sets `AppState.draining = true` (atomic bool). The drain behavior:
- `Tables.Seed` returns 503 immediately.
- Existing tables continue serving; the drain loop polls `Registry.active_table_count()` and
  `Registry.evict_abandoned()`.
- Tables with no `Game.Stream` subscribers for more than `ABANDONED_TABLE_GRACE` (60 seconds)
  are evicted (ghost tables from closed browsers).
- Process exits when `active_table_count() == 0` or when `terminationGracePeriodSeconds`
  (default 24h in the Argo chart) elapses (SIGKILL).

**Health probes (Axum `:8080`):**

| Endpoint | Behavior |
|----------|----------|
| `GET /health/live` | Always 200; body `{"version": "…"}` |
| `GET /health/ready` | Always 200 while process is up; body `"ok"` |
| `GET /health/drain` | `{"active_tables": N, "draining": bool}` — observation only; not reachable via public tunnel (NetworkPolicy) |

`/health/ready` stays 200 while draining because draining pods still own active tables and
Kubernetes readiness probes should not reroute those connections.

**Idle lobby TTL:** abandoned lobbies (no activity for 30 minutes on `mtgfr_web`) are swept by
the BFF so that their absence does not block drain of an API pod that was never seeded.

---

## User Stories

- As a **host**, I create a lobby; the table id appears in a share link
  (`https://edh.example.com/play/{table_id}`). I claim the first seat, pick a deck, and wait for
  friends to join.
- As a **player joining via share link**, I claim the next open seat, pick a deck from my list
  (which includes precons), and toggle ready.
- As a **host**, once all claimed seats (≥2) are ready, I press Start. The game seeds and all
  players are taken to the board.
- As a **player in-game**, I receive live `DeltaEnvelope` frames as the game progresses. I submit
  intents by clicking cards or actions; the server applies them and fans the result to all seats.
- As a **player** who briefly loses connection, I reconnect within 60 seconds; the stream
  re-opens, I receive a fresh snapshot, and the game continues.
- As a **player on an old API binary**, I continue playing while a new binary is rolled out. My
  game stays on the Terminating pod via headless pod DNS; new games start on the new pod.
- As an **eliminated player**, I continue watching the game as a spectator (public projection,
  `viewer = 255`) until the game ends.

---

## Behavior

### Seed flow (`Tables.Seed` / `lobby::seed_table_core`)

1. Guard: if `AppState.draining`, return 503.
2. Guard: 2..=4 seats; caller must be the host; host must be one of the seats.
3. Guard: `table_id` not already in the registry.
4. Resolve decks for each seat (precon negative ids → static fixtures; positive ids → Postgres,
   guarded to `seat.user_id`). Deck resolution and legality re-validation happen **outside** the
   registry lock — no DB await across the lock.
5. Resolve entropy before inserting the table: `settings.master_seed` or `MTGFR_MASTER_SEED` supplies a fixed 64-hex-character master seed with `beacon_round = 0`; otherwise the API fetches `https://drand.cloudflare.com/public/latest`, retrying across `https://api.drand.sh/public/latest`. The drand `randomness` becomes the `[u8; 32]` engine master seed and `round` is recorded as `beacon_round`.
6. If beacon entropy is unavailable or malformed and no fixed seed is configured, return 503. No partial table is created and there is no silent production fallback to `OsRng`.
7. Under the lock: build `Table::seeded(...)`, record `table.seed` and `table.beacon_round`, fill
   `table.prints` (Card id → Printing UUID per seat), seed game via `decks::seed_game`, call
   `registry.try_insert(table_id, table)`.
8. Return `SeedResponse { table_id, pod_dns: settings.pod_dns, version: settings.version }`.
   BFF writes `table_routes`.

`decks::seed_game` constructs `Game::with_master_seed`, designates commanders, stacks and shuffles each library with per-seat derived RNG, draws seven cards per seat, and calls `begin_mulligans()`. It deliberately does **not** call `begin_first_turn()` at seed time; the first turn begins only after all living seats keep their hands.

### Stream subscribe (`Game.Stream`)

1. Auth: `x-session-token` → `AuthUser`. Spectators (watchers without a seat) are not supported
   on the current stream path — the stream requires auth to identify the viewer's seat for
   redaction.
2. Open `Game.Stream(StreamRequest { table_id })`. Server reads current snapshot at `seq`,
   sends `SnapshotFrame`, then subscribes to `table.tx` broadcast channel.
3. Loop: receive `Broadcast` frame → redact for viewer → send `DeltaEnvelope`. Heartbeat frames
   are sent periodically (SSE keepalive through Cloudflare Tunnel's ~100s idle timeout).
4. On reconnect after a gap: the client re-opens the stream; a new snapshot is sent.

### Intent flow (`Game.SubmitIntent`)

1. Auth: `x-session-token` → `AuthUser`. The user must have a seat at `table_id`.
2. `game_loop::submit_core` → `with_seated_drive` → lock registry → find table → find seat →
   `TableSession::apply(intent)`.
3. `apply` dispatches the engine intent, runs the settle loop (auto-pass, hold arm/clear), then
   publishes a `DeltaEnvelope` to `table.tx` for all subscribers.
4. Returns `Ack { accepted, reason }`. Deltas arrive on the stream, not in the ack.
5. Action log is appended outside the lock (`action_log::append`) — TOON format, written to
   `ACTION_LOG_DIR`.

While the engine is `mulliganing`, clients submit `KeepHand` or `Mulligan` just like other intents. The server does not auto-advance priority or begin turn one until `MulligansFinished` has occurred; disconnected undecided seats remain undecided.

### Yield / dwell (`SetYield`, `SetTurnYield`, `SetStackDwell`)

- `SetYield { enabled }` arms or disarms per-seat stack yield on `ChromeState`. While armed,
  the settle loop auto-passes that seat for the rest of the current stack; the flag clears when
  the stack empties.
- `SetTurnYield { enabled }` arms or disarms turn yield (turn-priority-and-stack spec). While armed, the
  settle loop auto-passes that seat through every priority window until: the seat becomes active
  player, an attacker is declared targeting that seat, or the seat takes an intentional action.
  When armed while the seat is active, this is **End Turn** (turn-priority-and-stack spec).
- `SetStackDwell { dwelling }` sets the per-seat helpless dwell flag (turn-priority-and-stack spec). While any seat
  is dwelling, the stack-hold timer is paused. The hold timer resumes when all dwell flags clear.

### ChromeState (priority chrome)

Owned by `Table.chrome` (type `ChromeState`). Mutated only via `TableSession` (never by gRPC
adapters directly):

| Field | Meaning |
|-------|---------|
| `yields[4]` | Per-seat stack yield (clear on stack empty) |
| `turn_yields[4]` | Per-seat turn yield / End Turn |
| `stack_hold` | Active hold: `(seq, Instant)` when uncontested stack is pausing |
| `stack_dwell[4]` | Per-seat helpless dwell (pauses hold timer) |

`stack_hold_remaining_ms()` on `Table` computes the countdown from `stack_hold` and `any_dwell`.
`publish_hold_tick()` fans hold-countdown updates to subscribers without bumping game `seq` —
this advances `broadcast_seq` only, keeping the countdown display live without generating phantom
deltas.

### Registry lifecycle

| Event | Registry change |
|-------|-----------------|
| `Tables.Seed` (non-draining) | `try_insert(table_id, Table)` |
| Game ends (all players lost) | Table removed by the game loop |
| Abandoned (no subscribers ≥ 60s) | `evict_abandoned()` during drain sweep |
| SIGKILL after grace | Process terminates; registry gone |

`active_table_count()` counts only tables where `table.game.is_some()`. Tables are born with a
seeded game; there are no "empty" table shells in the production registry.

---

## Implementation Decisions

- **In-memory only, no durable games** (lobby-table-routing-and-live-game spec): `SavedGame` / `SavedIntent` / persist module
  deleted. The registry is the sole home of live games. `UnknownAction` from a reconnect client
  means a genuinely stale action id, not a replay gap.
- **BFF-owned lobby** (lobby-table-routing-and-live-game spec): pre-game state lives on `mtgfr_web` (Drizzle). No lobby
  tables in the API pod; no lobby fan-out needed across pods. The BFF owns seat claim, ready-up,
  host start, and `table_routes`.
- **table_routes for pod affinity** (lobby-table-routing-and-live-game spec): no affinity cookie, no ConfigMap peer registry.
  `table_id` in the path is the routing key; `pod_dns` in `mtgfr_web.table_routes` is the
  destination. Headless Service `publishNotReadyAddresses` keeps Terminating pods dialable.
- **Seed outside lock, insert under lock** (`lobby.rs`): deck resolution, DB queries, and beacon entropy resolution happen
  outside the registry mutex (no DB await across the lock). `try_insert` under the lock handles
  a concurrent duplicate-id race.
- **Drand over silent fallback** (`beacon.rs`): production seed calls must get Cloudflare/drand randomness or fail; fixed master seeds are explicit test/dev configuration (`settings.master_seed` or `MTGFR_MASTER_SEED`).
- **Settle loop bounded** (turn-priority-and-stack spec): up to 256 auto-passes per intent, preventing infinite
  loops on engine bugs. The bound is high enough for real priority chains.
- **Abandoned table grace = 60s** (`table.rs`): long enough for a reconnect blip; short enough
  that ghost tables from closed browsers don't pin Terminating pods for the full 24h grace.
  The first no-subscriber sweep *arms* grace from `now` rather than using the seed timestamp,
  so long-running games whose streams briefly drop get the full 60s.
- **`quiet_since = None` = "had listeners"**: subscribe clears `quiet_since` to `None`; the
  first quiet sweep sets it to `Some(now)`. This prevents the seed-era timestamp from causing
  an instant eviction of a game that was actually watched.
- **Poison recovery on registry mutex** (`table.rs::lock`): a panic under the lock (e.g., an
  engine bug) poisons the mutex. `lock()` recovers the guard instead of propagating the poison
  so other tables remain operational.
- **gRPC on `:50051`, health on `:8080`**: tonic and Axum bind to different ports, both on the
  same host. The headless Service exposes `:50051` for BFF in-game dials; the ClusterIP Service
  `edh-api` (active instances only) also exposes `:50051` for seed calls.

---

## Testing Decisions

- `crates/server/src/table.rs` contains unit tests for: `try_insert` duplicate rejection,
  `active_table_count` correctness, poison recovery, abandoned eviction (past grace), live-
  subscriber retention, inside-grace retention, and the previously-watched "arm grace from now"
  invariant.
- `crates/server/src/health.rs` tests: `live` version report, `ready` always 200 while
  draining, `drain` status with zero active tables.
- `crates/server/src/grpc/tests.rs` contains integration tests for `Tables.Seed` (including
  draining rejection, duplicate table id, invalid seat counts, beacon failure, and recorded beacon entropy) and `Game.SubmitIntent` (seated
  vs. non-seated auth).
- `crates/server/src/decks.rs` tests assert seeded games deal opening hands, enter mulligans, and delay the first turn until keeps.
- Engine-level tests (`tests/game.rs`) cover the full game loop: variable players, elimination,
  multiplayer combat, lobby start.

---

## Out of Scope

- **Durable game resume across server restart** (lobby-table-routing-and-live-game spec): explicitly excluded. Players must
  restart the game manually after an API pod restart.
- **Horizontal same-tag replicas sharing a registry**: not in scope. Concurrent Terminating
  pods during a roll are in scope; same-image horizontal scale-out is not (would require Redis
  or a shared broadcast bus).
- **Watcher join (non-seated observers)**: the stream path requires auth and seat resolution.
  An unauthenticated or non-seated user cannot currently open a stream.
- **Table recovery for abandoned lobbies**: if a lobby's `table_routes` row is stale or missing,
  the BFF returns 404 `UnknownTable`. Clients must rejoin via the lobby.
- **Lobby real-time updates (WebSocket / push)**: the lobby is polled or rebuilt on page load,
  not push-updated. The BFF manages lobby state synchronously on requests.

---

## Further Notes

- The `table_id` is a random 128-bit hex string (`format!("{:032x}", rand_u128)`). It is the
  stable public identifier in share links (`/play/{table_id}`) and in `table_routes`.
- `SeedResponse.version` (the API binary's version string) lets the BFF detect a rolling deploy
  crossing versions mid-game — the BFF can surface a "game running on older version" warning
  if desired, though no UI for this currently exists.
- The action log (`crates/server/src/action_log.rs`) writes TOON-format trace files to
  `ACTION_LOG_DIR`. These contain full hidden game state (hand, library order) and must not be
  sent to Loki/observability — they are for local debugging only.
- Stack-hold countdown (`stack_hold_remaining_ms`) reaches the client on every `DeltaEnvelope`
  and every hold-tick publish. The client renders a countdown UI on the Stack view when
  `stack_hold_remaining_ms > 0`.
- Per the CONTEXT.md, a **spectator** is an eliminated player still receiving the stream; a
  **watcher** is a client with no seat. The stream redaction path supports both via
  `Option<PlayerId>` (spectator = `None`), but the current gRPC stream handler requires auth
  and resolves to a seated user — fully unseated watch access is not wired end-to-end.
