# Deck List and Builder

**Status:** Current (as of 2026-07-27)
**Module:** `client/app/shell/decks/**`, `client/app/domain/deck-builder/**`, `client/app/domain/ui/card-art.ts`, `client/app/domain/image-cache.ts`, `client/app/domain/deck-builder/scryfall.ts`

---

## Problem Statement

Players need to browse saved and precon decks, open a deck into play or edit, and build or revise Commander lists against the catalog — including printing (art) preference and legality feedback — without loading a full offline card database into the browser.

---

## Solution

The **deck list** at `/` is a compact commander-tile grid over the deck list submodel (shared account chrome, create tile, search, precon ordering, owned-deck context menu). The **deck builder** at `/decks/new` and `/decks/:id` is a split-pane pool + decklist UI over catalog search RPCs. `client/app/view.ts` renders both surfaces through Foldkit submodel boundaries: deck-owned child messages lift into the app through `GotDeckListMessage` / `GotDeckBuilderMessage`, child Commands lift with `Command.mapMessages`, and route entry / post-session cold-load call the deck surfaces' `informRouteChanged` helpers so the children own their own reset/load transitions. Shared shell messages such as auth chrome toggles, modal open no-ops, card-art ticks, and deck-card FLIP ticks pass through unchanged. Card art is keyed by Scryfall Printing UUID via `client/app/domain/deck-builder/scryfall.ts` and rendered through `client/app/domain/ui/card-art.ts` against `sharedImageCache` in `client/app/domain/image-cache.ts`. Deck persistence and legality rules are owned by [accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md); route/auth shell by [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).

---

## User Stories

- As a returning player on `/`, I use the first grid tile — the dashed create tile (`deck-list-new-deck`) — to open `/decks/new`; when I have no saved decks yet, the page also shows an empty state (`deck-list-empty`) that points me to deck creation.
- As a returning player on `/`, I scan commander tiles (each link tile shows a Play label), search by name, click a tile to play, and right-click an owned deck to edit or delete it.
- As a returning player on `/`, I use the header chrome to jump to `/leaderboard` or open my avatar menu to reach Gravatar settings or sign out.
- As a returning player, I navigate directly to `/decks/new` and the deck builder loads, showing the full card pool on the left and a blank decklist on the right.
- As a deck builder, I click a pool card to add it, right-click to pick a different printing (art preference), and see the commander picker auto-populate with legendary creatures in my list.

---

## Behavior

### Deck list (`client/app/shell/decks/**`, `/`)

**Deck list** (`/`) shows saved decks from the deck list submodel as a compact tile grid.
The page title (`Your decks`) lives in the shared `shellFrame` header title slot, not a
custom left-side title block. The shared header trailing slot holds account chrome: a
`Leaderboard` link (`header-leaderboard-link`) plus an avatar trigger backed by the same
circular Gravatar/monogram face helper used for seats. Opening the avatar menu shows a
username heading, `Change at Gravatar` (`account-menu-gravatar`), which opens
`https://gravatar.com` in a new tab, and `Sign out` (`account-menu-sign-out`). The menu is
a `@foldkit/ui` `Menu` submodel owned by the root model — see
[shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md). Search and grid share the
shell stage max-width column (no nested 960px wrappers).

Tiles use a raised `minmax(220px, 1fr)` track, landscape commander `art_crop`
(~1.37:1), deck name, color-identity pips, and a Precon chip when `id < 0`. Names stay
single-line truncate. There is no cursor-follow card hover preview on this surface. The
first grid cell is always a dashed create tile linking to `/decks/new`
(`deck-list-new-deck`) whenever the grid renders; it uses the same footprint as deck
tiles, shows no commander art, no `Play` label, no FLIP morph, and no context menu. If
the player has zero decks after loading, an empty-state panel (`deck-list-empty`) invites
deck creation above the grid, and the create tile remains the only grid tile. While
`loading`, the page still shows `Loading decks…` instead of the grid.

The whole deck tile links to `/play/{id}` and shows a quiet `Play` label
(`deck-play-label`) in link mode; static lobby deck-card chrome omits that label. Home ↔
`/play/{id}` morphs the shared deck-card chrome with a short FLIP animation
(`deck-card-nav.ts`; skipped for reduced motion). A **Search decks…** field appears only
when at least one actual deck exists and filters by deck name and commander display name
(client-only). The create tile stays first and is never filtered out; if search matches
no deck tiles, the grid keeps the create tile first and shows `No decks match.`
(`deck-list-filter-empty`). Load errors use the shared `alertClass` recipe. Display order:
owned decks first (API relative order), then precons by
ascending id (newest release first). Right-click on an owned deck opens Edit
(`/decks/{id}`) and Delete; precons do not open a context menu. The context menu is
pointer-positioned, so it stays hand-rolled markup dressed with the shared
`menuPanelClass` / `menuItemClass` chrome rather than a `@foldkit/ui` `Menu`, which anchors
to a trigger button. Delete raises `confirm-delete-dialog`, a `confirmDialog`
([ui-component-layer](2026-07-28-ui-component-layer.md)) over a `Dialog` submodel held as
`confirmDialog` on the deck list model; Escape, a backdrop click, and Cancel all close it
through the dialog's `Closed` out-message, and confirming issues the delete.

