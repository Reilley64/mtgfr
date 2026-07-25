# Hand bar hover bring-to-front

**Status:** approved design  
**Module:** `client/app/board/html/hand.ts`, `docs/superpowers/specs/2026-07-20-hand-and-zone-bar.md`

## Goal

Match Arena: when the pointer hovers a card in the bottom action bar, that card paints in front of every other bar tile so the full face is readable.

## Non-goals

- Elevating **discard-selected** tiles in z-order (they keep raise + green ring only)
- Keyboard-focus bring-to-front
- Changing raise distance, fan tilt, peek/hit geometry, or playable borders
- Priority chrome or mulligan overlay faces

## Behavior

1. Resting stack order is unchanged: within each section, left cards sit under right cards (`z-index: index + 1` on the tile root).
2. While the pointer hovers a bar tile (command, hand, graveyard, or exile), that tile’s **root slot** stacks above **all** other action-bar tiles (whole bar, not section-only).
3. On hover leave, the tile returns to its resting z-order.
4. Discard-selected tiles do **not** receive elevated z from selection; hover on a selected tile still brings it to front while hovered.

## Implementation

- Apply resting and hover z on the tile root (`group/hand-tile`) so sibling slots compete in one stacking context.
- Resting order uses a CSS custom property (`--hand-z: index + 1`) with class `[z-index:var(--hand-z)]` — **not** an inline `z-index` (inline beats Tailwind hover utilities and made elevate inert).
- Hover elevate: `hover:[z-index:50]` on the root (above any realistic resting index). Parent `:hover` matches while the hit strip child is hovered.
- Remove face-only hover `z-30` (it cannot beat a higher-index sibling slot).
- Keep face raise via `translateY` on hover and discard-selected as today.

## Spec truth

Update `2026-07-20-hand-and-zone-bar.md` Behavior to note hover bring-to-front on the slot; cross-link this design. Do not rewrite drag or playable-border rules.

## Testing

- Unit/Scene: a hovered tile root carries the elevate class (or equivalent); discard-selected without hover does not.
- Prefer asserting the root class/`z-index` contract over brittle computed paint order if the test harness cannot read stacking across siblings.
