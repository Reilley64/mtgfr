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
- As a player assigning combat damage among battlefield blockers, I can click a blocker to move 1 damage onto it (steppers remain for fine control).
- As a player naming a card, I get a focused text field with a Card name placeholder, optional catalog typeahead suggestions, and can confirm with Name or Enter.
- As a player searching their library, I filter faces by name in docked `pending-library-aim` chrome (title, filter, and Choose/Fail to find stay pinned above the scroll strip).
- As a player choosing a creature type, I can filter the long option list by name before picking.
- As a player choosing a color or mana color, I pick from mana-font pip buttons (not letter labels).
- As a player answering a one-click on-board target choice, I aim at highlighted permanents/players (no card grid); optional choices keep Decline on the aim chrome.
- As a player targeting only cards in one graveyard, I click selectable pile cards under `pending-gy-aim` instead of a modal grid.
- As an opponent choosing a revealed card for the graveyard, I click a face in the docked `pending-revealed-aim` strip (or Choose none).
- As a player choosing battlefield or hand for a revealed card, I see the face in docked `pending-revealed-destination-aim` with Battlefield / Hand.
- As a player choosing target players, I aim at life-orb avatars on the board (multi-pick accumulates until Confirm).
- As a player scrying or surveilling, I assign looked-at cards into Top vs Bottom (or Graveyard) lanes in docked `pending-arrange-aim` instead of a center modal.
- As a player selecting from the top of my library, I assign cards into Take vs Bottom lanes in docked `pending-select-top-aim` (up to the allowed count).
- As a player distributing revealed cards from the top, I click cards across Hand / Bottom / Exile lanes in docked `pending-distribute-aim` (capacity per lane).
- As a player partitioning revealed cards, I click cards between Pile A and Pile B lanes in docked `pending-partition-aim`.
- As a player ordering triggers, I drag rows, click-to-place, or use ↑↓ in docked `pending-order-aim` so the last listed resolves first.
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
- When every `assign_combat_damage` blocker is on the battlefield, blockers highlight for on-board clicks (`pendingDamageAssignOverlay`); a click moves 1 damage onto that blocker (`clickDamageAssign` — steals from the largest other share, or adds under trample power). Chrome shows `pending-damage-aim` coach copy; Enter or Space submits when `damageAssignReady`. Blockers with amount > 0 paint Priority Gold (`pickedObjects`) and a crimson assign-amount badge (`assignAmounts`). On-board mode hides per-blocker steppers (board clicks + Assign only); off-board blockers keep steppers.
- When every `divide_spell_damage` target is a battlefield permanent, targets highlight for on-board clicks (`pendingDivideSpellOverlay`); a click moves 1 damage onto that target (index-keyed divide draft via `clickDamageAssign`). Chrome shows `pending-divide-aim` coach copy; Enter or Space submits when the assignment totals match. Targets with amount > 0 paint Priority Gold and crimson assign-amount badges. Player or off-board targets keep the modal steppers only. On-board mode hides per-target steppers.
- When every `divide_counters` permanent is on the battlefield, targets highlight via `pendingDamageAssignOverlay`; a click moves 1 counter onto that permanent. Chrome shows `pending-divide-counters-aim`; Enter or Space submits when `damageAssignReady`. Amount badges reuse combat-assign paint. On-board mode hides per-permanent steppers.
- `choose_card_name` uses an autofocused text field (`prompt-name-input`) with placeholder “Card name”; Enter submits when the trimmed name is non-empty (same gate as the Name button). Typing ≥2 characters fires `SearchCardNames`; matching results render under `prompt-name-suggestions` (click fills the draft). Catalog suggestions assist only — free-typed / nonexistent names remain submittable.
- `search_library` uses docked `pending-library-aim` (not the center modal): autofocused `pick-card-filter` (“Filter by name…”), face dedupe by label, filtered grid inside `pick-card-scroll`, with title / filter / Choose+Fail-to-find pinned above the scroll strip. Other card-pick kinds stay unfiltered.
- `choose_creature_type` shows an autofocused `prompt-type-filter` (“Filter types…”) and a scrolling option strip (`prompt-type-scroll`); only matching `pending.options` are clickable. Free-typed types outside the option list are not allowed.
- `choose_color` / `choose_mana_color` render WUBRG as mana-font pip buttons (`prompt-color-{i}` / `prompt-color-pip-{i}`) with color aria-labels; click still emits `choose_color` / `choose_mana_color` intents.
- One-click on-board `choose_target` / spell / ability targets suppress the `pending-choice` card grid and show `pending-target-aim` label chrome (plus optional Decline). Multi-target on-board aim shows `pending-target-count` and Confirm instead of the card grid. Off-board items keep the card / player picker.
- On-board battlefield sacrifice / proliferate / attach / phase-out / keep-tapped card-picks reuse the same `pending-target-aim` chrome (one-click or Confirm accumulate).
- Engine `discard` / `may_discard` with every item in hand suppress the card grid for `pending-discard-aim` hand-bar coach; `put_land_from_hand` / `put_creature_from_hand` / `put_from_hand_on_top` / `cast_creature_face_down` use `pending-hand-aim` (Decline stays for optional put-land/put-creature).
- Local `gyExilePick` with every choice in one graveyard shows `gy-exile-cost-aim` plus a selectable pile overlay (`pile-card-{id}`) instead of the modal `gy-exile-pick` grid.
- Engine GY card-picks (`exile_from_graveyard`, `may_return_from_graveyard`, `shuffle_from_graveyard`, `choose_dredge`, `pay_cumulative_upkeep_or_sacrifice`, GY-based `choose_activation_cost_targets`) and GY-only target prompts (`choose_target` / `choose_spell_targets` / `choose_ability_targets`) with a shared pile show `pending-gy-aim` and the same selectable pile overlay instead of the modal card grid.
- Battlefield `choose_activation_cost_targets` reuse `pending-target-aim` when every legal item is on the canvas.
- Engine exile card-picks (`choose_exiled_*` / `opponent_chooses_exiled_nonland`) with a shared exile pile show `pending-exile-aim` and selectable exile pile cards.
- `opponent_chooses_revealed_to_graveyard` shows docked `pending-revealed-aim` with one-click revealed faces (and Choose none) instead of the center card grid.
- `revealed_card_to_battlefield_or_hand` shows docked `pending-revealed-destination-aim` with the revealed face plus Battlefield / Hand.
- `choose_countered_spell_destination` shows docked `pending-destination-aim` with Top / Bottom.
- `may_yes_no` / `dance_exile_more` / `trade_secrets_repeat` show docked `pending-yes-no-aim` with Yes / No.
- `choose_target_players` / `choose_splitting_opponent` with seat-tagged items aim at life orbs (`pending-player-aim`); one-click when `max === 1` (or splitting); multi-pick accumulates seats in the player-pick draft with Confirm. Enter / Space submit when ready. Picked seats paint a solid Priority Gold ring (`pickedPlayers`).
- `scry` / `surveil` use docked `pending-arrange-aim` with two-lane arrange chrome (`prompt-arrange-lanes`): cards start in Bottom (library bottom or Graveyard for Surveil); click toggles a card between Top and Bottom, preserving left-to-right order in each lane. Done always submits `arrange_top` via partition draft `{ top, bottom }`.
- `select_from_top` uses docked `pending-select-top-aim` with Take vs Bottom lanes (`prompt-select-top-lanes`); click toggles into Take (capped at `up_to`); Done submits `select_from_top` with the Take ids.
- `distribute_top` uses docked `pending-distribute-aim` with Revealed / Hand / Bottom / Exile lanes (`prompt-distribute-lanes`); click cycles a card through lanes with room (`nextDistributeBucket`), then back to Revealed; Distribute enables when each lane hits its exact count.
- `partition_revealed` uses docked `pending-partition-aim` with Pile A / Pile B lanes (`prompt-partition-lanes`); click toggles a card between piles via `PromptCardToggled`.
- `order_triggers` uses docked `pending-order-aim`; rows support HTML5 drag reorder (`Draggable` / `OnDrop` → `PromptOrderRowClicked`, `OnDragEnd` → `PromptOrderDragEnded`), click-to-place (`orderPickPos`), and ↑↓ (`PromptOrderMoved`); list lives under `prompt-order-list`. Submit still emits `choose_order`.
- Enter or Space submits a ready lane / order draft (`order_triggers`, `scry`, `surveil`, `select_from_top`, `distribute_top`, `partition_revealed`) the same way the Done / Confirm button does (`trySubmitReadyPendingDraft`).
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
- Unit tests cover `clickDamageAssign` redistribution and trample under-assign.
- Scene tests cover trample overflow copy, damage steppers (no number inputs), on-board click coach (`pending-damage-aim`), and Assign enabled when under-assigned.
- Board pointer tests cover clicking a blocker during `assign_combat_damage` moves 1 damage onto it.
- Board pointer tests cover clicking a battlefield `divide_spell_damage` target moves 1 damage onto it; Space/Enter submit when the total matches.
- Scene tests cover on-board divide coach (`pending-divide-aim`).
- Board pointer tests cover clicking a battlefield `divide_counters` target moves 1 counter onto it; Space/Enter submit when ready (`pending-divide-counters-aim`).
- Scene tests cover Space/Enter submitting ready scry / order_triggers / distribute_top drafts (and refusing incomplete distribute_top).
- Scene/unit tests cover dredge decline (`Draw normally` → `dredger: null`) and single-pick readiness for Dredge.
- Scene tests cover pay-cost button copy (`Pay {…}` and kind-specific declines).
- Scene/unit tests cover join-forces mana stepper (no per-N buttons; draft submit).
- Scene/unit tests cover library-search docked aim (`pending-library-aim`), filter, face dedupe, pinned scroll chrome, Choose, and Fail to find.
- Scene tests cover `choose_card_name` typeahead list when suggestions match the draft query.
- Scene tests cover on-board pending aim chrome (`pending-target-aim`, no card grid) and optional Decline → empty `choose_targets`.
- Scene/pointer tests cover GY pile aim for `choose_target` when every legal item shares one graveyard (`pending-gy-aim`).
- Scene tests cover `opponent_chooses_revealed_to_graveyard` docked aim (one-click face → `choose_exiled_with_card`, Choose none declines).
- Scene tests cover `revealed_card_to_battlefield_or_hand` docked destination aim (Battlefield / Hand intents).
- Scene/pointer tests cover on-board `sacrifice_edict` one-click and `proliferate` accumulate → Confirm.
- Scene/unit tests cover docked scry/surveil `pending-arrange-aim` Top↔Bottom (Graveyard) lanes and click toggle → `arrange_top`.
- Scene tests cover docked `pending-select-top-aim` Take/Bottom lanes (no center `pending-choice`).
- Scene/unit tests cover docked `pending-distribute-aim` Hand/Bottom/Exile lanes and `nextDistributeBucket` cycling.
- Scene/unit tests cover docked `pending-partition-aim` Pile A / Pile B lanes and click → pile_a.
- Scene/unit tests cover order_triggers drag rows, click-to-place reorder, drag-end cancel, and ↑↓ chrome.
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Changing `.proto` choice shapes.
- Client-side inference of unavailable pending-choice kinds.
- Sparse illegal-X denylists and oracle “enters as N/N” hints.
- Engine `pending_choice` X formulators (none today); choose-X here is the board-local `xPrompt` path only.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
