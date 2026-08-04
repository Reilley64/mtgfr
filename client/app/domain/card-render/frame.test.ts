import { describe, expect, it } from "vitest";
import { ZONE } from "../../board/geometry/layout";
import type { ObjectView } from "../wire/types";
import { ASSET_H, ASSET_W } from "./assets";
import type { Blit, FaceData, FaceVariant, Rect, SlotRects } from "./frame";
import { CANONICAL, faceDataFrom, frameKey, slotRects } from "./frame";

/** `colors` and `mana_cost.colored` are both WUBRG-indexed — see `engine::Color::index`. */
const W = 0;
const B = 2;
const R = 3;
const G = 4;

/** A Llanowar Elves on the battlefield. Only the fields the renderer reads matter here. */
function view(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    id: 1,
    name: "Llanowar Elves",
    controller: 0,
    owner: 0,
    zone: ZONE.Battlefield,
    print: "p",
    kind: { kind: "creature", power: 1, toughness: 1 },
    mana_cost: { colored: [0, 0, 0, 0, 1], generic: 0 },
    colors: [G],
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

/** Measured off the vendored 750x1050 assets — see the constants in `frame.ts`. */
const ART_WINDOW = { x: 58, y: 119, w: 634, h: 463 };
const TITLE_BAR = { x: 58, y: 43, w: 634, h: 66 };
const TYPE_BAR = { x: 58, y: 592, w: 634, h: 61 };
const TEXT_BOX = { x: 58, y: 662, w: 634, h: 267 };
const PT_PLATE = { x: 579, y: 932, w: 130, h: 64 };
const TOP_STRIP_H = 195;

function scaled(r: Rect, sx: number, sy: number): Rect {
  return { x: r.x * sx, y: r.y * sy, w: r.w * sx, h: r.h * sy };
}

function expectRectClose(actual: Rect | null, want: Rect): void {
  expect(actual).not.toBeNull();
  if (actual == null) return;
  expect(actual.x).toBeCloseTo(want.x, 3);
  expect(actual.y).toBeCloseTo(want.y, 3);
  expect(actual.w).toBeCloseTo(want.w, 3);
  expect(actual.h).toBeCloseTo(want.h, 3);
}

const VARIANTS: FaceVariant[] = ["permanent", "full", "stack"];

function textRects(slots: SlotRects): (Rect | null)[] {
  return [slots.art, slots.title, slots.type, slots.text, slots.pt];
}

function blits(slots: SlotRects): Blit[] {
  return [
    ...slots.frame,
    ...(slots.crown == null ? [] : [slots.crown]),
    ...(slots.ptPlate == null ? [] : [slots.ptPlate]),
  ];
}

describe("frameKey", () => {
  it("picks the printed colour's frame", () => {
    expect(frameKey(face({ colors: [G] }))).toBe("g");
    expect(frameKey(face({ colors: [1] }))).toBe("u");
  });

  it("picks the gold frame for a multicolour card", () => {
    expect(frameKey(face({ colors: [W, G] }))).toBe("m");
  });

  it("picks the land frame for a land whatever its colour identity", () => {
    expect(frameKey(face({ kind: { kind: "land", colors: [G] }, colors: [G] }))).toBe("land");
  });

  it("picks the colourless frame for an artifact with no colour", () => {
    expect(frameKey(face({ kind: { kind: "artifact" }, colors: [] }))).toBe("c");
  });

  // Balefire Liege is {2}{R/W}{R/W}{R/W} — three hybrid pips, so no monocoloured pip at all. Reading
  // the cost would draw a red-white creature in the colourless frame.
  it("picks the gold frame for a card whose only pips are hybrid", () => {
    expect(frameKey(face({ mana_cost: { colored: [0, 0, 0, 0, 0], generic: 2 }, colors: [W, R] }))).toBe("m");
  });

  // A token has no mana cost; its colour is stated on the token itself.
  it("picks a token's stated colour, not its (absent) cost", () => {
    expect(frameKey(face({ is_token: true, mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 }, colors: [G] }))).toBe(
      "g",
    );
  });

  // Smothering Abomination is {2}{B}{B} with devoid (CR 702.114a) — black pips, colourless card.
  it("picks the colourless frame for a devoid card with coloured pips", () => {
    expect(frameKey(face({ mana_cost: { colored: [0, 0, 2, 0, 0], generic: 2 }, colors: [] }))).toBe("c");
  });

  it("treats a missing colours field as colourless rather than throwing", () => {
    expect(frameKey(face({ colors: undefined, kind: { kind: "artifact" } }))).toBe("c");
  });

  it("never mistakes black for white when only black is set", () => {
    expect(frameKey(face({ colors: [B] }))).toBe("b");
  });
});

