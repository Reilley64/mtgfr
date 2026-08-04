import { describe, expect, it, vi } from "vitest";
import { ASSET_W, BODY_FONT, TITLE_FONT } from "./assets";
import { CANONICAL, type FaceData, slotRects } from "./frame";
import { drawFace, faceAssetUrls } from "./render";

/** Records the ops a draw makes, so the test asserts what was drawn without pixels. */
function fakeCtx() {
  const ops: Array<{ op: string; args: unknown[] }> = [];
  const record =
    (op: string) =>
    (...args: unknown[]) => {
      ops.push({ op, args });
    };
  return {
    ops,
    drawn: (): unknown[][] => ops.filter((o) => o.op === "drawImage").map((o) => o.args),
    texts: (): string[] => ops.filter((o) => o.op === "fillText").map((o) => String(o.args[0])),
    ctx: {
      canvas: { width: 745, height: 745 },
      save: record("save"),
      restore: record("restore"),
      beginPath: record("beginPath"),
      translate: record("translate"),
      rotate: record("rotate"),
      roundRect: record("roundRect"),
      rect: record("rect"),
      clip: record("clip"),
      fill: record("fill"),
      fillRect: record("fillRect"),
      stroke: record("stroke"),
      drawImage: record("drawImage"),
      fillText: record("fillText"),
      measureText: vi.fn((text: string) => ({ width: text.length * 8 })),
      set font(value: string) {
        ops.push({ op: "font", args: [value] });
      },
      set fillStyle(value: string) {
        ops.push({ op: "fillStyle", args: [value] });
      },
      set textAlign(value: string) {
        ops.push({ op: "textAlign", args: [value] });
      },
      set textBaseline(value: string) {
        ops.push({ op: "textBaseline", args: [value] });
      },
    } as unknown as CanvasRenderingContext2D,
  };
}

function face(overrides: Partial<FaceData> = {}): FaceData {
  return {
    print: "p",
    name: "Llanowar Elves",
    colors: [4], // G — see `engine::Color::index`
    isLand: false,
    isToken: false,
    legendary: false,
    power: "1",
    toughness: "1",
    loyalty: "",
    // The permanent variant draws neither; the full-variant tests below set them the way the
    // catalog lookup does.
    typeLine: "",
    oracle: "",
    ...overrides,
  };
}

/** Scryfall's `art_crop` is a wide rectangle — the shape the square variant has to crop. */
const artImage = { width: 626, height: 457 } as CanvasImageSource;
const assetImage = { width: 750, height: 1050 } as CanvasImageSource;

function inputs(overrides: Record<string, unknown> = {}) {
  return {
    face: face(),
    variant: "permanent" as const,
    art: artImage,
    frameImage: assetImage,
    ptImage: assetImage,
    crownImage: null,
    ...overrides,
  };
}

