# Lobby deck card path + view transitions

**Status:** Implemented  
**Date:** 2026-07-24  
**Module:** `client/app/routes.ts`, `client/app/shell/lobby/**`, `client/app/shell/decks/list/**`, thin navigation VT seam in `client/app/`  
**Related current-behavior specs:** [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)  
**Supersedes (slice):** Lobby non-goals in [deck-list-tile-chooser](2026-07-24-deck-list-tile-chooser-design.md) and [deck-list-tile-layout-polish](2026-07-24-deck-list-tile-layout-polish-design.md) that deferred lobby Bring/`<select>` redesign — this design covers that slice only  
**Approach:** Required deck id in play path params; lobby shows the same commander-card chrome as Your decks; CSS View Transitions between home tile and lobby card

---

## Problem

Players pick a deck on **Your decks** via a rich commander tile, then land on host/join and see only a name (“Bring: …”) or a plain `<select>`. The deck feels discarded between screens. Deck identity also lives in an optional query param (`?deck=`), which fights the project routing rule that required ids belong in path params.

---

## Goals

- Show the chosen deck as a **commander card** on the host/join (and claim-seat) lobby surfaces — same content language as the Your decks tile (art crop, deck name, commander name, color pips, Precon chip).
- Put the deck id in the **path**: `/play/:deckId` and `/play/:deckId/:table`.
- **404** missing, malformed, or not-in-library deck ids (no lobby `<select>`, no name-only Bring strip).
- Use the **CSS View Transitions API** so the home tile and lobby card read as one continuous object when navigating between `/` and `/play/:deckId` (and Back), when the API is available.

## Non-goals

- Commander art on table **seat rows** (seats stay username + `deck_name` text).
- App-wide view transitions on every route change.
- Soft redirects / compatibility for old `/play`, `/play/:table`, or `?deck=` URLs.
- Lobby deck search, multi-deck picker, or changing deck without returning to Your decks.
- Wire / proto / BFF contract changes (join/host/ready still send deck id as today).
- Full shared “deck chooser” abstraction beyond a tiny shared render helper if markup must match for VT.

---

## User stories

- As a player, I click a deck tile on `/` and arrive on `/play/{id}` seeing that same deck as a card, with a smooth morph when the browser supports View Transitions.
- As a player on host/join, I Host or Join with the deck already chosen; to bring a different deck I go **Back** to Your decks and pick another tile.
- As a player opening a bad or unknown play URL, I see the not-found surface — not an empty lobby or a deck dropdown.
- As a player claiming a seat at `/play/{deckId}/{table}`, I see the same deck card before Ready (no select).

---

## Behavior

### Routing

| Route | Path | Meaning |
|-------|------|---------|
| `PlayRoute` | `/play/:deckId` | Host/join entry with required deck |
| `TableRoute` | `/play/:deckId/:table` | Seated lobby with required deck |
| — | `/play`, `/play/:table`, `/play?deck=…` | Not found (hard cut; no redirects) |

- Router keeps `deckId` as a path string; lobby/update parses it to an integer (including negative precon ids).
- Host success navigates to `/play/:deckId/:table` (deck remains in the path; no `?deck=`).
- Join-by-code stays on `/play/:deckId`: pick a deck on `/` first, then enter the table code.
- Inviting others is still **share the table code** (existing copy control). There is no deck-free `/play/:table` URL after this change. A friend opens Your decks, picks their tile (`/play/{theirDeckId}`), pastes the code, joins.
- Deep link `/play/:deckId/:table` means claim (or resume) a seat at that table **with that deck** — useful for refresh/bookmark; only works when that deck id is in the opener’s library (shared precon ids can work across players; custom ids generally do not).

### 404 rules

Treat as `NotFoundRoute` when:

1. The URL has no deck segment (including legacy shapes above), or
2. `deckId` is not a valid integer string, or
3. After the player’s deck library has loaded, that id is not in the library (customs + precons).

Malformed ids can 404 before decks load. “Not in library” waits for the list; while loading, show a brief loading state in the lobby card area (same spirit as today’s “Loading decks…”) — do not flash wrong deck chrome. If the deck disappears mid-session (deleted), treat as 404 once the list reflects that.

Lobby API errors (`UnknownDeck`, table full, etc.) stay as today’s inline humanized messages; they are separate from route 404.

### Lobby UI

- Remove `lobby-deck` `<select>` and `lobby-bring` name strip.
- On entry (host/join) and on claim-seat (before ready), render one **non-interactive** deck card with the same content as Your decks tiles:
  - Commander `art_crop` from `commander_print` / known-commanders default print, or empty glass placeholder
  - Deck name, commander display name, color-identity pips, **Precon** chip when `id < 0`
- Card is not a link/button; **Back** links to `/` to change decks.
- Host / Join / Ready / Start controls and table-code join field remain; seat rows stay text-only.
- Lobby must resolve art/pips the same way the deck list does (reuse `DeckSummary` fields + known-commanders; load known-commanders for lobby if not already available there).

### View transitions

- Home tile and lobby card share one `view-transition-name` keyed by deck id (e.g. `deck-card-{id}`) on a single root wrapper around the card chrome.
- Navigations between `/` and `/play/:deckId` (tile → lobby, and Back when the matching tile is present) run under `document.startViewTransition` when the API exists; otherwise instant navigation (no polyfill).
- Prefer matching structure between tile and lobby card so the morph is stable; extract a tiny shared render helper only if duplication would break name/structure pairing — not a full chooser module.
- Scope is that card pair only for this change.
- Respect `prefers-reduced-motion` (no custom flashy overrides that fight the preference).

---

## State and modules

| Piece | Role |
|-------|------|
| `routes.ts` | `PlayRoute({ deckId })`, `TableRoute({ deckId, table })`; path build/parse; legacy shapes → not found |
| App `update` / lobby enter | `selectedDeckId` from route `deckId`, not `?deck=` search parsing |
| `shell/lobby/view.ts` | Deck card instead of select/Bring; keep host/join/table chrome |
| `shell/decks/list/view.ts` | Tile `href` → `/play/{id}`; `view-transition-name` on tile root |
| Navigation VT seam | Thin helper wrapping navigations that should animate the card pair |
| Known commanders | Available on lobby path so card art/pips resolve |

No wire / proto / BFF schema changes.

---

## Testing

- **Routes:** `/play/:deckId` and `/play/:deckId/:table` parse/build; `/play`, `/play/:table`, invalid id → not found.
- **Scene (shell):** Valid deck lobby shows deck card testids; **no** `lobby-deck` / `lobby-bring`; Host/Join present; Back → `/`.
- **Scene:** After decks load, unknown `deckId` shows not-found (not lobby).
- **Deck list:** Tile href is `/play/{id}` (replace `?deck=` assertions).
- **Outcome:** Activating a tile lands on lobby with that deck’s card visible.
- **VT helper:** Guard that the navigation path uses `startViewTransition` when available; do not flake Scene tests on animation frames.

Implementation updates [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) (and lobby routing mentions if path shapes appear there) to describe path-param play routes and lobby deck card as **current** behavior.

---

## Spec follow-up

Implementation plan (next) should list concrete file edits, red→green tests, and the shell/lobby Scene coverage above. Do not expand into seat-row art or app-wide VT in that plan unless a follow-up design says so.
