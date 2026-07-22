# Wire Protocol and Visibility

**Status:** Current (as of 2026-07-20)
**Module:** `proto/mtgfr/v1/`, `crates/schema/`, `crates/server/src/grpc/`, `client/lib/wire/`

---

## Problem Statement

Four players share a single authoritative game state, but each player may see only what the rules
allow: their own hand and library, all public zones, and opponent data at the count level only.
Additionally, the client must receive live updates without polling, remain recoverable after a
network blip, and survive rolling deploys where the SPA may momentarily talk to an older API pod
running an older wire schema.

Two secondary constraints shape the protocol architecture:
- The browser cannot speak gRPC directly — it must go through a same-origin BFF.
- The game stream is a long-lived server-push channel; intermediate proxies (Cloudflare Tunnel)
  impose idle-timeout limits and may buffer responses.

---

## Solution

`.proto` files under `proto/mtgfr/v1/` are the **sole wire contract** (wire-protocol-and-visibility spec). The five files
(`common.proto`, `catalog.proto`, `intent.proto`, `stream.proto`, `mtgfr.proto`) define every
message type exchanged between server and client. No JSON-in-string escape hatches exist in the
protocol — all trees are native protobuf.

**Transport layer:**

| Hop | Transport |
|-----|-----------|
| Browser → BFF | `@effect/rpc` over HTTP/JSON (same-origin `/api`) |
| BFF → API pod | `@effect-grpc/effect-grpc` (Connect native-gRPC) → tonic on `:50051` |
| Game stream | gRPC server-streaming `Game.Stream` → BFF bridges to SSE on `/api/rpc/.../stream` |

The BFF (Nitro) handles cookie termination: the session cookie never
travels beyond the BFF, and the resolved session token flows as gRPC metadata
(`x-session-token`) to tonic. Health-check HTTP lives on `:8080` (Axum `GET /health/live`,
`/health/ready`, `/health/drain`); all game, auth, deck, and catalog RPCs live on `:50051`
(tonic).

**Per-viewer redaction** happens at the `schema` crate boundary, before any bytes leave the
server process. The engine emits full-information canonical `Event`s and a full-information
`Game` struct; the `schema` crate maps these to `VisibleEvent` and `VisibleState` per viewer,
stripping or blanking fields that viewer may not see. The engine is audience-unaware and stays
pure.

**Stream framing (lobby-table-routing-and-live-game spec / wire-protocol-and-visibility spec):** A connecting client receives an initial `SnapshotFrame`
at the current `seq`, then a sequence of `DeltaEnvelope` frames. Each `DeltaEnvelope` carries:
- a monotonic `seq` (resume watermark),
- a batch of `VisibleEvent`s (for the game log and stack panel),
- the viewer's **complete** `VisibleState` after applying those events.

The client folds deltas in place — replace the board from `state`, grow the log from `events`.
No mid-stream snapshot refetch is needed. On reconnect after a `seq` gap, the client re-fetches
a snapshot via `Game.Stream` (which always opens with a snapshot at the current `seq`). Game
setup emits no events, so the very first frame for a newly seeded table is always a snapshot.

**Heartbeats** (`Heartbeat` frames at regular intervals) keep the Cloudflare Tunnel idle
timeout from dropping the SSE edge stream. Cloudflare Configuration Rules disable response
buffering on `edh.example.com` so frames are not held at the edge.

**Expand-only wire compat** (see `docs/WIRE_COMPAT.md`, which remains the living authoritative
rule): during a rolling deploy, the active SPA may talk to a Terminating API pod that runs an
older binary. All concurrent binaries must share a parseable protocol. This means: additive
optional fields only, new field numbers for new fields, new RPC/intent/event variants that old
peers never send — never rename, remove, or reuse field numbers while older pods serve tables.

---

## User Stories

- As a **player**, I see my own hand cards by name and can cast them; I see opponents' hands
  only as a count — their identities are never sent to my stream.
- As a **player**, I reconnect after a network blip and return to the same board state without
  losing game log history.
- As a **spectator or eliminated player**, I receive the public projection of the game
  (`viewer = SPECTATOR_VIEWER = 255`); hand and library contents appear only as counts.
- As an **operator**, I can roll a new API binary while a game is in progress; mid-game clients
  connected to the Terminating pod continue to receive `VisibleState` and submit intents until
  the game ends.
- As a **developer**, I add a new event variant (e.g. `VisibleEventFoo`) by adding a new field
  number to the `VisibleEvent` oneof in `stream.proto`, regenerating bindings, and shipping API
  + web; old peers that receive the new variant ignore the unknown field silently.

