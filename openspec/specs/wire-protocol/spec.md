# wire-protocol Specification

## Purpose

Define the sole client–server game and account wire contract, per-viewer visibility redaction, live stream framing, and expand-only compatibility rules so concurrent binaries remain parseable during rolling deploys.

## Requirements

### Requirement: Protocol buffers are the sole wire contract
All messages exchanged between API and clients for auth, decks, ratings, catalog, game stream/intents, and table seed SHALL be native protocol-buffer messages under a versioned package path. The protocol MUST NOT use JSON-in-string escape hatches for game trees, intents, decks, cards, or seed payloads. Generated bindings on API and client sides SHALL be regenerated from the same `.proto` sources after contract changes.

#### Scenario: Intent carries structured payload
- **WHEN** a client submits a game intent
- **THEN** the payload is a native intent envelope message, not a serialized string blob inside a generic field

#### Scenario: Codegen follows proto change
- **WHEN** a field or RPC is added to the contract
- **THEN** API and client bindings are regenerated from the proto sources before the change ships

### Requirement: Named services cover auth, decks, ratings, catalog, game, and seed
The contract SHALL expose Buf STANDARD `*Service` service names for authentication, deck CRUD, ratings leaderboard, card catalog/search/lookup, live game stream and intent/yield controls, and table seed. Service renames change gRPC paths and SHALL be treated as hard breaks. Card catalog RPCs MAY be unauthenticated; deck, ratings, intent, and yield RPCs SHALL require authentication. Table seed SHALL be callable by the same-origin backend on behalf of a resolved lobby, not by browsers as a direct public game-join API.

#### Scenario: Catalog search needs no session
- **WHEN** a client searches the card catalog
- **THEN** the search succeeds without an authenticated session

#### Scenario: Intent submission requires auth
- **WHEN** an unauthenticated caller submits a game intent
- **THEN** the request is rejected

### Requirement: Browser reaches the API through a same-origin backend
Browsers SHALL speak a same-origin RPC surface to the web backend. The backend SHALL terminate the session cookie and forward the resolved session token as gRPC metadata to the API. The live game stream SHALL be a server-streaming RPC on the API, bridged by the backend to a browser-safe server-push channel. Health probes MAY live on a separate HTTP port from gRPC. Native WebSocket is not part of the protocol.

#### Scenario: Session cookie never leaves the backend
- **WHEN** the backend dials the API for an authenticated call
- **THEN** it sends the session as gRPC metadata and does not forward the browser cookie beyond the same-origin boundary

#### Scenario: Stream connect failure before first event
- **WHEN** the backend cannot establish the game stream
- **THEN** the failure is observable as an HTTP-shaped error before any stream event is delivered

### Requirement: Per-viewer redaction happens before bytes leave the API
The rules engine SHALL emit full-information events and game state and remain audience-unaware. A projection layer SHALL map those to a per-viewer visible state and visible events, stripping or blanking facts the viewer must not see, before any response leaves the API process. Spectators and eliminated or non-seated observers SHALL receive the public projection (viewer sentinel 255): public zones and counts only — no hand or library identities.

#### Scenario: Opponent hand is count-only
- **WHEN** player A views the board while player B holds cards
- **THEN** A's visible state includes B's hand count but no object identities for B's hand cards

#### Scenario: Private draw event hides card from others
- **WHEN** a player draws a card
- **THEN** only that player receives the card identity on the drawn event; other viewers and spectators see a redacted form

#### Scenario: Private pending choices reach only the awaited seat
- **WHEN** the engine pauses on a private library-top, search, or discard choice
- **THEN** the pending-choice view is emitted only to the awaited seat

