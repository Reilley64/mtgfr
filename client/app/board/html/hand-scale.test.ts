/**
 * The bar is sized as a fraction of the window. A face pinned at 208 CSS px is unreadably small on
 * a 27" 2560x1440 desktop and overwhelms a small laptop, so every bar length scales with the window
 * (clamped) instead of being a constant.
 */
import { describe, expect, it } from "vitest";
import { HAND_DESIGN_VIEWPORT, handMetrics, handUiScale } from "./hand";

describe("hand bar scale", () => {
  it("keeps the design sizes at the design window", () => {
    const m = handMetrics(HAND_DESIGN_VIEWPORT);
    expect(m.scale).toBe(1);
    expect(m.cardW).toBe(208);
    expect(m.visibleH).toBe(178);
    expect(m.barH).toBe(218);
  });

  it("grows the faces on a 2560x1440 desktop", () => {
    const wide = handMetrics({ width: 2560, height: 1440 });
    const base = handMetrics(HAND_DESIGN_VIEWPORT);
    expect(wide.cardW).toBeGreaterThan(base.cardW);
    expect(wide.barH).toBeGreaterThan(base.barH);
    // The window is 1.6x the design height but the scale caps at 1.5x.
    expect(wide.cardW).toBe(Math.round(base.cardW * 1.5));
    // Wider than a physical Magic card at ~109 PPI, so the art and title read at desk distance.
    expect(wide.cardW / 109).toBeGreaterThan(2.5);
  });

  it("shrinks the faces on a small laptop so the bar does not eat the board", () => {
    const small = handMetrics({ width: 1280, height: 720 });
    expect(small.cardW).toBeLessThan(208);
    expect(small.barH).toBeLessThan(218);
  });

  it("clamps both ends so extreme windows do not distort the layout", () => {
    expect(handUiScale({ width: 7680, height: 4320 })).toBe(1.5);
    expect(handUiScale({ width: 640, height: 400 })).toBe(0.75);
    expect(handUiScale({ width: 0, height: 0 })).toBe(1);
  });

  it("keeps every derived length in step with the face", () => {
    for (const viewport of [{ width: 1280, height: 720 }, HAND_DESIGN_VIEWPORT, { width: 2560, height: 1440 }]) {
      const m = handMetrics(viewport);
      expect(m.overlap).toBe(m.cardW - m.peek);
      expect(m.cardH).toBe(Math.round(m.cardW / 0.716));
      expect(m.stickyBand).toBe(m.barH - m.visibleH + m.cardH);
      // The bar must never reserve more room than the faces it holds.
      expect(m.barH).toBeLessThan(m.cardH + m.pipRowH);
    }
  });
});
