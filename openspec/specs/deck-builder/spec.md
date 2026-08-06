# deck-builder Specification

## Purpose

The deck list and deck builder let authenticated players browse saved and precon decks, open decks into play or edit, and build Commander lists against server catalog search with printing (art) preference — without loading a full offline card database into the browser.

## Requirements

### Requirement: Deck List Home

`/` SHALL show saved and precon decks as a compact commander-tile grid through the deck-list submodel (`GotDeckListMessage`, child-owned `informRouteChanged`). The first grid cell SHALL always be a dashed create tile to `/decks/new`. Owned decks SHALL link to `/play/{id}` with a quiet Play label; right-click on owned decks SHALL offer Edit and Delete (precons SHALL NOT open a context menu). Delete SHALL use `confirmDialog`. Search SHALL filter by deck name and commander display name when at least one deck exists; the create tile SHALL stay first and never filter out. Display order SHALL be owned decks first, then precons by ascending id. Empty state SHALL invite creation. Home ↔ `/play/{id}` SHALL morph shared deck-card chrome with a short FLIP animation (skipped under reduced motion). There SHALL be no cursor-follow hover preview on this surface.

#### Scenario: Empty library
- **WHEN** the player has zero decks after loading
- **THEN** `deck-list-empty` invites creation and the create tile is the only grid tile

#### Scenario: Filter with no matches
- **WHEN** search matches no deck tiles
- **THEN** the grid keeps the create tile first and shows `No decks match.`

#### Scenario: Precon context menu
- **WHEN** the player right-clicks a precon tile
- **THEN** no context menu opens

### Requirement: Deck Builder Split Pane

`/decks/new` and `/decks/:id` SHALL render through `shellFrame` with Cancel / Save header actions and a split-pane body: left card pool from `/api/rpc/cards/search` (100-card pages; text/set/subtype filters), right decklist (name, commander picker from legendary creatures in the list, counts, running total, client legality problems). Click pool to add; click decklist row to remove one. Singleton caps SHALL apply to non-basic non-commander cards. Full Commander legality SHALL be enforced server-side on save; the client SHALL surface `CreateDeck422` / `UpdateDeck422` errors. Edit route SHALL show a loading state while the stored deck is in flight. Cancel on a dirty builder SHALL raise discard `confirmDialog`; clean Cancel SHALL navigate home.

#### Scenario: Fresh deck empty coach
- **WHEN** a new deck has no cards yet
- **THEN** the decklist shows `builder-decklist-empty` teaching click-to-add

#### Scenario: Dirty discard
- **WHEN** the builder is dirty and the player chooses Cancel
- **THEN** a discard confirm appears; confirming leaves the builder

### Requirement: Windowed Pool and Scroll Lock

The pool and print picker SHALL use `windowedGrid` over `VirtualList`. The builder SHALL pass `lockStageScroll` so only the pool window and decklist scrollports scroll (`overscroll-contain`). Pool column count and row height SHALL derive from measured container width. Scrolling within overscan of the loaded pool end SHALL request the next page; an arriving page that does not fill a tall column SHALL request the next page without further scroll. Query changes SHALL empty the pool and scroll to top.

#### Scenario: Unmeasured pool
- **WHEN** the pool grid has not been measured
- **THEN** it paints no tiles and requests no pages

#### Scenario: Print picker open freezes panes
- **WHEN** the print picker dialog is open
- **THEN** catalog and decklist scrollports freeze while the print grid keeps scrolling

### Requirement: Printing Preference

Card rules identity SHALL be the Scryfall oracle id; Printing UUIDs SHALL be art preference only. `preferredPrint` SHALL be session-sticky per oracle id. Right-click or long-press SHALL open printing options. The print picker SHALL use shared `modalDialog`, page printings sequentially (`searchPrintPage`), and drop stale pages whose URL is not the pending page. On HTTP 429, print search SHALL honor `Retry-After` (default 30s, clamp 60s) and retry up to twice. Deck rows SHALL store `(id, count, print)` with print required.

#### Scenario: Reuse preferred print
- **WHEN** the player previously picked a printing for an oracle id and adds the card again
- **THEN** the builder reuses that preferred print

#### Scenario: Partial print page failure
- **WHEN** a later print page fails after earlier pages arrived
- **THEN** already-loaded printings stay on screen

### Requirement: Card Art URLs and Cache

Art SHALL be keyed by Printing UUID. `imageUrlByPrint` SHALL emit `{base}/{size}/{face}/{a}/{b}/{id}.webp` where `base` is `VITE_CARD_CDN` when set else `https://cards.scryfall.io`. Surfaces SHALL request the size they render (`art` / `grid` / `thumb` / `display` as specified per surface). There SHALL be no client-side Scryfall API image fallback on failed `<img>` load. `sharedImageCache` SHALL cache decoded images for HTML `cardArt` and board bitmap preload. While a print is in flight, hosts SHALL show an `aria-hidden` skeleton; failed loads leave the host empty. `cardBackUrl` SHALL return `/card-back.webp` for face-down / library backs.

#### Scenario: CDN origin switch
- **WHEN** `VITE_CARD_CDN` is baked at build time
- **THEN** art URLs use that origin while preserving Scryfall path layout

#### Scenario: Builder avoids oversampling
- **WHEN** pool, print picker, commander, or decklist rows render art
- **THEN** they request `grid` or `thumb`, not default `display`
