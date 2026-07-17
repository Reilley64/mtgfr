import { describe, expect, it } from "vitest";
import { notePlayOrigin, stackInFromDelta, takePlayOrigin } from "~/lib/playOrigin";

describe("playOrigin", () => {
  it("notes and takes by card id without disturbing other entries", () => {
    const map = new Map<number, { x: number; y: number }>();
    notePlayOrigin(map, 10, { x: 1, y: 2 });
    notePlayOrigin(map, 20, { x: 3, y: 4 });
    expect(takePlayOrigin(map, 10)).toEqual({ x: 1, y: 2 });
    expect(map.has(10)).toBe(false);
    expect(takePlayOrigin(map, 20)).toEqual({ x: 3, y: 4 });
    expect(takePlayOrigin(map, 10)).toBeNull();
  });

  it("computes stack-in CSS delta so from+delta lands on to", () => {
    const from = { x: 100, y: 200 };
    const to = { x: 400, y: 250 };
    const { dx, dy } = stackInFromDelta(from, to);
    expect(to.x + dx).toBe(from.x);
    expect(to.y + dy).toBe(from.y);
  });
});
