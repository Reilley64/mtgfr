import { describe, expect, it, vi } from "vitest";
import { colors } from "~/design-tokens.generated";
import { BLANK_FACE } from "../../domain/card-render/frame";
import type { RenderCard } from "../geometry/layout";
import { ZONE } from "../geometry/layout";
import {
  paintCard,
  paintCardAssignAmount,
  paintCardPickedHighlight,
  paintCardTargetHighlight,
  TARGET_COLOR,
} from "./paint-cards";

function card(overrides: Partial<RenderCard> = {}): RenderCard {
  return {
    cardId: "card",
    cluster: 0,
    clusterMembers: [],
    controller: 0,
    counters: 0,
    faceDown: false,
    goaded: false,
    face: BLANK_FACE,
    h: 134,
    hasHaste: false,
    id: 1,
    isCommander: false,
    keywords: [],
    kind: "creature",
    markedDamage: 0,
    name: "Grizzly Bears",
    owner: 0,
    pile: 0,
    prepared: false,
    print: "print-id",
    pt: "2/2",
    summoningSick: false,
    tapped: false,
    tapsForMana: false,
    w: 96,
    x: 10,
    y: 20,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function mockCtx(calls: string[] = []): CanvasRenderingContext2D {
  const state = { fillStyle: "", strokeStyle: "" };
  const ctx = {
    arc: vi.fn(),
    beginPath: vi.fn(),
    clip: vi.fn(),
    drawImage: vi.fn(),
    fill: vi.fn(() => calls.push(`fill:${state.fillStyle}`)),
    fillText: vi.fn(),
    measureText: vi.fn(() => ({ width: 0 })),
    restore: vi.fn(),
    rotate: vi.fn(),
    roundRect: vi.fn(),
    save: vi.fn(),
    setLineDash: vi.fn(),
    stroke: vi.fn(() => calls.push(`stroke:${state.strokeStyle}`)),
    strokeText: vi.fn(),
    translate: vi.fn(),
  } as unknown as CanvasRenderingContext2D;
  Object.defineProperty(ctx, "fillStyle", {
    get: () => state.fillStyle,
    set: (value) => {
      state.fillStyle = String(value);
    },
  });
  Object.defineProperty(ctx, "strokeStyle", {
    get: () => state.strokeStyle,
    set: (value) => {
      state.strokeStyle = String(value);
    },
  });
  return ctx;
}

describe("paintCardTargetHighlight", () => {
  it("strokes a dashed glow around the card footprint", () => {
    const ctx = mockCtx();

    paintCardTargetHighlight(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), 0);

    expect(ctx.shadowColor).toBe(TARGET_COLOR);
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.setLineDash).toHaveBeenCalledWith([2, 6]);
  });
});

describe("paintCardPickedHighlight", () => {
  it("strokes a solid Priority Gold ring around the card footprint", () => {
    const ctx = mockCtx();

    paintCardPickedHighlight(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), 0);

    expect(ctx.shadowColor).toBe(colors.priorityGold);
    expect(ctx.stroke).toHaveBeenCalled();
    expect(ctx.setLineDash).toHaveBeenCalledWith([]);
  });
});

describe("paintCardAssignAmount", () => {
  it("draws a crimson assign-amount badge when amount is positive", () => {
    const calls: string[] = [];
    const ctx = mockCtx(calls);

    paintCardAssignAmount(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), 0, 3);

    expect(calls).toContain(`fill:${colors.damageCrimson}`);
    expect(ctx.fillText).toHaveBeenCalled();
  });

  it("skips painting when amount is zero", () => {
    const ctx = mockCtx();
    paintCardAssignAmount(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), 0, 0);
    expect(ctx.fillText).not.toHaveBeenCalled();
  });
});

