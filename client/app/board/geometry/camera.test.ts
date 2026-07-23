import { describe, expect, it } from "vitest";
import { type Camera, panBy, screenToWorld, worldToScreen, zoomAt } from "./camera";

const identity: Camera = { panX: 0, panY: 0, zoom: 1 };

describe("camera transform", () => {
  it("screenToWorld inverts worldToScreen", () => {
    const cam: Camera = { panX: 40, panY: -25, zoom: 1.5 };
    const screen = worldToScreen(cam, 12, 8);
    const world = screenToWorld(cam, screen.x, screen.y);
    expect(world.x).toBeCloseTo(12);
    expect(world.y).toBeCloseTo(8);
  });

  it("worldToScreen applies zoom then pan (independent hand calc)", () => {
    const cam: Camera = { panX: 100, panY: 50, zoom: 2 };
    // world (10, 5) -> 10*2+100 = 120, 5*2+50 = 60
    expect(worldToScreen(cam, 10, 5)).toEqual({ x: 120, y: 60 });
  });

  it("panBy shifts by a screen delta without touching zoom", () => {
    const moved = panBy({ panX: 10, panY: 10, zoom: 3 }, 5, -7);
    expect(moved).toEqual({ panX: 15, panY: 3, zoom: 3 });
  });

  it("zoomAt keeps the world point under the cursor fixed", () => {
    const before = screenToWorld(identity, 300, 200);
    const zoomed = zoomAt(identity, 300, 200, 1.5);
    const after = screenToWorld(zoomed, 300, 200);
    expect(zoomed.zoom).toBeCloseTo(1.5);
    expect(after.x).toBeCloseTo(before.x);
    expect(after.y).toBeCloseTo(before.y);
  });

  it("clamps zoom to the allowed range", () => {
    expect(zoomAt(identity, 0, 0, 100).zoom).toBe(5);
    expect(zoomAt(identity, 0, 0, 0.001).zoom).toBe(0.2);
  });
});
