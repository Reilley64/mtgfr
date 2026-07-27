# Task 2 Report: hide priority actions when a classified prompt is open

## Scope

- Updated `client/app/board/html/priority-bar.ts` to suppress idle priority-bar actions whenever
  `promptPresentation(board, state).mode !== "none"`.
- Added Scene coverage in `client/app/board/html/surfaces.test.ts` for both a simple prompt
  (`may_yes_no`) and a modal prompt (`search_library`).
- Updated `client/app/board/scene.test.ts` to reflect the new `playModePick` takeover behavior.
- Updated `docs/superpowers/specs/2026-07-20-turn-and-priority-chrome.md` so the surface spec
  matches the shipped behavior.
- Applied a lint-only import cleanup in `client/app/board/promptPresentation.ts` so the repo's
  `client-check` gate passes cleanly.

## TDD

### RED

Command:

```bash
cd /workspace/client && bunx vitest run app/board/html/surfaces.test.ts -t "hides idle priority"
```

Result:

```text
RUN  v4.1.10 /workspace/client

❯ app/board/html/surfaces.test.ts (90 tests | 2 failed | 88 skipped)
  × may_yes_no hides idle priority actions
  × library search hides idle priority actions

FAIL  ... Expected element matching testId "board-primary" to be absent but it exists.
```

Why red was valid:

- The new assertions were added before touching `priority-bar.ts`.
- Both prompt classifications still rendered the idle primary bar, so the failure proved the tests
  were exercising the missing behavior.

### GREEN

Command:

```bash
cd /workspace/client && bunx vitest run app/board/html/surfaces.test.ts -t "hides idle priority"
```

Result:

```text
RUN  v4.1.10 /workspace/client

Test Files  1 passed (1)
Tests       2 passed | 88 skipped (90)
```

## Implementation summary

- `priorityBarView` now classifies prompt takeover up front via `promptPresentation`.
- Any `simple` or `modal` prompt path returns the same priority-bar shell without idle controls,
  leaving `board-reject` available.
- Existing idle controls remain unchanged for `mode: "none"`.
- The parked `playModePick` test now asserts prompt-local cancel instead of bar-level cancel.

## Verification

Commands run:

```bash
cd /workspace/client && bunx vitest run app/board/html/surfaces.test.ts -t "hides idle priority"
cd /workspace/client && bunx vitest run app/board/scene.test.ts -t "playModePick prompt hides bar-level Cancel and keeps prompt Cancel"
cd /workspace/client && bunx vitest run app/board/html/surfaces.test.ts app/board/html/chrome.test.ts
cd /workspace && just client-migrate
cd /workspace && just client-check
```

Results:

- Targeted red/green prompt Scene test passed after the implementation.
- The updated `playModePick` scene regression passed.
- Related board chrome/surface suites passed (`106` tests).
- `just client-check` passed after applying the repo-required `client-migrate` step for the web DB.

## Self-review

- The change is intentionally narrow: it does not introduce prompt actions into the bar yet.
- `board-reject` still renders during prompt takeover, matching the task brief.
- The spec wording now distinguishes prompt-classified sessions from pure on-board staged targeting
  so `Cancel` semantics are explicit instead of implied.

## Commit

- Pending
