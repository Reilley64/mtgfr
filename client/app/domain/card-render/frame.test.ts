import { describe, expect, it } from "vitest";
import type { ObjectView } from "../wire/types";
import type { FaceData } from "./frame";
import { CANONICAL, faceDataFrom, frameKey, slotRects } from "./frame";

/** A Llanowar Elves on the battlefield. Only the fields the renderer reads matter here. */
function view(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    id: 1,
    name: "Llanowar Elves",
    controller: 0,
    owner: 0,
    zone: 2,
    print: "p",
    kind: { kind: "creature", power: 1, toughness: 1 },
    mana_cost: { colored: [0, 0, 0, 0, 1], generic: 0 },
    power: 1,
    toughness: 1,
    marked_damage: 0,
    plus_counters: 0,
    legendary: false,
    is_token: false,
    is_commander: false,
    has_haste: false,
    needs_target: false,
    summoning_sick: false,
    tapped: false,
    ...overrides,
  } as ObjectView;
}

function face(overrides: Partial<ObjectView> = {}): FaceData {
  return faceDataFrom(view(overrides));
}

/** `mana_cost.colored` is WUBRG-indexed pip counts — see `engine::Color::index`. */
const W = 0;
const U = 1;
const G = 4;

function pips(...colors: number[]): ObjectView["mana_cost"] {
  const colored = [0, 0, 0, 0, 0];
  for (const c of colors) colored[c] += 1;
  return { colored, generic: 0 };
}

describe("frameKey", () => {
  it("picks the printed colour's frame", () => {
    expect(frameKey(face({ mana_cost: pips(G) }))).toBe("g");
    expect(frameKey(face({ mana_cost: pips(U) }))).toBe("u");
  });

  it("picks the gold frame for a multicolour card", () => {
    expect(frameKey(face({ mana_cost: pips(G, W) }))).toBe("m");
  });

  it("picks the land frame for a land whatever its colour identity", () => {
    expect(frameKey(face({ kind: { kind: "land", colors: [G] }, mana_cost: pips() }))).toBe("land");
  });

  it("picks the colourless frame for an artifact with no coloured pip", () => {
    expect(frameKey(face({ kind: { kind: "artifact" }, mana_cost: { colored: [0, 0, 0, 0, 0], generic: 2 } }))).toBe(
      "c",
    );
  });
});

describe("faceDataFrom", () => {
  it("reads P/T off a creature and leaves it blank on a noncreature", () => {
    expect(faceDataFrom(view()).power).toBe("1");
    expect(faceDataFrom(view({ kind: { kind: "instant" } })).power).toBe("");
  });

  it("takes the buffed power on the battlefield, not the printed one", () => {
    expect(faceDataFrom(view({ power: 3, toughness: 4 })).power).toBe("3");
  });

  it("reads loyalty off a planeswalker", () => {
    expect(faceDataFrom(view({ kind: { kind: "planeswalker", loyalty: 4 }, loyalty: 4 })).loyalty).toBe("4");
  });
});

describe("slotRects", () => {
  it("gives a permanent a title slot and art, and no text box", () => {
    const slots = slotRects("permanent", face());
    expect(slots.title).not.toBeNull();
    expect(slots.text).toBeNull();
    expect(slots.type).toBeNull();
    expect(slots.art.w).toBeGreaterThan(0);
  });

  it("gives a token art with no title slot", () => {
    const slots = slotRects("permanent", face({ is_token: true }));
    expect(slots.title).toBeNull();
    expect(slots.art.y).toBeLessThan(slotRects("permanent", face()).art.y);
  });

  it("keeps every slot inside the canonical face", () => {
    const { w, h } = CANONICAL.permanent;
    const slots = slotRects("permanent", face());
    for (const rect of [slots.frame, slots.art, slots.title, slots.pt]) {
      if (rect == null) continue;
      expect(rect.x).toBeGreaterThanOrEqual(0);
      expect(rect.y).toBeGreaterThanOrEqual(0);
      expect(rect.x + rect.w).toBeLessThanOrEqual(w);
      expect(rect.y + rect.h).toBeLessThanOrEqual(h);
    }
  });

  it("gives a full face a type line, a text box, and a P/T box for a creature", () => {
    const slots = slotRects("full", face());
    expect(slots.type).not.toBeNull();
    expect(slots.text).not.toBeNull();
    expect(slots.pt).not.toBeNull();
  });

  it("gives a noncreature no P/T box", () => {
    expect(slotRects("full", face({ kind: { kind: "instant" } })).pt).toBeNull();
  });
});
