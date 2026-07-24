# Deck list tile chooser (Your decks redesign)

**Status:** Design (approved for planning)  
**Date:** 2026-07-24  
**Module:** `client/app/shell/decks/list/**`, Scene coverage in `client/app/shell/surfaces.test.ts`  
**Related current-behavior spec:** [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) (update in the same implementation change)  
**Approach:** Client-only redesign (no deck-list API / `DeckSummary` schema change)

---

## Problem

The home **Your decks** surface is a plain list row: small art crop, name, always-visible Play/Edit/Delete. It is hard to scan when the library grows, underuses commander art, and does not present a clear product hierarchy between owned decks and read-only precons.

---

## Goals

- Richer **compact tile grid** so each deck reads as a choosable object (commander art first).
- Fast scan via **client-side search** on deck name and commander display name.
- Stable hierarchy: **owned decks first**, **precons last** in **reverse release order**.
- Primary action is Play; Edit/Delete move to a **right-click context menu** (owned decks only).

## Non-goals

- Lobby deck `<select>` / Bring strip redesign.
- Server-side list order or search API.
- Color-identity filter toggles, sort controls, or a shared chooser abstraction for lobby.
- Branding rename (`mtgfr` → `edh.reilley.dev`).

---

## User stories

- As a returning player on `/`, I see my decks as a tile grid with commander art, name, and color-identity pips; I click a tile to go to `/play?deck={id}`.
- As a player with many decks, I type in **Search decks…** and only matching deck/commander names remain; clearing the field restores the full ordered grid.
- As a player, my custom decks appear above all precons; among precons, the newest release appears first (Mirror Mastery before Silverquill Influence).
- As a deck owner, I right-click a custom deck tile to Edit or Delete; Delete still confirms in the existing dialog. Right-clicking a precon does not offer Edit/Delete.

---

## Behavior

### Layout and chrome

- Route stays `/` (`decks-page`). Header: title **Your decks**, username, **Sign out**, **New deck**.
- Body: responsive compact tile grid on felt (`feltClass`), tiles use glass/vine chrome consistent with `DESIGN.md` (interactive tiles, not decorative marketing cards).
- Tile contents:
  - Commander `art_crop` from `commander_print` / `knownCommanders[…].default_print`, or empty glass placeholder.
  - Deck name.
  - Color-identity pips from the looked-up commander’s `color_identity` (display-only).
  - **Precon** chip when `id < 0`.
- No always-visible Play / Edit / Delete buttons on the tile.
- Whole tile is the Play affordance: navigates to `/play?deck={id}` (focusable link; Enter activates).
- Keep the existing commander hover preview on the tile (mousemove); it must not prevent the Play navigation on primary click.
- Loading / empty-library / error messaging keep today’s meaning (`Loading decks…`, build-first empty copy, error alert). When the library is non-empty but search matches nothing, show **No decks match.**

### Context menu

- Right-click on an owned deck (`id > 0`) opens a builder-style context menu at the pointer (`contextmenu` Mount + submodel menu state).
- Menu items: **Edit** → `/decks/{id}`; **Delete** → existing `AskedDeckDelete` → confirm dialog → delete command.
- Right-click on a precon does **not** open a menu (prevent default / ignore); tests assert Edit/Delete controls are absent.
- Closing: click-away / Escape / choosing an item clears menu state (same pattern as the deck builder menu).

### Search

- Single text field above the grid; placeholder **Search decks…**.
- Client-only filter; no list API change.
- Case-insensitive substring match on:
  - deck `name`, and
  - resolved commander display name from `knownCommanders` (fallback: commander id string until lookup resolves).
- Filter applies, then ordering below. Clearing the query shows the full ordered list.

### Ordering

Always apply after filter:

1. Owned/custom decks (`id > 0`) first, preserving relative order from `list_decks`.
2. Precons (`id < 0`) after all customs, sorted by **ascending id** (more negative first): `-9` … `-1`.

Release order is the precon registration convention in `crates/server/src/precons.rs` (each new precon takes the next id down). Reverse release = newest id first. The server currently appends SOURCES order (`-1`…`-9`); **the client reorders** precons for this surface.

---

## State and modules

| Piece | Role |
|-------|------|
| `DeckListSubmodel` | Add `searchQuery: string`, `contextMenu: null \| { deckId, x, y }` |
| Messages | Search changed; open/close context menu; menu Edit (navigate); menu Delete → `AskedDeckDelete` |
| Pure helpers | `visibleDecks(decks, knownCommanders, query)` filter+order; menu item list for a deck id — unit-tested |
| `list/view.ts` | Tile grid, search field, context menu overlay; reuse `confirmDialog` |
| Lobby | Unchanged |

No wire / proto / BFF changes.

---

## Testing

- **Unit:** `visibleDecks` ordering (customs before precons; precon ids ascending); search match on name and commander; precon excluded from Edit/Delete menu helper.
- **Scene / outcome** (`surfaces.test.ts` and/or focused list tests):
  - Tiles present; always-visible Play/Edit/Delete buttons absent.
  - Tile href is `/play?deck={id}`.
  - Search narrows visible tiles; no-match copy when appropriate.
  - With mixed fixtures, first visible precon is newer than later precons; all customs before any precon.
  - Right-click owned deck → Edit/Delete present; right-click precon → those items absent.
- Interaction policy: assert outcomes (selected deck URL, menu actions), not migration/parity framing.

---

## Spec follow-up

Implementation updates [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) to describe the tile grid, search, ordering, and context menu as **current** behavior, and removes the list-row description for `/`.
