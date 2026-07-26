# Task 5 report: Docked chooser + choose mode -> pipeline

## Status

Implemented.

## Changes

- Added `PlayModeChosen { actionId }` to board messages.
- Added docked `play-mode-aim` prompt chrome with `play-mode-{i}` buttons and Cancel.
- Prioritized `playModePick` in `promptsView`.
- Wired `PlayModeChosen` to clear `playModePick`, preserve stack park on success, and continue through `continueAfterCostPick`.
- Restored the parked hand card when the chosen continuation rejects or the chosen action id is stale.
- Cleared existing local action sessions when entering the play-mode chooser.
- Updated hand/prompts living specs for the shipped chooser behavior.

## TDD evidence

- RED: `bun test app/board/html/surfaces.test.ts app/board/scene.test.ts`
  - Failed on missing `play-mode-aim`.
  - Failed on missing `PlayModeChosen` handling.
  - Failed on missing local-session cleanup.
- RED follow-up: `bun test app/board/scene.test.ts`
  - Added rejection regression; failed because the parked hand card stayed hidden after a local reject.
- GREEN: `bun test app/board/html/surfaces.test.ts app/board/scene.test.ts`
  - 167 pass, 0 fail, 190 expectations.

## Self-review

- `continueAfterCostPick` was private but used from the new handler inside `submodel.ts`, so no export wrapper was needed.
- Successful take/cycle paths keep the existing stack park until game sync removes the card.
- Reject and stale-action paths clear the park so the hand card returns without submitting.
- Existing discard-cost hand-selection behavior remains intact; entering choose mode clears other active local sessions once the choose branch is actually entered.
