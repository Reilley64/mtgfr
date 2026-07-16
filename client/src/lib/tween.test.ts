import { describe, expect, it } from "vitest";
import { type Positions, snapAll, stepScalar, stepToward } from "~/lib/tween";

const card = (id: number, x: number, y: number) => ({ id, x, y });

function expectPos(pos: Positions, id: number) {
  const p = pos.get(id);
  expect(p).toBeDefined();
  if (!p) throw new Error(`expected position for ${id}`);
  return p;
}

describe("position tween", () => {
  it("moves toward the target each step without overshooting", () => {
    let pos: Positions = new Map([[1, { x: 0, y: 0 }]]);
    let dist = Math.hypot(100, 50);
    for (let i = 0; i < 5; i++) {
      pos = stepToward(pos, [card(1, 100, 50)], 16).positions;
      const p = expectPos(pos, 1);
      const d = Math.hypot(100 - p.x, 50 - p.y);
      expect(d).toBeLessThan(dist); // strictly closer every frame
      dist = d;
    }
  });

  it("settles exactly on the target within a bounded time", () => {
    let pos: Positions = new Map([[1, { x: 0, y: 0 }]]);
    let settled = false;
    let elapsed = 0;
    while (!settled) {
      const r = stepToward(pos, [card(1, 400, 0)], 16);
      pos = r.positions;
      settled = r.settled;
      elapsed += 16;
      expect(elapsed).toBeLessThanOrEqual(1000); // must not glide forever
    }
    expect(pos.get(1)).toEqual({ x: 400, y: 0 });
  });

  it("new ids enter from below their slot and glide up to the target", () => {
    // A card appearing on the canvas starts below its slot (same x, larger y) and is unsettled.
    const first = stepToward(new Map(), [card(7, 300, 200)], 16);
    const p0 = expectPos(first.positions, 7);
    expect(p0.x).toBe(300);
    expect(p0.y).toBeGreaterThan(200); // starts below the slot
    expect(first.settled).toBe(false);
    // And it converges to the target over subsequent frames.
    let pos = first.positions;
    let settled = false;
    for (let i = 0; i < 100 && !settled; i++) {
      const r = stepToward(pos, [card(7, 300, 200)], 16);
      pos = r.positions;
      settled = r.settled;
    }
    expect(pos.get(7)).toEqual({ x: 300, y: 200 });
  });

  it("drops ids that are no longer in the layout", () => {
    const prev: Positions = new Map([
      [1, { x: 0, y: 0 }],
      [2, { x: 9, y: 9 }],
    ]);
    const r = stepToward(prev, [card(1, 0, 0)], 16);
    expect(r.positions.has(2)).toBe(false);
  });

  it("stepScalar eases the tap fraction toward its target and settles", () => {
    // Untapping: fraction 1 → 0. Monotonically decreases, lands exactly on 0.
    let vals = new Map([[1, 1]]);
    let settled = false;
    let prev = 1;
    for (let i = 0; i < 200 && !settled; i++) {
      const r = stepScalar(vals, new Map([[1, 0]]), 16);
      vals = r.values;
      settled = r.settled;
      const v = vals.get(1);
      expect(v).toBeDefined();
      if (v === undefined) throw new Error("expected tap fraction");
      expect(v).toBeLessThanOrEqual(prev);
      prev = v;
    }
    expect(vals.get(1)).toBe(0);
  });

  it("stepScalar: a newly-seen id starts at its target (a land entering tapped doesn't spin)", () => {
    const r = stepScalar(new Map(), new Map([[5, 1]]), 16);
    expect(r.values.get(5)).toBe(1);
    expect(r.settled).toBe(true);
  });

  it("snapAll is the instant path: everything at target in one step", () => {
    const pos = snapAll([card(1, 10, 20), card(2, 30, 40)]);
    expect(pos.get(1)).toEqual({ x: 10, y: 20 });
    expect(pos.get(2)).toEqual({ x: 30, y: 40 });
    // And a subsequent step from there is already settled.
    expect(stepToward(pos, [card(1, 10, 20), card(2, 30, 40)], 16).settled).toBe(true);
  });
});
