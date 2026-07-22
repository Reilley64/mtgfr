import { describe, expect, it } from "vitest";
import { HAND_FACE_W } from "../motion/flights";
import {
  HAND_BAR_PEEK,
  type HandBarPeekSlot,
  handBarFaceLeft,
  handBarHitHeight,
  handBarHitWidth,
  handBarRaiseTranslateY,
  hitHandBarSlot,
} from "./handBarHit";

const PEEK = HAND_BAR_PEEK;
const FACE = HAND_FACE_W;

/** Two slots whose right-aligned peeks sit at x=100 and x=164 → left face edges adjacent. */
const FIRST_PEEK_LEFT = 100;
const TWO: HandBarPeekSlot[] = [
  { faceLeft: handBarFaceLeft(FIRST_PEEK_LEFT, FACE, PEEK) },
  { faceLeft: handBarFaceLeft(FIRST_PEEK_LEFT + PEEK, FACE, PEEK) },
];

/**
 * Pre-fix / wrong model: a raised non-rightmost card's entire face wins at z-top.
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

describe("handBarHitWidth", () => {
  it("uses the left peek for buried cards and the full face for the rightmost", () => {
    expect(handBarHitWidth(0, 3, PEEK, FACE)).toBe(PEEK);
    expect(handBarHitWidth(1, 3, PEEK, FACE)).toBe(PEEK);
    expect(handBarHitWidth(2, 3, PEEK, FACE)).toBe(FACE);
  });

  it("uses the full face for a single-card section (commander)", () => {
    expect(handBarHitWidth(0, 1, PEEK, FACE)).toBe(FACE);
  });
});

describe("hitHandBarSlot", () => {
  it("returns null when the pointer misses every hit region", () => {
    expect(hitHandBarSlot(TWO[0].faceLeft - 1, TWO, PEEK, FACE)).toBeNull();
    expect(hitHandBarSlot(TWO[1].faceLeft + FACE + 1, TWO, PEEK, FACE)).toBeNull();
  });

  it("hits adjacent left peeks on buried cards", () => {
    expect(TWO[1].faceLeft).toBe(TWO[0].faceLeft + PEEK);
    expect(hitHandBarSlot(TWO[0].faceLeft, TWO, PEEK, FACE)).toBe(0);
    expect(hitHandBarSlot(TWO[0].faceLeft + PEEK - 1, TWO, PEEK, FACE)).toBe(0);
    expect(hitHandBarSlot(TWO[1].faceLeft, TWO, PEEK, FACE)).toBe(1);
  });

  it("hits the full face of the rightmost card, including its right half", () => {
    const onRightHalf = TWO[1].faceLeft + PEEK + 20;
    expect(onRightHalf).toBeLessThan(TWO[1].faceLeft + FACE);
    expect(hitHandBarSlot(onRightHalf, TWO, PEEK, FACE)).toBe(1);
  });

  it("keeps the left neighbor reachable while the right card is raised", () => {
    const overLeftPeek = TWO[0].faceLeft + 10;
    expect(hitHandBarSlot(overLeftPeek, TWO, PEEK, FACE)).toBe(0);
    expect(hitHandBarSlotFullFaceWhenRaised(overLeftPeek, TWO, PEEK, FACE, 1)).toBe(0);
  });

  it("keeps the right neighbor reachable while the left card is raised", () => {
    // Raised left face overhangs the right card's left peek — full-face steals; peek-only does not.
    const overRightPeek = TWO[1].faceLeft + 10;
    expect(hitHandBarSlot(overRightPeek, TWO, PEEK, FACE)).toBe(1);
    expect(hitHandBarSlotFullFaceWhenRaised(overRightPeek, TWO, PEEK, FACE, 0)).toBe(0);
  });

  it("still hits the raised rightmost card across its full face", () => {
    const onRaisedRight = TWO[1].faceLeft + PEEK + 30;
    expect(hitHandBarSlot(onRaisedRight, TWO, PEEK, FACE)).toBe(1);
    expect(hitHandBarSlotFullFaceWhenRaised(onRaisedRight, TWO, PEEK, FACE, 1)).toBe(1);
  });
});

/** Slot-local Y: 0 at the top of the resting peek, `visibleH` at its bottom. */
function hitBandBottomAnchored(raised: boolean, visibleH: number, cardH: number): { top: number; bottom: number } {
  const height = handBarHitHeight(raised, visibleH, cardH);
  // Bottom edge stays on the resting peek bottom; raise only grows upward.
  return { top: visibleH - height, bottom: visibleH };
}

describe("handBarHitHeight / raise translate (vertical thrash)", () => {
  const VISIBLE = 130;
  const CARD = Math.round(FACE / 0.716);

  it("keeps the resting visible bottom inside the hit band when raised", () => {
    const restBottom = VISIBLE;
    const resting = hitBandBottomAnchored(false, VISIBLE, CARD);
    const raised = hitBandBottomAnchored(true, VISIBLE, CARD);
    expect(restBottom).toBeGreaterThanOrEqual(resting.top);
    expect(restBottom).toBeLessThanOrEqual(resting.bottom);
    expect(restBottom).toBeGreaterThanOrEqual(raised.top);
    expect(restBottom).toBeLessThanOrEqual(raised.bottom);
    // Bottom edge must not move — moving it out from under the cursor is the thrash.
    expect(raised.bottom).toBe(resting.bottom);
  });

  it("lifts the face by cardH - visibleH without growing the layout footprint", () => {
    expect(handBarRaiseTranslateY(false, VISIBLE, CARD)).toBe(0);
    expect(handBarRaiseTranslateY(true, VISIBLE, CARD)).toBe(VISIBLE - CARD);
    expect(handBarHitHeight(false, VISIBLE, CARD)).toBe(VISIBLE);
    expect(handBarHitHeight(true, VISIBLE, CARD)).toBe(CARD);
  });
});
