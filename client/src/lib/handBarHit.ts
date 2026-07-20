// Hand-bar hit policy for the dense Arena fan.
//
// Stacking: left lowest, right highest (so resting cards show a left-edge name strip).
// Layout: each flex slot is still peek-wide; the full face hangs left of that right-aligned
// peek. The visible resting strip under right-on-top stacking is the LEFT `peekW` of each face.
//
// Hits stay on that left peek only. A raised card may paint its full face above neighbors, but
// must not capture their peeks — otherwise the card to the right (under the overhang) is hard
// to reach, the mirror of the old left-neighbor bug under left-on-top stacking.

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

/**
 * Which hand-bar slot owns `pointerX` under the left-peek hit policy.
 * Ideal peeks are adjacent (`faceLeft[i+1] === faceLeft[i] + peekW`). When they overlap,
 * prefer the rightmost slot (Arena paint order). Returns the slot index, or null on a miss.
 */
export function hitHandBarSlot(pointerX: number, slots: readonly HandBarPeekSlot[], peekW: number): number | null {
  if (peekW <= 0 || slots.length === 0) return null;
  // Right-on-top: scan right→left so an overlapping peek prefers the higher card.
  for (let i = slots.length - 1; i >= 0; i--) {
    const left = slots[i].faceLeft;
    if (pointerX >= left && pointerX < left + peekW) return i;
  }
  return null;
}
