# Task 6 Report: `/leaderboard` shell surface

## Status

Implemented, committed, and pushed.

## TDD evidence

1. Red Scene test added first in `client/app/shell/surfaces.test.ts`.
2. Red command:
   - `cd client && bun test app/shell/surfaces.test.ts`
3. Red result:
   - Failed with `SyntaxError: Export named 'LeaderboardRoute' not found in module '/workspace/client/app/routes.ts'.`
   - This confirmed the test was exercising the missing `/leaderboard` shell surface.
4. Green focused command:
   - `cd client && bun test app/routes.test.ts app/shell/surfaces.test.ts`
5. Green focused result:
   - `22 pass`, `0 fail`, including:
     - `renders leaderboard rows with usernames and ratings`
     - `LeaderboardRoute loads the first page on protected route entry`

## Implementation summary

- Added `LeaderboardRoute` in `client/app/routes.ts` for `/leaderboard`.
- Added `client/app/shell/leaderboard/{submodel,messages,update,view}.ts`.
- Wired leaderboard state into `Model`, `init`, app `Message`, parent `update`, and `view`.
- Route entry loads page 0 with limit 50 using `client.ratings.leaderboard`.
- View renders `data-testid="leaderboard-page"` and duplicate `data-testid="leaderboard-row"` rows showing rank, username, and rating.
- Added a guarded "Load more" action when loaded entries are below `total`.
- Updated `docs/superpowers/specs/2026-07-20-shell-routes-and-auth.md`.

## Verification

- `cd client && bun test app/shell/surfaces.test.ts`:
  - Initial red: failed on missing `LeaderboardRoute`.
- `cd client && bun test app/routes.test.ts app/shell/surfaces.test.ts`:
  - `22 pass`, `0 fail`.
- `cd client && bun run lint`:
  - Exit 0.
  - Existing warnings remain in `client/lib/favicon-assets.test.ts` for non-null assertions.
- `cd client && bun run typecheck`:
  - Exit 0.
- `just client-check`:
  - Exit 0.
  - Passed tokens, mana-oracle, codegen, format, lint, typecheck, and Vitest.
  - Vitest result: `102 passed (102)` files, `1021 passed (1021)` tests.

## Notes / concerns

- `just client-check` mutates generated token CSS through its format step, so reruns may require `cd client && bun run gen:tokens` first from a working tree that has just been formatted. The final focused diff excludes generated-token churn.
- Lint still reports two pre-existing warnings in `client/lib/favicon-assets.test.ts`; they are not introduced by Task 6 and do not fail lint.

## Follow-up review fixes

- Added Scene assertions for visible leaderboard rank text (`#1`, `#2`) so the row contract is explicit.
- Added a retry Story test proving `RequestedLeaderboardRefresh` clears stale rows/error state and reloads page 0.
- Fixed the contradictory error UX by hiding `Load more` while `status === "error"`; `Try again` remains the full refresh path.
- Removed the dead `/leaderboard` link from the fallback shell nav in `client/app/view.ts`; Home/Leaderboard surfaces still own their own chrome.

### Follow-up verification

- Red: `cd client && bunx vitest run app/shell/surfaces.test.ts app/routes.test.ts`
  - Failed on `hides load more while the leaderboard shows a retry error` because `Load more` still rendered during the error state.
- Green: `cd client && bunx vitest run app/shell/surfaces.test.ts app/routes.test.ts`
  - `24 passed`, `0 failed`.
- Extra safety: `cd client && bun run typecheck`
  - Exit 0.
