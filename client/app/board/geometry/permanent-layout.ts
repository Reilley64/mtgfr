export const PERMANENT_SIDE = 96;
export const PERMANENT_TAP_TILT = Math.PI / 12;
export const PERMANENT_CLEARANCE = 8;
export const PERMANENT_MAX_CHROME_OUTSET = 3;

const TILT_AXIS_FACTOR = Math.abs(Math.cos(PERMANENT_TAP_TILT)) + Math.abs(Math.sin(PERMANENT_TAP_TILT));

export function tiltedPermanentExtent(side: number): number {
  return (side + 2 * PERMANENT_MAX_CHROME_OUTSET) * TILT_AXIS_FACTOR;
}

export const PERMANENT_STEP = Math.ceil(tiltedPermanentExtent(PERMANENT_SIDE) + PERMANENT_CLEARANCE);

export type PermanentRowMetrics = Readonly<{ side: number; step: number }>;

export function permanentRowMetrics(slotCount: number, rowWidth: number): PermanentRowMetrics {
  if (slotCount <= 1) return { side: PERMANENT_SIDE, step: PERMANENT_STEP };
  if (slotCount <= 7) return { side: PERMANENT_SIDE, step: PERMANENT_STEP };

  const gaps = slotCount - 1;
  const numerator = rowWidth - gaps * (2 * PERMANENT_MAX_CHROME_OUTSET * TILT_AXIS_FACTOR + PERMANENT_CLEARANCE);
  const side = Math.min(PERMANENT_SIDE, Math.max(1, numerator / (1 + gaps * TILT_AXIS_FACTOR)));
  return { side, step: (rowWidth - side) / gaps };
}
