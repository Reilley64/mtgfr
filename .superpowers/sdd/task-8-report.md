# Task 8 report

## Status

Complete.

## Living spec audit

- Updated `docs/superpowers/specs/2026-07-20-accounts-decks-and-catalog.md` to document
  persisted `commander_proxy_art_url`, per-deck-line proxy URL granularity, current precons
  shipping without proxy-art URLs, and the actual precon id/count range (`-1` through `-10`).
- Updated `docs/superpowers/specs/2026-07-20-wire-protocol-and-visibility.md` to document the
  shipped expand-only proxy-art field numbers in `catalog.proto` and `stream.proto`, and to make
  the `Decks` service contract explicit about `DeckDetail` / `SaveDeckRequest` carrying optional
  proxy-art fields while `DeckSummary` stays list-view chrome only.
- Audited the rest of the accepted-design surfaces and left them unchanged because they already
  described the shipped proxy-art behavior with no TBDs:
  `deck-list-and-builder`, `shell-routes-and-auth`, `battlefield`, `hand-and-zone-bar`, `stack`,
  `card-inspect`, `flights`, `prompts-and-pending-choices`, `system-overlays`, and
  `turn-and-priority-chrome`.

## Test coverage polish

- Added `crates/server/src/decks_api.rs` regression coverage for invalid card-line
  `proxy_art_url` rejection:
  `an_invalid_card_line_proxy_art_url_is_rejected_with_problems`.

## Verification

### Server / schema

- `just migrate`
  - PASS. Database already up to date.
- `cargo nextest run --profile ci proxy_art complete_visible_overlays_seat_proxy deck_card_entry`
  - PASS. 12 tests passed, including the new invalid card-line proxy-art rejection test.

### Client

- Attempted brief command:
  `bun test app/domain/card-art app/domain/ui/card-art.test.ts app/domain/deck-builder/menu.test.ts app/shell/decks/builder/story.test.ts app/board/bitmap/paint-cards.test.ts`
  - FAIL, but due to harness mismatch rather than product behavior: Bun's native runner ignored the
    `@vitest-environment happy-dom` header in `app/domain/ui/card-art.test.ts`, so `document` was
    undefined.
- Reran the same focused file set through the repo's actual test script:
  `bun run test -- app/domain/card-art app/domain/ui/card-art.test.ts app/domain/deck-builder/menu.test.ts app/shell/decks/builder/story.test.ts app/board/bitmap/paint-cards.test.ts`
  - PASS. 6 files, 60 tests passed.
- `bun run typecheck`
  - PASS.

### Lint

- `cargo clippy --all-targets -- -D warnings`
  - PASS.

## Notes

- If future SDD briefs want a copy-pasteable client command for these files, prefer
  `bun run test -- ...` over `bun test ...` so Vitest environment annotations are honored.
- Medium Task 8 follow-up: aligned `accounts-decks-and-catalog` art-resolution bullet with
  `deck-list-and-builder` (CDN/`imageUrlByPrint`, `art_crop` Scryfall fallback, proxy BFF fallback).
