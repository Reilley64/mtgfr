# Action Session and Targeting
**Status:** Current (as of 2026-07-27)
**Module:** `client/app/board/action/session.ts`, `client/app/board/action/execution.ts`, `client/app/board/action/targeting.ts`, `client/app/board/geometry/combat-staging.ts`, `client/app/board/submodel.ts`

## Problem Statement

Playing a spell or activating an ability may require local choices before submission: X, modes, discard/delve/sacrifice costs, target selection, or combat declarations. The client must stage these choices without becoming a second rules engine.

## Solution

Keep an action session in the board model. Pure planners decide whether an action needs a prompt, target arrow, modal picker, or immediate submit. The engine remains authoritative for payment and legality; the client submits the existing action id and selected local inputs.

## User Stories

- As a player, I can drag or click a playable card and complete required local choices.
- As a player casting a targeted spell, I aim an arrow at legal targets on the board.
- As a player answering a one-click on-board engine target choice, I aim the same arrow instead of a card grid.
- As a player targeting a spell or ability on the stack, I click the highlighted stack face instead of a modal picker.
- As an active combat player, I can stage attackers or blockers before confirming.
- As a player with a stack of identical creatures, I hold Shift on a combat drop to send or block with the whole pile in one drag.
- As a player, I can cancel local staging without answering or corrupting an engine pending choice.
- As a player aiming or paying for a cast/activate, I keep seeing which lands will auto-tap until I submit or cancel.

## Behavior

- `planCostPipeline` sequences sacrifice, discard, graveyard-exile, modal, X, target, and run steps.
- `planRunAction` stages targeted actions, plays lands, casts spells, or submits simple actions.
- Targeting uses engine-projected legal targets. Battlefield, stack, and player targets become arrow aiming; graveyard/exile (and other off-board) targets become picker prompts.
- Arrow aiming highlights legal objects and players and submits only after a legal target click.
- Stack faces that are legal targets show an Island Blue ring (`legal-target`) and click via `TargetChosen` to complete staged or pending aim.
- Engine `choose_target` with `max === 1`, and `choose_spell_targets` / `choose_ability_targets` with `min === max === 1`, use the same on-board aim when every legal item is a battlefield permanent, stack object, or player (`pendingBoardTargetMode` / `pendingTargetingOverlay`). Multi-target or any off-board item stays on the modal card picker. Staged local cast aim wins over pending aim for the overlay.
- While a pending board-aim choice aims from the stack and the source is not a stack entry (ability placement `choose_target`, or mid-resolution onboard card-picks like `proliferate` after `AbilityResolved`), the stack overlay shows a source-art ghost (`pendingStackGhost`) matching local staged casts. Resolving spells that remain on the stack are not duplicated.
- Battlefield card-picks (`sacrifice_edict`, `choose_own_sacrifices`, `may_sacrifice`, `devour`, `proliferate`, `phase_out`, `decline_untap`, `choose_attach_host`, `sacrifice_unless_return_land`, `choose_copy_target`, `choose_counter_target_for_player`, `caster_keep_permanents`, `choose_activation_cost_targets`) use the same on-board aim when every item is on the canvas; one-click when the required count is 1, otherwise accumulate until Confirm / Enter / Space.
- `choose_target_players` / `choose_splitting_opponent` highlight life-orb avatars (`pendingPlayerAimOverlay`); one-click when `max === 1` (or splitting); multi-pick accumulates seats until Confirm / Enter / Space. Picked seats paint Priority Gold (`pickedPlayers`). Proliferate stores seats on `card-pick.players` and paints them through the same `pickedPlayersFromDraft` path; toggling a permanent does not clear those seats.
- On-board `divide_spell_damage` highlights battlefield targets (`pendingDivideSpellOverlay`) and redistributes 1 damage per click; player/off-board targets stay modal-only.
- On-board `divide_counters` reuses `pendingDamageAssignOverlay` / `clickDamageAssign` for battlefield permanents.
- On-board pending aim clicks pack `answerFromBoardTarget` → `choiceIntent` → `SubmitIntent`. Optional `choose_target` keeps a Decline control on `pending-target-aim` chrome.
- Multi-target on-board aim (`max > 1` or spell/ability min/max ranges) accumulates picks in the card-pick draft with `k / max` + Confirm chrome (`pending-target-count`); one-click still auto-submits when `pendingTargetOneClick`. Picked permanents paint a solid Priority Gold ring (`pickedObjects`) while other legal targets keep the dashed Island Blue aim ring. Enter or Space submits when the multi-aim draft is ready. Stack faces use the same accumulate path via `TargetChosen` (not a premature one-target submit).
- Combat staging resolves attack drops onto opponent life-orb targets and block drops onto declared attackers.
- Holding Shift during a combat drop commits every copy in the dragged cluster tile: `handleCombatDrop` takes the face's clustermates as `alsoIds` and applies the same drop to each, so a shift-drop of five identical creatures declares five attackers against that defender, or five blocks of that attacker. Cluster members are identical by construction, so an illegal face rejects the whole pile rather than staging part of it, and a shift-drop of a non-clustered card is the ordinary single declaration.
- Required attacks are merged with staged attacks before confirmation.
- `CancelActionClicked` and Escape call `cancelAll`, clearing staged action, X prompt, modal cast, cost picks, the activation menu, stack expand, pile expand, prompt draft, hand drag, and reject text.
- `session.cancel` means local pre-submit cancellation only; engine `pending_choice` is handled by `PromptHost`.
- Payment is engine-side. The client previews `auto_tap`, but it does not tap lands or solve mana costs before submit.
- Auto-tap preview prefers the in-flight session action (`staged`, choose-X, modal, sacrifice/discard/gy-exile pick) over `hoverActionId`, so payment glyphs stay visible after hand or activation-menu hover clears on activate.
- Local pre-submit sacrifice costs (`sacrificePick`) highlight battlefield `sacrifice_choices` (`sacrificeCostOverlay`); a click settles the cost (`SacrificeChosen` path). Chrome shows `sacrifice-cost-aim` instead of the modal grid when every choice is on the battlefield.
- Local pre-submit discard costs (`discardPick`) with choices in hand aim at hand tiles: click toggles selection (does not settle on click); Confirm via `DiscardCostConfirmed`. Chrome shows `discard-cost-aim` / `discard-cost-count` when every choice is in the viewer's hand.
- Local pre-submit graveyard-exile costs (`gyExilePick`) open the shared GY pile when every choice is in one graveyard (`gyExileCostPile` / `gy-exile-cost-aim`); pile cards with Island Blue rings emit `PileCardClicked` → `GyExileChosen`. One-click when `max <= 1`; exact `min === max` accumulates then auto-settles; `min < max` accumulates until Exile / Enter / Space (`GyExileConfirmed`). Mixed owners keep the modal `gy-exile-pick` grid.
- Engine `exile_from_graveyard` / `may_return_from_graveyard` / `shuffle_from_graveyard` / `choose_dredge` / `pay_cumulative_upkeep_or_sacrifice` / `choose_activation_cost_targets` with every item in one GY use the same pile aim (`pendingGraveyardPickIds` / `pending-gy-aim`); one-click when required count is 1 (or dredge / shuffle max 1); others accumulate until Confirm / Enter / Space. Decline stays for dredge and cumulative upkeep.
- Engine exile card-picks (`choose_exiled_with_card`, `choose_exiled_with_card_to_cast`, `choose_exiled_dig_to_cast_free`, `opponent_chooses_exiled_nonland`, `choose_exiled_to_cast_free`) with every item in one exile pile use `pendingExilePickIds` / `pending-exile-aim`; one-click except multi `choose_exiled_to_cast_free`.
- `choose_exiled_dig_to_cast_free` with non-empty wire `cast_targets` (Aura dig hosts) is two-step: exile pile click stages the card-pick draft (no submit), then on-board aim at `cast_targets` (`pendingDigCastHostMode` / `pending-target-aim` “Choose what to enchant”); host click submits `choice` + `target: { kind: "object", id }`. Empty `cast_targets` keeps one-step exile submit with `target: null`. Decline still sends `choice: null, target: null`.
- Engine `discard` / `may_discard` pending choices with every item in hand use the same hand-bar aim (`pendingHandPickIds` / `pending-discard-aim`); click toggles selection; Confirm / Continue submits when ready, including when count is 1 (`pendingHandPickOneClick` is false for these kinds).
- Engine `put_land_from_hand` / `put_creature_from_hand` / `put_from_hand_on_top` / `cast_creature_face_down` with every item in hand use hand-bar aim (`pendingHandPickIds` / `pending-hand-aim`); one-click for put-land/put-creature/face-down and `put_from_hand_on_top` when count is 1; multi `put_from_hand_on_top` accumulates until Put on top / Enter / Space. Optional put-land/put-creature keep Decline on the coach chrome.

