# Task 7 report

## Status

Complete.

## Scope shipped

- Added builder context-menu action `Set proxy art…` for deck rows and commander.
- Added builder proxy-art dialog state/messages, inline URL validation, Save/Clear/Close actions,
  and scroll lock that mirrors the existing print-picker behavior.
- Persisted `proxy_art_url` / `commander_proxy_art_url` through builder hydrate + save body.
- Rendered quiet `Proxy` chips on deck rows and commander, and passed `proxyArtUrl` into builder
  thumbnails / hover-preview art resolution when present.
- Updated the living `deck-list-and-builder` spec for the shipped dialog UX and proxy override behavior.

## TDD evidence

### Red

`bun run test -- app/domain/deck-builder/menu.test.ts app/shell/decks/builder/story.test.ts app/shell/surfaces.test.ts`

- failed on missing `Set proxy art…` menu labels
- failed on missing builder proxy-art dialog/action schema
- failed on missing deck load/save proxy URL round-trip

### Green

`bun run test -- app/domain/deck-builder/menu.test.ts app/shell/decks/builder/story.test.ts app/shell/surfaces.test.ts`

- passed: 3 files, 56 tests

## Verification

- `bun run test -- app/domain/deck-builder/menu.test.ts app/shell/decks/builder/story.test.ts app/shell/surfaces.test.ts`
- `bun run test -- app/domain/card-art/proxy-url.test.ts app/domain/ui/card-art.test.ts`
- `bun run typecheck`
- `bun run lint`

## Notes

- The focused builder Scene/story coverage exercises menu open, dialog validation, save, clear,
  proxy chips, and scroll lock.

## Task 7 follow-up: commander vs entry proxy target

### Red

`bunx vitest run "app/shell/decks/builder/story.test.ts" -t "commander proxy art save does not write into the deck row when both share the same card id"`

- failed because `commander.proxyArtUrl` stayed unset when the commander and a deck row shared the same oracle id

### Green

`bunx vitest run "app/shell/decks/builder/story.test.ts" -t "commander proxy art save does not write into the deck row when both share the same card id"`

- passed after threading explicit `entry` vs `commander` proxy-art targets through the menu action, picker message, and picker state