### Deck builder (`client/app/shell/decks/builder/**`, `/decks/new`, `/decks/:id`)

**Deck builder** renders through `shellFrame`: the shared header title slot says `New deck`
or `Edit deck`; the leading slot holds Cancel (`builder-cancel`); the trailing slot holds
primary Save (`save-deck`) before the same avatar account chrome as the deck list. The
stage body is a split-pane layout (no duplicate page title in the decklist pane):

- **Left: card pool grid.** Loads from `/api/rpc/cards/search` in 100-card pages. Filters: text search (tokenized LIKE over `search_blob`), set, subtypes ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). Pool tiles are `POOL_CARD` style: art thumbnail + name + type + cost pips, click-to-add. Card names are one truncated line — a name that wrapped would make its row taller than the rest. Right-click (or 500 ms long-press) opens a context menu with printing options and basics shortcuts.
- **Right: decklist panel.** Deck name field (`deck-name`), commander picker (legendary creatures in the list), 99-card decklist with per-card counts and a running total, and client legality problems (`deck-problems`) via the shared `alertClass` recipe. Click a row to remove one. Decklist rows (and pool tiles / commander chip) are keyed by oracle id so `BindBuilderCardPointer` remounts after list churn — Mount args are captured at insert, so unkeyed reuse left later rows activating the removed card until refresh. Deck save calls `/api/rpc/decks` or `/api/rpc/decks/:id` with `SaveDeckRequest` from the header Save control. On the edit route (`/decks/:id`), while the stored deck is in flight (`loadingDeck`), the panel shows a centered `builder-deck-loading` state and disables the name field instead of flashing an empty "New deck / 0 cards" builder.
- **Builder chrome shares the domain recipes.** Pool tiles and decklist rows compose `listRowClass()`; the context menu panel is `menuPanelClass(...)` with `menuItemClass()` rows — no builder-local copies of the surface/menu recipes.
- **Printing preference.** Card identity is the Scryfall oracle id (`CardDef.id`); a Printing is a Scryfall UUID used only for art ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). `preferredPrint` is session-sticky per oracle id — once you pick a printing for a card, adding it again reuses that choice. `printSearchUrl(oracleId)` gives the first page of a card's printings and `searchPrintPage(url)` fetches exactly that one page — up to 175 printings, plus the URL of the next. On HTTP **429**, it waits `Retry-After` (integer seconds or HTTP-date; default **30s** when absent/invalid, clamped to **60s**) and retries the same page up to **2** times before failing. Non-429 failures fail immediately with no wait.
- **Singleton enforcement.** Non-basic non-commander cards cap at 1. Commander is set via the context menu only; `canBeCommander` restricts to legendary creatures.
- **Full Commander legality** is enforced server-side on save; the client surfaces validation errors returned as `CreateDeck422` / `UpdateDeck422` tagged Schema errors.
- **Card lookup.** `lookupCardsByIds(ids, client)` fetches oracle data for deck hydration through `/api/rpc/cards/lookup`.
- **Pool grid.** The catalog runs to tens of thousands of cards and every tile that renders also fetches its art, so the pool is a `windowedGrid` ([ui-component-layer](2026-07-28-ui-component-layer.md)) over a `VirtualList.Model` held as `poolGrid` — container id and `data-testid` `builder-pool-scroll`. Its subscription is lifted in `client/app/subscriptions.ts` alongside the print grid's; until measured it paints no tiles. Skeletons and "No cards match." render as a plain grid — there is nothing to window.
- **Pool grid geometry.** The pool sizes itself to its column, not the viewport, and `VirtualList` measures only height — so a `ResizeObserver` Mount (`ObservePoolWidth`) on the wrapper around the grid reports width as `MeasuredPoolGrid`, kept as `poolWidth`. `poolGridColumns(width)` divides it by the 120px minimum tile plus gap (never below one column) and `poolGridRowHeightPx(width)` derives row height from the resulting tile width, its `aspect-[0.72]` art, the name line, and the row gap, rounding **up** — a row taller than its tiles reads as gap, a shorter one clips them. The column count reaches the row as an inline `grid-template-columns`, since Tailwind cannot generate a class for a number measured at runtime; that also means a scrollbar narrows tiles rather than rewrapping the row. A width change rewrites `rowHeightPx` on the existing `poolGrid` rather than re-initing it, which would throw away the container measurement the observer will not re-fire.
- **Pool paging.** Scrolling within `POOL_PAGE_OVERSCAN_ROWS` (12) rows of the end of the loaded pool requests the next 100-card page. The trigger is `VirtualList.visibleWindow`'s `endIndex` against the row count, checked on every grid message, on `MeasuredPoolGrid`, and on each arriving page — a windowed grid puts no element at the bottom of the list to hang an `IntersectionObserver` on, because the bottom is not in the DOM until you scroll to it. Checking on arrival is what keeps a page that does not fill a tall pool column from stalling: nothing else would ask. Changing the query empties the pool and drives the container back to the top with `VirtualList.scrollToIndex`, since the element itself survives the change.
- **Scroll.** `shellFrame` is a viewport-contained flex column (`overflow-hidden`); the builder passes `lockStageScroll` so the stage is `flex-1 min-h-0 overflow-hidden`. The builder page fills that stage (`h-full min-h-0 flex-1`, single `minmax(0,1fr)` grid row, `overflow-hidden`) and does not scroll the page. The right decklist is an `overflow-y-auto` scrollport and the left pool scrolls inside its windowed grid; both are `overscroll-contain`, so wheel/trackpad in one pane does not move the other or the document. Both columns use `min-h-0` so their scroll hosts form real scrollports inside the grid instead of growing the page.
- **Print picker modal.** The choose-printing dialog renders on the shared `modalDialog` frame ([ui-component-layer](2026-07-28-ui-component-layer.md)), so it gets `@foldkit/ui` `Dialog`'s focus trap, focus restore, Escape, and managed close. It is not a `confirmDialog` — it supplies its own heading, Close button, and print grid as `modalDialog` children. `printDialog` (a `Dialog.Model`, id `builder-print-picker`) holds open/closed; `printPicker` holds the picked card, the prints loaded so far, and `pendingPage` — the URL of the page in flight, or null once every page has landed. Escape, a backdrop click, and Close all arrive as `Dialog`'s `Closed` and clear both, so the prints it loaded go with it.
- **Print grid.** A card can have hundreds of printings (basic lands especially), so the picker's tile grid is a `windowedGrid` ([ui-component-layer](2026-07-28-ui-component-layer.md)) over a `VirtualList.Model` held as `printGrid` on the builder model — container id and `data-testid` `builder-print-picker-scroll`, two tiles per row. Only rows near the viewport are in the DOM, so only their art is requested. The grid learns its height and scroll position from `VirtualList.subscriptions`, lifted in `client/app/subscriptions.ts`; until it is measured it paints no tiles. Loading skeletons, the load-failure line, and "No printings found." render as a plain grid — there is nothing to window.
- **Printings arrive a page at a time.** A basic land runs to a thousand printings across six Scryfall pages, and waiting for the last one leaves the picker on skeletons for seconds. `SearchBuilderPrints({ cardId, url })` fetches one page; `ReceivedBuilderPrints` appends it and re-issues the command for `nextPage` until there is none, so the first page paints while the rest are still in flight. `pendingPage` is both the loading flag and the token that matches a page to the request it answers — a page whose `url` is not what the picker is waiting on came from a run that has since been closed and reopened, and is dropped. A page that fails leaves the printings that already arrived on screen; skeletons show only until the first page lands. *ponytail:* the picker then shows a short list with no hint that it is short.
- **Uniform print tiles.** Windowing needs one row height for the whole grid, so every print tile reserves two lines (`h-10`) for its set / collector / date badges whether they wrap or not. `printGridRowHeightPx(viewportWidth)` derives the row height from the tile's `w-[min(38vw,200px)]` width, its `aspect-[0.72]` art, the badge block, and the row gap. Opening the picker re-inits `printGrid` at the current viewport width — which also returns the grid to the top. *ponytail:* rotating the device with the picker already open misaligns rows until it is reopened.
- **Print picker scroll lock.** While the picker is open (`printPicker` set), catalog and decklist scrollports freeze — the decklist with `overflow-hidden`, the pool with Tailwind's important modifier `overflow-hidden!`, since `VirtualList` writes `overflow: auto` on its container as an inline style that a plain class cannot beat. The print grid keeps scrolling for the same reason. Closing the picker restores independent pane scrolling. `Dialog`'s own scroll lock only freezes `documentElement`, so freezing the two inner scrollports stays the builder's concern.
- **Discard confirm.** Cancel navigates home directly when the builder is clean. When `dirty`, it raises `builder-discard-confirm`, a `confirmDialog` ([ui-component-layer](2026-07-28-ui-component-layer.md)) over a `Dialog` submodel held as `discardDialog` on the builder model. Escape, a backdrop click, and Cancel all dismiss it through the dialog's `Closed` out-message; confirming leaves the builder.