describe("drawFace", () => {
  it("draws the art, then the frame over it, then the name", () => {
    const { ctx, ops, texts } = fakeCtx();
    drawFace(ctx, inputs());

    const order = ops.filter((o) => o.op === "drawImage" || o.op === "fillText").map((o) => o.op);
    expect(order[0]).toBe("drawImage"); // art first, frame over its transparent window
    expect(order).toContain("fillText");
    expect(texts()).toContain("Llanowar Elves");
  });

  it("never draws a mana cost — the pip tray owns the cost", () => {
    const { ctx, texts } = fakeCtx();
    drawFace(ctx, inputs({ variant: "full", face: face({ typeLine: "Creature — Elf Druid" }) }));

    expect(texts().some((t) => t.includes("{"))).toBe(false);
    expect(texts()).toContain("Creature — Elf Druid");
  });

  it("draws no name on a token", () => {
    const { ctx, texts } = fakeCtx();
    drawFace(ctx, inputs({ face: face({ isToken: true }) }));

    expect(texts()).not.toContain("Llanowar Elves");
  });

  it("draws no frame at all on a token — the art is the whole tile", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs({ face: face({ isToken: true }) }));

    expect(drawn()).toHaveLength(1);
  });

  it("draws power and toughness for a creature", () => {
    // The `permanent` variant returns `pt: null` on purpose — `paint-cards.ts` already paints a
    // live P/T badge that tracks counters and damage without a face redraw.
    const { ctx, texts } = fakeCtx();
    drawFace(ctx, inputs({ variant: "full" }));

    expect(texts()).toContain("1/1");
  });

  // A real M15 card at this asset's 750x1050 sets its name in roughly 41px, its type line in 36px
  // and its rules text in 35px. The face is drawn at 745x1040, so the same numbers land here.
  it("sets each slot at the size a printed card uses", () => {
    const { ctx, ops } = fakeCtx();
    drawFace(ctx, inputs({ variant: "full", face: face({ typeLine: "Creature — Elf Druid", oracle: "Haste." }) }));

    const sizesIn = (font: string) =>
      ops
        .filter((o) => o.op === "font" && String(o.args[0]).includes(font))
        .map((o) => Number.parseFloat(String(o.args[0])));
    const [name, typeLine] = sizesIn(TITLE_FONT);

    expect(name).toBeGreaterThan(39);
    expect(name).toBeLessThan(43);
    expect(typeLine).toBeGreaterThan(34);
    expect(typeLine).toBeLessThan(38);
    expect(Math.max(...sizesIn(BODY_FONT))).toBeGreaterThan(33);
    expect(Math.max(...sizesIn(BODY_FONT))).toBeLessThan(37);
  });

  it("draws a planeswalker's loyalty instead of a power/toughness", () => {
    const { ctx, texts } = fakeCtx();
    drawFace(ctx, inputs({ variant: "full", face: face({ power: "", toughness: "", loyalty: "3" }) }));

    expect(texts()).toContain("3");
    expect(texts().some((t) => t.includes("/"))).toBe(false);
  });

  it("draws nothing but the frame when the art has not loaded", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs({ art: null }));

    expect(drawn()).toHaveLength(slotRects("permanent", face()).frame.length);
    expect(drawn().every((blit) => blit[0] === assetImage)).toBe(true);
  });

  it("lays only the frame's top strip over the square, not the squashed whole card", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs());

    const [, frame] = drawn();
    const strip = slotRects("permanent", face()).frame[0];
    // src: the strip inside the printed border, not the 1050-tall card. dst: scaled by width in
    // both axes, so it keeps shape.
    expect(frame.slice(1, 5)).toEqual([strip?.src.x, strip?.src.y, strip?.src.w, strip?.src.h]);
    expect(frame[7]).toBe(CANONICAL.permanent.w);
    expect(frame[8]).toBeCloseTo((strip?.src.h ?? 0) * (CANONICAL.permanent.w / (ASSET_W - 60)), 3);
  });

  // M15 has no coloured band along the card's bottom, so the square's bottom edge is the side rail
  // turned on its side — drawn under a quarter turn, into a box with its own w and h swapped.
  it("lays the square's bottom edge on its side", () => {
    const { ctx, ops, drawn } = fakeCtx();
    drawFace(ctx, inputs());

    const band = slotRects("permanent", face()).frame.at(-1)?.dst;
    expect(ops.some((o) => o.op === "rotate" && o.args[0] === -Math.PI / 2)).toBe(true);
    expect(drawn().at(-1)?.slice(5)).toEqual([-(band?.h ?? 0) / 2, -(band?.w ?? 0) / 2, band?.h, band?.w]);
  });

  it("crops the art to fill the square instead of squashing it", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs());

    const [art] = drawn();
    // A 626x457 crop into a 1:1 window keeps the full height and takes a centred 457-wide slice.
    expect(art.slice(1, 5)).toEqual([(626 - 457) / 2, 0, 457, 457]);
    expect(art.slice(5)).toEqual([0, 0, CANONICAL.permanent.w, CANONICAL.permanent.h]);
  });

  it("crowns a legendary permanent over the same strip as the frame", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs({ face: face({ legendary: true }), crownImage: assetImage }));

    // Art, then the frame's four edges, then the crown — over the top strip it replaces.
    const [, topStrip] = drawn();
    expect(drawn().at(-1)?.slice(1)).toEqual(topStrip?.slice(1));
  });

  it("blits the P/T plate from its corner of the asset on a full face", () => {
    const { ctx, drawn } = fakeCtx();
    drawFace(ctx, inputs({ variant: "full" }));

    const plate = drawn().at(-1);
    expect(plate?.slice(1, 5)).toEqual([579, 932, 130, 64]);
  });
});

describe("faceAssetUrls", () => {
  it("asks for the crown only for a legendary permanent", () => {
    expect(faceAssetUrls(face({ legendary: true })).crown).not.toBeNull();
    expect(faceAssetUrls(face()).crown).toBeNull();
  });

  it("asks for no P/T plate on a noncreature", () => {
    expect(faceAssetUrls(face({ power: "", toughness: "" })).pt).toBeNull();
  });

  it("crowns a legendary land but asks for no P/T plate — no land prints one", () => {
    const cradle = faceAssetUrls(face({ isLand: true, legendary: true, power: "", toughness: "" }));
    expect(cradle.crown).toContain("land");
    expect(cradle.pt).toBeNull();
  });

  it("names the frame matching the card's colour", () => {
    expect(faceAssetUrls(face()).frame).toContain("/g.");
    expect(faceAssetUrls(face({ colors: [0, 3] })).frame).toContain("/m.");
  });
});
