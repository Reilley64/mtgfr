import { describe, expect, it } from "vitest";
import { colors } from "~/design-tokens.generated";
import { TARGET_COLOR } from "../action/targeting";
import type { RenderCard } from "../geometry/layout";
import { aimArrowShapes, combatDragArrowShapes, stackTargetArrowShapes } from "./arrows";

function card(id: number, over: Partial<RenderCard> = {}): RenderCard {
  return {
    id,
    x: 100,
    y: 200,
    w: 96,
    h: 134,
    name: "Bear",
    cardId: "",
    print: "",
    pt: "2/2",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: false,
    zone: 1,
    controller: 1,
    owner: 1,
    kind: "creature",
    tapsForMana: false,
    summoningSick: false,
    hasHaste: false,
    keywords: [],
    goaded: false,
    isCommander: false,
    prepared: false,
    pile: 0,
    cluster: 0,
    clusterMembers: [],
    ...over,
  };
}

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

describe("stackTargetArrowShapes", () => {
  it("draws Island Blue arrows from stack faces to declared object/player targets", () => {
    const shapes = stackTargetArrowShapes({
      viewport: { width: 1440, height: 900 },
      stack: [
        { controller: 0, kind: "spell", label: "Lightning Bolt", source: 1, target: { kind: "object", id: 22 } },
        { controller: 0, kind: "spell", label: "Shock", source: 2, target: { kind: "player", player: 1 } },
        { controller: 0, kind: "spell", label: "Divination", source: 3, target: null },
      ],
      cards: [card(22)],
      avatars: { 0: { x: 200, y: 800 }, 1: { x: 720, y: 80 } },
      camera: { panX: 0, panY: 0, zoom: 1 },
    });
    expect(shapes).toHaveLength(4);
    const strokes = shapes.filter((s) => s._tag === "Path" && s.stroke === TARGET_COLOR);
    expect(strokes.length).toBe(2);
  });

  it("skips stack entries without a resolvable target", () => {
    const shapes = stackTargetArrowShapes({
      viewport: { width: 1440, height: 900 },
      stack: [{ controller: 0, kind: "spell", label: "Divination", source: 3 }],
      cards: [],
      avatars: {},
      camera: { panX: 0, panY: 0, zoom: 1 },
    });
    expect(shapes).toEqual([]);
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
    expect(shapes[0].stroke).toBe(colors.wallGreen);
  });
});
