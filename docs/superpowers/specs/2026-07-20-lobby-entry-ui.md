# Lobby Entry UI

**Status:** Current (as of 2026-07-27)
**Module:** `client/app/shell/lobby/**`, `client/app/domain/lobby/client.ts`, `client/app/domain/lobby-store.ts`

---

## Problem Statement

After choosing a deck, a player must host or join a table, claim a seat, ready up, wait for the host to start, or stay as a watcher — with clear chrome for table codes, stale table links, seat colors, and a clean handoff from the home deck tile into the play route — without flashing the wrong lobby surface during Host create→redirect.

---

## Solution

Lobby UI lives under `client/app/shell/lobby/**` on path-param play routes (`/play/:deckId` entry, `/play/:deckId/:table` seated pregame, `/play/:table` table-scoped table/game links). The lobby submodel boundary lifts through `GotLobbyMessage`; route entry and post-session cold-load call `informRouteChanged` so lobby reset/load stays child-owned while the parent keeps auth redirects and game-slice activation. Entry uses a single Layout C surface: selected deck card on the left, Host as the primary Llanowar action, inline join-code input + ghost Join action, and a ghost Back link; seated chrome still shows seat-color dots, Gravatar/monogram seat faces, Ready/Start, and table-code copy. Entry renders without the enclosing panel so the deck card and action stack breathe inside the shell stage, while seated lobby keeps the panel chrome. Polling is a Foldkit subscription over `lobbyPoll`. Server lobby/seed/affinity mechanics are owned by [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md); the route table and auth guard by [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md). Face behavior follows [Gravatar Seat Faces Design](2026-07-25-gravatar-seat-faces-design.md).

---

## User Stories

- As a player, I visit `/play/:deckId`, review the selected deck, Host immediately, or paste a copied table code into the inline Join row, ready up, and wait for the host to start.
- As a host, after creating a table I land on `/play/:deckId/:table` without seeing claim-seat chrome flash on the entry route.
- As a player, I can copy the table code (or fall back to a manual-copy input if clipboard permission is denied) and unlock table audio by pressing Ready.
- As a signed-in watcher, I can stay on a table link without claiming a seat and understand that the game will open in spectator view when it starts.
- As a player following an old table link, I see stale-link copy that asks me to get a fresh code from the host.

---

## Behavior

### Path-param play routes (UI side)

Required identifiers live in path params (see route table in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)):

| Path | Lobby UI surface |
|---|---|
| `/play/:deckId` | Entry (`surface: "entry"`) — deck-left Layout C with Host primary and inline Join |
| `/play/:deckId/:table` | Seated lobby / board mount after start |
| `/play/:table` | Table-scoped seated lobby / board mount for generated table codes containing at least one letter |

Bare `/play` and `?deck=` entry points are Not found (hard cut). Single-segment `/play/...` paths normalize by segment shape: integer-looking segments stay deck-entry routes, while table codes become table-scoped routes. Minted table codes are six characters from `23456789ABCDEFGHJKMNPQRSTUVWXYZ` and are regenerated until they contain at least one letter, so generated lobby ids cannot collide with numeric deck ids. Malformed / not-in-library deck ids still 404.

`tableId()` reads the table id from either `/play/:deckId/:table` or `/play/:table`. `parseTableCode` normalizes guest input into a table id: bare codes (uppercased), pasted `/play/:deckId/:table` pregame links, pasted `/play/:table` in-game share links, and legacy `?table=` query params. Ambiguous paths (`/play/`, three or more `/play/...` segments, or unparseable URLs) return `null`. Pregame two-segment paths take precedence when both shapes could apply. `setTableUrl` reflects a joined table into the URL via `history.replaceState`.

