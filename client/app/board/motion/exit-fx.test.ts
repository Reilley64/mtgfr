import { describe, expect, it } from "vitest";
import {
  EXIT_FX_DURATION_MS,
  EXIT_FX_MAX_PARTICLES,
  exitFxParticles,
  particleAllowancePerFx,
  spawnExitFx,
  stepExitFx,
} from "./exit-fx";

describe("exit-fx", () => {
  it("spawnExitFx starts at progress 0", () => {
    const fx = spawnExitFx({
      id: 1,
      print: "abc",
      name: "Bear",
      kind: "destroy",
      x: 100,
      y: 200,
      scale: 1,
      seed: 7,
    });
    expect(fx.progress).toBe(0);
    expect(fx.kind).toBe("destroy");
  });

  it("stepExitFx advances and completes after duration", () => {
    const spawned = spawnExitFx({
      id: 1,
      print: "",
      name: "",
      kind: "exile",
      x: 0,
      y: 0,
      scale: 1,
      seed: 1,
    });
    const mid = stepExitFx(new Map([[1, spawned]]), EXIT_FX_DURATION_MS / 2, false);
    expect(mid.exitFx.get(1)?.progress).toBeGreaterThan(0.4);
    expect(mid.exitFx.get(1)?.progress).toBeLessThan(1);
    expect(mid.active).toBe(true);

    const done = stepExitFx(mid.exitFx, EXIT_FX_DURATION_MS, false);
    expect(done.exitFx.has(1)).toBe(false);
    expect(done.completedIds).toContain(1);
    expect(done.active).toBe(false);
  });

  it("reduced motion completes immediately", () => {
    const spawned = spawnExitFx({
      id: 2,
      print: "",
      name: "",
      kind: "destroy",
      x: 0,
      y: 0,
      scale: 1,
      seed: 2,
    });
    const next = stepExitFx(new Map([[2, spawned]]), 16, true);
    expect(next.exitFx.size).toBe(0);
    expect(next.completedIds).toEqual([2]);
  });

  it("caps particle allowance across many concurrent FX", () => {
    expect(particleAllowancePerFx(1)).toBeGreaterThan(0);
    expect(particleAllowancePerFx(20) * 20).toBeLessThanOrEqual(EXIT_FX_MAX_PARTICLES);
  });

  it("destroy and exile particle palettes differ", () => {
    const destroy = spawnExitFx({
      id: 1,
      print: "",
      name: "",
      kind: "destroy",
      x: 0,
      y: 0,
      scale: 1,
      seed: 3,
    });
    const exile = spawnExitFx({
      id: 2,
      print: "",
      name: "",
      kind: "exile",
      x: 0,
      y: 0,
      scale: 1,
      seed: 3,
    });
    const d = exitFxParticles({ ...destroy, progress: 0.5 }, 8);
    const e = exitFxParticles({ ...exile, progress: 0.5 }, 8);
    expect(d.length).toBeGreaterThan(0);
    expect(e.length).toBeGreaterThan(0);
    expect(d[0]?.color).not.toBe(e[0]?.color);
  });
});
