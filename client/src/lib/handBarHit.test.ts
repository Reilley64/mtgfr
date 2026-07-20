import { describe, expect, it } from "vitest";
import { HAND_FACE_W } from "~/lib/cardFlight";
import { HAND_BAR_PEEK, type HandBarPeekSlot, handBarFaceLeft, hitHandBarSlot } from "~/lib/handBarHit";

const PEEK = HAND_BAR_PEEK;
const FACE = HAND_FACE_W;

/** Two slots whose right-aligned peeks sit at x=100 and x=164 → left face edges adjacent. */
const FIRST_PEEK_LEFT = 100;
const TWO: HandBarPeekSlot[] = [
  { faceLeft: handBarFaceLeft(FIRST_PEEK_LEFT, FACE, PEEK) },
  { faceLeft: handBarFaceLeft(FIRST_PEEK_LEFT + PEEK, FACE, PEEK) },
];

/**
 * Pre-fix / wrong model: a raised card's entire face wins at z-top.
 * With Arena right-on-top stacking that steals the *right* neighbor's left peek.
 */
function hitHandBarSlotFullFaceWhenRaised(
  pointerX: number,
  slots: readonly HandBarPeekSlot[],
  peekW: number,
  faceW: number,
  raisedIndex: number | null,
): number | null {
  if (peekW <= 0 || faceW <= 0 || slots.length === 0) return null;
  if (raisedIndex != null && raisedIndex >= 0 && raisedIndex < slots.length) {
    const faceLeft = slots[raisedIndex].faceLeft;
    if (pointerX >= faceLeft && pointerX < faceLeft + faceW) return raisedIndex;
  }
  // Resting Arena paint: rightmost on top.
  for (let i = slots.length - 1; i >= 0; i--) {
    if (i === raisedIndex) continue;
    const faceLeft = slots[i].faceLeft;
    if (pointerX >= faceLeft && pointerX < faceLeft + faceW) return i;
  }
  return null;
}

describe("handBarFaceLeft", () => {
  it("hangs the face left of a right-aligned peek slot", () => {
    expect(handBarFaceLeft(100, FACE, PEEK)).toBe(100 - (FACE - PEEK));
  });
});

describe("hitHandBarSlot", () => {
  it("returns null when the pointer misses every left peek", () => {
    expect(hitHandBarSlot(TWO[0].faceLeft - 1, TWO, PEEK)).toBeNull();
    expect(hitHandBarSlot(TWO[1].faceLeft + PEEK + 1, TWO, PEEK)).toBeNull();
  });

  it("hits adjacent left peeks (visible name strips under right-on-top stacking)", () => {
    expect(TWO[1].faceLeft).toBe(TWO[0].faceLeft + PEEK);
    expect(hitHandBarSlot(TWO[0].faceLeft, TWO, PEEK)).toBe(0);
    expect(hitHandBarSlot(TWO[0].faceLeft + PEEK - 1, TWO, PEEK)).toBe(0);
    expect(hitHandBarSlot(TWO[1].faceLeft, TWO, PEEK)).toBe(1);
    expect(hitHandBarSlot(TWO[1].faceLeft + PEEK - 1, TWO, PEEK)).toBe(1);
  });

  it("keeps the left neighbor reachable while the right card is raised", () => {
    // Left peeks sit outside the raised right face to its left — full-face also preserves this.
    const overLeftPeek = TWO[0].faceLeft + 10;
    expect(hitHandBarSlot(overLeftPeek, TWO, PEEK)).toBe(0);
    expect(hitHandBarSlotFullFaceWhenRaised(overLeftPeek, TWO, PEEK, FACE, 1)).toBe(0);
  });

  it("keeps the right neighbor reachable while the left card is raised", () => {
    // Raised left face overhangs the right card's left peek — full-face steals; peek-only does not.
    const overRightPeek = TWO[1].faceLeft + 10;
    expect(hitHandBarSlot(overRightPeek, TWO, PEEK)).toBe(1);
    expect(hitHandBarSlotFullFaceWhenRaised(overRightPeek, TWO, PEEK, FACE, 0)).toBe(0);
  });

  it("still hits the raised card on its own left peek", () => {
    const onRaisedPeek = TWO[1].faceLeft + 10;
    expect(hitHandBarSlot(onRaisedPeek, TWO, PEEK)).toBe(1);
    expect(hitHandBarSlotFullFaceWhenRaised(onRaisedPeek, TWO, PEEK, FACE, 1)).toBe(1);
  });
});
