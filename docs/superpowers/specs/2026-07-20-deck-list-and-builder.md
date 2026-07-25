# Deck List and Builder

**Status:** Current (as of 2026-07-25)
**Module:** `client/app/shell/decks/**`, `client/lib/deck-builder/**`, `client/lib/ui/card-art.ts`, `client/lib/image-cache.ts`, `client/lib/deck-builder/scryfall.ts`

---

## Problem Statement

Players need to browse saved and precon decks, open a deck into play or edit, and build or revise Commander lists against the catalog — including printing (art) preference and legality feedback — without loading a full offline card database into the browser.

---

## Solution

The **deck list** at `/` is a compact commander-tile grid over the deck list submodel (search, precon ordering, owned-deck context menu). The **deck builder** at `/decks/new` and `/decks/:id` is a split-pane pool + decklist UI over catalog search RPCs. Card art is keyed by Scryfall Printing UUID via `client/lib/deck-builder/scryfall.ts` and rendered through `client/lib/ui/card-art.ts` against `sharedImageCache` in `client/lib/image-cache.ts`. Deck persistence and legality rules are owned by [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md); route/auth shell by [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).

---

## User Stories

- As a returning player on `/`, I scan commander tiles (each link tile shows a Play label), search by name, click a tile to play, and right-click an owned deck to edit or delete it.
- As a returning player, I navigate directly to `/decks/new` and the deck builder loads, showing the full card pool on the left and a blank decklist on the right.
- As a deck builder, I click a pool card to add it, right-click to pick a different printing (art preference), and see the commander picker auto-populate with legendary creatures in my list.

---

## Behavior

### Deck list (`client/app/shell/decks/**`, `/`)

**Deck list** (`/`) shows saved decks from the deck list submodel as a compact tile grid.
Header, search, and grid share one `max-w-[960px]` column. Tiles use a raised
`minmax(220px, 1fr)` track, landscape commander `art_crop` (~1.37:1), deck name,
color-identity pips, and a Precon chip when `id < 0`. Names stay single-line truncate.
There is no cursor-follow card hover preview on this surface. The whole tile links to
`/play/{id}` and shows a quiet `Play` label (`deck-play-label`) in link mode; static
lobby deck-card chrome omits that label. Home ↔ `/play/{id}` morphs the shared deck-card chrome with a short
FLIP animation (`deck-card-nav.ts`; skipped for reduced motion). A **Search decks…** field filters by deck name and commander display
name (client-only). Display order: owned decks first (API relative order), then precons
by ascending id (newest release first). Right-click on an owned deck opens Edit
(`/decks/{id}`) and Delete (confirm dialog); precons do not open a context menu. A New
Deck button navigates to `/decks/new`.

### Deck builder (`client/app/shell/decks/builder/**`, `/decks/new`, `/decks/:id`)

**Deck builder** is a split-pane layout:

- **Left: card pool grid.** Loads from `/api/rpc/cards/search` in 100-card pages via an `IntersectionObserver` sentinel at the grid bottom. Filters: text search (tokenized LIKE over `search_blob`), set, subtypes ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). Pool tiles are `POOL_CARD` style: art thumbnail + name + type + cost pips, click-to-add. Right-click (or 500 ms long-press) opens a context menu with printing options and basics shortcuts.
- **Right: decklist panel.** Commander picker (legendary creatures in the list), deck name field, 99-card decklist with per-card counts and a running total. Click a row to remove one. Deck save calls `/api/rpc/decks` or `/api/rpc/decks/:id` with `SaveDeckRequest`.
- **Printing preference.** Card identity is the Scryfall oracle id (`CardDef.id`); a Printing is a Scryfall UUID used only for art ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). `preferredPrint` is session-sticky per oracle id — once you pick a printing for a card, adding it again reuses that choice. `searchPrints(oracleId)` fetches Scryfall prints for the picker.
- **Singleton enforcement.** Non-basic non-commander cards cap at 1. Commander is set via the context menu only; `canBeCommander` restricts to legendary creatures.
- **Full Commander legality** is enforced server-side on save; the client surfaces validation errors returned as `CreateDeck422` / `UpdateDeck422` tagged Schema errors.
- **Card lookup.** `lookupCardsByIds(ids, client)` fetches oracle data for deck hydration through `/api/rpc/cards/lookup`.
- **Scroll.** The builder page shell does not scroll. The left catalog grid and the right decklist are independent `overflow-y-auto` scrollports with `overscroll-contain` so wheel/trackpad in one pane does not move the other or the document. The right column uses `min-h-0` so the decklist forms a real scrollport inside the grid.
- **Print picker scroll lock.** While the choose-printing `<dialog>` is open (`printPicker` set), catalog and decklist scrollports use `overflow-hidden` (background frozen). The print tile grid inside the dialog remains `overflow-y-auto` with `overscroll-contain`. Closing the picker restores independent pane scrolling.

