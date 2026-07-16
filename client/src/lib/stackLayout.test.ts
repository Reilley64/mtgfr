import { describe, expect, it } from "vitest";
import {
  STACK_CARD_W,
  STACK_EXPAND_COUNT,
  STACK_PEEK,
  STACK_STRIP_MIN_PEEK,
  stackCardH,
  stackExpandAvailable,
  stackPeekFor,
  stackPresentation,
  stackStripFits,
  stackStripPeek,
  shouldAutoCollapseStackExpand,
} from "~/lib/boardDraw";

describe("stackPeekFor", () => {
  it("keeps full peek when the pile fits the usable band", () => {
    expect(stackPeekFor(3, 900, 120)).toBe(STACK_PEEK);
  });

  it("compresses peek so the pile stays inside the usable band", () => {
    const n = 20;
    const viewportH = 600;
    const reserved = 120;
    const peek = stackPeekFor(n, viewportH, reserved);
    expect(peek).toBeLessThan(STACK_PEEK);
    expect(peek).toBeGreaterThanOrEqual(0);
    const pileH = stackCardH() + (n - 1) * peek;
    expect(pileH).toBeLessThanOrEqual(viewportH - reserved + 0.5);
  });

  it("returns full peek for a single card", () => {
    expect(stackPeekFor(1, 400, 120)).toBe(STACK_PEEK);
  });
});

describe("stackExpandAvailable", () => {
  it("opens at the reading count threshold even at full peek", () => {
    expect(stackExpandAvailable(STACK_EXPAND_COUNT, STACK_PEEK)).toBe(true);
    expect(stackExpandAvailable(STACK_EXPAND_COUNT - 1, STACK_PEEK)).toBe(false);
  });

  it("opens once peek compression has started", () => {
    expect(stackExpandAvailable(3, STACK_PEEK - 1)).toBe(true);
  });
});

describe("stackStripFits / stackStripPeek", () => {
  it("fits a short strip at comfortable peek", () => {
    expect(stackStripFits(4, 1200)).toBe(true);
    expect(stackStripPeek(4, 1200)).toBeGreaterThanOrEqual(STACK_STRIP_MIN_PEEK);
  });

  it("rejects a strip that cannot fit even at min peek", () => {
    const n = 40;
    expect(stackStripFits(n, 800)).toBe(false);
  });

  it("compresses horizontal peek before overflowing", () => {
    const peek = stackStripPeek(12, 900);
    expect(peek).toBeLessThanOrEqual(STACK_PEEK);
    const width = STACK_CARD_W + 11 * peek;
    expect(width).toBeLessThanOrEqual(900 - 48 + 0.5);
  });
});

describe("shouldAutoCollapseStackExpand", () => {
  it("collapses when the stack empties", () => {
    expect(shouldAutoCollapseStackExpand({ expanded: true, count: 0, peek: STACK_PEEK, staged: false })).toBe(true);
  });

  it("collapses when both expand thresholds clear", () => {
    expect(
      shouldAutoCollapseStackExpand({
        expanded: true,
        count: STACK_EXPAND_COUNT - 1,
        peek: STACK_PEEK,
        staged: false,
      }),
    ).toBe(true);
  });

  it("stays open while a staged target is live even if thresholds clear", () => {
    expect(
      shouldAutoCollapseStackExpand({
        expanded: true,
        count: 2,
        peek: STACK_PEEK,
        staged: true,
      }),
    ).toBe(false);
  });

  it("still collapses an empty stack while staged", () => {
    expect(shouldAutoCollapseStackExpand({ expanded: true, count: 0, peek: STACK_PEEK, staged: true })).toBe(true);
  });
});

describe("stackPresentation", () => {
  it("stays pile until expand is open", () => {
    expect(stackPresentation({ count: 10, expandedOpen: false, viewportW: 1200, viewportH: 800 })).toBe("pile");
  });

  it("uses expanded when the strip fits", () => {
    expect(stackPresentation({ count: 5, expandedOpen: true, viewportW: 1200, viewportH: 800 })).toBe("expanded");
  });

  it("escalates to full when the strip will not fit", () => {
    expect(stackPresentation({ count: 40, expandedOpen: true, viewportW: 800, viewportH: 800 })).toBe("full");
  });
});
