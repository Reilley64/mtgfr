import { describe, expect, it, vi } from "vitest";
import type { RenderCard } from "../geometry/layout";
import { ZONE } from "../geometry/layout";
import { paintCard, paintCardTargetHighlight, TARGET_COLOR } from "./paint-cards";

function card(overrides: Partial<RenderCard> = {}): RenderCard {
  return {
    cardId: "card",
    cluster: 0,
    clusterMembers: [],
    controller: 0,
    counters: 0,
    faceDown: false,
    fanAngle: 0,
    goaded: false,
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

describe("paintCard", () => {
  it("draws the cached print image", () => {
    const ctx = mockCtx();
    const image = {} as HTMLImageElement;
    const cache = { get: vi.fn(() => image) };

    paintCard(ctx, { panX: 0, panY: 0, zoom: 1 }, card(), cache, 0);

    expect(ctx.drawImage).toHaveBeenCalledWith(image, 10, 20, 96, 134);
  });

  it("keeps commander gold when adding a playable outline", () => {
    const calls: string[] = [];
    const ctx = mockCtx(calls);
    const cache = { get: vi.fn(() => undefined) };

    paintCard(ctx, { panX: 0, panY: 0, zoom: 1 }, card({ isCommander: true, pt: "" }), cache, 0, {
      outline: { color: "#EAFFF0", dash: [] },
    });

    expect(calls).toContain("stroke:#E9B84A");
    expect(calls).toContain("stroke:#EAFFF0");
    expect(calls).not.toContain("fill:rgba(0,0,0,0.45)");
    const goldAt = calls.lastIndexOf("stroke:#E9B84A");
    const playableAt = calls.lastIndexOf("stroke:#EAFFF0");
    // Playable border on the card edge, then gold as the outer halo.
    expect(playableAt).toBeGreaterThan(-1);
    expect(goldAt).toBeGreaterThan(playableAt);
  });
});
