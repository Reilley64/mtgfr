# Lobby Entry UI

**Status:** Current (as of 2026-07-25)
**Module:** `client/app/shell/lobby/**`, `client/lib/lobby/client.ts`, `client/lib/lobby-store.ts`

---

## Problem Statement

After choosing a deck, a player must host or join a table, claim a seat, ready up, and wait for the host to start — with clear chrome for table codes, seat colors, and a clean handoff from the home deck tile into the play route — without flashing the wrong lobby surface during Host create→redirect.

---

## Solution

Lobby UI lives under `client/app/shell/lobby/**` on path-param play routes (`/play/:deckId` entry, `/play/:deckId/:table` seated/board). Host/Join uses `entryMode` (`choose` | `join`); seated chrome shows seat-color dots, Ready/Start, and table-code copy. Polling is a Foldkit subscription over `lobbyPoll`. Server lobby/seed/affinity mechanics are owned by [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md); the route table and auth guard by [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).

---

## User Stories

- As a player, I visit `/play/:deckId`, choose Host or Join for the selected deck, optionally paste a copied table code in the focused Join panel, ready up, and wait for the host to start.
- As a host, after creating a table I land on `/play/:deckId/:table` without seeing claim-seat chrome flash on the entry route.
- As a player, I can copy the table code (or fall back to a manual-copy input if clipboard permission is denied) and unlock table audio by pressing Ready.

---

## Behavior

### Path-param play routes (UI side)

Required identifiers live in path params (see route table in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)):

| Path | Lobby UI surface |
|---|---|
| `/play/:deckId` | Entry (`surface: "entry"`) — Host/Join choose or focused join panel |
| `/play/:deckId/:table` | Seated lobby / board mount after start |

Legacy `/play`, `/play/:table`, and `?deck=` entry points are Not found (hard cut). Malformed / not-in-library deck ids still 404.

`tableId()` reads the table id from `/play/:deckId/:table` path. `parseTableCode` normalizes bare codes and pasted play URLs, reading the table segment from `/play/:deckId/:table`. `setTableUrl` reflects a joined table into the URL via `history.replaceState`.

`selectedDeckId` is set from the required play route deck id. Lobby paint is route-keyed: `/play/:deckId` always renders the entry surface (`surface: "entry"`), and `/play/:deckId/:table` renders the seated surface — so Host’s create→redirect handoff does not flash claim-seat chrome while `tableId` is already set on the entry route.

Home ↔ `/play/{id}` morphs the shared deck-card chrome with a short FLIP animation (`deck-card-nav.ts`; skipped for reduced motion) — list-side detail in [deck-list-and-builder](2026-07-20-deck-list-and-builder.md).

### Host / Join entry (`entryMode`)

On entry, `entryMode` is `choose` | `join`. **Choose** shows twin destination cards: Host wraps the deck-card chrome (`lobby-deck-card` / `lobby-deck-card-{id}`) and hosts immediately; Join (`lobby-open-join`) opens **join** mode. **Join** replaces the twin row with a focused panel (`lobby-bringing`, `lobby-join-code`, `lobby-join`, `lobby-join-cancel`). Claim-seat and seated lobby still use the deck card + Ready/Start chrome as before. It does not render the old deck `<select>` or `Bring:` name strip in entry or claim-seat states; claim-seat shows **Back** to Your decks without a deck picker.

### Seated lobby chrome

The lobby polls `GET /tables/{table}/lobby` via a Foldkit subscription until `started`. Seat rows show seat-color dots (`seat-forest`, `seat-island`, `seat-mountain`, `seat-arcane`). The host (first joiner) sees a Start button when ≥2 seats are claimed and all are ready. Table-code copy uses `navigator.clipboard.writeText` from an Effect-backed command — denied permission reveals a manual-copy input instead of throwing. `unlockTableAudio()` is called on Ready-up (the required user-gesture unlock for the shared `AudioContext`).

### Lobby poll and table lifecycle (`client/app/shell/lobby/poll.ts`, `client/app/shell/lobby/subscriptions.ts`, `client/lib/lobby-store.ts`)

`lobbyPoll(tableId)` is an Effect stream consumed by a Foldkit subscription. The subscription polls lobby state while a table is present and stops when `started` is true. `client/lib/lobby-store.ts` holds lobby helpers for multi-seat coordination. Once the lobby moves to `started`, the app transitions from the lobby view to the board mount, preserving the table id in the route.

Helpers also live in `client/lib/lobby/client.ts` for table URL / code parsing used by the entry UI.

---

## Implementation Decisions

- **Route-keyed lobby paint** prevents Host create→redirect from flashing seated/claim chrome on `/play/:deckId` while `tableId` may already be set in memory.
- **Clipboard denial is non-throwing.** Failed `navigator.clipboard.writeText` reveals a manual-copy input rather than surfacing an uncaught error.
- **Ready unlocks audio.** `unlockTableAudio()` on Ready-up is the required user-gesture unlock for the shared `AudioContext` ([table-audio](2026-07-20-table-audio.md)).

---

## Testing Decisions

- `client/app/shell/lobby/**/*.test.ts` — lobby stories and helpers (Host/Join entry, seated chrome, poll).
- `client/lib/lobby-store.test.ts` — lobby state helpers.
- Scene assertions for lobby entry / seated surfaces live with shell Scene coverage (`just client-check`).

---

## Out of Scope

- Server lobby tables, seed affinity, and drain ([lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)).
- Full route/auth table and portrait gate ([shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Deck list tile grid and builder ([deck-list-and-builder](2026-07-20-deck-list-and-builder.md)).
- Cross-browser clipboard fallbacks beyond the existing try/catch reveal pattern.

---

## Further Notes

- **The lobby is on `mtgfr_web`** (Nitro BFF / Drizzle / Postgres `mtgfr_web`), not `mtgfr` (Toasty / game/user data). `just client-migrate` applies Drizzle migrations; `just migrate` applies Toasty migrations. Both must run before DB-touching work.
- Idle lobby TTL (30 minutes on `mtgfr_web`) is an ops/BFF concern documented under [production-topology-and-operations](2026-07-20-production-topology-and-operations.md) Further Notes / [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md).
