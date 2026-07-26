# Task 4 Report: Leaderboard page uses shared account chrome

## Status

**Complete.** The leaderboard header now uses the shared avatar account chrome from home, keeps the Play link, hides the leaderboard self-link, and clears the menu when leaderboard loads start.

## Changes

### `client/app/shell/leaderboard/view.ts`
- Replaced the standalone `Sign out` button with `accountChrome(...)`.
- Removed the `Signed in as ...` subtitle.
- Added `meGravatarHash` to the view signature and passed `showLeaderboardLink: false`.

### `client/app/view.ts`
- Passed `model.session.meGravatarHash` into `leaderboardView(...)`.

### `client/app/shell/leaderboard/update.ts`
- Updated `loadLeaderboard(...)` to set `accountMenuOpen: false` whenever a leaderboard load starts.

### `client/app/shell/surfaces.test.ts`
- Extended the leaderboard scene to assert:
  - `Play` remains visible.
  - the avatar trigger exists.
  - the shared chrome does not render the leaderboard self-link.
  - the old `Signed in as alice` subtitle is gone.
- Added a scene proving the leaderboard account menu renders and closes through `BindAccountMenuEscape`.

### `client/app/routes.test.ts`
- Extended the leaderboard refresh story to assert that retry/reset closes the account menu while re-entering the loading state.

## TDD evidence

1. **RED:** `cd /workspace/client && bun test app/shell/surfaces.test.ts -t "leaderboard"`
   Result: 2 failures — missing `account-menu-trigger`, missing open `account-menu`.
2. **GREEN:** implemented the shared chrome wiring and load reset.
3. **GREEN verification:** `cd /workspace/client && bun test app/shell/surfaces.test.ts app/routes.test.ts`
   Result: `31 pass, 0 fail`.

## Verification

Passed:

- `cd /workspace && just client-migrate`
- `cd /workspace/client && bun run lint && bun run typecheck && bun test app/shell/surfaces.test.ts app/routes.test.ts lib/lobby-store.test.ts`

Results:

- lint: completed with existing repo warnings only
- typecheck: pass
- tests: `36 pass, 0 fail`

## Self-review

No blocking issues found.

- Scope stayed inside the task brief; living specs were not updated.
- Reused the existing home/decks `accountChrome` pattern instead of duplicating header behavior.
- Added coverage for both visible chrome and the `loadLeaderboard` menu-reset path.

## Commit

`feat(client): share leaderboard account chrome`

## Concerns

- Repo lint still reports pre-existing `noNonNullAssertion` warnings outside this task (`app/board/motion/exit-fx.test.ts`, `lib/favicon-assets.test.ts`, `lib/gravatar.ts`), but they do not block the targeted verification above.
