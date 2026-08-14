import { describe, expect, it, vi } from "vitest";
import type { FaceData } from "../../domain/card-render/frame";
import { COMMANDER_GOLD, PLAYABLE_BORDER } from "../chrome";
import { LIFT_SHADOW_COLOR } from "../lift-shadow";
import type { DragGhost } from "../motion/screen-motion";
import { paintDragGhost, paintScreenMotion } from "./paint-screen-motion";

function fakeCtx(calls: string[]): CanvasRenderingContext2D {
  const state = {
    fillStyle: "",
    strokeStyle: "",
    shadowColor: "",
    shadowBlur: 0,
    shadowOffsetY: 0,
    lineWidth: 1,
  };
  const ctx = {
    beginPath: vi.fn(),
    clip: vi.fn(),
    drawImage: vi.fn(),
    fill: vi.fn(() => calls.push(`fill:${state.fillStyle}`)),
    fillText: vi.fn(),
    restore: vi.fn(),
    roundRect: vi.fn(),
    save: vi.fn(),
    stroke: vi.fn(() => calls.push(`stroke:${state.strokeStyle}`)),
  } as unknown as CanvasRenderingContext2D;

  for (const key of ["fillStyle", "strokeStyle", "shadowColor"] as const) {
    Object.defineProperty(ctx, key, {
      get: () => state[key],
      set: (value) => {
        state[key] = String(value);
        if (key === "shadowColor" && value) calls.push(`shadow:${value}`);
      },
    });
  }
  Object.defineProperty(ctx, "shadowBlur", {
    get: () => state.shadowBlur,
    set: (value) => {
      state.shadowBlur = Number(value);
    },
  });
  Object.defineProperty(ctx, "shadowOffsetY", {
    get: () => state.shadowOffsetY,
    set: (value) => {
      state.shadowOffsetY = Number(value);
    },
  });
  Object.defineProperty(ctx, "lineWidth", {
    get: () => state.lineWidth,
    set: (value) => {
      state.lineWidth = Number(value);
    },
  });

  return ctx;
}

const FACE: FaceData = {
  print: "p1",
  name: "Bolt",
  colors: [1],
  isLand: false,
  isToken: false,
  legendary: false,
  power: "",
  toughness: "",
  loyalty: "",
  typeLine: "Instant",
  oracle: "",
  flavor: "",
};

const request = vi.fn();

function ghost(overrides: Partial<DragGhost> = {}): DragGhost {
  return {
    print: "",
    name: "Bolt",
    x: 100,
    y: 200,
    scale: 2,
    zone: "hand",
    ...overrides,
  };
}

describe("paintDragGhost", () => {
  it("applies the shared lift shadow then a mint playable ring for hand zone", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    paintDragGhost(ctx, ghost({ zone: "hand" }), 1, { get: () => undefined });
    expect(calls).toContain(`shadow:${LIFT_SHADOW_COLOR}`);
    expect(calls).toContain(`stroke:${PLAYABLE_BORDER}`);
  });

  it("strokes commander gold outside the mint ring for command zone", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    paintDragGhost(ctx, ghost({ zone: "command" }), 1, { get: () => undefined });
    expect(calls).toContain(`stroke:${PLAYABLE_BORDER}`);
    expect(calls).toContain(`stroke:${COMMANDER_GOLD}`);
  });

  // The tile the player grabbed wears its rendered face, so the card under the cursor must not
  // flip back to the printed scan for the length of the drag.
  it("flies the dragged card's rendered face, not its printed image", () => {
    const ctx = fakeCtx([]);
    const drawn = { width: 745, height: 1040 } as CanvasImageSource;
    const printed = { width: 488, height: 680 } as HTMLImageElement;

    paintDragGhost(ctx, ghost({ print: "p1", face: FACE }), 1, { get: () => printed }, { get: () => drawn, request });

    expect(ctx.drawImage).toHaveBeenCalledWith(
      drawn,
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
    );
  });

  it("asks for the face and shows the printed image until it is drawn", () => {
    const ctx = fakeCtx([]);
    const printed = { width: 488, height: 680 } as HTMLImageElement;
    const asked = vi.fn();

    paintDragGhost(
      ctx,
      ghost({ print: "p1", face: FACE }),
      1,
      { get: () => printed },
      { get: () => undefined, request: asked },
    );

    expect(asked).toHaveBeenCalledWith(FACE, "full");
    expect(ctx.drawImage).toHaveBeenCalledWith(
      printed,
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
      expect.any(Number),
    );
  });
});

describe("paintScreenMotion", () => {
  it("paints the drag ghost playable ring when a ghost is present", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    paintScreenMotion(ctx, {
      dragGhost: ghost(),
      flights: [
        {
          id: 1,
          print: "",
          name: "Flight",
          x: 10,
          y: 10,
          scale: 1,
          targetX: 10,
          targetY: 10,
          targetScale: 1,
          phase: "flying",
          kind: "stack",
        },
      ],
      exitFx: [],
      zoom: 1,
      cache: { get: () => undefined },
    });
    expect(calls).toContain(`stroke:${PLAYABLE_BORDER}`);
    expect(calls).toContain(`shadow:${LIFT_SHADOW_COLOR}`);
  });

  it("skips drag paint when dragGhost is null", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    paintScreenMotion(ctx, {
      dragGhost: null,
      flights: [],
      exitFx: [],
      zoom: 1,
      cache: { get: () => undefined },
    });
    expect(calls).not.toContain(`stroke:${PLAYABLE_BORDER}`);
  });
});
