import { describe, expect, it } from "vitest";
import { fitCanvasLabel } from "~/lib/fitCanvasLabel";

/** Approximate measureText: ~0.6em per character for the fake canvas font. */
function fakeCtx(charWidth = 6): CanvasRenderingContext2D {
  return {
    measureText: (text: string) => ({ width: text.length * charWidth }),
  } as CanvasRenderingContext2D;
}

describe("fitCanvasLabel", () => {
  it("returns the text unchanged when it already fits", () => {
    expect(fitCanvasLabel(fakeCtx(), "alice", 60)).toBe("alice");
  });

  it("ellipsis-truncates long seat names to the max width", () => {
    // charWidth 6: max 48px → 8 glyphs including ellipsis → 7 name chars + "…"
    expect(fitCanvasLabel(fakeCtx(), "p1784530275", 48)).toBe("p178453…");
  });

  it("returns empty for non-positive max width", () => {
    expect(fitCanvasLabel(fakeCtx(), "alice", 0)).toBe("");
  });

  it("returns only the ellipsis when nothing else fits", () => {
    expect(fitCanvasLabel(fakeCtx(10), "alice", 10)).toBe("…");
  });
});
