# Task 3 Report: Layout C entry view + Scene tests

## What I implemented

- Replaced the lobby entry choose/open-join UI in `client/app/shell/lobby/view.ts` with the Layout C entry surface from the brief.
- Added the new entry root `data-testid="lobby-entry"` and kept the selected deck card on the left with the action stack on the right.
- Rendered Host as the primary button, Join as a ghost inline action beside the always-present `lobby-join-code` field, and Back as a ghost link.
- Removed the old entry-only affordances from the entry view: `lobby-open-join`, `lobby-entry-join`, `lobby-bringing`, and `lobby-join-cancel`.
- Kept seated lobby rendering inside the panel while rendering the entry surface without the enclosing panel.
- Rewrote the focused Scene coverage in `client/app/shell/lobby/entry.test.ts` and `client/app/shell/surfaces.test.ts` for Layout C.
- Confirmed `client/app/shell/lobby/story.test.ts` had no remaining `entryMode` fixture, so no edit was needed there.
- Updated `docs/superpowers/specs/2026-07-20-lobby-entry-ui.md` so the shipped surface spec matches Layout C.

## What I tested and results

- RED:
  - `cd client && bun run test -- app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts`
  - Result: FAIL as expected.
  - Evidence: `lobby-entry` did not exist in `entry.test.ts` and `surfaces.test.ts`, and the host handoff Scene still failed on the missing `lobby-entry` root.
- GREEN:
  - `cd client && bun run test -- app/shell/lobby/entry.test.ts app/shell/lobby/story.test.ts app/shell/surfaces.test.ts app/shell/lobby/update.test.ts`
  - Result: PASS.
  - Evidence: `Test Files  4 passed (4)` and `Tests  45 passed (45)`.

## TDD Evidence

### RED

Command:

```bash
cd client && bun run test -- app/shell/lobby/entry.test.ts app/shell/surfaces.test.ts
```

Observed output:

```text
FAIL  app/shell/surfaces.test.ts > shell surface scenes > renders lobby entry with Host primary and soft-inline Join
Error: Expected element matching "[data-testid=\"lobby-entry\"]" to exist but it does not.

FAIL  app/shell/lobby/entry.test.ts > entry shows deck hero, Host primary, and soft-inline Join
Error: Expected element matching testId "lobby-entry" to exist but it does not.

FAIL  app/shell/lobby/entry.test.ts > host handoff on PlayRoute keeps entry UI (no claim-seat flash)
Error: Expected element matching testId "lobby-entry" to exist but it does not.
```

### GREEN

Command:

```bash
cd client && bun run test -- app/shell/lobby/entry.test.ts app/shell/lobby/story.test.ts app/shell/surfaces.test.ts app/shell/lobby/update.test.ts
```

Observed output:

```text
Test Files  4 passed (4)
Tests  45 passed (45)
```

## Files changed

- `client/app/shell/lobby/view.ts`
- `client/app/shell/lobby/entry.test.ts`
- `client/app/shell/surfaces.test.ts`
- `docs/superpowers/specs/2026-07-20-lobby-entry-ui.md`

## Self-review findings

- No functional issues found in self-review.
- The entry route now matches the task brief exactly: deck-left Layout C, primary Host, inline ghost Join, ghost Back, and no old join-mode affordances.
- The surface spec was updated in the same change, satisfying the project feature-spec rule for shipped UI behavior.

## Issues or concerns

- No blocking concerns.
