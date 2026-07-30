# Activation Menu
**Status:** Current (as of 2026-07-25)
**Module:** `client/app/board/html/activation-menu.ts`, `client/app/board/geometry/radial.ts`, `client/app/board/submodel.ts`

## Problem Statement

Selecting a battlefield permanent must expose its legal activations and tap-for-mana affordance with reliable pointer and keyboard behavior. The surface must stay readable, show activation costs when current action data can supply them, and avoid fragile synthesized clicks.

## Solution

Render a card-anchored activation menu beside the selected battlefield permanent. The menu is a DOM overlay list with a full-screen dismiss scrim, arm-on-press / commit-on-release interaction, optional cost chips, and viewport-clamped placement computed by pure helpers in `client/app/board/geometry/radial.ts`.

## User Stories

- As a player, I can select a permanent and see its available activations in a readable vertical menu.
- As a player, I can tap a mana source from the same menu.
- As a player, sliding off a row before release cancels that press instead of misfiring.
- As a player, I can tell when an activation is unavailable because its row stays visible but disabled.
- As a keyboard user, I can focus a row and press Enter or Space to pick it.

## Behavior

- The activation menu opens only for a selected battlefield permanent with at least one option.
- Options are synthesized from `tap_for_mana` plus battlefield `ActionView` entries for the selected object. When the selection is a cluster face, the entries for every member id (`[face.id, ...clusterMembers]`) are counted together.
- One row per distinct ability label. The row carries the `ActionView` of a copy that can still act, so activation routes to an unspent copy; an ability stays on offer while any copy has an entry and disappears when the last copy's is gone.
- A row carrying more than one copy shows a `data-testid="activation-menu-available-{key}"` chip reading `×k`, where `k` counts the distinct member ids that can still activate it.
- Nothing splits out of the cluster for an activation. Visible consequences (tapped, counters, modifiers) split it through `clusterKey` as usual.
- Empty option lists render nothing; no panel appears.
- The menu anchor is the selected card's screen-space center.
- Placement prefers right of the card, then left, then above, then below, and clamps the panel fully on-screen.
- Placement is emitted in viewport-percentage CSS so the overlay stays aligned with the board's stretched viewport math.
- The wrapper uses `data-testid="activation-menu"`.
- The menu panel uses `data-testid="activation-menu-panel"`, `role="group"`, and `aria-label="Activation options"`.
- Each row uses `data-testid="activation-menu-row-{key}"`, where `key` comes from `radialOptionKey`.
- Rows keep `data-wedge="{index}"` for element-from-point helpers.
- Each row exposes the option label, uses `aria-label` from that label, and sets `aria-disabled="true"` for disabled rows.
- Rows show an optional `data-testid="activation-menu-cost"` chip when current option data exposes a displayable cost. The client does not parse labels to invent costs.
- Tap-for-mana rows show a tap chip. Action rows show a tap chip when `taps_self` is true, mana pips when a mana cost is present, and both when both are present.
- The label column is readable at menu width and clamps to two lines.
- Pointer-down on a row arms that row index.
- Pointer-up on the same row commits. Pointer-up on a different row cancels that press. Pointer-up on the scrim dismisses the menu.
- Disabled rows never commit.
- Hovering an action row updates `hoverActionId` for auto-tap preview when no local session action is open.
- Staged, choose-X, modal, and local cost-pick sessions keep payment preview sourced from the in-flight session action instead of menu hover.
- Pressing Enter or Space on a focused row commits that row.
- Commit clears selection before submitting tap-for-mana or the chosen action.
- Payment remains engine-side. The client previews `auto_tap`, but it does not pre-tap lands before submit.
- Legal listed activations with payable costs must commit cleanly; truly illegal options stay disabled.
- The menu panel uses Forest HUD / Vine board chrome, keeps active and hover emphasis on the row, and scrolls when the option list exceeds the capped panel height. Row chrome is attribute-driven: rows carry `data-active` (hover/armed) and `aria-disabled`, and Tailwind `data-[active=true]:…` / `aria-disabled:…` variants own the gold emphasis and dimmed looks.

## Implementation Decisions

- Keep option building, stable option identity, press-state reduction, card screen-center math, placement, and cost-chip derivation in `client/app/board/geometry/radial.ts`.
- Keep board message names `RadialWedgeArmed`, `RadialWedgeReleased`, `RadialWedgeHovered`, and `RadialOptionPicked`; the interaction channel stays index-based for menu rows.
- Keep the activation surface in DOM, not canvas, so focus, keyboard handlers, and accessibility roles remain available.
- `selectedRadialOptions` remains the board-side selector for the chosen permanent's menu rows, and resolves a selected non-face cluster member back to its face card.
- `commitRadialIndex` still clears selection before submitting tap-for-mana or a battlefield action. It resolves the acting object from `action.object` so a cluster row commits against the copy that owns it; tap-for-mana stays on the selected face.
- The engine is the authority on which copies can act — `state.actions` lists one entry per object that passes the activation gate, so a spent once-per-turn ability simply has no entry and needs no client bookkeeping.

## Testing Decisions

- Geometry tests cover menu placement preference order, viewport clamping, estimated menu height, cost-chip derivation, and the cluster count: one row for three copies labelled `×3`, distinct copies rather than rows counted, the row surviving on the unspent copy's action, and no row when every copy has spent it.
- Board update tests cover arm / release behavior, disabled rows, keyboard commit, and submit outcomes.
- Scene tests cover `activation-menu`, `activation-menu-panel`, `activation-menu-row-*`, `activation-menu-cost`, `activation-menu-available-*`, disabled visibility, and panel placement beside the selected card.

## Out of Scope

- Canvas-painted activation UI.
- Client-side payment solving or speculative pre-tapping.
- Oracle-text parsing to infer activation costs the wire data does not expose.
- Secondary overflow surfaces for large activation lists.

## Further Notes

- Permanent selectability and playable borders are battlefield chrome concerns; this spec covers the activation menu once a permanent is selected.
