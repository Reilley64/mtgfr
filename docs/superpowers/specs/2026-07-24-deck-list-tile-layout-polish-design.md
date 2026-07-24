# Deck list tile layout polish (Your decks)

**Status:** Implemented  
**Date:** 2026-07-24  
**Module:** `client/app/shell/decks/list/**`, `client/lib/deck-builder/scryfall.ts` (`imageUrlByPrint`), Scene coverage in `client/app/shell/surfaces.test.ts`  
**Related current-behavior spec:** [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) (update in the same implementation change)  
**Related prior design:** [deck-list-tile-chooser](2026-07-24-deck-list-tile-chooser-design.md) (this follow-up supersedes that design’s 110px art strip and hover-preview requirements for `/`)  
**Approach:** Client-only layout + art URL polish (no deck-list API / `DeckSummary` schema change)

---

## Problem

The home **Your decks** surface feels scattered after the tile-grid redesign:

- Header and search sit in a narrower column (`max-w-[720px]`) than the tile grid (`max-w-[960px]`).
- Tiles are too narrow (`minmax(140px, 1fr)`), so deck and commander names clip heavily under single-line truncate.
- Tiles request `size: "art_crop"`, but when `VITE_CARD_CDN` is set, `imageUrlByPrint` ignores size and always serves `/large/...webp` full card faces — name and oracle text appear in the image.
- Cursor-follow card hover preview on tiles adds noise once art crops are correct.

---

## Goals

- One shared ~960px column for header, search, and grid.
- Larger tiles so single-line names and commander lines clip less in practice.
- Real commander **art crops** on tiles: prefer CDN `art_crop` assets; fall back to Scryfall when the CDN crop is missing.
- Remove the deck-list cursor-follow hover preview.

## Non-goals

- Lobby deck `<select>` / Bring strip redesign.
- Ingesting or backfilling art_crop assets into the CDN (ops/CDN work outside this change).
- Scryfall fallback for missing `large` (or any non-`art_crop` size).
- Fixed column counts by breakpoint, or a shared chooser abstraction for lobby.
- Behavior changes to Play navigation, search, ordering, Precon chip, or Edit/Delete context menu.

---

## User stories

- As a returning player on `/`, I see header, search, and deck tiles aligned in one column.
- As a player scanning decks, I see larger tiles with commander art crops (illustration only, not full card frames) and can read more of each deck/commander name before truncate.
- As a player, hovering a tile does not open a floating card preview; primary click still goes to `/play/{id}`, and right-click on owned decks still offers Edit/Delete.

---

## Behavior

### Layout and chrome

- Route stays `/` (`decks-page`). Header content unchanged: title **Your decks**, username, **Sign out**, **New deck**.
- Header row, search field, and tile grid all use `mx-auto max-w-[960px]` (remove the 720px caps on header and search). Search is full width of that column.
- Grid keeps `auto-fill` with a raised track minimum of about `minmax(220px, 1fr)` (implementation may tune toward ~240–260px if long names still clip too aggressively). Fewer, wider tiles; uneven last rows from `auto-fill` are acceptable.
- Loading / empty-library / no-match / error messaging keep today’s meaning.

### Tile

- Top: commander art at Scryfall **art_crop** aspect (~1.37:1), full tile width. Use `object-cover` only for tiny ratio differences — not a fixed ~110px-tall strip.
- Below (unchanged structure): deck name (semibold, single-line truncate), commander display name (lichen, single-line truncate), color-identity pips, **Precon** chip when `id < 0`.
- Whole tile remains the Play affordance: `/play/{id}` (focusable link; Enter activates).
- Right-click Edit/Delete for owned decks (`id > 0`) unchanged; precons do not open a menu.

### Hover preview

- Remove deck-list cursor-follow preview entirely: no `BindDeckListCommanderHover`, no hover submodel fields, no `MovedDeckListHover` / `ClearedDeckListHover`, no preview render on this surface.
- Builder and board inspect previews are unchanged.

### Art URL resolution

`imageUrlByPrint(printId, size, face)`:

| Condition | URL |
|-----------|-----|
| `size === "art_crop"` and CDN set | Prefer `${CDN}/art_crop/{face}/{a}/{b}/{printId}.webp` (same UUID fan-out as `large`) |
| `size === "art_crop"` and that CDN asset missing / fails to load | Fall back to Scryfall `https://api.scryfall.com/cards/{id}?format=image&version=art_crop` (+ `&face=back` when needed) |
| `size === "art_crop"` and CDN unset | Scryfall `version=art_crop` (today’s non-CDN path) |
| Any other size, CDN set | Unchanged: `${CDN}/large/{face}/…` — **no** Scryfall fallback on miss |
| Any other size, CDN unset | Unchanged: Scryfall `version={size}` |

Deck list tiles keep requesting `size: "art_crop"`. The load-failure → Scryfall swap lives at the lowest layer that owns image painting for that URL (shared with `cardArt` / image cache as needed so the list does not special-case a second URL builder).

---

## State and modules

| Piece | Role |
|-------|------|
| `client/app/shell/decks/list/view.ts` | Shared 960 column; larger grid minmax; taller art aspect; drop hover preview UI/mounts |
| `DeckListSubmodel` / list messages / update | Remove hover state and hover messages |
| `client/lib/deck-builder/scryfall.ts` | CDN path includes `size` for `art_crop`; document non-`art_crop` still maps to `large` |
| `cardArt` / image host | On `art_crop` CDN load failure, retry Scryfall art_crop URL once |

No wire / proto / BFF / lobby changes.

---

## Testing

- **Unit — URL builder:** With CDN set, `art_crop` → CDN `…/art_crop/…`; without CDN → Scryfall `version=art_crop`. Non-`art_crop` sizes still resolve to CDN `large` when CDN is set.
- **Unit — fallback:** CDN `art_crop` miss / load error resolves to the Scryfall art_crop URL (assert at the helper or paint layer that owns the swap).
- **Scene / outcome:** Header, search, and grid share the 960 column; larger tiles present; hover-preview testids/nodes absent; Play href, search, order, and context-menu coverage remain green.
- Drop or rewrite list tests that asserted hover preview.
- Interaction policy: assert outcomes (art crop URL / no preview), not migration framing.

---

## Spec follow-up

Implementation updates [client-shell-deck-builder-and-observability](2026-07-20-client-shell-deck-builder-and-observability.md) so current behavior describes:

- Aligned 960px chrome and larger tiles without hover preview.
- CDN URL shape for `art_crop`, and the intentional Scryfall fallback **only** when an `art_crop` CDN asset is missing.

The prior [deck-list-tile-chooser design](2026-07-24-deck-list-tile-chooser-design.md) remains historical for the original grid/search/menu work; this doc owns the layout/art-crop follow-up.