describe("faceDataFrom", () => {
  it("reads P/T off a creature and leaves it blank on a noncreature", () => {
    expect(face().power).toBe("1");
    expect(face({ kind: { kind: "instant" } }).power).toBe("");
  });

  it("takes the buffed power on the battlefield, not the printed one", () => {
    expect(face({ power: 3, toughness: 4 }).power).toBe("3");
  });

  it("reads loyalty off a planeswalker on the battlefield", () => {
    expect(face({ kind: { kind: "planeswalker", loyalty: 4 }, loyalty: 4 }).loyalty).toBe("4");
  });

  // Off the battlefield the engine has no permanent to measure, so it reports 0/0. The printed
  // numbers on `kind` are the only real ones for a card in hand or on the stack.
  it("falls back to the printed P/T for a creature that is not a permanent", () => {
    const inHand = face({
      zone: ZONE.Hand,
      kind: { kind: "creature", power: 2, toughness: 3 },
      power: 0,
      toughness: 0,
    });
    expect(inHand.power).toBe("2");
    expect(inHand.toughness).toBe("3");
  });

  it("falls back to the printed starting loyalty for a planeswalker on the stack", () => {
    expect(face({ zone: ZONE.Stack, kind: { kind: "planeswalker", loyalty: 5 }, loyalty: 0 }).loyalty).toBe("5");
  });

  it("reads a battle's defence into the badge slot", () => {
    expect(face({ kind: { kind: "battle", defense: 6 }, loyalty: 4 }).loyalty).toBe("4");
    expect(face({ zone: ZONE.Stack, kind: { kind: "battle", defense: 6 }, loyalty: 0 }).loyalty).toBe("6");
  });

  it("carries the object's colours through unchanged", () => {
    expect(face({ colors: [W, R] }).colors).toEqual([W, R]);
    expect(face({ colors: undefined }).colors).toEqual([]);
  });
});

