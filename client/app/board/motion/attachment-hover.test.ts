import { describe, expect, it } from "vitest";
import {
  attachmentHoverNeedsFrame,
  easedAttachmentHoverProgress,
  reconcileAttachmentHover,
  stepAttachmentHover,
} from "./attachment-hover";

describe("attachment hover motion", () => {
  it("starts a newly hovered attachment at rest", () => {
    expect(reconcileAttachmentHover(new Map(), 7)).toEqual(new Map([[7, 0]]));
  });

  it("preserves a departing attachment while a new hover starts", () => {
    expect(reconcileAttachmentHover(new Map([[6, 0.5]]), 7)).toEqual(
      new Map([
        [6, 0.5],
        [7, 0],
      ]),
    );
  });

  it("advances hover entry over 120 ms", () => {
    expect(stepAttachmentHover(new Map([[7, 0]]), 7, 60, false)).toEqual(new Map([[7, 0.5]]));
  });

  it("reverses a departed hover and removes it at rest", () => {
    expect(stepAttachmentHover(new Map([[7, 1]]), null, 60, false)).toEqual(new Map([[7, 0.5]]));
    expect(stepAttachmentHover(new Map([[7, 0.5]]), null, 60, false)).toEqual(new Map());
  });

  it("settles immediately when reduced motion is preferred", () => {
    expect(stepAttachmentHover(new Map([[7, 0]]), 7, 1, true)).toEqual(new Map([[7, 1]]));
    expect(stepAttachmentHover(new Map([[7, 1]]), null, 1, true)).toEqual(new Map());
  });

  it("keeps the shared animation clock alive only while motion is unfinished", () => {
    expect(attachmentHoverNeedsFrame(new Map([[7, 1]]), 7)).toBe(false);
    expect(attachmentHoverNeedsFrame(new Map([[7, 0.5]]), 7)).toBe(true);
  });

  it("smoothsteps clamped paint progress without changing the raw clock", () => {
    expect(easedAttachmentHoverProgress(-1)).toBe(0);
    expect(easedAttachmentHoverProgress(0.25)).toBe(0.15625);
    expect(easedAttachmentHoverProgress(2)).toBe(1);
  });
});
