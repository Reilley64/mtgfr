// Hand-bar hit policy for the dense Arena fan.
//
// Stacking: left lowest, right highest (so resting cards show a left-edge name strip).
// Layout: each flex slot is still peek-wide and visible-tall; the full face hangs left of that
// right-aligned peek and tucks under the screen. Buried cards are hit on the LEFT `peekW` only.
// The rightmost card in a section has no neighbor to protect, so its entire face is hittable.
// Raise is paint-only (`handBarRaiseTranslateY`); the hit strip bottom-anchors and grows upward
// (`handBarHitHeight`) so a cursor on the resting visible bottom never leave/enter-thrashes.

/** Visible strip width at rest — left edge of the face (card name), Arena-style. */
export const HAND_BAR_PEEK = 64;

export type HandBarPeekSlot = {
  /** Screen X of the card face's left edge. */
  faceLeft: number;
};

/** Left edge of a face that is right-aligned in a peek-wide flex slot. */
export function handBarFaceLeft(peekLeft: number, faceW: number, peekW: number): number {
  return peekLeft - (faceW - peekW);
}

/** Hit width for slot `index` in a section of `count` — full face on the rightmost only. */
export function handBarHitWidth(index: number, count: number, peekW: number, faceW: number): number {
  if (peekW <= 0 || faceW <= 0 || count <= 0) return 0;
  if (index < 0 || index >= count) return 0;
  return index === count - 1 ? faceW : peekW;
}

/**
 * Hit strip height for a hand-bar slot. Bottom-anchored in the resting peek box so raise
 * grows upward — a cursor parked on the resting visible bottom stays inside (no enter/leave
 * thrash). Growing the layout slot height with a top-anchored face is the failing model.
 */
export function handBarHitHeight(raised: boolean, visibleH: number, cardH: number): number {
  if (visibleH <= 0 || cardH <= 0) return 0;
  return raised ? Math.max(visibleH, cardH) : visibleH;
}

/**
 * Paint-only raise: translateY (px) applied to the face while the layout slot stays
 * `visibleH` tall. Negative = toward the board.
 */
export function handBarRaiseTranslateY(raised: boolean, visibleH: number, cardH: number): number {
  if (!raised || visibleH <= 0 || cardH <= 0) return 0;
  return visibleH - cardH;
}

/**
 * Which hand-bar slot owns `pointerX`.
 * Buried slots use the left peek; the rightmost uses the full face. When hit regions overlap,
 * prefer the rightmost slot (Arena paint order). Returns the slot index, or null on a miss.
 */
export function hitHandBarSlot(
  pointerX: number,
  slots: readonly HandBarPeekSlot[],
  peekW: number,
  faceW: number,
): number | null {
  if (peekW <= 0 || faceW <= 0 || slots.length === 0) return null;
  // Right-on-top: scan right→left so an overlapping region prefers the higher card.
  for (let i = slots.length - 1; i >= 0; i--) {
    const left = slots[i].faceLeft;
    const width = handBarHitWidth(i, slots.length, peekW, faceW);
    if (pointerX >= left && pointerX < left + width) return i;
  }
  return null;
}