describe("paintCard", () => {
  it("draws the cached print image", () => {
    const ctx = mockCtx();
    const image = {} as HTMLImageElement;
    const cache = { get: vi.fn(() => image) };

    paintCard(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), cache, 0);

    expect(ctx.drawImage).toHaveBeenCalledWith(image, 10, 20, 96, 134);
  });

  // A square tile has no long edge to swing, so a quarter turn reads as "nothing moved". Arena
  // tilts the tile instead and darkens it, which is legible at four-seat zoom.
  it("tilts a tapped card off square and leaves an untapped one upright", () => {
    const cache = { get: vi.fn(() => undefined) };

    const tappedCtx = mockCtx();
    paintCard(tappedCtx, { panX: 0, panY: 0, zoom: 1 }, card({ tapped: true }), cache, 0);
    const [angle] = (tappedCtx.rotate as unknown as { mock: { calls: number[][] } }).mock.calls[0];
    expect(angle).toBeGreaterThan(0);
    expect(angle).toBeLessThan(Math.PI / 8);

    const uprightCtx = mockCtx();
    paintCard(uprightCtx, { panX: 0, panY: 0, zoom: 1 }, card(), cache, 0);
    expect(uprightCtx.rotate).not.toHaveBeenCalled();
  });

  it("veils a tapped card in black and leaves an untapped one clear", () => {
    const cache = { get: vi.fn(() => undefined) };

    const calls: string[] = [];
    paintCard(mockCtx(calls), { panX: 0, panY: 0, zoom: 1 }, card({ tapped: true }), cache, 0);
    expect(calls.some((c) => c.startsWith("fill:rgba(0,0,0,"))).toBe(true);

    const upright: string[] = [];
    paintCard(mockCtx(upright), { panX: 0, panY: 0, zoom: 1 }, card(), cache, 0);
    expect(upright.some((c) => c.startsWith("fill:rgba(0,0,0,"))).toBe(false);
  });

  // The veil follows the tap animation in, so a card mid-turn is only part-way darkened.
  it("fades the veil in with the tap animation", () => {
    const cache = { get: vi.fn(() => undefined) };
    const veil = (calls: string[]) => calls.find((c) => c.startsWith("fill:rgba(0,0,0,")) ?? "";

    const half: string[] = [];
    paintCard(mockCtx(half), { panX: 0, panY: 0, zoom: 1 }, card({ tapped: true, tapFrac: 0.5 }), cache, 0);
    const full: string[] = [];
    paintCard(mockCtx(full), { panX: 0, panY: 0, zoom: 1 }, card({ tapped: true }), cache, 0);

    expect(veil(half)).not.toBe("");
    expect(veil(half)).not.toBe(veil(full));
  });

  it("keeps commander gold when adding a playable outline", () => {
    const calls: string[] = [];
    const ctx = mockCtx(calls);
    const cache = { get: vi.fn(() => undefined) };

    paintCard(ctx, { panX: 0, panY: 0, zoom: 1 }, card({ isCommander: true, pt: "" }), cache, 0, {
      outline: { color: colors.playableBorder, dash: [] },
    });

    expect(calls).toContain(`stroke:${colors.commanderGold}`);
    expect(calls).toContain(`stroke:${colors.playableBorder}`);
    expect(calls).not.toContain("fill:rgba(0,0,0,0.45)");
    const goldAt = calls.lastIndexOf(`stroke:${colors.commanderGold}`);
    const playableAt = calls.lastIndexOf(`stroke:${colors.playableBorder}`);
    // Playable border on the card edge, then gold as the outer halo.
    expect(playableAt).toBeGreaterThan(-1);
    expect(goldAt).toBeGreaterThan(playableAt);
  });
});

describe("paintCard: the rendered Arena face", () => {
  const cam = { panX: 0, panY: 0, zoom: 1 };
  const face = {} as CanvasImageSource;
  const printed = {} as unknown as HTMLImageElement;
  const printedCache = { get: vi.fn(() => printed) };

  function drawn(ctx: CanvasRenderingContext2D) {
    return (ctx.drawImage as unknown as { mock: { calls: unknown[][] } }).mock.calls.map((args) => args[0]);
  }

  it("blits the rendered face instead of the printed image once the face is drawn", () => {
    const ctx = mockCtx();

    paintCard(ctx, cam, card(), printedCache, 0, { faces: { get: () => face, request: () => {} } });

    expect(drawn(ctx)).toContain(face);
    expect(drawn(ctx)).not.toContain(printed);
  });

  it("falls back to the printed image while the face is still being drawn", () => {
    const ctx = mockCtx();

    paintCard(ctx, cam, card(), printedCache, 0, { faces: { get: () => undefined, request: () => {} } });

    expect(drawn(ctx)).toContain(printed);
  });

  it("asks the face cache for the permanent variant of this card's face", () => {
    const ctx = mockCtx();
    const request = vi.fn();
    const bear = { ...BLANK_FACE, name: "Grizzly Bears", print: "print-id" };

    paintCard(ctx, cam, card({ face: bear }), printedCache, 0, { faces: { get: () => undefined, request } });

    expect(request).toHaveBeenCalledWith(bear, "permanent");
  });

  // The Arena square is a battlefield treatment. A graveyard/exile/commander pile is a stack of
  // cards seen edge on, so it keeps the printed image.
  it("leaves the zone-column piles on the printed image", () => {
    const ctx = mockCtx();
    const request = vi.fn();

    paintCard(ctx, cam, card({ zone: ZONE.Graveyard, pile: 3 }), printedCache, 0, {
      faces: { get: () => face, request },
    });

    expect(request).not.toHaveBeenCalled();
    expect(drawn(ctx)).toContain(printed);
    expect(drawn(ctx)).not.toContain(face);
  });

  // The rendered square carries the printed plate art, so a badge over it would be a second box.
  it("writes the live power/toughness onto the rendered square's printed plate", () => {
    const calls: string[] = [];
    const ctx = mockCtx(calls);
    const bear = { ...BLANK_FACE, name: "Grizzly Bears", power: "2", toughness: "2" };

    paintCard(ctx, cam, card({ face: bear, pt: "4/4" }), printedCache, 0, {
      faces: { get: () => face, request: () => {} },
    });

    expect(ctx.fillText).toHaveBeenCalledWith("4/4", expect.any(Number), expect.any(Number));
    expect(calls).not.toContain("fill:#f4efe2"); // no badge behind it — the plate is the box
  });

  it("keeps the badge on a token, which prints no plate", () => {
    const calls: string[] = [];
    const ctx = mockCtx(calls);
    const token = { ...BLANK_FACE, name: "Beast", isToken: true, power: "3", toughness: "3" };

    paintCard(ctx, cam, card({ face: token, pt: "3/3" }), printedCache, 0, {
      faces: { get: () => face, request: () => {} },
    });

    expect(calls).toContain("fill:#f4efe2");
  });

  it("does not ask for a face-down permanent — it is a card back, not a printing", () => {
    const ctx = mockCtx();
    const request = vi.fn();

    paintCard(ctx, cam, card({ faceDown: true }), printedCache, 0, { faces: { get: () => face, request } });

    expect(request).not.toHaveBeenCalled();
    expect(drawn(ctx)).not.toContain(face);
  });
});
