import { describe, expect, it } from "vitest";
import { HAND_FACE_W } from "~/lib/cardFlight";
import { HAND_BAR_PEEK, type HandBarPeekSlot, hitHandBarSlot } from "~/lib/handBarHit";

const PEEK = HAND_BAR_PEEK;

/** Two adjacent peeks: card 0 at x=100, card 1 at x=164. */
const TWO: HandBarPeekSlot[] = [{ peekLeft: 100 }, { peekLeft: 100 + PEEK }];

/**
 * Pre-fix model: a raised card's entire face (hanging left of its peek) wins at z-top.
 * Used only to prove the regression — product code uses peek-only `hitHandBarSlot`.
 */
function hitHandBarSlotFullFaceWhenRaised(
  pointerX: number,
  slots: readonly HandBarPeekSlot[],
  peekW: number,
  faceW: number,
  raisedIndex: number | null,
): number | null {
  if (peekW <= 0 || faceW <= 0 || slots.length === 0) return null;
  const overhang = faceW - peekW;
  if (raisedIndex != null && raisedIndex >= 0 && raisedIndex < slots.length) {
    const peekLeft = slots[raisedIndex].peekLeft;
    const faceLeft = peekLeft - overhang;
    if (pointerX >= faceLeft && pointerX < peekLeft + peekW) return raisedIndex;
  }
  // Resting paint order: earlier (left) cards stack above later ones (`count - index`).
  for (let i = 0; i < slots.length; i++) {
    if (i === raisedIndex) continue;
    const peekLeft = slots[i].peekLeft;
    const faceLeft = peekLeft - overhang;
    if (pointerX >= faceLeft && pointerX < peekLeft + peekW) return i;
  }
  return null;
}

describe("hitHandBarSlot", () => {
  it("returns null when the pointer misses every peek", () => {
    expect(hitHandBarSlot(50, TWO, PEEK)).toBeNull();
    expect(hitHandBarSlot(300, TWO, PEEK)).toBeNull();
  });

  it("hits the peek strip of the left and right cards", () => {
    expect(hitHandBarSlot(100, TWO, PEEK)).toBe(0);
    expect(hitHandBarSlot(163, TWO, PEEK)).toBe(0);
    expect(hitHandBarSlot(164, TWO, PEEK)).toBe(1);
    expect(hitHandBarSlot(200, TWO, PEEK)).toBe(1);
  });

  it("lets the left neighbor keep its peek while the right card is raised", () => {
    // Pointer sits in card 0's peek. Card 1's full face hangs left over that strip when raised
    // (faceW 180, peek 64 → 116px overhang). Peek-only policy must still return card 0.
    const overLeftPeek = 120;
    expect(hitHandBarSlot(overLeftPeek, TWO, PEEK)).toBe(0);

    // The pre-fix full-face policy steals that point for the raised right card — the bug.
    expect(hitHandBarSlotFullFaceWhenRaised(overLeftPeek, TWO, PEEK, HAND_FACE_W, 1)).toBe(1);
  });

  it("still hits the raised card on its own peek strip", () => {
    expect(hitHandBarSlot(180, TWO, PEEK)).toBe(1);
    expect(hitHandBarSlotFullFaceWhenRaised(180, TWO, PEEK, HAND_FACE_W, 1)).toBe(1);
  });
});
