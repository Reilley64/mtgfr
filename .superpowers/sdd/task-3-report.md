# Task 3 report: Seed maps + snapshot overlay

## Status

Complete.

## TDD log

### Red

- Added failing regressions in `crates/schema/src/snapshot.rs` for proxy-art overlay on:
  - visible objects
  - library-search `ChoiceItem`s
  - stack entries
- Added a failing server regression in `crates/server/src/lobby.rs` for seeding per-seat `proxy_art_urls`.
- Ran:

```bash
cargo nextest run --profile ci complete_visible_overlays_seat_proxy_art_urls
```

- Result: failed at compile time because `ViewExtras` did not yet expose `proxy_art_urls`, which was the missing feature surface the tests were written to require.

### Green

- Added `proxy_art_urls` alongside `prints` on:
  - `schema::ViewExtras`
  - `server::decks::SeatDeck`
  - `server::table::Table`
  - `server::stream::TableSubscription`
- Seeded non-empty deck/commander proxy-art URLs in `lobby::resolve_deck`.
- Copied seat maps into live tables during `seed_table_core`.
- Threaded the maps through `stream::view_extras` / `table_view_extras` / `GameSvc::stream`.
- Overlaid non-empty proxy-art URLs in `schema::complete_visible` for:
  - `ObjectView`
  - `ChoiceItem`
  - `StackObjectView`
- Preserved the existing empty-value contract: empty map values do not clobber an existing/default view.

## Living spec updates

- Updated `docs/superpowers/specs/2026-07-20-accounts-decks-and-catalog.md` to state that live-table seeding materializes non-empty proxy-art URLs into per-seat `card_id` maps.
- Updated `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md` to state that `complete_visible` overlays print/proxy art from per-seat maps onto objects, stack entries, and choice items for every viewer.

## Verification

Ran:

```bash
cargo nextest run --profile ci proxy_art
cargo nextest run --profile ci frame_for_stamps_table_extras_onto_the_visible_state
git diff --check
```

Results:

- `cargo nextest run --profile ci proxy_art`: 11 passed
- `cargo nextest run --profile ci frame_for_stamps_table_extras_onto_the_visible_state`: 1 passed
- `git diff --check`: clean

## Self-review

- Confirmed the engine remains URL-agnostic: all URL handling stays in deck resolution, table state, stream extras, and schema completion.
- Confirmed the overlay is seat-scoped rather than viewer-scoped, so every player/spectator sees the same alter for a given seat's card.
- Confirmed empty map values are ignored at overlay time, matching the existing print overlay contract.
- Found and fixed one adjacent test-helper issue while verifying: test deck creation now fills the required `commander_proxy_art_url` column with an empty string.
