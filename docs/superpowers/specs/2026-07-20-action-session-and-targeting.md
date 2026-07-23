# Action Session and Targeting
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/action/session.ts`, `client/app/board/action/execution.ts`, `client/app/board/action/targeting.ts`, `client/app/board/geometry/combat-staging.ts`, `client/app/board/submodel.ts`

## Problem Statement

Playing a spell or activating an ability may require local choices before submission: X, modes, discard/delve/sacrifice costs, target selection, or combat declarations. The client must stage these choices without becoming a second rules engine.

## Solution

Keep an action session in the board model. Pure planners decide whether an action needs a prompt, target arrow, modal picker, or immediate submit. The engine remains authoritative for payment and legality; the client submits the existing action id and selected local inputs.

## User Stories

- As a player, I can drag or click a playable card and complete required local choices.
- As a player casting a targeted spell, I aim an arrow at legal targets on the board.
- As an active combat player, I can stage attackers or blockers before confirming.
- As a player, I can cancel local staging without answering or corrupting an engine pending choice.
- As a player aiming or paying for a cast/activate, I keep seeing which lands will auto-tap until I submit or cancel.

## Behavior

- `planCostPipeline` sequences sacrifice, discard, graveyard-exile, modal, X, target, and run steps.
- `planRunAction` stages targeted actions, plays lands, casts spells, or submits simple actions.
- Targeting uses engine-projected legal targets. Board targets become arrow aiming; off-board targets become picker prompts.
- Arrow aiming highlights legal objects and players and submits only after a legal target click.
- Combat staging resolves attack drops onto opponent life-orb targets and block drops onto declared attackers.
- Required attacks are merged with staged attacks before confirmation.
- `CancelActionClicked` and Escape call `cancelAll`, clearing staged action, X prompt, modal cast, cost picks, radial, stack expand, pile expand, prompt draft, hand drag, and reject text.
- `session.cancel` means local pre-submit cancellation only; engine `pending_choice` is handled by `PromptHost`.
- Payment is engine-side. The client previews `auto_tap`, but it does not tap lands or solve mana costs before submit.
- Auto-tap preview prefers the in-flight session action (`staged`, choose-X, modal, sacrifice/discard/gy-exile pick) over `hoverActionId`, so payment glyphs stay visible after hand/radial hover clears on activate.

## Implementation Decisions

- Planners are pure TypeScript functions, while `updateBoard` turns plans into model changes and `SubmitIntent` commands.
- `buildTakeActionIntent` is the single take-action intent builder for cast, activate, cycle, and related action ids.
- `stagedPickTargets` keeps graveyard/exile/stack targets in DOM pickers when they are not reliable canvas click targets.
- Combat staging functions return new `WireAttack[]` / `WireBlock[]` values without mutating state.

## Testing Decisions

- Unit tests cover `paymentPreviewAction` preferring staged/X session actions over hover.
- Action execution tests cover cost pipeline ordering, X prompt creation, target staging, and submit intent shape.
- Targeting tests cover arrow versus picker target modes.
- Combat staging tests cover attacker/blocker drops, required attack merge, and step-transition clearing.
- Board tests cover cancel behavior and keyboard Escape ordering.

## Out of Scope

- Client-side payment solving.
- Client-derived target legality.
- Changing engine pending-choice semantics.

## Further Notes

- Stack, prompts, and radial specs document the UI surfaces that the action session opens.
