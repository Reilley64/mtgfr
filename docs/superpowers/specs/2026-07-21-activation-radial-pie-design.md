# Activation radial pie redesign

**Date:** 2026-07-21  
**Status:** Approved for planning  
**Context:** The activation radial (select permanent → pick tap-for-mana / activate) can feel unresponsive on mouse/trackpad: the option shows press feedback, the menu stays open, and the action does not fire until a second or later try. Observed pattern matches an incomplete browser `click` (mousedown and mouseup not finishing on the same element) — not a dismiss-on-miss and not a server reject. Current UI uses floating rectangular `Button` chips over a full-screen dismiss scrim, with `game-quiet` press scale/translate and option list remount risk under Solid `For` without stable keys.

## Goals

1. **Reliable pick on the first deliberate press** — pointer-down on an option and pointer-up on the same option commits; no dependence on synthesized `click`.
2. **Large angular hit targets** — continuous donut ring (filled sectors sharing edges) so aiming at a label is not required; the whole wedge counts.
3. **Preserve today’s open/dismiss/option set and play path** — click-to-select permanent, Esc / outside dismiss, `radialOptions` contents, hover `auto_tap` preview, then `tap_for_mana` or `session.play`.

## Non-goals (v1)

- Touch-specific gesture tuning (long-press open, etc.) beyond whatever the pointer-up model already gives.
- Changing when the radial opens (still click-to-select; not right-click-only).
- Rewriting payment preview, cost pipeline, or `session.play` / `take_action`.
- True canvas-painted radial (stays DOM overlay per dual-surface invariant).
- Overflow / scroll menus for huge ability lists — truncate labels; equal wedges only.

## Approach

**Continuous ring pie (DOM SVG) + pointer-up commit**, replacing floating chip buttons.

Chosen over “keep `click` and harden chips” because the failure mode is click synthesis itself; chosen over gap-separated arcs so every angle in the ring belongs to a wedge (no dead gaps that cancel by accident).

## Interaction contract

| Step | Behavior |
|------|----------|
| Open | Click your battlefield permanent → selection + radial (unchanged). |
| Options | `radialOptions(objectId, actions, tapsForMana, tapped, canAct)` — tap-for-mana when legal, plus battlefield activates for that object (unchanged). |
| Empty | If there are no options, do not show a hollow ring — clear selection / do not open. |
| Hit model | Angular wedges of a continuous donut around the card. Labels sit in wedges; the whole slice is the hit target. |
| Commit | `pointerdown` on wedge arms; `pointerup` on the **same** wedge commits. Slide off that wedge before release → cancel that press (menu stays open). |
| Hover | Hovering an `action` wedge still drives `auto_tap` payment preview; tap-for-mana does not. |
| After pick | Clear selection, then `tap_for_mana` intent or `session.play(action)` as today. |
| Dismiss | Esc, or pointer on dismiss scrim (outside the ring / in the card hole) clears selection. |

## Visual layout

**Continuous ring (option C):** one unbroken donut with divider ticks at wedge boundaries. Forest HUD fill, priority-gold stroke/dividers, stronger Llanowar fill + gold emphasis on hover/press. No gaps between wedges; no floating rectangular chips; no press `scale`/`translate` that shrinks the hit target under the cursor.

**One option:** the ring is a single 360° wedge (large hit); label at top.

**Many options:** equal angles; long labels truncate/ellipsis in-wedge; full string on `aria-label` / title.

## Structure & rendering

**Surface:** DOM overlay (sibling to board chrome), not canvas paint — keeps AT and pointer events off the canvas hit path ([client-canvas-map](../../client-canvas-map.md) dual-surface invariant).

**Component:** Replace internals of `components/molecules/activation-radial.tsx`. Board wiring (`selectedId`, `onRadialPick`, hover preview, empty handling) stays in `board.tsx` with only the props the pie needs.

**Geometry (pure, `lib/radial.ts`):**

- Outer radius ≈ today’s `activationRadialRadius(zoom)` (tracks on-screen card size).
- Inner radius just outside the on-screen card (half-height or half-diagonal + padding — exact formula in the implementation plan).
- `n` equal wedges; first wedge centered at top (`-π/2`), matching today’s chip angles.
- Pure helpers: wedge path `d`, angle → wedge index, label anchor. No DOM in the lib.

**SVG:** Adjacent wedge `<path>`s sharing edges; divider ticks; SVG `<text>` labels at mid-angle (foreignObject only if wrapping proves necessary).

**Position freeze:** Capture screen center + zoom when selection opens; do not rebind the ring to tween / hover-raise jitter while open. Dismiss and reopen to retarget.

**Identity:** Stable key per option (`tap_for_mana` / `action:{id}`) so action-list churn does not remount mid-press.

**Pointer layering:** Full-screen dismiss scrim behind the SVG (same role as today). Wedges above the scrim handle press/commit; scrim handles outside dismiss.

## Accessibility

- Each wedge is a focusable control (`role="button"`, tab order).
- Enter / Space on focused wedge = pick.
- Visible focus ring on the focused wedge.
- Accessible name = full option label (not the truncated visual).

## Testing

TDD default:

1. **Geometry unit tests** — outer/inner radius; wedge index from angle; anchors/paths for 1, 2, and 6 options.
2. **Pointer reducer / harness** — down→up same wedge commits; slide-off cancels; dismiss outside unchanged.
3. **Keep** existing `radialOptions` and outer-radius tests; extend helpers rather than duplicating option logic.

Regression bar for the reported bug: a press that shows hover/press styling and releases on the same wedge must call `onPick` once without requiring a second gesture.

## Files (expected touch set)

| Area | Path |
|------|------|
| Geometry + options | `client/src/lib/radial.ts`, `client/src/lib/radial.test.ts` |
| Pie UI | `client/src/components/molecules/activation-radial.tsx` (+ component test if valuable) |
| Wire-up | `client/src/components/organisms/board.tsx` (freeze center/zoom; empty radial; props only) |
| Specs | this doc; update activation-radial notes in `docs/superpowers/specs/2026-07-20-client-game-board-and-interaction.md` when implementing |

## Success criteria

- Mouse/trackpad: first deliberate press-and-release on a wedge activates (or taps for mana) without needing a retry under normal board motion.
- Radial does not close on a cancelled slide-off; does close on outside dismiss / Esc / successful pick.
- Payment preview on action hover still works.
- No change to engine or wire contracts.
