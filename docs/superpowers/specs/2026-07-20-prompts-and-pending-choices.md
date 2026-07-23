# Prompts and Pending Choices
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/prompts.ts`, `client/lib/choice.ts`, `client/lib/xCost.ts`, `client/app/board/action/execution.ts`, `client/lib/ui/card-art.ts`, `client/lib/wire/types.ts`

## Problem Statement

The board must handle both local pre-submit prompts and engine `pending_choice` prompts. Each engine choice kind needs faithful UI that creates a valid answer without custom intent construction in the view.

## Solution

`PromptHost` is `promptsView`. It prioritizes local board prompts first, then renders `state.pending_choice` only for the awaited player. Engine choices use `FORMULATOR_FOR_KIND` to select a formulator and submit through `choiceIntent(pc, answer)`. Local choose-X uses a clamped Min/−/value/+/Max stepper with a live resolved-cost preview from wire `x_cost`.

## User Stories

- As the awaited player, I get the correct prompt for the engine choice I must answer.
- As a non-deciding player or spectator, I do not see interactive prompt buttons for someone else’s choice.
- As a player choosing X, I adjust a clamped stepper within server `min_x`…`max_x` and see what I will pay before confirming.
- As a player choosing cards, prompts use the same cached card art behavior as hand and stack.

## Behavior

- Local prompts render in this order: X prompt, modal cast, sacrifice pick, discard pick, graveyard-exile pick, staged target picker.
- Engine pending choices render only when `pending_choice.player === state.viewer` and the viewer is an active seated player.
- `pendingChoicePrompt` switches on `FORMULATOR_FOR_KIND[pending.kind]` and uses an exhaustive `never` default.
- All engine submissions go through `choiceIntent`.
- Card-pick prompts use `cardArt(h, opts)` for faces.
- `boardXPrompt` is a stepper over `[minX, maxX]`:
  - Draft value lives on `XPromptState.draftX` (defaults to `maxX`, clamped).
  - Min / − / + / Max dispatch `XDraftSet`; Confirm dispatches `XSubmitted` with the draft.
  - Preview row shows Pay + `costPips(costWithChosenX(xCost, draftX))` (`x-prompt-preview`).
- Wire fields `min_x`, `max_x`, and `x_cost` / `x_symbols` remain the server-authoritative contract; the client does not invent affordability.

## Implementation Decisions

- Formulators collect `AnswerInput`; they do not build wire intents directly.
- `initPromptDraft`, `buildAnswerFromDraft`, and readiness helpers own draft validation.
- Local pre-submit prompts live in `BoardModel` and are not derived from shared `pending_choice`.
- `cardArt(h, opts)` has one DOM API and supports optional `style`.
- Pure X helpers live in `client/lib/xCost.ts` (`clampX`, `costWithChosenX`); preview reuses hand-bar `costPips`.

## Testing Decisions

- Formulator registry tests ensure every `PendingChoiceView["kind"]` maps to a formulator.
- Scene tests cover awaited-player prompt visibility and non-decider/spectator suppression.
- X prompt Scene tests assert `x-prompt`, stepper controls, preview, and confirm — not one button per X.
- Unit tests cover `clampX` / `costWithChosenX` (multi-symbol X and colored pips).
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Changing `.proto` choice shapes.
- Client-side inference of unavailable pending-choice kinds.
- Sparse illegal-X denylists and oracle “enters as N/N” hints.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
- Design for the stepper restore: [2026-07-23-choose-x-stepper-design.md](2026-07-23-choose-x-stepper-design.md).
