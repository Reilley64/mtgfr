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
- As a player assigning combat damage with trample, I can leave leftover damage for the defending player and see that overflow before Assign.
- As a player offered dredge, I can pick one dredger or decline with Draw normally.
- As a player answering an optional-pay prompt, I see the mana cost on Pay and an outcome-specific decline label.
- As a player choosing cards, prompts use the same cached card art behavior as hand and stack.

## Behavior

- Local prompts render in this order: X prompt, modal cast, sacrifice pick, discard pick, graveyard-exile pick, staged target picker.
- Engine pending choices render only when `pending_choice.player === state.viewer` and the viewer is an active seated player.
- `pendingChoicePrompt` switches on `FORMULATOR_FOR_KIND[pending.kind]` and uses an exhaustive `never` default.
- All engine submissions go through `choiceIntent`.
- Card-pick prompts use `cardArt(h, opts)` for faces.
- `boardXPrompt` is a stepper over `[minX, maxX]`:
  - Draft value lives on `XPromptState.draftX`, initialized to `clampX(maxX, minX, maxX)` when the prompt opens.
  - Min / − / + / Max dispatch `XDraftSet` (clamped into `[minX, maxX]`); Confirm dispatches `XSubmitted` with a clamped `x`.
  - − is disabled at `minX`; + is disabled at `maxX`.
  - Preview row (`x-prompt-preview`) shows `Pay ${costText(costWithChosenX(xCost, draftX))}` — brace text so resolved generics outside mana-font’s 0–20 range stay accurate.
  - Cancel clears the prompt via existing `CancelActionClicked`.
- Wire fields `min_x`, `max_x`, and `x_cost` / `x_symbols` remain the server-authoritative contract; the client does not invent affordability.
- When `x_symbols` is omitted, `costWithChosenX` treats it as `1` if `has_x`, else `0`.
- When `maxX < minX`, `clampX` returns `minX` (client stays safe if the server sends a bad range).
- `assign_combat_damage` readiness (`damageAssignReady`) mirrors the engine: non-trample requires the sum of non-negative blocker amounts to equal the attacker’s power; trample requires `0 ≤ sum ≤ power` (overflow trampling is automatic).
- Trample’s prompt shows `assigned N / power` plus a `to defender: R` overflow line (`prompt-damage-overflow`). Non-trample prompts omit that line.
- `choose_dredge` requires exactly one selected dredger to enable Dredge; `prompt-decline` (“Draw normally”) submits `dredger: null` via `declineAnswer`.
- Optional-pay prompts (`pay_cost`, `pay_or_counter`, `pay_or_controller_draws`, `pay_echo_or_sacrifice`, `pay_recover_or_exile`, `sacrifice_unless_pay`) label the affirm button `Pay ${costText(cost)}` and use outcome-specific declines: Don’t pay / Let it be countered / Let them draw / Sacrifice / Exile.

## Implementation Decisions

- Formulators collect `AnswerInput`; they do not build wire intents directly.
- `initPromptDraft`, `buildAnswerFromDraft`, and readiness helpers own draft validation.
- Local pre-submit prompts live in `BoardModel` and are not derived from shared `pending_choice`.
- `cardArt(h, opts)` has one DOM API and supports optional `style`.
- Pure X helpers live in `client/lib/xCost.ts` (`clampX`, `costWithChosenX`, `costText`).
- Choose-X preview uses brace text rather than hand-bar mana-font pips so large resolved generics cannot collapse to a false `{0}`.

## Testing Decisions

- Formulator registry tests ensure every `PendingChoiceView["kind"]` maps to a formulator.
- Scene tests cover awaited-player prompt visibility and non-decider/spectator suppression.
- X prompt Scene tests assert stepper controls, preview text (e.g. `Pay {4}`), confirm, disabled `+` at max, and absence of per-X buttons (`x-prompt-n`).
- Unit tests cover `clampX`, `costWithChosenX` (multi-symbol X and colored pips), and `costText` for large generics.
- Unit tests cover `damageAssignReady` for exact-sum non-trample and under-assign / over-assign / negative trample cases.
- Scene tests cover trample overflow copy and Assign enabled when under-assigned.
- Scene/unit tests cover dredge decline (`Draw normally` → `dredger: null`) and single-pick readiness for Dredge.
- Scene tests cover pay-cost button copy (`Pay {…}` and kind-specific declines).
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Changing `.proto` choice shapes.
- Client-side inference of unavailable pending-choice kinds.
- Sparse illegal-X denylists and oracle “enters as N/N” hints.
- Engine `pending_choice` X formulators (none today); choose-X here is the board-local `xPrompt` path only.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