### Card art CDN (`client/lib/deck-builder/scryfall.ts`, `client/lib/ui/card-art.ts`, `client/lib/image-cache.ts`)

Art is keyed by Scryfall **Printing** UUID. `imageUrlByPrint(printId, size, face)` returns:
- When `VITE_CARD_CDN` is set and `size === "art_crop"`: CDN
  `VITE_CARD_CDN/art_crop/{face}/{a}/{b}/{id}.webp`. If that asset fails to load, `cardArt`
  falls back once to Scryfall `version=art_crop` (deck-list tiles use this path).
- When `VITE_CARD_CDN` is set and `size` is any other value: CDN
  `VITE_CARD_CDN/large/{face}/{a}/{b}/{id}.webp` — missing `large` does **not** fall back to Scryfall.
- When `VITE_CARD_CDN` is unset: Scryfall image API
  (`https://api.scryfall.com/cards/{id}?format=image&version={size}`) (local/dev).

Missing ordinary (non-`art_crop`) CDN art stays empty after load failure (no Scryfall fallback in production). The CDN path replicates Scryfall's folder fan (`first two hex chars` of the UUID). DFC backs are fetched with `face=back` in the Scryfall path; CDN serves the same `large` webp. `imageFaceAfterLoadError` falls back from `back` to `front` on load error (DFC prepare/flip cards have no Scryfall `/back/` — transformer backs that exist load on first try).

`cardBackUrl()` returns `/card-back.webp` for library piles and face-down cards.

**Image cache and board preload:** `client/lib/image-cache.ts` provides a URL→HTMLImageElement cache (`sharedImageCache`) with a subscriber list for canvas redraws on image settle. HTML `cardArt` mounts subscribe to that cache. On the board bitmap Mount (`client/app/board/bitmap/mount.ts`), `preloadFrameArt` collects face/print URLs for the published frame's cards and flights and calls `sharedImageCache.preload(urls)` so gameplay paint hits the cache. There is no separate `deckImagePreload` / `preloadDecksIntoCache` module.

---

## Implementation Decisions

- **Deck-builder search is server-side.** The client holds no full catalog. `/api/rpc/cards/search` calls `Cards.Search` with tokenized LIKE over `search_blob` (includes `otags`) on the server ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). The pool grid pages in 100-card chunks via IntersectionObserver — no client-side filtering of a local dataset.
- **Printing is art-preference only.** Card rules identity is the oracle id. Decks store `(id, count, print)` with `print` required. The engine is print-agnostic. Wire DTOs carry `print` for consistent art across all clients.
- **`VITE_CARD_CDN` is build-time baked**, not runtime. Changing CDN requires a new image build.
- **No Scryfall fallback for ordinary CDN art.** Missing non-`art_crop` CDN art does not hit Scryfall (avoids rate-limiting). The intentional exception is CDN `art_crop` load failure → Scryfall `version=art_crop` once.

---

## Testing Decisions

- `client/app/shell/decks/**/*.test.ts` — decks list/builder stories and helpers.
- `client/lib/deck-builder/*.test.ts` — print prefs, menus, hover preview.
- `client/lib/ui/card-art.test.ts` — art URL / host sync against `ImageCache`.
- `client/lib/image-cache.test.ts` — cache settle / subscriber behavior.
- Scene coverage for shell deck surfaces lives with other shell Scene tests (see [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) Testing Decisions / `just client-check`).

---

## Out of Scope

- Auth routing and portrait gate ([shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Lobby Host/Join and seated table chrome ([lobby-entry-ui](2026-07-20-lobby-entry-ui.md)).
- Server catalog projection and legality engine ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)).
- Card art CDN origin operations (`cards.example.com` / CDN infra) — only client URL selection and cache behavior here.

---

## Further Notes

- Deck builder scroll layout and print-picker lock: [2026-07-25-deck-builder-scroll-design.md](2026-07-25-deck-builder-scroll-design.md).
- FLIP morph of shared deck-card chrome between Home and `/play/{id}` is specified here and reused by [lobby-entry-ui](2026-07-20-lobby-entry-ui.md).
- Scryfall / tooling User-Agent identity `edh.reilley.dev/0.1` is documented under brand display in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).
