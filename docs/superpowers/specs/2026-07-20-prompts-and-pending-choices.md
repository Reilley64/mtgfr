# Prompts and Pending Choices
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/prompts.ts`, `client/app/board/html/pending-choice-waiting.ts`, `client/lib/choice.ts`, `client/lib/choiceWaiting.ts`, `client/lib/cardPickSearch.ts`, `client/lib/optionFilter.ts`, `client/lib/xCost.ts`, `client/app/board/action/execution.ts`, `client/lib/ui/card-art.ts`, `client/lib/wire/types.ts`

## Problem Statement

The board must handle both local pre-submit prompts and engine `pending_choice` prompts. Each engine choice kind needs faithful UI that creates a valid answer without custom intent construction in the view.

## Solution

`PromptHost` is `promptsView`. It prioritizes local board prompts first, then renders `state.pending_choice` only for the awaited player. Non-deciders and spectators see a passive waiting banner (`pending-choice-waiting`) naming the awaited seat. Engine choices use `FORMULATOR_FOR_KIND` to select a formulator and submit through `choiceIntent(pc, answer)`. Local choose-X uses a clamped Min/−/value/+/Max stepper with a live resolved-cost preview from wire `x_cost`.

## User Stories

- As the awaited player, I get the correct prompt for the engine choice I must answer.
- As a non-deciding player or spectator, I do not see interactive prompt buttons for someone else’s choice.
- As a non-deciding player or spectator, I see who the table is waiting on while a pending choice is open.
- As a player choosing X, I adjust a clamped stepper within server `min_x`…`max_x` and see what I will pay before confirming.
- As a player assigning combat damage with trample, I can leave leftover damage for the defending player and see that overflow before Assign.
- As a player dividing combat damage, spell damage, or counters, I adjust each share with a clamped stepper instead of typing numbers.
- As a player naming a card, I get a focused text field with a Card name placeholder, optional catalog typeahead suggestions, and can confirm with Name or Enter.
- As a player searching their library, I can filter faces by name while title, filter, and Choose stay pinned above a scrolling card grid.
- As a player choosing a creature type, I can filter the long option list by name before picking.
- As a player choosing a color or mana color, I pick from mana-font pip buttons (not letter labels).
- As a player offered dredge, I can pick one dredger or decline with Draw normally.
- As a player answering an optional-pay prompt, I see the mana cost on Pay and an outcome-specific decline label.
- As a player joining forces (`pay_any_amount_of_mana`), I adjust a Min/−/value/+/Max stepper up to my affordable max and confirm (0 declines).
- As a player choosing cards, prompts use the same cached card art behavior as hand and stack.

## Behavior

- Local prompts render in this order: X prompt, modal cast, sacrifice pick, discard pick, graveyard-exile pick, staged target picker.
- Engine pending choices render only when `pending_choice.player === state.viewer` and the viewer is an active seated player.
- When `pending_choice` is set for another seat (and the game is not mulliganing), `pendingChoiceWaitingView` shows `Waiting for {name}…` (`pending-choice-waiting`) for non-deciders and spectators. The awaited seat never sees this banner. Username falls back to `P{seat}` when empty.
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
- Combat damage, divide-spell damage, and divide-counters rows use Min/−/value/+/Max steppers (`prompt-damage-{id}-*`) capped at the attacker’s power or the division total — no raw `type=number` fields.
- `choose_card_name` uses an autofocused text field (`prompt-name-input`) with placeholder “Card name”; Enter submits when the trimmed name is non-empty (same gate as the Name button). Typing ≥2 characters fires `SearchCardNames`; matching results render under `prompt-name-suggestions` (click fills the draft). Catalog suggestions assist only — free-typed / nonexistent names remain submittable.
- `search_library` card picks are searchable: autofocused `pick-card-filter` (“Filter by name…”), face dedupe by label, filtered grid inside `pick-card-scroll`, with title / filter / Choose+Fail-to-find pinned (dialog `overflow-hidden`, scroll only on the card strip). Other card-pick kinds stay unfiltered.
- `choose_creature_type` shows an autofocused `prompt-type-filter` (“Filter types…”) and a scrolling option strip (`prompt-type-scroll`); only matching `pending.options` are clickable. Free-typed types outside the option list are not allowed.
- `choose_color` / `choose_mana_color` render WUBRG as mana-font pip buttons (`prompt-color-{i}` / `prompt-color-pip-{i}`) with color aria-labels; click still emits `choose_color` / `choose_mana_color` intents.
- `choose_dredge` requires exactly one selected dredger to enable Dredge; `prompt-decline` (“Draw normally”) submits `dredger: null` via `declineAnswer`.
- Optional-pay prompts (`pay_cost`, `pay_or_counter`, `pay_or_controller_draws`, `pay_echo_or_sacrifice`, `pay_recover_or_exile`, `sacrifice_unless_pay`) label the affirm button `Pay ${costText(cost)}` and use outcome-specific declines: Don’t pay / Let it be countered / Let them draw / Sacrifice / Exile.
- `pay_any_amount_of_mana` (join forces) uses a clamped stepper over `[0, max]` with draft on `promptDraft` (`PromptNumberSet`); Confirm submits via `PromptSubmitted`. Per-N buttons (`prompt-number-N`) are not used for this kind. `may_draw_up_to` / `trade_secrets_caster_draw` keep one-click number buttons.

## Implementation Decisions

- Formulators collect `AnswerInput`; they do not build wire intents directly.
- `initPromptDraft`, `buildAnswerFromDraft`, and readiness helpers own draft validation.
- Local pre-submit prompts live in `BoardModel` and are not derived from shared `pending_choice`.
- `cardArt(h, opts)` has one DOM API and supports optional `style`.
- Pure X helpers live in `client/lib/xCost.ts` (`clampX`, `costWithChosenX`, `costText`).
- Choose-X preview uses brace text rather than hand-bar mana-font pips so large resolved generics cannot collapse to a false `{0}`.
- Waiting copy lives in `client/lib/choiceWaiting.ts`; the banner is composed in `boardOverlays` (not inside `promptsView`) so spectators see it without seated prompt chrome.
- Library-search filter helpers live in `client/lib/cardPickSearch.ts`; filter draft is optional `filter` on `PromptDraft` `card-pick`, updated via `PromptCardFilterSet`.

## Testing Decisions

- Formulator registry tests ensure every `PendingChoiceView["kind"]` maps to a formulator.
- Scene tests cover awaited-player prompt visibility and non-decider/spectator suppression plus waiting-banner copy.
- Unit tests cover `pendingChoiceWaitingText` (null for decider / absent / mulligan; named seat and `P{seat}` fallback).
- X prompt Scene tests assert stepper controls, preview text (e.g. `Pay {4}`), confirm, disabled `+` at max, and absence of per-X buttons (`x-prompt-n`).
- Unit tests cover `clampX`, `costWithChosenX` (multi-symbol X and colored pips), and `costText` for large generics.
- Unit tests cover `damageAssignReady` for exact-sum non-trample and under-assign / over-assign / negative trample cases.
- Scene tests cover trample overflow copy, damage steppers (no number inputs), and Assign enabled when under-assigned.
- Scene/unit tests cover dredge decline (`Draw normally` → `dredger: null`) and single-pick readiness for Dredge.
- Scene tests cover pay-cost button copy (`Pay {…}` and kind-specific declines).
- Scene/unit tests cover join-forces mana stepper (no per-N buttons; draft submit).
- Scene/unit tests cover library-search filter, face dedupe, and pinned scroll chrome (`pick-card-filter`, `pick-card-scroll`).
- Scene tests cover `choose_card_name` typeahead list when suggestions match the draft query.
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Changing `.proto` choice shapes.
- Client-side inference of unavailable pending-choice kinds.
- Sparse illegal-X denylists and oracle “enters as N/N” hints.
- Engine `pending_choice` X formulators (none today); choose-X here is the board-local `xPrompt` path only.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