---

## Behavior

### gRPC Services

**`Auth`** — `Signup`, `Login`, `Logout`, `GetMe`. Signup/login return `AuthSession` carrying
the session token; the BFF sets it as an HttpOnly cookie (`session`) on the browser. `GetMe`
resolves the token from `x-session-token` metadata.

**`Decks`** — `Create`, `List`, `Get`, `Update`, `Delete`. All operations require auth. Decks
are owned by the authenticated user; `DeckDetail` carries the full `(id, count, print)` card
list with Printing UUIDs.

**`Cards`** — `Catalog`, `Search`, `Lookup`. No auth required. `Search` accepts a freetext
query `q` plus `limit`/`offset`; `Lookup` accepts a list of card ids for deck hydration.
Results are `CatalogCard` — engine-true stats, keywords, ability summary, printing, and
optional `oracle` and `approximates` fields.

**`Game`** — `Stream`, `SubmitIntent`, `SetYield`, `SetTurnYield`, `SetStackDwell`. Auth
required for `SubmitIntent` and the yield/dwell setters. `Stream` is a server-streaming RPC;
it sends `StreamFrame` (snapshot → deltas → heartbeats). Intent and yield routes carry
`table_id` in the path (not just the body) for BFF routing via `table_routes`.

**`Tables`** — `Seed`. Called by the BFF Start handler, never by the browser directly. Seeds a
new game from a lobby the BFF already resolved; returns `SeedResponse` with `pod_dns` so the
BFF can pin later `table_id` hops to this pod.

### VisibleState

`VisibleState` is the complete, redacted per-viewer board snapshot (wire-protocol-and-visibility spec):

| Field | What it carries |
|-------|-----------------|
| `viewer` | Seat index, or 255 for a spectator |
| `active_player`, `step`, `priority` | Turn structure discriminants |
| `players` | Per-seat `PlayerView`: life, commander tax, commander damage, `hand_count`, `library_count`, mana pool |
| `objects` | Every `ObjectView` visible to this viewer: hand cards (own only), battlefield, stack, graveyard, exile, command zone |
| `stack` | `StackObjectView` list, bottom-first; label and optional target per entry |
| `combat` | `CombatView`: declared attackers with defenders, declared blocks, confirmed flags |
| `pending_choice` | The `PendingChoiceView` the engine is blocked on, if any |
| `actions` | `ActionView` list for this viewer's own legal actions (empty for spectators) |
| `can_act`, `yielded`, `turn_yielded` | Priority / yield state for auto-pass logic |
| `stack_hold_remaining_ms` | Countdown until an uncontested stack auto-resolves |

### Redaction rules

Implemented in `crates/schema/` (`schema::redact` / `schema::snapshot`):

- `CardDrawn`, `SearchedToHand`, `PutFromHandOnTop` — `card` and `from` are `None` for all
  viewers except the player who drew/searched/put.
- `ObjectView` for hand cards — emitted only to the owner; opponents receive `hand_count`.
- Library order is never event-sourced; `library_count` is the only library fact in `PlayerView`.
- `PendingChoiceView` variants that carry private items (Scry, Surveil, SearchLibrary,
  SelectFromTop, DistributeTop, MayDiscard, Discard, PutFromHandOnTop, PutLandFromHand,
  PutCreatureFromHand, ChooseDredge, MayReturnFromGraveyard) — emitted only to the awaited seat.
- Spectator (`viewer = None`) receives the same as a neutral observer: public zones, counts,
  all public events — no hand or library identities.

### PendingChoice

The `PendingChoiceView` oneof has 65 arms (as of 2026-07-20), covering every engine pause:
target selection, optional triggers, cost payments (PayCost, PayOrCounter, PayEchoOrSacrifice,
PayRecoverOrExile, PayCumulativeUpkeepOrSacrifice), combat damage assignment, library-top
operations (Scry, Surveil, SelectFromTop, DistributeTop), search, sacrifice edicts, proliferate,
phase-out choice, mode selection, copy target, mana color choice, and many card-specific variants
(Dance, Piles, Partition, Dredge, Trade Secrets, etc.). The `ChoiceItem` embedded in most
variants carries the item's `label` so the prompt UI does not need to join against the object list.

### Intent wire format

`IntentEnvelope` (in `intent.proto`) is the client-to-server action submission. Its one oneof
arm per intent kind — `TakeAction`, `AnswerChoice`, `PassPriority`, etc. — wraps the minimal
stable-id or answer payload. `TakeAction { id }` references a `LegalAction` id from the most
recent `actions` list in `VisibleState`; stable ids survive across multiple intents as long as
the underlying action remains legal (lobby-table-routing-and-live-game spec).

