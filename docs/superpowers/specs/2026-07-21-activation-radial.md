# Activation Radial
**Status:** Current (as of 2026-07-23)
**Module:** `client/app/board/html/activation-radial.ts`, `client/app/board/geometry/radial.ts`, `client/app/board/submodel.ts`

## Problem Statement

Selecting a battlefield permanent must expose its legal activations and tap-for-mana affordance with reliable pointer behavior. The interaction cannot depend on fragile synthesized clicks.

## Solution

Render a continuous SVG donut radial around the selected battlefield permanent. Wedges arm on pointer-down and commit only when pointer-up lands on the same wedge. The geometry is pure and lives in `client/app/board/geometry/radial.ts`; the DOM/SVG view lives in `client/app/board/html/activation-radial.ts`.

## User Stories

- As a player, I can click my permanent and pick an activation from a large wedge.
- As a player, I can tap a mana source from the same radial.
- As a player, sliding off a wedge before release cancels that press instead of misfiring.
- As a keyboard user, I can focus a wedge and press Enter or Space.

## Behavior

- The radial opens only for a selected battlefield permanent with at least one option.
- Options are `tap_for_mana` plus battlefield `ActionView` entries for the selected object.
- Empty option lists render nothing; no hollow ring appears.
- The ring is centered on the selected card’s screen-space center.
- Inner and outer radii scale with camera zoom and card size.
- A single option renders as a full donut ring.
- Pointer-down on a wedge stores `radialPress.armed`.
- Pointer-up on the same wedge commits; pointer-up on a different wedge cancels that press; pointer-up on the scrim dismisses.
- Disabled wedges remain visible but do not commit.
- Hovering an action wedge updates `hoverActionId` for auto-tap preview.
- Payment is engine-side (`settle_payment` / `auto_tap` preview only); the client must not pre-tap lands before submit.
- Legal listed activates with payable costs must commit without a spurious `CannotActivate` / “That ability isn't available” toast; true illegals stay disabled.

## Implementation Decisions

- `wedgePath`, `wedgeLabelPoint`, `wedgeIndex`, and `radialPressUp` stay pure and testable.
- `radialOptionKey` provides stable option identity.
- The radial is a DOM overlay, not canvas paint, so accessibility roles and keyboard handlers remain available.
- `commitRadialIndex` clears selection before submitting tap-for-mana or running an action.
- `take_action` ids must round-trip (`ActionView.id` → wire → engine lookup); `UnknownAction` means a stale id, distinct from `CannotActivate`.

## Testing Decisions

- Geometry tests cover radii, full-ring path, wedge paths, label points, and wedge index math.
- Pointer tests cover down/up same wedge, slide-off cancel, disabled wedge no-op, scrim dismiss, and keyboard pick.
- Scene tests assert current paths and `data-testid="activation-radial"` / `radial-wedge-*` behavior.

## Out of Scope

- Touch-specific long-press tuning.
- Canvas-painted radial.
- Overflow menus for very large ability lists.

## Further Notes

- Permanent selectability and playable borders are chrome concerns; this spec covers the radial once a permanent is selected.
