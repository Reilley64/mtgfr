# Lobby host/join entry redesign

**Status:** Design  
**Date:** 2026-07-25  
**Module:** `client/app/shell/lobby/**` (entry surface on `/play/:deckId` only)  
**Related current-behavior specs:** [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md), [lobby-table-routing-and-live-game](2026-07-20-lobby-table-routing-and-live-game.md)  
**Builds on:** [lobby-deck-card-path-and-view-transitions (design)](2026-07-24-lobby-deck-card-path-and-view-transitions-design.md) (required deck path param + deck-card FLIP)  
**Approach:** Twin destination cards + client-only `choose` / `join` entry modes; no wire or route changes

---

## Problem

The pre-table host/join entry (`/play/:deckId`) stacks “Host a table” and a table-code field + Join as flat primary controls in a form-like panel. Players do not get a clear choice between two destinations, and the screen reads as shell chrome rather than “Arena, Unplugged” game-client UI. Recent work made the chosen deck a commander card on this surface, but Host vs Join hierarchy and polish were left mostly unchanged.

---

## Goals

- Make **Host** and **Join** peer destinations with equal visual weight on entry.
- Host creates a table on one click (same behavior as today).
- Join opens a **focused join panel** (code field, Join CTA, Cancel) instead of always showing the code field.
- Keep the chosen deck visible: Host destination card carries the full deck-card chrome; join mode shows a compact **Bringing** strip (small art + deck name).
- Preserve path-param deck routing, library 404 rules, host/join RPC, and `parseTableCode` behavior.
- Keep the Your decks → lobby deck-card FLIP morph, targeting the deck chrome inside the Host card.

## Non-goals

- Seated lobby (`/play/:deckId/:table`): table-code copy, seat rows, Ready, Start.
- New routes, query params, or soft redirects for legacy URLs.
- Wire / proto / BFF contract changes.
- Deck picker or changing deck without **Back** to Your decks.
- App-wide motion system or heavy staged choreography beyond a light `choose` ↔ `join` swap and existing FLIP.

---

## User stories

- As a player on `/play/:deckId`, I see two equal destinations — Host a table and Join a table — with my deck art on the Host card.
- As a host, I click Host a table and immediately create a table (same as today), then land on the seated lobby.
- As a joiner, I click Join a table, see a focused code panel with a compact Bringing strip for my deck, paste a code, and join.
- As a joiner who mis-taps Join, I Cancel back to the twin destinations without creating or joining a table.
- As a player who wants a different deck, I use Back to Your decks (both modes).

---

## Behavior

### Routing (unchanged)

| Route | Meaning |
|-------|---------|
| `/play/:deckId` | Host/join entry (this redesign) |
| `/play/:deckId/:table` | Seated lobby (out of scope) |

Deck id remains a required path param. Malformed or not-in-library ids still 404 per the lobby deck-card path design.

### Entry modes (client-only)

Lobby slice gains `entryMode: "choose" | "join"` (default `"choose"`).

| Mode | UI |
|------|-----|
| `choose` | Twin destination cards side by side |
| `join` | Focused join panel replacing the twin row |

- Pick **Join a table** → `entryMode = "join"` (no network).
- **Cancel** → `entryMode = "choose"`, clear table code (and clear a join-attempt error if shown).
- Reset to `"choose"` and clear code when leaving the play-entry route or after successful host/join navigation.
- **Host a table** → existing host request immediately (no confirm step).

### Choose mode — twin destination cards

Shell stays: centred felt panel, brand `edh.reilley.dev`, quiet “Lobby” title, version badge, landscape-first. No new nav chrome.

| Card | Content | Action |
|------|---------|--------|
| **Host** | Existing non-interactive deck-card chrome (art crop, deck name, commander name, color pips, Precon chip) + “Host a table” + short helper (“with this deck”) | Whole card is the Host CTA → `RequestedLobbyHost` |
| **Join** | Matching destination tile with a code motif (not deck art) + “Join a table” + helper (“enter a code”) | Whole card → `entryMode = "join"` |

Cards are equal visual weight: inset felt / vine-border destination tiles (game destinations, not a primary button stacked above a text field). **Back** (ghost) below the row links to `/`.

Destination cards render only when a deck is resolved for the route. Loading / empty / pick-a-deck amber gates stay as today’s entry gates before the twin row.

### Join mode — focused panel

Replaces the twin row:

- Compact **Bringing** strip: small art crop + deck name (context that this deck is what you join with; not a second full deck card).
- “Join a table” + short helper (“Paste the code your host shared”).
- Table-code field (same normalize / `parseTableCode` as today, including pasted play URLs).
- Primary **Join table** → existing `RequestedLobbyJoin`.
- **Cancel** → return to `choose`.
- **Back** still available to Your decks.

### Errors

- API failures keep today’s humanized inline `role="alert"` messages under the active mode content (`TableFull`, `UnknownTable`, `UnknownDeck`, `Draining`, `Unreachable`, etc.).
- Stay in the current mode so the user can retry or Cancel.
- Empty/invalid code → no join request (same as today).
- Disable Host / Join submit CTAs while `submitting`.
- Route 404 rules unchanged and separate from lobby API errors.

### Motion

- Light swap animation between `choose` and `join`.
- Your decks ↔ entry FLIP continues; the FLIP target is the deck chrome **inside** the Host destination card (`data-deck-card-flip="{id}"` on that chrome). Join mode does not own the FLIP target.
- Skip swap and FLIP when `prefers-reduced-motion`.

---

## State and modules

| Piece | Role |
|-------|------|
| `shell/lobby/submodel.ts` | `entryMode: "choose" \| "join"` |
| `shell/lobby/messages.ts` | Entry-local messages for open-join / cancel-join (names as implemented) |
| `shell/lobby/update.ts` | Mode transitions; Cancel clears code; reset on leave/success; host/join unchanged |
| `shell/lobby/view.ts` | Twin destination cards + focused join panel; keep seated `tableLobby` as today |
| `deck-card-nav.ts` | FLIP still keyed by deck id; target resolves to Host-card deck chrome |
| Existing lobby client / RPC | Unchanged host/join/ready/start |

No wire / proto / BFF schema changes.

---

## Testing

Assert outcomes, not only presence ([client interaction test policy](2026-07-22-client-interaction-test-policy-design.md)):

- **Scene / surfaces:** `choose` shows Host + Join destination cards; Host card exposes deck chrome / `lobby-deck-card-{id}`; Back → `/`.
- **Scene:** Choosing Join shows Bringing strip, code field, Join table, Cancel; twin cards are gone.
- **Scene / update:** Cancel returns to `choose` and clears code.
- **Update:** Host still requests host; Join submit still requests join with parsed code; `submitting` disables CTAs; `entryMode` resets on leave/success.
- **FLIP:** Existing capture / should-animate / Mount resolution tests still pass with the target inside the Host card.
- **Routes / 404:** Unchanged; no new route cases required for this design.

Out of scope for this change’s new tests: seated-lobby Scene expansions, BFF/integration, wire contract.

Implementation updates [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) so the entry surface description matches this behavior as **current** once shipped.

---

## Spec follow-up

Implementation plan (next) should list concrete file edits, red→green tests, and Scene coverage above. Do not expand into seated-lobby redesign or new routes in that plan unless a follow-up design says so.