`selectedDeckId` is set from the required play route deck id when the route carries one. When the route carries no deck (`/play/:table`), route entry clears `selectedDeckId` to `null` even if the previous play route had a deck selected, so claim-seat/watcher chrome cannot reuse stale deck state. Lobby paint is route-keyed: `/play/:deckId` always renders the entry surface (`surface: "entry"`), while `/play/:deckId/:table` and `/play/:table` render the seated surface — so Host’s create→redirect handoff does not flash claim-seat chrome while `tableId` is already set on the entry route. Once the lobby view flips to `started`, the parent replaces `/play/:deckId/:table` with `/play/:table` and leaves Host create/join handoff on the pregame two-segment path until that start transition happens.

Home ↔ `/play/{id}` morphs the shared deck-card chrome with a short FLIP animation (`deck-card-nav.ts`; skipped for reduced motion) — list-side detail in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md).

### Host / Join entry

On `/play/:deckId` with a selected deck, the lobby stage is a deck-anchored composition (no full-stage enclosing panel): selected deck-card chrome (`lobby-deck-card` / `lobby-deck-card-{id}`) on the left; an action stack on the right with display beat ("Ready to play?"), primary Llanowar **Host a table** (`lobby-host`), soft-inline Join ("Have a code?" + `lobby-join-code` + ghost **Join table** `lobby-join`), and ghost **Back** (`lobby-back`) to Your decks. There is no choose→join mode switch, dashed Join card, Bringing strip, or join-cancel control. Host is the only solid Llanowar CTA; Join and Back use the shared strengthened `buttonClass("ghost")` recipe (`text-snow-mint` on vine border). When the player has no decks or no selected deck, amber copy (`lobby-empty`) points them back to Your decks. Transport errors use `lobby-error` via `alertClass` (burn-red). Seated (`surface: "table"`) may keep the panel wrapper; claim-seat and Ready/Start chrome are unchanged by the entry reflow. It does not render the old deck `<select>` or `Bring:` name strip in entry or claim-seat states; claim-seat shows **Back** to Your decks without a deck picker.

### Seated lobby chrome

The lobby polls `GET /tables/{table}/lobby` via a Foldkit subscription until `started`. The table code keeps `lobby-table-code` and renders with display typography for quick reading, while the copy-code action is a quieter ghost control. Seat rows show seat-color dots (`seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane`) plus a circular face (`seat-face-{player}`) beside the username and deck name. When public `gravatar_hash` is present, the face is a Gravatar image loaded with `d=404`; otherwise it falls back to the username initial / seat number monogram. Lobby seats never carry email. A signed-in user on the seated lobby who has not claimed a seat sees `lobby-watch-note`, explaining that staying on the link enters spectator view when the host starts the game. Ready and actionable Start use the primary Llanowar button treatment; while Start is disabled, `lobby-start-error` shows the gate reason in caution amber (`NeedTwoPlayers` → “Need at least two players.”, `NotAllReady` → “Waiting for everyone to Ready…”). Table-code copy uses `navigator.clipboard.writeText` from an Effect-backed command — denied permission reveals a manual-copy input instead of throwing. `unlockTableAudio()` is called on Ready-up (the required user-gesture unlock for the shared `AudioContext`).

### Lobby poll and table lifecycle (`client/app/shell/lobby/poll.ts`, `client/app/shell/lobby/subscriptions.ts`, `client/app/domain/lobby-store.ts`)

`lobbyPoll(tableId)` is an Effect stream consumed by a Foldkit subscription. The subscription polls lobby state through the app lobby HTTP singleton while a table is present, skips tagged transport/decode failures for that tick, and stops when a successfully decoded `LobbyView` has `started: true`. Lobby commands use the injected `LobbyClient` service for host, join, ready, and start. `client/app/domain/lobby/client.ts` decodes lobby JSON with the `LobbyView` schema and preserves structured lobby bodies, so a 404 response containing a valid `LobbyView` with `error: "UnknownTable"` still reaches `model.error`. A transport-level `LobbyNotFound` maps to the same stale-link copy; `LobbyUnauthorized`, `LobbyBadRequest`, `LobbyDecodeError`, and `LobbyHttpError` map to `Unreachable`. `UnknownTable` tells the player the table link is stale or expired and they should ask the host for a new code. `client/app/domain/lobby-store.ts` holds lobby helpers for multi-seat coordination. Once the lobby moves to `started`, the app transitions from the lobby view to the board mount and replaces seated pregame URLs with the table-only route while preserving the same table id.