describe("slotRects", () => {
  it("keeps every slot inside the canonical face and every source inside the asset", () => {
    for (const variant of VARIANTS) {
      const { w, h } = CANONICAL[variant];
      for (const face_ of [
        face(),
        face({ legendary: true }),
        face({ is_token: true }),
        face({ kind: { kind: "instant" } }),
      ]) {
        const slots = slotRects(variant, face_);
        for (const rect of textRects(slots)) {
          if (rect == null) continue;
          expect(rect.x, variant).toBeGreaterThanOrEqual(0);
          expect(rect.y, variant).toBeGreaterThanOrEqual(0);
          expect(rect.x + rect.w, variant).toBeLessThanOrEqual(w + 0.001);
          expect(rect.y + rect.h, variant).toBeLessThanOrEqual(h + 0.001);
        }
        for (const blit of blits(slots)) {
          expect(blit.src.x + blit.src.w, variant).toBeLessThanOrEqual(ASSET_W);
          expect(blit.src.y + blit.src.h, variant).toBeLessThanOrEqual(ASSET_H);
          expect(blit.dst.x + blit.dst.w, variant).toBeLessThanOrEqual(w + 0.001);
          expect(blit.dst.y + blit.dst.h, variant).toBeLessThanOrEqual(h + 0.001);
        }
      }
    }
  });

  it("puts every full-face slot where the printed frame puts it", () => {
    const { w, h } = CANONICAL.full;
    const sx = w / ASSET_W;
    const sy = h / ASSET_H;
    const slots = slotRects("full", face());
    expectRectClose(slots.art, scaled(ART_WINDOW, sx, sy));
    expectRectClose(slots.title, scaled(TITLE_BAR, sx, sy));
    expectRectClose(slots.type, scaled(TYPE_BAR, sx, sy));
    expectRectClose(slots.text, scaled(TEXT_BOX, sx, sy));
    expectRectClose(slots.pt, scaled(PT_PLATE, sx, sy));
  });

  // The bug: the shipped fractions stopped the art at 0.4413 of the face and put the type bar at
  // 0.4713, so the type line and the top of the text box drew inside the transparent art window,
  // over the artwork, while the printed bars below them sat empty.
  it("starts a full face's type line at or below the bottom of the art window", () => {
    const slots = slotRects("full", face());
    expect(slots.type).not.toBeNull();
    expect(slots.type?.y).toBeGreaterThanOrEqual(slots.art.y + slots.art.h);
  });

  it("does not overlap a full face's text box with its P/T box", () => {
    const slots = slotRects("full", face());
    expect(slots.text).not.toBeNull();
    expect(slots.pt).not.toBeNull();
    if (slots.text == null || slots.pt == null) return;
    expect(slots.text.y + slots.text.h).toBeLessThanOrEqual(slots.pt.y);
  });

  it("blits the whole asset onto a full face and adds the crown only when legendary", () => {
    const { w, h } = CANONICAL.full;
    const whole = { x: 0, y: 0, w: ASSET_W, h: ASSET_H };
    const slots = slotRects("full", face());
    expect(slots.frame).toEqual([{ src: whole, dst: { x: 0, y: 0, w, h } }]);
    expect(slots.crown).toBeNull();
    expect(slotRects("full", face({ legendary: true })).crown).toEqual({ src: whole, dst: { x: 0, y: 0, w, h } });
  });

  it("blits the P/T plate for a creature and leaves it off a noncreature", () => {
    const { w, h } = CANONICAL.full;
    const plate = slotRects("full", face()).ptPlate;
    expect(plate?.src).toEqual(PT_PLATE);
    expectRectClose(plate?.dst ?? null, scaled(PT_PLATE, w / ASSET_W, h / ASSET_H));
    expect(slotRects("full", face({ kind: { kind: "instant" } })).ptPlate).toBeNull();
    expect(slotRects("full", face({ kind: { kind: "instant" } })).pt).toBeNull();
  });

  it("gives the stack variant the same slots as a full face", () => {
    expect(slotRects("stack", face())).toEqual(slotRects("full", face()));
  });

  it("fills the Arena square with art and lays the top strip over it", () => {
    const { w, h } = CANONICAL.permanent;
    const s = w / ASSET_W;
    const slots = slotRects("permanent", face());
    expect(slots.art).toEqual({ x: 0, y: 0, w, h });
    expect(slots.frame[0]?.src).toEqual({ x: 0, y: 0, w: ASSET_W, h: TOP_STRIP_H });
    expectRectClose(slots.frame[0]?.dst ?? null, { x: 0, y: 0, w, h: TOP_STRIP_H * s });
    expectRectClose(slots.title, scaled(TITLE_BAR, s, s));
    expect(slots.type).toBeNull();
    expect(slots.text).toBeNull();
  });

  // The top strip alone left the square open below y=195: no side border past the title and no
  // bottom edge at all, so the art bled to three edges. The border has to close the ring.
  it("borders the Arena square on all four edges", () => {
    const { w, h } = CANONICAL.permanent;
    const s = w / ASSET_W;
    const side = ART_WINDOW.x * s;
    const top = TOP_STRIP_H * s;
    const [, left, right, bottom] = slotRects("permanent", face()).frame;

    // Each edge is sourced from the asset's matching edge, so it keeps the card's own colour. M15
    // prints no coloured band under the text box, so the bottom is the side rail laid on its side.
    expect(left?.src.x).toBe(0);
    expect(right?.src.x).toBe(ASSET_W - ART_WINDOW.x);
    expect(bottom?.src).toEqual(left?.src);
    expect(bottom?.turn).toBe("ccw");

    // The sides run from under the top strip down to the bottom edge, leaving no gap.
    expectRectClose(left?.dst ?? null, { x: 0, y: top, w: side, h: h - top - side });
    expectRectClose(right?.dst ?? null, { x: w - side, y: top, w: side, h: h - top - side });
    expectRectClose(bottom?.dst ?? null, { x: 0, y: h - side, w, h: side });
  });

  // A 750x1050 frame squashed into the square would run at 71% of its own height; the strip keeps
  // the frame art's own aspect, which is why it scales by the square's width in both axes.
  it("scales the square's top strip by width in both axes, not by the square's height", () => {
    const { w } = CANONICAL.permanent;
    const strip = slotRects("permanent", face()).frame[0];
    expect(strip?.dst.h).toBeCloseTo(TOP_STRIP_H * (w / ASSET_W), 3);
    expect(strip?.dst.h).not.toBeCloseTo(TOP_STRIP_H * (CANONICAL.permanent.h / ASSET_H), 3);
  });

  // `board/bitmap/paint-cards.ts` already paints a live P/T badge in that corner; a printed one
  // would double it up.
  it("draws no printed P/T on the Arena square", () => {
    expect(slotRects("permanent", face()).pt).toBeNull();
    expect(slotRects("permanent", face()).ptPlate).toBeNull();
  });

  it("crowns a legendary permanent over the same strip", () => {
    const strip = slotRects("permanent", face({ legendary: true }));
    expect(strip.crown).toEqual(strip.frame[0]);
    expect(slotRects("permanent", face()).crown).toBeNull();
  });

  it("gives a token art alone — no frame and no title", () => {
    const slots = slotRects("permanent", face({ is_token: true }));
    const { w, h } = CANONICAL.permanent;
    expect(slots.frame).toEqual([]);
    expect(slots.title).toBeNull();
    expect(slots.crown).toBeNull();
    expect(slots.art).toEqual({ x: 0, y: 0, w, h });
  });
});
