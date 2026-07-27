/**
 * @vitest-environment happy-dom
 */

import { describe, expect, it } from "vitest";
import {
  clientToBoardPoint,
  pinchCenter,
  pinchDistance,
  pinchZoomFactor,
  wheelZoomFactor,
} from "./camera-gesture-mount";

function boardHost(): HTMLElement {
  const host = document.createElement("div");
  host.dataset.boardWidth = "1440";
  host.dataset.boardHeight = "900";
  host.getBoundingClientRect = () =>
    ({
      left: 20,
      top: 10,
      width: 720,
      height: 450,
      right: 740,
      bottom: 460,
      x: 20,
      y: 10,
      toJSON: () => ({}),
    }) as DOMRect;
  return host;
}

describe("clientToBoardPoint", () => {
  it("maps browser client coordinates into board-internal coordinates", () => {
    expect(clientToBoardPoint(boardHost(), 380, 235)).toEqual({ x: 720, y: 450 });
  });

  it("returns null outside the board rect", () => {
    expect(clientToBoardPoint(boardHost(), 10, 235)).toBeNull();
    expect(clientToBoardPoint(boardHost(), 380, 470)).toBeNull();
  });
});

describe("wheelZoomFactor", () => {
  it("zooms in for wheel-up and out for wheel-down", () => {
    expect(wheelZoomFactor(-100)).toBeGreaterThan(1);
    expect(wheelZoomFactor(100)).toBeLessThan(1);
    expect(wheelZoomFactor(-100) * wheelZoomFactor(100)).toBeCloseTo(1);
  });
});

describe("pinch helpers", () => {
  it("uses distance ratio as the pinch zoom factor", () => {
    const previous = pinchDistance({ x: 0, y: 0 }, { x: 100, y: 0 });
    const next = pinchDistance({ x: 0, y: 0 }, { x: 150, y: 0 });

    expect(pinchZoomFactor(previous, next)).toBeCloseTo(1.5);
  });

  it("uses the midpoint as the zoom anchor", () => {
    expect(pinchCenter({ x: 20, y: 10 }, { x: 100, y: 70 })).toEqual({ x: 60, y: 40 });
  });
});
