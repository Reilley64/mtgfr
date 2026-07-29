# Prompts and Pending Choices
**Status:** Current (as of 2026-07-28)
**Module:** `client/app/board/html/prompts.ts`, `client/app/board/html/prompt-bar-actions.ts`, `client/app/board/html/pending-choice-waiting.ts`, `client/app/domain/choice.ts`, `client/app/domain/choiceWaiting.ts`, `client/app/domain/cardPickSearch.ts`, `client/app/domain/optionFilter.ts`, `client/app/domain/xCost.ts`, `client/app/board/action/execution.ts`, `client/app/domain/ui/card-art.ts`, `client/app/domain/wire/types.ts`

## Problem Statement

The board must handle both local pre-submit prompts and engine `pending_choice` prompts. Each engine choice kind needs faithful UI that creates a valid answer without custom intent construction in the view.

## Solution

`PromptHost` is `promptsView`. It prioritizes local board prompts first, then renders `state.pending_choice` only for the awaited player. Non-deciders and spectators see a passive waiting banner (`pending-choice-waiting`) naming the awaited seat. Engine choices use `FORMULATOR_FOR_KIND` to select a formulator and submit through `choiceIntent(pc, answer)`. Effect titles, mode labels, and trigger-order labels arrive as `MessageRef` and are formatted with `formatMessage`; `ChoiceItem.label` remains the visible object/seat name. Public graveyard single-pick prompts such as `may_exile_discarded_to_play` reuse the graveyard card-pick path and still answer through `choose_sacrifices` with either one chosen id or an empty decline. Local choose-X uses a clamped Min/−/value/+/Max stepper with a live resolved-cost preview from wire `x_cost`.

## User Stories

- As the awaited player, I get the correct prompt for the engine choice I must answer.
- As a non-deciding player or spectator, I do not see interactive prompt buttons for someone else’s choice.
- As a non-deciding player or spectator, I see who the table is waiting on while a pending choice is open.
- As a player choosing X, I adjust a clamped stepper within server `min_x`…`max_x` and see what I will pay before confirming.
- As a player assigning combat damage with trample, I can leave leftover damage for the defending player and see that overflow before Assign.
- As a player dividing combat damage, spell damage, or counters, I assign on-board targets via board clicks and Assign in the primary bar; clamped steppers remain only when off-board blockers or targets remain.
- As a player assigning combat damage among battlefield blockers, I click blockers to move 1 damage and confirm with Assign in the primary bar (no per-blocker steppers on-board).
- As a player naming a card, I use centered `pending-card-name-modal` with a focused Card name field, arrow through catalog typeahead suggestions, and submit with Name.
- As a player searching their library, I filter faces by name in centered `pending-library-modal` chrome while Choose / Fail to find stay inside the modal footer.
- As a player choosing a creature type, I filter the long option list by name in centered `pending-creature-type-modal` before picking.
- As a player choosing a color or mana color, I pick from mana-font pip buttons in centered `pending-color-modal` (not letter labels).
- As a player answering a one-click on-board target choice, I aim at highlighted permanents/players (no card grid); optional choices keep Decline in the primary bar (`priority-context-bar`).
- As a player targeting only cards in one graveyard, I click selectable pile cards under `pending-gy-aim` instead of a modal grid.
- As an opponent choosing a revealed card for the graveyard, I click a face in the docked `pending-revealed-aim` strip (or Choose none).
- As a player choosing battlefield or hand for a revealed card, I see the face in docked `pending-revealed-destination-aim` while Battlefield / Hand lives in `priority-context-bar`.
- As a player choosing target players, I aim at life-orb avatars on the board (multi-pick accumulates until Confirm).
- As a player scrying or surveilling, I assign looked-at cards into Top vs Bottom (or Graveyard) lanes in centered `pending-arrange-modal`.
- As a player selecting from the top of my library, I assign cards into Take vs Bottom lanes in centered `pending-select-top-modal` (up to the allowed count).
- As a player distributing revealed cards from the top, I click cards across Hand / Bottom / Exile lanes in centered `pending-distribute-modal` (capacity per lane).
- As a player partitioning revealed cards, I click cards between Pile A and Pile B lanes in centered `pending-partition-modal`.
- As a player ordering triggers, I drag rows, click-to-place, or use ↑↓ in centered `pending-order-modal` so the last listed resolves first.
- As a player offered dredge, I can pick one dredger or decline with Draw normally.
- As a player answering an optional-pay prompt, I see the mana cost on Pay and an outcome-specific decline label.
- As a player choosing an off-board card target or copy target, I use centered modal card-pick chrome instead of a docked coach.
- As a player joining forces (`pay_any_amount_of_mana`), I adjust a centered `pending-join-forces-modal` Min/−/value/+/Max stepper up to my affordable max and confirm (0 declines).
- As a player paying an off-board local sacrifice or discard cost, I use a centered modal picker; graveyard-exile costs stay on the graveyard coach only when every choice shares one visible pile.
- As a player choosing trigger modes, I multi-select mode rows in docked `pending-trigger-modes-aim` and then Choose or Cancel from `priority-context-bar`.
- As a player choosing between two piles (`opponent_chooses_pile` / `choose_pile_for_hand`), I inspect docked `pending-pile-aim` card labels and pick Pile A or Pile B from `priority-context-bar`.
- As a player choosing cards, prompts use the same cached card art behavior as hand and stack.
- As a player resolving `may_exile_discarded_to_play`, I pick one discarded nonland card from my graveyard to exile and play this turn, or decline with explicit `Don't exile` chrome.

