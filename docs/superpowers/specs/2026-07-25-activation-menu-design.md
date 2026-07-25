# Activation menu (replace radial)

**Status:** superseded by the living [activation-menu](2026-07-25-activation-menu.md) feature spec
**Module:** `client/app/board/html/activation-radial.ts` (rename to activation-menu), `client/app/board/geometry/radial.ts` (options + placement + press; retire wedge geometry), `client/app/board/submodel.ts`, board Scene tests

## Goal

Replace the battlefield activation **SVG donut radial** with a single **card-anchored list menu** that is visually polished (Arena-grade HUD) and dense enough to show readable ability labels plus a cost chip when a displayable cost exists.

One menu surface only — no hybrid radial for sparse cases.

## Non-goals

- Proto/schema fields for activation mana cost beyond what `ActionView` / local option data already expose
- Parsing oracle text or labels to invent costs
- Canvas-painted menu
- Touch-specific long-press tuning
- A shared generic board popover framework for unrelated pickers
- Keeping wedge geometry once nothing imports it

## Behavior

### Open / close

1. Selecting a battlefield permanent with at least one option opens the activation menu.
2. Empty option lists render nothing (no hollow panel); selection clears as today when there is nothing to show.
3. A full-screen dismiss scrim sits under the menu; pointer-up on the scrim dismisses.

### Options

- Options remain `tap_for_mana` (when applicable) plus battlefield `ActionView` entries for the selected object.
- Disabled rules unchanged: can’t act, already tapped, summoning sickness without haste block tap-for-mana; disabled rows stay visible and muted.
- Stable option identity via `radialOptionKey` (or renamed equivalent).

### Placement

- Anchor is the selected card’s screen-space center (same camera/world→screen path as today’s radial center).
- Prefer **right** of the card; if that overflows the viewport, try **left**, then **above**, then **below**.
- Always clamp so the panel stays fully on-screen.
- Placement uses percentage-of-viewport overlay math consistent with board CSS stretch (same constraint as `radialOverlayPlacement`).

### Rows

- Each row shows the full ability **label** at a readable width (wrap or ellipsis — not an 18-character hard truncate as the sole strategy).
- When a **displayable cost** is available from existing action/option data, show a **cost chip** on the row (e.g. tap-for-mana can show `{T}`). Omit the chip when no cost is available.
- Rows are focusable (`role="button"`) with `aria-label` from the option label; `aria-disabled` when disabled.

### Pointer (arm / commit)

- Pointer-down on a row arms that index.
- Pointer-up on the **same** row commits; pointer-up on a **different** row cancels that press; pointer-up on the scrim dismisses.
- Disabled rows never commit.
- No press `scale`/`translate` that shrinks the hit target under the cursor.

### Commit

- Commit clears selection before submitting tap-for-mana or running the action (same as today’s `commitRadialIndex` contract).
- Payment stays engine-side (`settle_payment` / `auto_tap` preview only); the client must not pre-tap lands before submit.
- Legal listed activates with payable costs must commit without a spurious `CannotActivate` / “That ability isn't available” toast; true illegals stay disabled.

### Hover / preview

- Hovering an action row updates `hoverActionId` for auto-tap preview when no local session action is open.
- Staged / X / modal / cost-pick sessions keep preview from their action instead (see action-session-and-targeting).

### Keyboard

- Focus a row and press Enter or Space to pick (same as wedge keyboard path).

## Visual

- Panel uses Forest HUD / Vine border language from `DESIGN.md` — translucent HUD panel, not a nested marketing card.
- Compact vertical list; max height with scroll when there are many abilities (no second “more…” menu).
- Motion: 150–250ms ease-out for open/hover; honor `prefers-reduced-motion`.
- Active/hover row uses existing Llanowar / priority-gold accents already used on board chrome; disabled rows stay muted.

## Module shape

| Concern | Home |
|---------|------|
| Option list + press reducer + placement helpers | `client/app/board/geometry/` (evolve `radial.ts` or split placement; delete unused wedge path/index helpers) |
| DOM menu view | `client/app/board/html/activation-menu.ts` (replace `activation-radial.ts`) |
| Armed / hover indices | `BoardModel` (keep semantics; rename only if call sites stay clear) |
| Overlay composition | `client/app/board/html/overlays.ts` |

## Testing

- Pure placement: prefer-right, flip left/above/below, clamp, viewport stretch.
- Press reducer: same-row commit, slide-off cancel, disabled no-op, scrim dismiss.
- Scene: `data-testid="activation-menu"` and `activation-menu-row-*`; label + optional cost chip; keyboard pick.
- Interaction outcomes: commit clears selection and fires tap-for-mana / action — assert product behavior, not “parity with radial.”

## Spec touch-up

In the same implementation change:

- Replace or rewrite the living surface spec as [`docs/superpowers/specs/2026-07-25-activation-menu.md`](2026-07-25-activation-menu.md).
- Update the specs README index row for activation accordingly.
- Retire this design doc’s “approved design” status by folding truth into the surface spec (or mark superseded once the surface spec is current).
