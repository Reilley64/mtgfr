# Task 7 Report: Home top-5 teaser

## Status

Implemented, verified, and ready to commit.

## TDD evidence

1. Red tests were added first:
   - `client/app/routes.test.ts` now expects HomeRoute route entry to dispatch both deck loading and `FetchDeckListLeaderboardTeaser({ limit: 5, offset: 0 })`.
   - `client/app/shell/surfaces.test.ts` now expects a home `data-testid="leaderboard-teaser"` surface with a `/leaderboard` link.
2. Red command:
   - `cd client && bun test ./app/routes.test.ts ./app/shell/surfaces.test.ts`
3. Red result:
   - `HomeRoute loads decks and the leaderboard teaser on protected route entry` failed because only `FetchDecks` was dispatched.
   - `renders a leaderboard teaser on the deck list home` failed because `[data-testid="leaderboard-teaser"]` did not exist.
4. Green implementation:
   - Added deck-list teaser state/messages and a dedicated `FetchDeckListLeaderboardTeaser` command.
   - Home route entry now calls `loadDeckList(model.decks.list, { includeLeaderboardTeaser: true })`.
   - The deck-list view now renders a compact top-5 teaser linking to `/leaderboard`.
5. Green focused command:
   - `cd client && bun test ./app/routes.test.ts ./app/shell/surfaces.test.ts ./app/shell/decks/list/story.test.ts`
6. Green focused result:
   - `33 pass`, `0 fail`.

## Implementation summary

- Added `leaderboardTeaser` to `client/app/shell/decks/list/submodel.ts`.
- Added `ReceivedDeckListLeaderboardTeaser` and `DeckListLeaderboardTeaserLoadFailed` in deck-list messages and parent message exports.
- Added `FetchDeckListLeaderboardTeaser` in `client/app/shell/decks/list/update.ts`, powered by `rpc.ratings.leaderboard({ limit: 5, offset: 0 })`.
- Scoped the teaser fetch to HomeRoute route entry in `client/app/update.ts`.
- Rendered a compact teaser card with rank, username, rating, and a full-board link in `client/app/shell/decks/list/view.ts`.
- Added HomeRoute Scene coverage and route-entry Story coverage.
- Updated `docs/superpowers/specs/2026-07-20-deck-list-and-builder.md`.

## Verification

- Focused:
  - `cd client && bun test ./app/routes.test.ts ./app/shell/surfaces.test.ts ./app/shell/decks/list/story.test.ts`
  - Result: `33 pass`, `0 fail`.
- Broader:
  - `just client-check`
  - Result: success.
  - Included `1027 passed (1027)` Vitest tests, format, lint, typecheck, token/codegen checks.

## Notes / concerns

- `just client-check` still reports two pre-existing Biome warnings in `client/lib/favicon-assets.test.ts` for non-null assertions. Task 7 introduces no new lint failures.
- The worktree already had an unrelated modification in `.superpowers/sdd/task-6-report.md`; it was left untouched and is not part of Task 7.
