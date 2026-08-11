import { describe, expect, it, vi } from "vitest";
import { BLANK_FACE } from "../../domain/card-render/frame";
import { LIFT_SHADOW_BLUR, LIFT_SHADOW_COLOR, LIFT_SHADOW_OFFSET_Y } from "../lift-shadow";
import type { CardFlight } from "../motion/flights";
import type { FaceSource } from "./paint-cards";
import { FLIGHT_SHADOW_BLUR, FLIGHT_SHADOW_COLOR, FLIGHT_SHADOW_OFFSET_Y, paintFlightCard } from "./paint-flights";

describe("flight lift shadow", () => {
  it("uses the shared drag-token lift constants", () => {
    expect(FLIGHT_SHADOW_BLUR).toBe(LIFT_SHADOW_BLUR);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(LIFT_SHADOW_OFFSET_Y);
    expect(FLIGHT_SHADOW_COLOR).toBe(LIFT_SHADOW_COLOR);
    expect(FLIGHT_SHADOW_BLUR).toBe(36);
    expect(FLIGHT_SHADOW_OFFSET_Y).toBe(16);
    expect(FLIGHT_SHADOW_COLOR).toBe("rgba(0,0,0,0.72)");
  });
});

function mockCtx(): CanvasRenderingContext2D {
  return {
    beginPath: vi.fn(),
    clip: vi.fn(),
    drawImage: vi.fn(),
    fill: vi.fn(),
    fillText: vi.fn(),
    restore: vi.fn(),
    roundRect: vi.fn(),
    save: vi.fn(),
    stroke: vi.fn(),
  } as unknown as CanvasRenderingContext2D;
}

function flight(): CardFlight {
  return {
    id: 1,
    print: "print-id",
    name: "Grizzly Bears",
    face: BLANK_FACE,
    x: 100,
    y: 100,
    scale: 1,
    targetX: 100,
    targetY: 100,
    targetScale: 1,
    phase: "settled",
    kind: "stack",
  };
}

describe("flight rendered face", () => {
  it("paints a face that a warm cache draws synchronously on request", () => {
    const rendered = { width: 745, height: 1040 } as unknown as CanvasImageSource;
    const printed = { width: 745, height: 1040 } as unknown as HTMLImageElement;
    let ready = false;
    const faces: FaceSource = {
      get: () => (ready ? rendered : undefined),
      request: () => {
        ready = true;
      },
    };
    const ctx = mockCtx();

    paintFlightCard(ctx, flight(), 1, { get: () => printed }, faces);

    expect(vi.mocked(ctx.drawImage).mock.calls[0]?.[0]).toBe(rendered);
  });
});
