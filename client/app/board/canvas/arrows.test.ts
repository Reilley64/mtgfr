import { describe, expect, it } from "vitest";
import { TARGET_COLOR } from "../action/targeting";
import { aimArrowShapes, combatDragArrowShapes } from "./arrows";

describe("aimArrowShapes", () => {
  it("draws a target arrow from the stack ghost to the cursor", () => {
    const shapes = aimArrowShapes({
      from: { x: 1300, y: 450 },
      to: { x: 500, y: 300 },
    });

    expect(shapes).toHaveLength(2);
    expect(shapes[0]._tag).toBe("Path");
    if (shapes[0]._tag !== "Path") return;
    expect(shapes[0].stroke).toBe(TARGET_COLOR);
  });
});

describe("combatDragArrowShapes", () => {
  it("uses attack red while declaring attackers", () => {
    const shapes = combatDragArrowShapes({
      from: { x: 100, y: 100 },
      to: { x: 300, y: 200 },
      declaringBlock: false,
    });
    expect(shapes[0]._tag).toBe("Path");
    if (shapes[0]._tag !== "Path") return;
    expect(shapes[0].stroke).toBe("#ff6b6b");
  });

  it("uses block green while declaring blockers", () => {
    const shapes = combatDragArrowShapes({
      from: { x: 100, y: 100 },
      to: { x: 300, y: 200 },
      declaringBlock: true,
    });
    expect(shapes[0]._tag).toBe("Path");
    if (shapes[0]._tag !== "Path") return;
    expect(shapes[0].stroke).toBe("#66ff99");
  });
});
