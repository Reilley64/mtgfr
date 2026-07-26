# Task 4 Report: Enter / cancel `playModePick` (stack park)

## Status

Complete.

## Changes

- Added `BoardModel.playModePick: PlayModePick | null`.
- Reworked hand activation to re-resolve all current hand-section modes for the object.
- Multi-mode hand plays now seed the stack flight, hide the hand card, and park a `playModePick`.
- Single-mode hand plays continue through the existing cost / modal / target / submit pipeline.
- `CancelActionClicked` now clears `playModePick`, removes the seeded flight, and restores the hand card.
- Priority bar Cancel now appears while `playModePick` is parked.
- Updated hand-bar and priority-chrome living specs.

## TDD evidence

### RED

Command:

```sh
cd /workspace/client && bun test app/board/scene.test.ts
```

Result:

- `HandActionActivated with two hand modes parks card and opens playModePick` failed because the first bound action submitted immediately.
- `CancelActionClicked clears playModePick and restores the hand card` failed because `playModePick` was not present.
- `HandActionActivated with one mode does not open playModePick` failed because the model did not expose the null state yet.
- `priority bar shows Cancel while playModePick is parked` failed because Cancel was absent.
- Summary: `77 pass, 4 fail`.

### GREEN

Command:

```sh
cd /workspace/client && bun test app/board/scene.test.ts
```

Result: `81 pass, 0 fail`.

## Verification

Passed:

- `cd /workspace/client && bun test app/board/scene.test.ts` -> `81 pass, 0 fail`
- `cd /workspace && just client-typecheck` -> `tsc --noEmit` completed successfully

## Self-review

- Scope matches Task 4: model state, park/cancel, single-mode auto path only.
- No docked chooser UI was added.
- The unrelated existing `.superpowers/sdd/task-3-report.md` working-tree change was left untouched.

## Commit

`feat(client): park hand card while choosing play mode`

## Concerns

- None.