---

## Implementation Decisions

- **`.proto` as sole contract** (wire-protocol-and-visibility spec): eliminates the utoipa/Orval OpenAPI layer (wire-protocol-and-visibility spec, superseded) and the OpenAPI-generated Effect client (wire-protocol-and-visibility spec, superseded). `crates/schema`
  remains the projection model; `crates/server/src/grpc/map/` converts at the gRPC edge to/from
  native proto. `client/lib/wire/types.ts` holds hand-maintained TypeScript mirror types that
  the BFF maps via `protoMap.ts`.
- **Hard cut for the REST→gRPC migration**: API + web shipped together; in-flight tables could
  drop. After that cut, expand-only proto rules apply (wire-protocol-and-visibility spec, `WIRE_COMPAT.md`).
- **BFF cookie termination**: cookies are host-only on `edh.example.com`; they never cross the
  same-origin boundary. The token moves as gRPC metadata inside the cluster.
- **Snapshot-then-deltas** (lobby-table-routing-and-live-game spec): a `tokio::broadcast` channel per table fans events to
  all subscribers; the subscribe edge redacts per viewer. Reconnect re-snapshots by opening a
  new `Game.Stream` — the initial frame is always a `SnapshotFrame`.
- **Self-sufficient deltas** (wire-protocol-and-visibility spec): each `DeltaEnvelope` carries `VisibleState` so the
  client never needs a side refetch. Render assembly (`schema::snapshot`) lives in the wire
  layer, not in fat engine events.
- **Spectator projection** (wire-protocol-and-visibility spec, partial): `snapshot`/`redact` takes `Option<PlayerId>`;
  `None` = spectator (all hands/libraries hidden, `viewer = SPECTATOR_VIEWER`). Eliminated
  players and signed-in non-seated users receive the spectator projection, not a 403.
- **Codegen lifecycle**: `just server-codegen` (Rust, via `build.rs` → `OUT_DIR`) and
  `bun run gen` (TypeScript via `scripts/gen.sh`) regenerate bindings from `.proto`. Generated
  TS files under `client/lib/wire/generated/` are gitignored and regenerated in-image for
  production builds.

---

## Testing Decisions

- Wire-level tests live in `crates/server/src/grpc/tests.rs` exercising the gRPC service
  handlers against an in-memory SQLite test database.
- Redaction correctness is tested in `crates/schema/` with fixture-driven round-trips
  (`schema::snapshot` + `schema::redact`).
- `PendingChoice` variants are tested via `crates/engine/` unit tests that verify each choice
  kind is raised, answered, and produces the correct events.
- Expand-only compliance is enforced by code review discipline, not an automated checker;
  `WIRE_COMPAT.md` documents the invariants for reviewers.

---

## Out of Scope

- **Multi-language / third-party clients**: the proto contract is available in the repo but no
  client SDK is published or supported.
- **WebSocket transport**: the stream is bridged to SSE at the BFF; native WebSocket is not in
  the protocol.
- **Proto package versioning (`/v2`)**: reserved for hard breaking changes per `WIRE_COMPAT.md`
  §3 ("hard breaks"); no `/v2` exists today.
- **Watcher (non-seated observer) dedicated RPC path**: the spectator projection (`viewer =
  SPECTATOR_VIEWER`) covers both eliminated players and watchers; there is no separate watcher
  join handshake at the proto level.

---

## Further Notes

- `docs/WIRE_COMPAT.md` remains the **living authoritative** rule for roll windows and proto
  authoring habits. This spec is a snapshot; `WIRE_COMPAT.md` is the ops reference.
- `ActionView.auto_tap` carries the battlefield object ids `Game::plan_auto_taps` would tap —
  the client shows a visual preview of which lands would be consumed before the intent fires.
- `VisibleEvent` has ~130 arms (stream.proto). Each arm is a purpose-built message; the large
  oneof is intentional — it avoids a generic "event with arbitrary payload" design and makes
  every event type visible to the compiler and code-generation tooling.
- The `seq` on `DeltaEnvelope` and `SnapshotFrame` is the game's monotonic event counter, not a
  wall-clock timestamp. It is stable across reconnects and is the correct resume watermark.
- Heartbeat frames have no payload; they exist solely to prevent edge proxy idle timeouts.
  The BFF must forward them as SSE comment lines or empty events.
