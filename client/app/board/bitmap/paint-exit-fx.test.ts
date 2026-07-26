import { describe, expect, it, vi } from "vitest";
import { spawnExitFx } from "../motion/exit-fx";
import { paintExitFx } from "./paint-exit-fx";

function fakeCtx(calls: string[]): CanvasRenderingContext2D {
  const state = {
    fillStyle: "",
    strokeStyle: "",
    globalAlpha: 1,
  };
  const ctx = {
    arc: vi.fn(),
    beginPath: vi.fn(),
    clip: vi.fn(),
    createRadialGradient: vi.fn(() => ({
      addColorStop: vi.fn(),
    })),
    drawImage: vi.fn((image: { label?: string }) => calls.push(`image:${image.label ?? "unknown"}`)),
    fill: vi.fn(() => calls.push(`fill:${state.fillStyle}`)),
    fillRect: vi.fn(() => calls.push(`fillRect:${state.fillStyle}`)),
    fillText: vi.fn(),
    restore: vi.fn(),
    roundRect: vi.fn(),
    save: vi.fn(),
    scale: vi.fn((x: number, y: number) => calls.push(`scale:${x.toFixed(2)}:${y.toFixed(2)}`)),
    stroke: vi.fn(() => calls.push(`stroke:${state.strokeStyle}`)),
    translate: vi.fn((x: number, y: number) => calls.push(`translate:${x}:${y}`)),
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
  Object.defineProperty(ctx, "globalAlpha", {
    get: () => state.globalAlpha,
    set: (value) => {
      state.globalAlpha = Number(value);
      calls.push(`alpha:${state.globalAlpha.toFixed(2)}`);
    },
  });

  return ctx;
}

describe("paintExitFx", () => {
  it("paints destroy FX with warm ember particles over the lifted card face", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    const fx = {
      ...spawnExitFx({
        id: 1,
        print: "destroy-print",
        name: "Ash Bear",
        kind: "destroy",
        x: 50,
        y: 50,
        scale: 1,
        seed: 1,
      }),
      progress: 0.5,
    };
    const cache = {
      get: vi.fn(() => ({ label: "destroy" }) as unknown as HTMLImageElement),
    };

    paintExitFx(ctx, fx, 1, cache, [{ x: 50, y: 40, r: 2, color: "#ffb040", alpha: 0.8 }]);

    expect(calls).toContain("image:destroy");
    expect(calls).toContain("fill:#ffb040");
  });

  it("paints exile FX with a center squash and cool void fade", () => {
    const calls: string[] = [];
    const ctx = fakeCtx(calls);
    const fx = {
      ...spawnExitFx({
        id: 2,
        print: "exile-print",
        name: "Void Bear",
        kind: "exile",
        x: 50,
        y: 50,
        scale: 1,
        seed: 2,
      }),
      progress: 0.85,
    };

    paintExitFx(ctx, fx, 1, { get: () => undefined }, [{ x: 54, y: 48, r: 2, color: "#3DDC97", alpha: 0.6 }]);

    expect(calls.some((call) => call.startsWith("scale:"))).toBe(true);
    expect(calls.some((call) => call === "fill:#3DDC97" || call === "fill:#7ee8d0")).toBe(true);
    expect(calls.some((call) => call.startsWith("alpha:0."))).toBe(true);
  });
});
