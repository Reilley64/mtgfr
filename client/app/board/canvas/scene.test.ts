import type { Canvas } from "foldkit";
import { describe, expect, it } from "vitest";
import type { ActionView, ObjectView, PlayerView, VisibleState } from "~/wire/types";
import { TARGET_COLOR } from "../action/targeting";
import { ZONE } from "../geometry/layout";
import { sceneShapes } from "./scene";

type Group = Canvas.Group;
type Shape = Canvas.Shape;

function player(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...overrides,
  };
}

function object(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 1,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    marked_damage: 0,
    name: "Grizzly Bears",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function battlefieldAction(objectId: number): ActionView {
  return {
    id: objectId + 100,
    kind: "activate",
    label: "Activate",
    needs_target: false,
    object: objectId,
    section: "battlefield",
  };
}

function boardFixture(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [object(), object({ id: 2, kind: { kind: "land", colors: [4] }, name: "Forest", power: 0, toughness: 0 })],
    pending_choice: null,
    players: [player(), player({ player: 1, username: "Bob" })],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function shapeContainsText(shape: Shape, content: string): boolean {
  switch (shape._tag) {
    case "Text":
      return shape.content === content;
    case "Group":
      return shape.shapes.some((child) => shapeContainsText(child, content));
    case "Rect":
    case "Circle":
    case "Path":
      return false;
    default: {
      const exhaustive: never = shape;
      return exhaustive;
    }
  }
}

function groupContainingText(shape: Shape, content: string): Group | null {
  if (shape._tag !== "Group") {
    return null;
  }

  if (shape.shapes.some((child) => shapeContainsText(child, content))) {
    return shape;
  }

  for (const child of shape.shapes) {
    const group = groupContainingText(child, content);
    if (group != null) {
      return group;
    }
  }

  return null;
}

function firstGroupContainingText(shapes: ReadonlyArray<Shape>, content: string): Group | null {
  for (const shape of shapes) {
    const group = groupContainingText(shape, content);
    if (group != null) {
      return group;
    }
  }

  return null;
}

function collectTextShapes(shapes: ReadonlyArray<Shape>): Canvas.Text[] {
  const texts: Canvas.Text[] = [];
  for (const shape of shapes) {
    switch (shape._tag) {
      case "Text":
        texts.push(shape);
        break;
      case "Group":
        texts.push(...collectTextShapes(shape.shapes));
        break;
      case "Rect":
      case "Circle":
      case "Path":
        break;
      default: {
        const exhaustive: never = shape;
        return exhaustive;
      }
    }
  }
  return texts;
}

function groupWithRectFill(shape: Shape, fill: string): Group | null {
  if (shape._tag !== "Group") {
    return null;
  }

  if (shape.shapes.some((child) => child._tag === "Rect" && child.fill === fill)) {
    return shape;
  }

  for (const child of shape.shapes) {
    const group = groupWithRectFill(child, fill);
    if (group != null) {
      return group;
    }
  }

  return null;
}

function firstGroupWithRectFill(shapes: ReadonlyArray<Shape>, fill: string): Group | null {
  for (const shape of shapes) {
    const group = groupWithRectFill(shape, fill);
    if (group != null) {
      return group;
    }
  }

  return null;
}

function firstRect(group: Group | null): Canvas.Rect | null {
  if (group == null) return null;
  const shape = group.shapes.find((child) => child._tag === "Rect");
  return shape?._tag === "Rect" ? shape : null;
}

function shapeHasFill(shape: Shape, fill: string): boolean {
  switch (shape._tag) {
    case "Rect":
    case "Circle":
      return shape.fill === fill;
    case "Group":
      return shape.shapes.some((child) => shapeHasFill(child, fill));
    case "Text":
    case "Path":
      return false;
    default: {
      const exhaustive: never = shape;
      return exhaustive;
    }
  }
}

function lastIndexWhere(shapes: ReadonlyArray<Shape>, predicate: (shape: Shape) => boolean): number {
  for (let i = shapes.length - 1; i >= 0; i--) {
    if (predicate(shapes[i])) {
      return i;
    }
  }

  return -1;
}

describe("sceneShapes", () => {
  it("builds a felt background rect", () => {
    const shapes = sceneShapes(boardFixture());
    expect(shapes.some((shape) => shape._tag === "Rect")).toBe(true);
  });

  it("builds seat/avatar vectors from the visible state", () => {
    const shapes = sceneShapes(boardFixture());
    const rects = shapes.filter((shape) => shape._tag === "Rect");
    const circles = shapes.filter((shape) => shape._tag === "Circle");

    expect(rects.length).toBeGreaterThanOrEqual(5);
    expect(circles.length).toBeGreaterThanOrEqual(2);
  });

  it("does not paint resting card names as canvas text", () => {
    const state = boardFixture();
    const shapes = sceneShapes(state);
    const texts = collectTextShapes(shapes);

    expect(texts.some((text) => text.content === "Grizzly Bears")).toBe(false);
    expect(texts.some((text) => text.content === "Forest")).toBe(false);
  });

  it("does not paint command-zone name or P/T labels over the commander face", () => {
    const state = boardFixture();
    state.objects = [
      ...state.objects,
      object({
        id: 10,
        name: "Atraxa, Praetors' Voice",
        zone: ZONE.Command,
        is_commander: true,
        kind: { kind: "creature", power: 4, toughness: 4 },
        power: 4,
        toughness: 4,
      }),
    ];
    const shapes = sceneShapes(state);
    const texts = collectTextShapes(shapes);

    expect(texts.some((text) => text.content === "Atraxa, Praetors' Voice")).toBe(false);
    expect(texts.some((text) => text.content === "4/4")).toBe(false);
    // Vestigial name-plate rect left over from under-card labels — must not cover zone art.
    expect(shapes.some((shape) => shapeHasFill(shape, "rgba(0,0,0,0.28)"))).toBe(false);
  });

  it("still paints creature power/toughness on resting cards", () => {
    const shapes = sceneShapes(boardFixture());
    const texts = collectTextShapes(shapes);

    expect(texts.some((text) => text.content === "2/2")).toBe(true);
  });

  it("rotates tapped card groups around their center", () => {
    const state = boardFixture();
    const shapes = sceneShapes({ ...state, objects: [object({ tapped: true })] });

    const group = firstGroupContainingText(shapes, "2/2");

    expect(group).not.toBeNull();
    if (group == null) {
      return;
    }

    expect(group.rotate).toBeCloseTo(Math.PI / 2);
  });

  it("paints arrows above resting cards and avatars", () => {
    const state = boardFixture();
    const shapes = sceneShapes({
      ...state,
      combat: { ...state.combat, attackers: [{ attacker: 1, defender: 1 }] },
    });

    const feltIndex = shapes.findIndex((shape) => shape._tag === "Rect" && shape.fill === "#0B1310");
    const cardIndex = shapes.findIndex((shape) => shapeContainsText(shape, "2/2"));
    const avatarIndex = shapes.findIndex((shape) => shape._tag === "Circle");
    const arrowIndex = lastIndexWhere(shapes, (shape) => shape._tag === "Path" && shape.stroke === "#ff6b6b");

    expect(feltIndex).toBeGreaterThanOrEqual(0);
    expect(cardIndex).toBeGreaterThan(feltIndex);
    expect(avatarIndex).toBeGreaterThan(cardIndex);
    expect(arrowIndex).toBeGreaterThan(avatarIndex);
  });

  it("paints a combat drag rubber-band from the creature to the cursor", () => {
    const state = boardFixture();
    const shapes = sceneShapes(state, {
      combatDrag: {
        from: { x: 200, y: 300 },
        to: { x: 500, y: 400 },
        declaringBlock: false,
      },
    });

    const dragArrow = shapes.find((shape) => shape._tag === "Path" && shape.stroke === "#ff6b6b");
    expect(dragArrow).toBeDefined();
  });

  it("highlights legal staged targets and paints an aim arrow to the cursor", () => {
    const state = boardFixture();
    const shapes = sceneShapes(state, {
      stagedTargeting: {
        targetObjects: new Set([1]),
        targetPlayers: new Set([1]),
        aimFrom: { x: 1300, y: 450 },
        cursor: { x: 500, y: 300 },
      },
    });

    const bearGroup = firstGroupContainingText(shapes, "2/2");
    expect(bearGroup).not.toBeNull();
    if (bearGroup == null) return;
    const bearRect = bearGroup.shapes.find((shape) => shape._tag === "Rect");
    expect(bearRect?._tag).toBe("Rect");
    if (bearRect?._tag !== "Rect") return;
    expect(bearRect.stroke).toBe(TARGET_COLOR);

    const aimArrow = shapes.find((shape) => shape._tag === "Path" && shape.stroke === TARGET_COLOR);
    expect(aimArrow).toBeDefined();
  });

  it("uses playable strokes for battlefield actions without outlining tap-only lands", () => {
    const state = boardFixture();
    state.actions = [battlefieldAction(1)];

    const shapes = sceneShapes(state);
    const bearRect = firstRect(firstGroupContainingText(shapes, "2/2"));
    const forestRect = firstRect(firstGroupWithRectFill(shapes, "#223820"));

    expect(bearRect?.stroke).toBe("#EAFFF0");
    expect(forestRect?.stroke).toBe("#1a1a1a");
  });

  it("keeps commander gold instead of controller seat strokes", () => {
    const state = boardFixture();
    state.objects = [object({ id: 9, is_commander: true })];

    const shapes = sceneShapes(state);
    const commanderRect = firstRect(firstGroupContainingText(shapes, "2/2"));

    expect(commanderRect?.stroke).toBe("#E9B84A");
  });
});
