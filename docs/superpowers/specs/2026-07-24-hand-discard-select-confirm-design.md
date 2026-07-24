# Hand discard select → confirm

**Status:** approved design  
**Module:** `client/app/board/html/hand.ts`, `client/app/board/html/prompts.ts`, `client/app/board/submodel.ts`, `client/app/board/action/targeting.ts`, `client/app/board/messages.ts` (as needed), prompts behavior spec

## Goal

Hand discard stops committing on the first click. The player selects (and can deselect) legal hand cards, sees a clear selected state, then confirms.

Applies to **all** hand discard flows:

- Local discard cost (`discardPick` / `discard-cost-aim`)
- Engine `discard` / `may_discard` when every legal item is in hand (`pending-discard-aim`), any required count

## Non-goals

- `put_land_from_hand`, `put_creature_from_hand`, `put_from_hand_on_top`, `cast_creature_face_down` stay on their current one-click / accumulate rules
- Off-board `discard-pick-aim` button strip (choices not in hand) stays a one-shot button list
- Battlefield sacrifice cost chrome is unchanged

## Behavior

### Selection

1. Legal discard candidates keep today’s **island-blue** “selectable” ring while the discard aim is live.
2. Click a legal hand card → **toggle** selection:
   - Selected: face **raises** (same raise as hand hover) and paints a **Llanowar green** border/ring (same family as selected card-pick faces).
   - Click again → deselect (raise and green chrome clear).
3. Selection never auto-submits, including when exactly one card is required.

### Confirm / cancel

- Docked aim chrome shows coach copy, optional `N / M selected` when a required or max count is known, **Confirm** (or **Discard** / **Continue** for engine kinds — keep existing submit labels), and Cancel where cancel already exists (local cost).
- Confirm enables only when the selection satisfies readiness (`cardPickReady` for engine; local cost typically exactly one id in `discard_choices`).
- Enter / Space submit when ready, same as other accumulate hand / board aims (`trySubmitReadyPendingDraft` for engine; local cost gets an explicit confirm message).
- Cancel on local `discardPick` clears the pick without settling the cost (existing cancel path).

### Engine discard

- `pendingHandPickOneClick` returns **false** for `discard` and `may_discard` (all counts).
- Hand clicks toggle `promptDraft` `card-pick` ids (existing accumulate path used today for multi-discard).
- `pending-discard-aim` always shows Confirm (or Continue for `may_discard`) plus count line when applicable — never coach-only one-click chrome for these kinds.

### Local discard cost

- Mirror GY-exile accumulate: clicking a legal hand card toggles that id in `discardPick.picks.discard_cost` and does **not** call `continueAfterCostPick`.
- A new confirm path (message + HUD button, e.g. `DiscardCostConfirmed` / reuse a shared confirm if one already fits) settles with `discard_settled: true` when exactly one legal id is selected (or the action’s required discard count if the wire ever exposes more than one — today local discard cost is one card).
- `HandActionActivated` / `DiscardChosen` during `discardPick` must toggle, not settle.

## Visual

| State | Hand chrome |
|-------|-------------|
| Legal, not selected | Island-blue selectable ring (existing) |
| Selected | Raised face + Llanowar green border/ring |
| Hover while selected | Stay raised; no flicker back to rest height |

Selected paint is driven from `discardPick.picks.discard_cost` (local) or `promptDraft.picked` (engine) into `hand.ts` tile args (e.g. `discardSelected`), not from battlefield `pickedObjects`.

## Testing

- Scene: `discard-cost-aim` shows Confirm disabled until a card is selected; after select, Confirm enabled; center modal still absent.
- Scene / board: selected hand tile carries green selected chrome (testid or class assertion as existing hand tests do).
- Unit / scene: click toggles selection; second click clears; Confirm settles local cost once.
- Regression: engine `discard` count 1 no longer one-clicks; requires Confirm; count 2 still needs two picks then Confirm.
- Interaction policy: assert outcomes (settled picks / intent), not presence-only.

## Spec touch-up

Update `docs/superpowers/specs/2026-07-20-prompts-and-pending-choices.md` in the same implementation change so behavior truth matches select → confirm for hand discard.