Helpers also live in `client/app/domain/lobby/client.ts` for table URL / code parsing used by the entry UI.

---

## Implementation Decisions

- **Route entry is child-owned.** Play-route entry runs through lobby `informRouteChanged`, which calls the child update with route-change messages instead of having the parent mutate lobby state directly.
- **Table-only routes clear deck choice.** `ChangedLobbyRoute` treats an explicit `selectedDeckId: null` as authoritative, so `/play/:table` wipes any prior deck-picked lobby state instead of preserving it.
- **Route-keyed lobby paint** prevents Host create→redirect from flashing seated/claim chrome on `/play/:deckId` while `tableId` may already be set in memory.
- **Seat faces share `seatFace`.** Claimed and open lobby seats use the same Gravatar/monogram helper as account chrome, keyed by public `gravatar_hash` rather than email.
- **Clipboard denial is non-throwing.** Failed `navigator.clipboard.writeText` reveals a manual-copy input rather than surfacing an uncaught error.
- **Ready unlocks audio.** `unlockTableAudio()` on Ready-up is the required user-gesture unlock for the shared `AudioContext` ([table-audio](2026-07-20-table-audio.md)).
- **Lobby HTTP uses `LobbyClient` for commands.** Foldkit subscriptions in this app have an empty resource environment, so `lobbyPoll` uses the app lobby HTTP singleton while command effects read the injected `LobbyClient`.

---

## Testing Decisions

- `client/app/shell/lobby/**/*.test.ts` — lobby stories and helpers (Host/Join entry, seated chrome, poll, `GotLobbyMessage` / `informRouteChanged` route entry, including `/play/:table` clearing stale deck selection and `started` redirecting seated pregame URLs to the table-only path).
- `client/app/domain/lobby/code.test.ts` — `parseTableCode` for both `/play/:deckId/:table` and `/play/:table` path shapes, bare codes, and junk rejection (`/play/`, three+ segments).
- `client/app/domain/lobby-store.test.ts` — lobby state helpers; with `WEB_DATABASE_URL`, asserts
  `loadLobby` on an empty table (requires migrate-applied `gravatar_hash`).
- Layout C lobby entry Scene coverage in `client/app/shell/surfaces.test.ts` and `client/app/shell/lobby/entry.test.ts` asserts `lobby-entry` (no enclosing panel), Host primary (`bg-llanowar`), ghost Join/Back (`text-snow-mint`, not solid Llanowar), and absence of legacy choose/join controls; seated surfaces including `seat-face-0` Gravatar/monogram chrome and the table-only `/play/:table` shell route are covered there too (`just client-check`).

---

## Out of Scope

- Server lobby tables, seed affinity, and drain ([lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)).
- Full route/auth table and CSS landscape rotate ([shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Deck list tile grid and builder ([deck-list-and-builder](2026-07-20-deck-list-and-builder.md)).
- Cross-browser clipboard fallbacks beyond the existing try/catch reveal pattern.

---

## Further Notes

- **The lobby is on `mtgfr_web`** (Nitro BFF / Drizzle / Postgres `mtgfr_web`), not `mtgfr` (Toasty / game/user data). `just client-migrate` applies Drizzle migrations; `just migrate` applies Toasty migrations. Both must run before DB-touching work.
- Idle lobby TTL (30 minutes on `mtgfr_web`) is an ops/BFF concern documented under [production-topology-and-operations](2026-07-20-production-topology-and-operations.md) Further Notes / [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md).
