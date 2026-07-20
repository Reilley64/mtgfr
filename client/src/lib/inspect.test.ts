import { describe, expect, it } from "vitest";
import {
  inspectRootChanged,
  pinFromHit,
  playFace,
  popInspectHistory,
  pushInspectSource,
  shownName,
} from "~/lib/inspect";

describe("playFace", () => {
  it("defaults to front when not prepared", () => {
    expect(playFace(false, true)).toBe("front");
  });

  it("defaults to back when prepared and a back exists", () => {
    expect(playFace(true, true)).toBe("back");
  });

  it("stays front when prepared but no back", () => {
    expect(playFace(true, false)).toBe("front");
  });
});

describe("shownName", () => {
  it("returns the front name on the front face", () => {
    expect(shownName("Front", "Back", "front")).toBe("Front");
  });

  it("returns the back name on the back face when present", () => {
    expect(shownName("Front", "Back", "back")).toBe("Back");
  });
});

describe("pinFromHit", () => {
  const BF = 2;

  it("pins a prepared battlefield permanent with objectId", () => {
    expect(pinFromHit(true, { name: "Bear", prepared: true, id: 7, zone: BF }, BF)).toEqual({
      name: "Bear",
      prepared: true,
      objectId: 7,
    });
  });

  it("omits objectId off the battlefield", () => {
    expect(pinFromHit(true, { name: "Bear", id: 7, zone: 1 }, BF)).toEqual({
      name: "Bear",
      prepared: false,
    });
  });

  it("rejects face-down, missing, or Alt-up", () => {
    expect(pinFromHit(true, { name: "Library", faceDown: true, id: 1, zone: BF }, BF)).toBeNull();
    expect(pinFromHit(true, null, BF)).toBeNull();
    expect(pinFromHit(false, { name: "Bear", id: 1, zone: BF }, BF)).toBeNull();
  });
});

describe("inspect history", () => {
  it("detects a new root pin vs the same permanent", () => {
    expect(inspectRootChanged(undefined, { name: "Bear", prepared: false, objectId: 1 })).toBe(true);
    expect(
      inspectRootChanged({ name: "Bear", prepared: false, objectId: 1 }, { name: "Bear", prepared: true, objectId: 1 }),
    ).toBe(false);
    expect(
      inspectRootChanged(
        { name: "Bear", prepared: false, objectId: 1 },
        { name: "Bear", prepared: false, objectId: 2 },
      ),
    ).toBe(true);
  });

  it("pushes catalog-only sources and pops back to the root", () => {
    const root = { name: "Bear", prepared: false, objectId: 1 };
    const stacked = pushInspectSource([root], {
      name: "Lightning Greaves",
      cardId: "greaves-id",
      print: "greaves-print",
    });
    expect(stacked).toEqual([
      root,
      { name: "Lightning Greaves", prepared: false, cardId: "greaves-id", print: "greaves-print" },
    ]);
    expect(popInspectHistory(stacked)).toEqual([root]);
    expect(popInspectHistory([root])).toEqual([root]);
  });
});