## Behavior

- Local prompts render in this order: hand play-mode chooser, X prompt, modal cast, sacrifice pick, discard pick, graveyard-exile pick, staged target picker.
- `promptPresentation(board, state)` owns prompt classification for both local sessions and engine `pending_choice`: local prompt sessions win first, then the seated viewer's own `pending_choice`; staged target pickers returned by `stagedPickTargets(...)` classify as local `modal`, while only pure staged on-board arrow aim that keeps the ordinary Cancel/priority path stays `none`.
- Classified `simple` prompts keep only informational coach chrome at the bottom dock (`*-aim`, title/count copy only) while their answer buttons move into `priority-context-bar`.
- Classified `modal` prompts use centered `*-modal` shells with a dimmed backdrop and keep answer actions inside the modal; the primary priority controls stay hidden while that modal is open.
- Pending-choice kinds without a dedicated simple mapping fall back to `modal` so new or off-board pickers do not invent primary-bar buttons.
- Local `playModePick` is board session chrome alongside `modalCast`, cost picks, and staged targets (not an engine `pending_choice`). Multi-action hand cards use docked `play-mode-aim` as a title-only coach while `priority-context-bar` carries one `play-mode-{i}` button per legal mode plus Cancel; choosing a row dispatches `PlayModeChosen` and keeps the parked stack flight while the selected action continues through cost, modal, target, or submit steps. Cancel clears the park and returns the card to hand without submitting an intent.
- While `play-mode-aim` is open, snapshot and delta sync prune buttons whose action ids are no longer legal. Exactly one remaining mode auto-continues that mode; zero remaining modes cancel the session, return the card to hand, and submit no intent. A stale `PlayModeChosen` for a pruned action id clears `playModePick`, returns the card to hand, and emits no command.
- Local modal spells use docked `modal-mode-aim` as a title-only coach while `priority-context-bar` carries the mode rows plus Cast/Cancel (center `modal-mode-picker` unused). Multi-select mode rows keep toggle semantics via `aria-pressed` so the selected set is exposed to assistive tech. After modes are chosen and a target is still needed, docked `modal-waiting-aim` replaces center `modal-waiting`, and its Cancel action also lives in `priority-context-bar`.
- Off-board staged-action targets use centered `target-pick-modal` (scrollable face/player strip + Cancel; center `target-pick` unused).
- Local cost pickers that cannot use the hand or graveyard board overlays fall back to centered modal shells: `sacrifice-pick-modal`, `discard-pick-modal`, and `gy-exile-pick` when the graveyard-exile cost cannot reuse shared-pile `gy-exile-cost-aim`.
- Engine pending choices render only when `pending_choice.player === state.viewer` and the viewer is an active seated player.
- When `pending_choice` is set for another seat (and the game is not mulliganing), `pendingChoiceWaitingView` shows `Waiting for {name}…` (`pending-choice-waiting`) for non-deciders and spectators. The awaited seat never sees this banner. Username falls back to `P{seat}` when empty.
- `pendingChoicePrompt` switches on `FORMULATOR_FOR_KIND[pending.kind]` and uses an exhaustive `never` default.
- All engine submissions go through `choiceIntent`.
- `pendingChoiceTitle`, mode rows, order-trigger rows, pay-cost effect text, and target-aim chrome format wire `MessageRef` labels with `formatMessage`.
- Missing catalog entries render the raw key, which keeps prompts visible and makes drift obvious in development.
- Card-pick prompts use `cardArt(h, opts)` for faces.
- `boardXPrompt` is a stepper over `[minX, maxX]` in centered `x-prompt-modal`:
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
- When every `assign_combat_damage` blocker is on the battlefield, blockers highlight for on-board clicks (`pendingDamageAssignOverlay`); a click moves 1 damage onto that blocker (`clickDamageAssign` — steals from the largest other share, or adds under trample power). Chrome shows docked `pending-damage-aim` coach copy plus assigned total, while `priority-context-bar` carries Assign; Enter or Space submits when `damageAssignReady`. Blockers with amount > 0 paint Priority Gold (`pickedObjects`) and a crimson assign-amount badge (`assignAmounts`). On-board mode hides per-blocker steppers; off-board blockers keep steppers and Assign in the same docked HUD.
- When every `divide_spell_damage` target is a battlefield permanent, targets highlight for on-board clicks (`pendingDivideSpellOverlay`); a click moves 1 damage onto that target (index-keyed divide draft via `clickDamageAssign`). Chrome shows docked `pending-divide-aim` coach copy plus assigned total, while `priority-context-bar` carries Assign; Enter or Space submits when the assignment totals match. Targets with amount > 0 paint Priority Gold and crimson assign-amount badges. Player or off-board targets keep steppers and Assign in the same docked HUD. On-board mode hides per-target steppers.
- When every `divide_counters` permanent is on the battlefield, targets highlight via `pendingDamageAssignOverlay`; a click moves 1 counter onto that permanent. Chrome shows docked `pending-divide-counters-aim` coach copy plus assigned total, while `priority-context-bar` carries Assign; Enter or Space submits when `damageAssignReady`. Amount badges reuse combat-assign paint. On-board mode hides per-permanent steppers; off-board targets keep steppers and Assign in the same docked HUD.
- `choose_card_name` shows centered `pending-card-name-modal` around a `@foldkit/ui` Combobox (`CardNameCombobox`, id `prompt-name`): an autofocused text field (`prompt-name-input`, placeholder “Card name”) whose suggestion list (`prompt-name-suggestions`, rows `prompt-name-suggestion-{i}`) opens as you type. Typing ≥2 characters fires `SearchCardNames`; ArrowUp/ArrowDown move the active row, Enter commits the highlighted name into the draft, Escape closes the list, and clicking a row still fills the draft. The Name button submits when the trimmed name is non-empty — Enter does not, because the combobox owns the input's keydown. Catalog suggestions assist only — free-typed / nonexistent names remain submittable. The list is anchored and portaled to `document.body`, so it carries `z-50` to clear the `z-40` prompt frame.
- `search_library` uses centered `pending-library-modal`: autofocused `pick-card-filter` (“Filter by name…”), face dedupe by label, filtered grid inside `pick-card-scroll`, and Choose / Fail-to-find actions in the modal footer. Other off-board card-pick kinds use centered `pending-card-pick-modal` (unfiltered grid + actions; no name filter).
- `choose_creature_type` shows centered `pending-creature-type-modal` with an autofocused `prompt-type-filter` (“Filter types…”) and a scrolling option strip (`prompt-type-scroll`); only matching `pending.options` are clickable. Free-typed types outside the option list are not allowed.
- `choose_color` / `choose_mana_color` use centered `pending-color-modal`: WUBRG as mana-font pip buttons (`prompt-color-{i}` / `prompt-color-pip-{i}`) with color aria-labels; click still emits `choose_color` / `choose_mana_color` intents.
- One-click on-board `choose_target` / spell / ability targets suppress the `pending-choice` card grid and show `pending-target-aim` label chrome. When `min === 0`, `priority-context-bar` carries Decline while the coach stays button-free. Multi-target on-board aim shows `pending-target-count` in the coach and Confirm in `priority-context-bar` instead of the card grid. Off-board card items use centered `pending-card-pick-modal`; seat-tagged `choose_target` lists that are not fully on-canvas (mixed with off-board objects) use centered `pending-player-pick-modal` player buttons.
- On-board battlefield sacrifice / proliferate / attach / phase-out / keep-tapped / legend-rule (`choose_legendary_keep`) card-picks reuse the same `pending-target-aim` coach (one-click or accumulated picks). Legend rule titles read `Legend rule — choose which {name} to keep`. `proliferate` items mix permanents and players (CR 701.27); clicking a life orb accumulates that seat in the card-pick draft's `players` list (player items all carry `id: 0`, so seats cannot ride in `picked`), picked seats paint Priority Gold via `pickedPlayersFromDraft` (same ring as `choose_target_players`), permanent toggles preserve those seats, and `priority-context-bar` submits Confirm for `choose_proliferate { permanents, players }`. CR 701.27b "proliferate twice" / "proliferate X times" re-raises with the same item ids, so `choiceDraftKey` alone does not distinguish iterations: after Confirm, `promptSubmitInFlight` still clears (and the draft re-inits) when a newer board `seq` arrives with that same key, so the next iteration's Confirm is not a silent no-op.
- `decline_untap` picks the permanents that stay tapped (`Keep tapped`). It carries `at_most_one` — the Smoke / Winter Orb groups (CR 502.2) from which at most one permanent may untap — and `cardPickReady` holds the submit action shut while any group would leave two members untapped, with the hint `Only one of the capped permanents may untap — keep the rest tapped.` A cap is a ceiling, not a quota: keeping a whole group tapped is a legal answer. An empty `at_most_one` is the plain Rubinia-style pause, a free yes/no per permanent.
- Engine `discard` / `may_discard` with every item in hand use `pending-discard-aim`: click toggles hand selection (raised face + Llanowar ring), the coach shows count, and `priority-context-bar` carries Confirm / Continue when ready (including count 1). Local `discardPick` with choices in hand uses the same select → Confirm pattern on `discard-cost-aim` (`discard-cost-count`, `DiscardCostConfirmed`), with Confirm/Cancel in `priority-context-bar`. Hand put / face-down prompts (`put_land_from_hand`, `put_creature_from_hand`, `put_from_hand_on_top`, `cast_creature_face_down`) use the same select → Confirm pattern on `pending-hand-aim` (`pending-hand-count`; Put onto battlefield / Put on top / Cast face down in `priority-context-bar`, plus optional Don't put a land / Don't put a creature when decline is legal).
- Local `gyExilePick` with every choice in one graveyard shows `gy-exile-cost-aim` plus a selectable pile overlay (`pile-card-{id}`) instead of the modal `gy-exile-pick` grid; any Cancel / Exile action moves into `priority-context-bar`.
- Engine GY card-picks (`exile_from_graveyard`, `may_return_from_graveyard`, `shuffle_from_graveyard`, `choose_dredge`, `pay_cumulative_upkeep_or_sacrifice`, GY-based `choose_activation_cost_targets`) and GY-only `choose_target` prompts with a shared pile show `pending-gy-aim` and the same selectable pile overlay instead of the modal card grid, with submit / decline buttons in `priority-context-bar`.
- `may_exile_discarded_to_play` uses the same shared-graveyard `pending-gy-aim` path: the coach copy names the discarded nonland exile, while `priority-context-bar` carries `Exile` and `Don't exile`; when the cards do not share one visible graveyard pile it falls back to the generic card-pick surface titled `Choose a discarded nonland card to exile and play this turn`.
- `may_return_from_graveyard` carries a projected wire `mandatory` flag (from the card's `mandatory` DSL axis). When `false` (optional "you may return" — Deadly Brew, Witch of the Moors) the graveyard aim keeps an explicit `Don't return` action in `priority-context-bar` and enables `Return` there only after one card is picked. When `true` (mandatory "you return" — Witherbloom Command mode 0) the prompt hides that decline action, keeps `Return` disabled until one legal card is picked, and only appears when at least one matching card is in the graveyard (no legal card means the effect does nothing and no prompt is raised).
- Battlefield `choose_activation_cost_targets` reuse `pending-target-aim` when every legal item is on the canvas.
- Engine exile card-picks (`choose_exiled_*` / `opponent_chooses_exiled_nonland`) with a shared exile pile show `pending-exile-aim` and selectable exile pile cards, with Choose / decline actions in `priority-context-bar` (above the pile backdrop).
- `choose_exiled_to_cast_free` is an up-to-N free-cast pick (Plargg and Nassari / Abstract Performance): any `0..=count` selection is submittable, the coach count reads `N / up to count selected`, and the modal title (when mixed-owner piles force the card grid) says “Choose up to N…”.
- `choose_exiled_dig_to_cast_free` with non-empty `cast_targets` (Aura dig) is two-step: after the exile pick, chrome switches to `pending-target-aim` (“Choose what to enchant”) over projected hosts; submit carries both `choice` and object `target`. Untargeted dig (empty `cast_targets`) stays one-step exile aim.
- `opponent_chooses_revealed_to_graveyard` shows docked `pending-revealed-aim` with one-click revealed faces, while `priority-context-bar` carries `Choose none`.
- `revealed_card_to_battlefield_or_hand` shows docked `pending-revealed-destination-aim` with the revealed face while Battlefield / Hand lives in `priority-context-bar`.
- `choose_countered_spell_destination` shows docked `pending-destination-aim` while Top / Bottom lives in `priority-context-bar`.
- `may_yes_no` / `dance_exile_more` keep docked `pending-yes-no-aim` as a bottom coach title only; the actual Yes / No actions move into `priority-context-bar` with the primary-bar silhouette. Trade Secrets uses the same generic `may_yes_no` prompt shape rather than a dedicated wire kind. Snarl / Port Town may-reveal-land uses the same shape with MessageRef `effect.choice_may_reveal_land_from_hand` (`answer_may`).
- `choose_mode` shows docked `pending-mode-aim` as a title-only coach while `priority-context-bar` carries the one-click mode labels (`prompt-mode-{i}`).
- `choose_trigger_modes` shows docked `pending-trigger-modes-aim` with multi-select mode rows while `priority-context-bar` carries Choose and Cancel (center `pending-choice` is unused for this kind).
- `pay_any_amount_of_mana` (join forces) shows centered `pending-join-forces-modal` with Min/−/value/+/Max stepper and Pay submit.
- `may_draw_up_to` shows centered `pending-draw-count-modal` with one-click number buttons (`0`…`max`) and a MessageRef-backed title. Trade Secrets uses this same generic draw-count prompt rather than a dedicated wire kind.
- `choose_copy_target` uses centered `pending-card-pick-modal`; when the wire carries `put_counter_on_creature = true` (the reused Zimone's Hypothesis primer) the same modal swaps from copy wording to `Choose a creature to get a +1/+1 counter` with a `Put counter` submit button.
- `opponent_chooses_pile` / `choose_pile_for_hand` show docked `pending-pile-aim` with Pile A / Pile B card labels while `priority-context-bar` carries the choose buttons.
- `choose_target_players` / `choose_splitting_opponent` with seat-tagged items aim at life orbs (`pending-player-aim`); one-click when `max === 1` (or splitting); multi-pick accumulates seats in the player-pick draft with Confirm in `priority-context-bar`. Enter / Space submit when ready. Picked seats paint a solid Priority Gold ring (`pickedPlayers`). Items without seat tags fall back to centered `pending-player-pick-modal` player buttons (Choose/Cancel for multi-pick).
- `scry` / `surveil` use centered `pending-arrange-modal` with two-lane arrange chrome (`prompt-arrange-lanes`): cards start in Bottom (library bottom or Graveyard for Surveil); click toggles a card between Top and Bottom, preserving left-to-right order in each lane. Done always submits `arrange_top` via partition draft `{ top, bottom }`. After Done, `promptSubmitInFlight` freezes that draft (no re-init into Bottom) until `pending_choice` changes, a newer board `seq` arrives for an equivalent-looking re-raised choice, or the intent is rejected.
- `select_from_top` uses centered `pending-select-top-modal` with Take vs Bottom lanes (`prompt-select-top-lanes`); click toggles into Take (capped at `up_to`); Done submits `select_from_top` with the Take ids.
- `distribute_top` uses centered `pending-distribute-modal` with Revealed / Hand / Bottom / Exile lanes (`prompt-distribute-lanes`); click cycles a card through lanes with room (`nextDistributeBucket`), then back to Revealed; Distribute enables when each lane hits its exact count.
- `partition_revealed` uses centered `pending-partition-modal` with Pile A / Pile B lanes (`prompt-partition-lanes`); click toggles a card between piles via `PromptCardToggled`.
- `order_triggers` uses centered `pending-order-modal`; rows support HTML5 drag reorder (`Draggable` / `OnDrop` → `PromptOrderRowClicked`, `OnDragEnd` → `PromptOrderDragEnded`), click-to-place (`orderPickPos`), and ↑↓ (`PromptOrderMoved`); list lives under `prompt-order-list`. Submit still emits `choose_order`.
- Enter or Space submits a ready lane / order draft (`order_triggers`, `scry`, `surveil`, `select_from_top`, `distribute_top`, `partition_revealed`) the same way the Done / Confirm button does (`trySubmitReadyPendingDraft`).
- `choose_dredge` requires exactly one selected dredger to enable Dredge; `prompt-decline` (“Draw normally”) submits `dredger: null` via `declineAnswer`.
- Optional-pay prompts (`pay_cost`, `pay_or_counter`, `pay_or_controller_draws`, `pay_echo_or_sacrifice`, `pay_recover_or_exile`, `sacrifice_unless_pay`, `pay_life_or_enters_tapped`) use docked `pending-pay-cost-aim` as a title/count coach while `priority-context-bar` carries the affirm/decline buttons. The affirm button reads `Pay ${costText(cost)}` and the decline reads the outcome-specific label: Don’t pay / Let it be countered / Let them draw / Sacrifice / Exile. `pay_cost` also carries `can_pay` (the engine’s own `Game::can_pay_cost`, the planner `settle_payment` runs); when it is false the Pay button is disabled rather than offering a payment the engine would reject. The “unless you pay” variants carry no flag — declining is a real answer there either way. `pay_life_or_enters_tapped` (shockland, CR 614.12) carries a life amount instead of a cost and no server label: it titles the prompt “Have it enter untapped?”, affirms with `Pay ${life} life`, and declines with “Enters tapped”.
- When `pay_cost` carries `discard_count > 0` (Conspiracy Theorist’s “pay {1} and discard a card”), the same docked aim shows `pending-pay-discard-count` and hand tiles from `discard_choices` highlight for select-then-Pay (Llanowar selected chrome). The bar Pay button stays disabled until exactly `discard_count` cards are picked and `can_pay` is true; the emitted `pay_optional_cost` intent includes `discard_cost`. Decline still emits `pay: false` with no discard cards.
- `pay_any_amount_of_mana` (join forces) uses centered `pending-join-forces-modal` with a clamped stepper over `[0, max]` and draft on `promptDraft` (`PromptNumberSet`); Confirm submits via `PromptSubmitted`. Per-N buttons (`prompt-number-N`) are not used for this kind. `may_draw_up_to` / `trade_secrets_caster_draw` use centered `pending-draw-count-modal` one-click number buttons.

## Implementation Decisions

- Formulators collect `AnswerInput`; they do not build wire intents directly.
- `initPromptDraft`, `buildAnswerFromDraft`, and readiness helpers own draft validation.
- Local pre-submit prompts live in `BoardModel` and are not derived from shared `pending_choice`.
- `cardArt(h, opts)` has one DOM API and supports optional `style`.
- Card-pick faces and the mulligan opening hand render through the shared `promptCardFace` (`board/html/prompt-card-face.ts`): the Magic-aspect face (art, or a name plate when no print resolves) in `sm` (120px), `md` (150px), and `fluid` (vw-capped) sizes — the `aspect-[150/209]` geometry lives in exactly one place.
- Pure X helpers live in `client/app/domain/xCost.ts` (`clampX`, `costWithChosenX`, `costText`).
- Choose-X preview uses brace text rather than hand-bar mana-font pips so large resolved generics cannot collapse to a false `{0}`.
- Waiting copy lives in `client/app/domain/choiceWaiting.ts`; the banner is composed in `boardOverlays` (not inside `promptsView`) so spectators see it without seated prompt chrome, and carries `role="status"` so another seat's pending choice announces politely.
- Library-search filter helpers live in `client/app/domain/cardPickSearch.ts`; filter draft is optional `filter` on `PromptDraft` `card-pick`, updated via `PromptCardFilterSet`.
- Prompt text from the engine stays as `MessageRef` until the view edge; formulators use formatted text for titles but submit only structured answers.
- `promptPresentation(board, state)` is the single presentation classifier for prompt chrome; it returns `none`, `simple`, or `modal`, tags whether a `simple` prompt is board-aim, and defaults uncategorized engine kinds to `modal`.
- `promptsView` renders content only: bottom-docked coach strips for `simple` flows and centered `promptModalFrame` shells for `modal` flows, including the dimmed backdrop and in-modal action rows.
- `promptModalFrame` stays hand-rolled rather than moving to the shared `Dialog` frame (`modalDialog`). `Dialog` bundles Escape and backdrop-click close into the frame with no way to drop either, and a pending choice that can be dismissed leaves the player with no way to answer it — the game waits on an answer the UI has thrown away. The same reasoning keeps the mulligan overlay hand-rolled. The dismissible board modals (result, concede) do use the shared frame; see [ui-component-layer](2026-07-28-ui-component-layer.md) and [system-overlays](2026-07-20-system-overlays.md).
- Interactive pick chrome prefers `data-selected` (and named `group/…` where hover-linked) over JS class ternaries — card-pick faces, order rows, trigger-mode / player-pick buttons, and pile overlay thumbs follow the AGENTS.md Tailwind data-attr pattern. Prompt buttons key their looks off attributes too: disabled item/submit buttons use native `disabled:` / `group-disabled:` variants (with `not-disabled` gating hover motion), and primary-vs-quiet choice buttons carry `data-primary` on the button with `group-data-[primary=…]` variants on the label span.
- Bottom-docked coach strips anchor above the hand bar via the `--b` CSS variable (`HAND_BAR_H + 12px`) read by a `bottom-(--b)` utility — the log-panel pattern — never a raw `bottom` style. Mana pips across prompts, the hand cost strip, and the activation menu render through the shared `pipChip` (`board/html/pip-chip.ts`), which sizes via `--sz` / `--fsz` / `--plate` variables and owns the one-off near-black pip ink.

## Testing Decisions

- Formulator registry tests ensure every `PendingChoiceView["kind"]` maps to a formulator.
- `promptPresentation.test.ts` covers `none` / `simple` / `modal`, local-before-pending precedence, board-aim simple prompts, staged on-board targeting staying `none`, and the modal fallback for uncategorized/off-board pending prompts.
- `client/app/domain/wire/wire-case-coverage.test.ts` asserts hand `FORMULATOR_FOR_KIND` keys match the
  generated `PendingChoiceView` proto oneof (camel→snake), so codegen drift fails `just client-check`.
- Scene tests cover awaited-player prompt visibility and non-decider/spectator suppression plus waiting-banner copy.
- Scene/unit tests for MessageRef-backed prompts assert formatted English for labels while catalog coverage guards every Rust-emitted key.
- Unit tests cover `pendingChoiceWaitingText` (null for decider / absent / mulligan; named seat and `P{seat}` fallback).
- X prompt Scene tests assert centered `x-prompt-modal`, stepper controls, preview text (e.g. `Pay {4}`), confirm, disabled `+` at max, and absence of per-X buttons (`x-prompt-n`).
- Scene tests cover docked `play-mode-aim` with `priority-context-bar` `play-mode-{i}` rows + Cancel, stale legality pruning, stale `PlayModeChosen` without intent, and `PlayModeChosen` clearing `playModePick` before continuing the selected action.
- Scene tests cover docked `modal-mode-aim` / `modal-waiting-aim` (no center `modal-mode-picker` / `modal-waiting`) with pre-choice mode rows, waiting-target Cancel, and Cast/Cancel in `priority-context-bar`, including `aria-pressed` on selected multi-mode rows.
- Scene tests cover centered `target-pick-modal` for off-board staged targets (no `target-pick-aim`).
- Scene tests cover centered `sacrifice-pick-modal` / `discard-pick-modal` for off-board cost fallbacks and the shared-pile `gy-exile-cost-aim` graveyard coach path.
- Scene tests cover engine `pending-discard-aim` and local `discard-cost-aim` select-then-Confirm (`pending-discard-count` / `discard-cost-count`, disabled Discard until ready, Llanowar selected chrome on hand faces).
- Scene/unit tests cover hand put / face-down select → Confirm (`put_land_from_hand`, `put_creature_from_hand`, `put_from_hand_on_top`, `cast_creature_face_down`: `pending-hand-count`, submit in `priority-context-bar`, hand click toggles draft without submitting, Llanowar selected chrome on picked faces / no Island blue on non-choices).
- Unit tests cover `clampX`, `costWithChosenX` (multi-symbol X and colored pips), and `costText` for large generics.
- Unit tests cover `damageAssignReady` for exact-sum non-trample and under-assign / over-assign / negative trample cases.
- Unit tests cover `clickDamageAssign` redistribution and trample under-assign.
- Scene tests cover trample overflow copy, damage steppers (no number inputs), docked `pending-damage-aim` (no center `pending-choice`), on-board click coach, and Assign enabled when under-assigned.
- Board pointer tests cover clicking a blocker during `assign_combat_damage` moves 1 damage onto it.
- Board pointer tests cover clicking a battlefield `divide_spell_damage` target moves 1 damage onto it; Space/Enter submit when the total matches.
- Scene tests cover docked `pending-divide-aim` coach (no center `pending-choice`).
- Board pointer tests cover clicking a battlefield `divide_counters` target moves 1 counter onto it; Space/Enter submit when ready (`pending-divide-counters-aim`, no center `pending-choice`).
- Scene tests cover Space/Enter submitting ready scry / order_triggers / distribute_top drafts (and refusing incomplete distribute_top).
- Scene/unit tests cover dredge decline (`Draw normally` → `dredger: null`) and single-pick readiness for Dredge.
- Scene tests cover pay-cost coach copy plus `priority-context-bar` button copy (`Pay {…}` and kind-specific declines).
- Scene/unit tests cover `pay_cost` with `discard_count` (count chrome, Pay disabled until picks, `discard_cost` on pay, omit on decline) and `may_yes_no` simple-bar behavior: MessageRef labels still surface on `pending-yes-no-aim`, bar Yes / No actions keep `prompt-yes` / `prompt-no`, and the coach stays button-free.
- Scene tests cover centered `pending-card-pick-modal` `choose_copy_target` wording for both the normal copy case and the reused counter-primer case.
- Scene tests cover centered `pending-color-modal` for `choose_color` / `choose_mana_color` (mana pips; no `pending-choice`).
- Scene tests cover docked `pending-mode-aim` for `choose_mode` with `priority-context-bar` mode buttons (no center `pending-choice`).
- Scene/unit tests cover centered join-forces `pending-join-forces-modal` mana stepper (no per-N buttons; draft submit; no center `pending-choice`).
- Scene/unit tests cover centered draw-count `pending-draw-count-modal` (number buttons; no center `pending-choice`; `choose_draw_count` intent).
- Scene tests cover docked `pending-trigger-modes-aim` with coach toggle rows and `priority-context-bar` Choose/Cancel (no center `pending-choice`).
- Scene/unit tests cover docked `pending-pile-aim` with coach card labels, `priority-context-bar` Pile A/B buttons, and `choose_opponent_pile` intent.
- Scene tests cover centered `pending-player-pick-modal` for untagged `choose_target_players` / `choose_splitting_opponent` lists (no center `pending-choice`).
- Scene/unit tests cover centered library-search modal (`pending-library-modal`), filter, face dedupe, scroll chrome, Choose, and Fail to find.
- Scene tests cover off-board card-pick grids as centered `pending-card-pick-modal` (no center `pending-choice`) and mixed `choose_target` player buttons as `pending-player-pick-modal`.
- Scene/unit tests cover centered `pending-creature-type-modal` (filter strip; no `pending-choice`; `choose_creature_type` intent).
- Scene/unit tests cover centered `pending-card-name-modal` (placeholder, Name submit, suggestions; no `pending-choice`).
- Scene tests cover the `choose_card_name` typeahead list when suggestions match the draft query, seeding an open `CardNameCombobox` and resolving its anchor / backdrop-portal Mounts via `resolveCardNameComboboxMounts`.
- Unit tests cover typing into the typeahead (input value and string draft both track the keystroke; `SearchCardNames` fires) and picking a suggestion (draft names that card; the list closes).
- Scene tests cover on-board pending aim chrome (`pending-target-aim`, no card grid), bar-local optional Decline → empty `choose_targets`, and bar-local Confirm / Assign / Cancel for the simple board-aim coach variants.
- Scene/pointer tests cover GY pile aim for `choose_target` when every legal item shares one graveyard (`pending-gy-aim`).
- Scene tests cover `opponent_chooses_revealed_to_graveyard` docked aim (one-click face → `choose_exiled_with_card`, Choose none declines).
- Scene/unit tests cover optional vs mandatory `may_return_from_graveyard` (`prompt-decline` present only when optional, `Return` disabled until a card is picked, `mandatory` decoded from wire).
- Scene/unit tests cover `may_exile_discarded_to_play` proto decode, formulator registration, `pending-gy-aim` chrome (`Exile` / `Don't exile`), decline → empty `choose_sacrifices`, and submit → chosen discarded card.
- Scene tests cover `revealed_card_to_battlefield_or_hand` docked destination aim with the face in coach and Battlefield / Hand intents in `priority-context-bar`.
- Scene/pointer tests cover on-board `sacrifice_edict` one-click and `proliferate` accumulate → Confirm (including seat + permanent mixed picks that keep both lists, Priority Gold paint for `card-pick.players`, and CR 701.27b "proliferate twice" unfreezing Confirm when the second iteration arrives with the same draft key).
- Scene/unit tests cover centered scry/surveil `pending-arrange-modal` Top↔Bottom (Graveyard) lanes and click toggle → `arrange_top`.
- Scene tests cover centered `pending-select-top-modal` Take/Bottom lanes (no center `pending-choice`).
- Scene/unit tests cover centered `pending-distribute-modal` Hand/Bottom/Exile lanes and `nextDistributeBucket` cycling.
- Scene/unit tests cover centered `pending-partition-modal` Pile A / Pile B lanes and click → pile_a.
- Scene/unit tests cover order_triggers drag rows, click-to-place reorder, drag-end cancel, and ↑↓ chrome.
- CardArt tests cover skeleton-to-image and shared cache readiness.

## Out of Scope

- Introducing brand-new pending-choice oneof arms when an expand-only field on an existing shape is enough.
- Client-side inference of unavailable pending-choice kinds.
- Sparse illegal-X denylists and oracle “enters as N/N” hints.
- Engine `pending_choice` X formulators (none today); choose-X here is the board-local `xPrompt` path only.

## Further Notes

- Wire projection may still send redacted `pending_choice` data to non-deciders; the interactive formulator gate is client-side.
- The local hand play-mode prompt follows [hand-play-mode-chooser-design](2026-07-26-hand-play-mode-chooser-design.md).
- Prompt presentation ownership and primary-bar takeover details are designed in [prompt-primary-bar-takeover-design](2026-07-27-prompt-primary-bar-takeover-design.md).
