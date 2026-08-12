import { describe, expect, it } from "vitest";
import { handMetrics } from "./handMetrics";
import {
  STACK_COMPACT_VISIBLE,
  STACK_PEEK,
  STACK_STRIP_MIN_PEEK,
  shouldAutoCollapseStackExpand,
  stackActionLane,
  stackExpandAvailable,
  stackFaceScreenOrigin,
  stackFanLayout,
  stackFanPlacement,
  stackFanVisualBounds,
  stackFullPerRow,
  stackPresentation,
  stackReservedActionRect,
  stackStripFits,
  stackStripPeek,
} from "./stackLayout";

describe("stackFanLayout", () => {
  it.each([
    [{ width: 1280, height: 720 }, 166, 232],
    [{ width: 1440, height: 900 }, 208, 291],
    [{ width: 2560, height: 1440 }, 312, 436],
  ] as const)("matches hand cards at $0", (viewport, cardW, cardH) => {
    expect(stackFanLayout(viewport, 4)).toMatchObject({ cardW, cardH });
  });

  it("shows the newest four rows and counts hidden objects", () => {
    expect(stackFanLayout({ width: 1440, height: 900 }, 7)).toMatchObject({
      visibleFrom: 3,
      visibleCount: 4,
      hiddenCount: 3,
    });
  });

  it("returns no placement for hidden rows", () => {
    const layout = stackFanLayout({ width: 1440, height: 900 }, 7);
    expect(stackFanPlacement(layout, 2)).toBeNull();
    expect(stackFanPlacement(layout, 3)).not.toBeNull();
  });

  it("orders compact origins left to right and keeps the top object rightmost", () => {
    const viewport = { width: 1440, height: 900 };
    const origins = [3, 4, 5, 6].map((row) => stackFaceScreenOrigin({ presentation: "pile", viewport, count: 7, row }));
    expect(origins.map(({ x }) => x)).toEqual([...origins.map(({ x }) => x)].sort((a, b) => a - b));
  });

  it.each([
    { width: 1280, height: 720 },
    { width: 1440, height: 900 },
    { width: 2560, height: 1440 },
  ] as const)("keeps every rotated compact face above the primary-action lane at $width×$height", (viewport) => {
    const layout = stackFanLayout(viewport, STACK_COMPACT_VISIBLE);
    const laneTop = viewport.height - handMetrics(viewport).barH - stackActionLane(viewport);

    for (let row = 0; row < STACK_COMPACT_VISIBLE; row++) {
      const placement = stackFanPlacement(layout, row);
      expect(placement).not.toBeNull();
      if (placement == null) continue;
      const radians = (Math.abs(placement.rotation) * Math.PI) / 180;
      const rotatedHeight = layout.cardW * Math.sin(radians) + layout.cardH * Math.cos(radians);
      const visualBottom = placement.y + layout.cardH / 2 + rotatedHeight / 2;
      expect(visualBottom).toBeLessThanOrEqual(laneTop);
    }
  });

  it.each([1, 2, 3, 4])(
    "keeps every transformed face in a %i-card short-landscape fan onscreen above the hand",
    (count) => {
      const viewport = { width: 844, height: 390 };
      const layout = stackFanLayout(viewport, count);
      const handTop = viewport.height - handMetrics(viewport).barH;

      for (let row = 0; row < count; row++) {
        const bounds = stackFanVisualBounds(layout, row);
        expect(bounds).not.toBeNull();
        if (bounds == null) continue;
        expect(bounds.left).toBeGreaterThanOrEqual(0);
        expect(bounds.top).toBeGreaterThanOrEqual(0);
        expect(bounds.right).toBeLessThanOrEqual(viewport.width);
        expect(bounds.bottom).toBeLessThanOrEqual(handTop);
      }
    },
  );

  it("reserves the action column at its CSS width instead of shrinking it with hand cards", () => {
    const viewport = { width: 844, height: 390 };
    expect(stackReservedActionRect(viewport).left).toBe(614);
  });

  it("keeps a short-landscape compact face clear of the reserved action column", () => {
    const viewport = { width: 844, height: 390 };
    const layout = stackFanLayout(viewport, 1);
    const bounds = stackFanVisualBounds(layout, 0);
    expect(bounds).not.toBeNull();
    if (bounds == null) return;

    const action = stackReservedActionRect(viewport);
    expect(bounds.right).toBeLessThanOrEqual(action.left);
  });

  it("keeps every transformed compact face onscreen in short landscape", () => {
    const viewport = { width: 844, height: 390 };
    const layout = stackFanLayout(viewport, STACK_COMPACT_VISIBLE);

    for (let row = 0; row < STACK_COMPACT_VISIBLE; row++) {
      const bounds = stackFanVisualBounds(layout, row);
      expect(bounds).not.toBeNull();
      if (bounds == null) continue;
      expect(bounds.left).toBeGreaterThanOrEqual(0);
      expect(bounds.top).toBeGreaterThanOrEqual(0);
      expect(bounds.right).toBeLessThanOrEqual(viewport.width);
      expect(bounds.bottom).toBeLessThanOrEqual(viewport.height);
    }
  });

  it.each([
    { width: 1280, height: 720 },
    { width: 1440, height: 900 },
    { width: 2560, height: 1440 },
  ] as const)("keeps the normal compact fan right-aligned at $width×$height", (viewport) => {
    const layout = stackFanLayout(viewport, STACK_COMPACT_VISIBLE);
    expect(layout.left + layout.fanW).toBe(viewport.width - 16);
  });
});

