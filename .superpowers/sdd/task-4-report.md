# Task 4 Report: Migrate lobby/coverage/version Commands + poll off `tryPromise`

## Summary

- Migrated lobby host/join/ready/start Commands to `yield* LobbyClient` and tagged error mapping.
- Migrated coverage and API-version Commands to `LobbyClient`.
- Removed `Effect.tryPromise` wrappers over lobby Promise APIs.
- Kept lobby polling on the module singleton because Foldkit subscriptions currently require an empty resource environment; poll still uses Effect and catches failures to skip a tick.
- Updated living specs for shell routes/auth, lobby entry UI, and coverage by set.

## TDD Evidence

### Red

Command:

```sh
cd client && bun run test app/shell/lobby/ app/shell/coverage/ app/fetch-api-version.test.ts
```

Result before implementation:

- Exit code: `1`
- Failures:
  - `FetchApiVersion > loads API metadata through LobbyClient` returned null metadata instead of injected client data.
  - `FetchCoverage > loads coverage metadata through LobbyClient` returned `CoverageLoadFailed` instead of injected client data.
  - `lobbyPoll > skips failed polls and keeps polling` emitted no recovered view with the service-backed draft.
  - `lobby commands > maps a missing lobby table to UnknownTable` returned `Unreachable` instead of `UnknownTable`.

### Green

Command:

```sh
cd client && bun run test app/shell/lobby/ app/shell/coverage/ app/fetch-api-version.test.ts
```

Result after implementation:

- Exit code: `0`
- `9` test files passed.
- `40` tests passed.

## Client Check Evidence

Initial command:

```sh
just client-check
```

Observed failures and fixes:

- Import ordering lint failure in `client/app/shell/lobby/update.test.ts`; fixed import order.
- Typecheck showed Foldkit lobby subscriptions require `R = never`; changed polling to the documented singleton fallback and kept Commands on `LobbyClient`.
- Full test run failed because local `mtgfr_web` was missing `lobbies`; ran `just client-migrate`.

Final command:

```sh
just client-check
```

Final result:

- Exit code: `0`
- Format: no fixes applied.
- Lint: `352` files checked, no errors.
- Typecheck: `tsc --noEmit` passed.
- Tests: `125` files passed, `1241` tests passed.

## Concerns

- Poll uses the app lobby HTTP singleton rather than injected `LobbyClient` because the current Foldkit subscription type does not carry app resources.
- Existing unrelated modification remains in `.superpowers/sdd/task-1-report.md` and was not staged for this task.
