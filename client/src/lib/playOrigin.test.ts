import { describe, expect, it } from "vitest";
import { stackInFromDelta } from "~/lib/playOrigin";

describe("playOrigin", () => {
  it("stackInFromDelta is from minus to", () => {
    const from = { x: 10, y: 20 };
    const to = { x: 3, y: 5 };
    const { dx, dy } = stackInFromDelta(from, to);
    expect(dx).toBe(7);
    expect(dy).toBe(15);
  });
});