### Requirement: Visible state is a complete redacted board snapshot
Each viewer's visible state SHALL carry turn structure, per-seat public player views (life, commander tax and damage, hand and library counts, mana pool, username, public avatar hash without email), every object visible to that viewer, the stack (bottom-first) with labels and targets, combat declaration state including attackers that remain blocked after blockers leave, optional pending choice, the viewer's own legal actions (empty for spectators), priority/yield flags, stack-hold countdown, and whether pre-game mulligans are in progress. During mulligans, each player view SHALL also carry public mulligan status fields while card identities remain private under ordinary hand redaction.

#### Scenario: Spectator actions list is empty
- **WHEN** a spectator projection is built
- **THEN** `actions` is empty and hand/library contents appear only as counts

#### Scenario: Blocked attackers survive blocker departure
- **WHEN** an attacker became blocked and its blockers later leave combat
- **THEN** the combat view still lists that attacker among blocked attackers for the rest of combat

### Requirement: Stream opens with snapshot then self-sufficient deltas
A connecting client SHALL receive an initial snapshot frame at the current sequence number, then delta frames and heartbeats. Each delta SHALL carry a monotonic sequence watermark, a batch of already-redacted visible events, the viewer's complete visible state after those events, and optional auto-action notices for forced or automatic submissions in the frame. Clients SHALL fold by replacing the board from state and appending events to the log without a mid-stream snapshot refetch. On reconnect after a sequence gap, the client SHALL open a new stream and treat the opening snapshot as resume. Heartbeat frames SHALL exist to prevent edge-proxy idle timeouts and MUST be forwarded on the browser-facing push channel.

#### Scenario: Fresh table opens on snapshot
- **WHEN** a newly seeded table has produced no events yet
- **THEN** the first stream frame is a snapshot at the current sequence

#### Scenario: Delta needs no side refetch
- **WHEN** a client receives a delta envelope
- **THEN** the enclosed visible state is sufficient to render the board without fetching another snapshot

### Requirement: Mulligan progress is snapshot-sourced on the wire
Until explicit mulligan visible-event arms exist on the stream contract, the API MUST NOT emit empty or placeholder mulligan event oneofs. Clients SHALL treat visible-state mulliganing and per-player mulligan status fields as the source of truth for mulligan UI. Keep and mulligan intents SHALL exist as dedicated intent arms; the authenticated seat SHALL be stamped at the projection boundary so a client cannot keep or mulligan for another player by altering the payload.

#### Scenario: Mulligan lifecycle events are omitted from deltas
- **WHEN** a player mulligans or keeps during setup
- **THEN** stream deltas omit mulligan lifecycle event arms and the client's mulligan UI updates from visible-state fields

#### Scenario: Keep stamps the authenticated seat
- **WHEN** a seated player submits keep-hand
- **THEN** the server applies the keep for that authenticated seat regardless of any other seat id in the payload

### Requirement: Intents reference stable action ids and typed answers
Client-to-server game actions SHALL use an intent envelope with one arm per intent kind. Taking a listed action SHALL reference a stable legal-action id from the most recent visible-state actions list; that id SHALL remain valid across subsequent intents while the underlying action stays legal. Acknowledgments SHALL return accepted plus an optional structured reject reason for game-text rejects.

#### Scenario: Reject carries structured game text
- **WHEN** an intent is rejected for a game rule reason
- **THEN** the acknowledgment includes a message reference the client can format, not only a free-form English string

### Requirement: Player-facing game text uses message references
Server-authored player-facing game text (rejects, stack and action labels, pending-choice effect and mode labels, auto-action notices, and catalog ability summaries) SHALL be carried as message references: a stable key, typed params, and optional child references. English prose SHALL live in the client catalog. Card and object names that identify visible objects MAY remain plain strings; hidden names MUST NOT be embedded in message-reference params. Auth, lobby, and deck gRPC status English messages are outside this game-text contract.

#### Scenario: Stack label is a message reference
- **WHEN** a spell is on the stack
- **THEN** its stack label is a message reference rendered through the client catalog