### Card art CDN (`client/app/domain/deck-builder/scryfall.ts`, `client/app/domain/ui/card-art.ts`, `client/app/domain/image-cache.ts`)

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

**Image cache and board preload:** `client/app/domain/image-cache.ts` provides a URL→HTMLImageElement cache (`sharedImageCache`) with a subscriber list for canvas redraws on image settle. HTML `cardArt` mounts subscribe to that cache. On the board bitmap Mount (`client/app/board/bitmap/mount.ts`), `preloadFrameArt` collects face/print URLs for the published frame's cards and flights and calls `sharedImageCache.preload(urls)` so gameplay paint hits the cache. There is no separate `deckImagePreload` / `preloadDecksIntoCache` module.

---

## Implementation Decisions

- **Deck-builder search is server-side.** The client holds no full catalog. `/api/rpc/cards/search` calls `Cards.Search` with tokenized LIKE over `search_blob` (includes `otags`) on the server ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)). The pool grid pages in 100-card chunks off its own scroll position — no client-side filtering of a local dataset.
- **Route entry is child-owned.** Home and `/decks/...` entry run through deck-surface `informRouteChanged` helpers, which call the child update with route-change messages instead of having the parent mutate deck-list or builder state directly.
- **Printing is art-preference only.** Card rules identity is the oracle id. Decks store `(id, count, print)` with `print` required. The engine is print-agnostic. Wire DTOs carry `print` for consistent art across all clients.
- **`VITE_CARD_CDN` is build-time baked**, not runtime. Changing CDN requires a new image build.
- **No Scryfall fallback for ordinary CDN art.** Missing non-`art_crop` CDN art does not hit Scryfall (avoids rate-limiting). The intentional exception is CDN `art_crop` load failure → Scryfall `version=art_crop` once.

