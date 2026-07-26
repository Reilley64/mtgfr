# Task 5 Report: Thin-wrap shell surfaces onto `shellFrame`

## Status

DONE

## Summary

- Added shell surface Scene assertions for `shell-frame`, absent `#portrait-gate`, and no nested `main` inside the frame stage.
- Wrapped auth, deck list, deck builder, lobby, leaderboard, coverage, and not-found shell surfaces in `shellFrame`.
- Moved Play links into frame `leading` for leaderboard and coverage.
- Moved signed-in account chrome into frame `trailing`; builder and lobby reuse the existing deck-list account-menu state instead of adding per-surface menu state.
- Removed surface-owned app version badges and felt full-bleed wrappers from the wrapped shell surfaces.

## TDD Evidence

### RED

Command:

```bash
cd /workspace/client && bun vitest run app/shell/surfaces.test.ts
```

Result: failed as expected before production changes.

- 8 failing route assertions.
- Failure reason: `Expected element matching "[data-testid=\"shell-frame\"]" to exist but it does not.`

### GREEN

Command:

```bash
cd /workspace/client && bun vitest run app/shell/surfaces.test.ts
```

Result:

- 1 test file passed.
- 22 tests passed.

## Final Verification

Requested command:

```bash
cd /workspace/client && bun vitest run app/shell app/smoke.test.ts
```

Result:

- 19 test files passed.
- 117 tests passed.

Additional checks:

```bash
cd /workspace/client && bun run typecheck
cd /workspace/client && bun biome check --formatter-enabled=false app/shell/surfaces.test.ts app/shell/auth/view.ts app/shell/decks/list/view.ts app/shell/decks/builder/view.ts app/shell/decks/builder/story.test.ts app/shell/lobby/view.ts app/shell/lobby/entry.test.ts app/shell/leaderboard/view.ts app/shell/coverage/view.ts app/view.ts app/update.ts
```

Results:

- TypeScript typecheck passed.
- Biome checked 11 touched files with no fixes applied.

## Concerns

None.