describe("stackExpandAvailable", () => {
  it("opens only beyond the compact visible count", () => {
    expect(stackExpandAvailable(STACK_COMPACT_VISIBLE + 1)).toBe(true);
    expect(stackExpandAvailable(STACK_COMPACT_VISIBLE)).toBe(false);
  });
});

describe("stackStripFits / stackStripPeek", () => {
  const viewport = { width: 1200, height: 900 };

  it("uses responsive card width for a short strip", () => {
    expect(stackStripFits(4, viewport)).toBe(true);
    const cardW = handMetrics(viewport).cardW;
    const peek = stackStripPeek(4, viewport, cardW);
    expect(cardW).toBe(handMetrics(viewport).cardW);
    expect(peek).toBeGreaterThanOrEqual(STACK_STRIP_MIN_PEEK * handMetrics(viewport).scale);
  });

  it("rejects a strip that cannot fit even at min peek", () => {
    expect(stackStripFits(50, { width: 800, height: 600 })).toBe(false);
  });

  it("compresses horizontal peek before overflowing", () => {
    const compact = { width: 900, height: 600 };
    const cardW = handMetrics(compact).cardW;
    const peek = stackStripPeek(12, compact, cardW);
    expect(peek).toBeLessThanOrEqual(STACK_PEEK * handMetrics(compact).scale);
    expect(cardW + 11 * peek).toBeLessThanOrEqual(compact.width - 48 * handMetrics(compact).scale + 0.5);
  });

  it("derives full-grid capacity from responsive card width", () => {
    const metrics = handMetrics(viewport);
    expect(stackFullPerRow(viewport, metrics.cardW)).toBeGreaterThan(1);
    expect(metrics.cardW).toBe(handMetrics(viewport).cardW);
  });
});

describe("shouldAutoCollapseStackExpand", () => {
  it("collapses when the stack empties", () => {
    expect(shouldAutoCollapseStackExpand({ expanded: true, count: 0, staged: false })).toBe(true);
  });

  it("collapses when only the compact rows remain", () => {
    expect(shouldAutoCollapseStackExpand({ expanded: true, count: STACK_COMPACT_VISIBLE, staged: false })).toBe(true);
  });

  it("stays open while staged", () => {
    expect(shouldAutoCollapseStackExpand({ expanded: true, count: STACK_COMPACT_VISIBLE, staged: true })).toBe(false);
  });
});

describe("stackPresentation", () => {
  it("defaults to pile when expand is closed", () => {
    expect(stackPresentation({ count: 8, expandedOpen: false, viewportW: 1200, viewportH: 900 })).toBe("pile");
  });

  it("uses expanded strip when cards fit horizontally", () => {
    expect(stackPresentation({ count: 5, expandedOpen: true, viewportW: 1200, viewportH: 900 })).toBe("expanded");
  });
});

describe("stackFaceScreenOrigin", () => {
  it("spreads expanded faces horizontally unlike the compact fan", () => {
    const viewport = { width: 1440, height: 900 };
    const left = stackFaceScreenOrigin({ presentation: "expanded", viewport, count: 3, row: 0 });
    const right = stackFaceScreenOrigin({ presentation: "expanded", viewport, count: 3, row: 2 });
    expect(left.x).toBeLessThan(right.x);
  });
});