## Implementation Decisions

- Planners are pure TypeScript functions, while `updateBoard` turns plans into model changes and `SubmitIntent` commands.
- `buildTakeActionIntent` is the single take-action intent builder for cast, activate, cycle, and related action ids.
- `stagedPickTargets` keeps graveyard/exile targets in DOM pickers when they are not reliable canvas click targets; stack objects are arrow-aimed via the stack overlay.
- Combat staging functions return new `WireAttack[]` / `WireBlock[]` values without mutating state.

## Testing Decisions

- Unit tests cover `paymentPreviewAction` preferring staged/X session actions over hover.
- Action execution tests cover cost pipeline ordering, X prompt creation, target staging, and submit intent shape.
- Targeting tests cover arrow versus picker target modes, pending on-board aim versus modal idle, `pendingStackGhost` for mid-resolution proliferate / phase-out (and no duplicate when the spell remains on the stack), and `pickedPlayersFromDraft` for proliferate `card-pick.players`.
- Board pointer tests cover pending on-board choose_target click → `choose_targets` intent and proliferate seat+permanent accumulate → Confirm.
- Combat staging tests cover attacker/blocker drops, required attack merge, step-transition clearing, and shift-dropping a cluster (every member staged against one defender or attacker; an illegal face stages nothing).
- Cluster dispense tests drive `ShiftDown` / `ShiftUp` through `updateBoard` so a shift-held drop stages the whole pile and a released Shift goes back to one attacker per drop.
- Board tests cover cancel behavior and keyboard Escape ordering.

## Out of Scope

- Client-side payment solving.
- Client-derived target legality.
- Changing engine pending-choice semantics.

## Further Notes

- Stack, prompts, and activation-menu specs document the UI surfaces that the action session opens.