### Requirement: Pending choices project to a stable generic wire shape
The pending-choice view oneof SHALL cover every engine pause the board renders (targets, payments, combat damage, digs, search, edicts, modes, copy target, legend-rule keep, mana color, piles, partition, dredge, and related prompts). Spell-target and ability-target pauses SHALL project as a shared choose-target shape. Repeatable yes/no and draw-up-to loops SHALL use shared generic arms. Choice items SHALL carry display labels so the prompt UI need not join against the object list for visible identity. The choose-copy-target arm SHALL carry a discriminator when the same answer shape is reused for a non-copy primer such as optional put-counter-on-creature.

#### Scenario: Optional put-counter reuses copy-target answers
- **WHEN** the engine pauses on optional put-counter-on-creature
- **THEN** the pending choice uses the choose-copy-target arm with the put-counter discriminator set so clients swap prompt wording without a new answer shape

### Requirement: Legal actions carry section, payment previews, and mana-only
Each action view SHALL identify kind, optional source object, section bucket, label, targeting and cost-payment choice lists as applicable, auto-tap preview object ids for mana payment, and combat declaration coverage seats for declare-attackers and declare-blockers. Activate actions from graveyard-functional cards SHALL section under graveyard. Paid tap-for-mana modes SHALL appear on the action list flagged mana-only so clients can omit the playable border while still rendering activation-menu rows; free-tap mana sources NEED NOT appear as action views and MAY be discovered from object taps-for-mana instead. Mana-only actions MUST NOT count as meaningful plays that should halt auto-pass.

#### Scenario: Paid filter-land activate is mana-only
- **WHEN** a seat's only legal activate is a paid tap-for-mana mode
- **THEN** the action view sets mana-only true and auto-pass is not blocked solely by that action

#### Scenario: Declare blockers names covered seats
- **WHEN** a player may declare blockers for one or more attacked seats
- **THEN** the action's declare-for list names those seats

### Requirement: Public avatar identity is hash-only
Player views and public lobby seats SHALL expose a gravatar hash (empty meaning monogram fallback) and MUST NOT expose account email on game or public seat projections. Email remains auth-private on the authenticated identity response.

#### Scenario: Seat face without email
- **WHEN** a client paints an opponent seat face
- **THEN** it uses gravatar_hash or a monogram and never receives that player's email on the game stream

### Requirement: Wire changes are expand-only across the drain window
During ordinary rolling deploys, all concurrent API binaries that may still serve in-flight tables SHALL share a parseable protocol. Changes SHALL add only optional fields with new field numbers, and new RPCs or oneof arms that old peers never send. Authors MUST NOT rename, remove, repurpose, or reuse field numbers while older binaries may still serve a table the current client reaches. Expand-only compliance SHALL be machine-checked with Buf STANDARD lint (no silenced rules) and Buf WIRE breaking against the main branch baseline on ordinary pull requests. Intentional hard cuts (service renames, removals, renumbering, incompatible type changes) SHALL use a semver-major pull request whose body includes an Angular `BREAKING CHANGE:` footer, skip the automated breaking check for that release, and prefer a new package path such as `mtgfr/v2` for later hard breaks when practical.

#### Scenario: Additive field is compatible
- **WHEN** a pull request adds a new optional field with a fresh field number
- **THEN** Buf WIRE breaking against main passes and older peers ignore the unknown field

#### Scenario: Field removal requires major hatch
- **WHEN** a pull request removes or renumbers a field without a BREAKING CHANGE marker
- **THEN** the wire verification job fails the breaking check

### Requirement: Hand-maintained client unions stay aligned with generated oneofs
Where the client keeps hand-maintained unions or registries for pending-choice and visible-event kinds, verification SHALL fail when those registries diverge from the generated proto oneof cases after codegen.

#### Scenario: New visible-event arm without registry update fails check
- **WHEN** a new VisibleEvent oneof arm is generated but the client presence registry omits it
- **THEN** the client wire case-coverage check fails
