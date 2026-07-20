// Hand-bar hit policy for the dense Arena fan. Each slot's flex footprint is peek-wide; the
// full face hangs left of that peek. Interactive hits must stay inside the peek strip — if a
// raised card's full face captured pointers, its left overhang would steal the left neighbor.

/** Visible strip width at rest — right edge of the face (mana-cost corner), Arena-style. */
export const HAND_BAR_PEEK = 64;

export type HandBarPeekSlot = {
  /** Screen X of the peek strip's left edge. */
  peekLeft: number;
};

/**
 * Which hand-bar slot owns `pointerX` under the peek-only hit policy.
 * Peeks are adjacent (no overlap), so raised state never expands the hit strip leftward.
 * Returns the slot index, or null on a miss.
 */
export function hitHandBarSlot(pointerX: number, slots: readonly HandBarPeekSlot[], peekW: number): number | null {
  if (peekW <= 0 || slots.length === 0) return null;
  for (let i = 0; i < slots.length; i++) {
    const left = slots[i].peekLeft;
    if (pointerX >= left && pointerX < left + peekW) return i;
  }
  return null;
}
