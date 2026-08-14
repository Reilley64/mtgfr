export const PERMANENT_SIDE = 96;
export const MIN_CROWDED_PERMANENT_SIDE = 24;
export const PERMANENT_TAP_TILT = Math.PI / 12;
export const PERMANENT_CLEARANCE = 8;
export const PERMANENT_MAX_CHROME_OUTSET = 3;

const TILT_AXIS_FACTOR = Math.abs(Math.cos(PERMANENT_TAP_TILT)) + Math.abs(Math.sin(PERMANENT_TAP_TILT));

export function tiltedPermanentExtent(side: number): number {
  return (side + 2 * PERMANENT_MAX_CHROME_OUTSET) * TILT_AXIS_FACTOR;
}

export const PERMANENT_STEP = Math.ceil(tiltedPermanentExtent(PERMANENT_SIDE) + PERMANENT_CLEARANCE);

export type PermanentRowMetrics = Readonly<{ side: number; step: number; width: number }>;

export function permanentRowMetrics(slotCount: number, rowWidth: number): PermanentRowMetrics {
  if (slotCount <= 1) return { side: PERMANENT_SIDE, step: PERMANENT_STEP, width: rowWidth };
  if (slotCount <= 7) return { side: PERMANENT_SIDE, step: PERMANENT_STEP, width: rowWidth };

  const gaps = slotCount - 1;
  const fittedSide = Math.min(
    PERMANENT_SIDE,
    (rowWidth - gaps * PERMANENT_CLEARANCE) / (slotCount * TILT_AXIS_FACTOR) - 2 * PERMANENT_MAX_CHROME_OUTSET,
  );
  if (fittedSide >= MIN_CROWDED_PERMANENT_SIDE) {
    return {
      side: fittedSide,
      step: tiltedPermanentExtent(fittedSide) + PERMANENT_CLEARANCE,
      width: rowWidth,
    };
  }

  const side = MIN_CROWDED_PERMANENT_SIDE;
  const step = tiltedPermanentExtent(side) + PERMANENT_CLEARANCE;
  return { side, step, width: gaps * step + tiltedPermanentExtent(side) };
}
