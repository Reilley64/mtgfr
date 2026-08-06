import { describe, expect, it } from "vitest";
import {
  MIN_CROWDED_PERMANENT_SIDE,
  PERMANENT_CLEARANCE,
  PERMANENT_SIDE,
  PERMANENT_STEP,
  permanentRowMetrics,
  tiltedPermanentExtent,
} from "./permanent-layout";

describe("permanent row metrics", () => {
  it("keeps eight world units around full-size maximum-tilt paint", () => {
    expect(PERMANENT_STEP - tiltedPermanentExtent(PERMANENT_SIDE)).toBeGreaterThanOrEqual(PERMANENT_CLEARANCE);
  });

  it("keeps seven slots full size", () => {
    const rowWidth = 6 * PERMANENT_STEP + PERMANENT_SIDE;
    expect(permanentRowMetrics(7, rowWidth)).toEqual({
      side: PERMANENT_SIDE,
      step: PERMANENT_STEP,
      width: rowWidth,
    });
  });

  it("shrinks an overcrowded row while retaining maximum-tilt clearance", () => {
    const rowWidth = 6 * PERMANENT_STEP + PERMANENT_SIDE;
    const metrics = permanentRowMetrics(12, rowWidth);
    expect(metrics.side).toBeLessThan(PERMANENT_SIDE);
    expect(11 * metrics.step + metrics.side).toBeCloseTo(rowWidth);
    expect(metrics.step - tiltedPermanentExtent(metrics.side)).toBeGreaterThanOrEqual(PERMANENT_CLEARANCE - 1e-9);
  });

  it("widens at the readable size floor instead of reducing clearance", () => {
    const rowWidth = 6 * PERMANENT_STEP + PERMANENT_SIDE;
    const metrics = permanentRowMetrics(59, rowWidth);
    expect(metrics.side).toBe(MIN_CROWDED_PERMANENT_SIDE);
    expect(metrics.width).toBeGreaterThan(rowWidth);
    expect(metrics.step - tiltedPermanentExtent(metrics.side)).toBeGreaterThanOrEqual(PERMANENT_CLEARANCE);
    expect(58 * metrics.step + metrics.side).toBeCloseTo(metrics.width);
  });
});