---

## Testing Decisions

- `client/app/shell/decks/**/*.test.ts` — decks list/builder stories and helpers (including sequential multi-card remove and keyed decklist rows for pointer-Mount remount). Builder stories assert that a 400-print picker paints one screenful of tiles (and so requests one screenful of art), and that `printGridRowHeightPx` grows with the viewport until the tile hits its 200px cap. Picker scenes seed a measured `printGrid`, since an unmeasured grid renders no tiles. The same holds for the pool: a 2000-card pool paints 45 tiles, `poolGridColumns` fits five 120px tiles into 640px and one into 200px, and scenes that want pool tiles seed both a `poolWidth` and a measured `poolGrid`. Print paging is asserted on the update — a first page appends and issues the request for the next, a later page appends under it, a page for a URL the picker is not waiting on is dropped, and a failed page keeps what arrived — plus a scene showing tiles while `pendingPage` is still set. Pool paging is asserted on the update: a scroll to the end of the loaded pool asks for the next page, an arriving page that does not fill a tall pool column asks for the next one with no scroll at all, and an unmeasured grid asks for nothing.
- `client/app/domain/deck-builder/*.test.ts` — print prefs, menus, hover preview; `scryfall.test.ts` covers `Retry-After` parsing, 429 wait-then-retry, and that `searchPrintPage` fetches only its own page and reports `nextPage`.
- `client/app/domain/ui/card-art.test.ts` — art URL / host sync against `ImageCache`.
- `client/app/domain/image-cache.test.ts` — cache settle / subscriber behavior.
- Scene coverage for shell deck surfaces lives with other shell Scene tests, including
  `header-leaderboard-link`, `account-menu-trigger`, `account-menu-*`,
  `deck-list-empty`, and `deck-list-new-deck`; the home surface does not render
  `data-testid="leaderboard-teaser"` or a header `New deck` control. Route-entry Stories cover the home fetch path
  without a separate teaser request (see
  [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md) Testing Decisions /
  `just client-check`).

---

## Out of Scope

- Auth routing and CSS landscape rotate ([shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md)).
- Lobby Host/Join and seated table chrome ([lobby-entry-ui](2026-07-20-lobby-entry-ui.md)).
- Server catalog projection and legality engine ([accounts-decks-and-catalog](2026-07-20-accounts-decks-and-catalog.md)).
- Card art CDN origin operations (`cards.example.com` / CDN infra) — only client URL selection and cache behavior here.

---

## Further Notes

- Deck builder scroll layout and print-picker lock: [2026-07-25-deck-builder-scroll-design.md](2026-07-25-deck-builder-scroll-design.md).
- FLIP morph of shared deck-card chrome between Home and `/play/{id}` is specified here and reused by [lobby-entry-ui](2026-07-20-lobby-entry-ui.md).
- Scryfall / tooling User-Agent identity `edh.reilley.dev/0.1` is documented under brand display in [shell-routes-and-auth](2026-07-20-shell-routes-and-auth.md).
