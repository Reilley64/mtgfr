# Prompts and Pending Choices
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/prompts.ts`, `client/lib/choice.ts`, `client/app/board/action/execution.ts`, `client/lib/ui/card-art.ts`, `client/lib/wire/types.ts`

## Problem Statement

The board must handle both local pre-submit prompts and engine `pending_choice` prompts. Each engine choice kind needs faithful UI that creates a valid answer without custom intent construction in the view.

## Solution

`PromptHost` is `promptsView`. It prioritizes local board prompts first, then renders `state.pending_choice` only for the awaited player. Engine choices use `FORMULATOR_FOR_KIND` to select a formulator and submit through `choiceIntent(pc, answer)`.

## User Stories

- As the awaited player, I get the correct prompt for the engine choice I must answer.
- As a non-deciding player or spectator, I do not see interactive prompt buttons for someone else’s choice.
- As a player choosing X, I see only legal X values as explicit buttons.
- As a player choosing cards, prompts use the same cached card art behavior as hand and stack.

## Behavior

- Local prompts render in this order: X prompt, modal cast, sacrifice pick, discard pick, graveyard-exile pick, staged target picker.
- Engine pending choices render only when `pending_choice.player === state.viewer` and the viewer is an active seated player.
- `pendingChoicePrompt` switches on `FORMULATOR_FOR_KIND[pending.kind]` and uses an exhaustive `never` default.
- All engine submissions go through `choiceIntent`.
- Card-pick prompts use `cardArt(h, opts)` for faces.
- `boardXPrompt` renders one button per legal X: `X = n` for every integer in `[minX, maxX]`.
- X prompts keep wire fields `min_x`, `max_x`, and `x_symbols` / `x_cost` as the server-authoritative contract, but the current UI is a button list, not a Min/−/field/+/Max stepper.

## Implementation Decisions

- Formulators collect `AnswerInput`; they do not build wire intents directly.
- `initPromptDraft`, `buildAnswerFromDraft`, and readiness helpers own draft validation.
- Local pre-submit prompts live in `BoardModel` and are not derived from shared `pending_choice`.
- `cardArt(h, opts)` has one DOM API and supports optional `style`.

## Testing Decisions

- Formulator registry tests ensure every `PendingChoiceView["kind"]` maps to a formulator.
- Scene tests cover awaited-player prompt visibility and non-decider/spectator suppression.
- X prompt tests assert `x-prompt` and `x-prompt-n` buttons for the legal range.
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Changing `.proto` choice shapes.
- Client-side inference of unavailable pending-choice kinds.
- Replacing the X button list with a stepper.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
