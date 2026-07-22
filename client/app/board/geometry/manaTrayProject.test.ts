import { describe, expect, it } from "vitest";
import { emptyManaPool } from "~/manaPips";
import { projectManaTrays } from "./manaTrayProject";

describe("projectManaTrays", () => {
  it("skips empty pools", () => {
    expect(projectManaTrays([{ player: 0, mana_pool: emptyManaPool() }], 0, 2, { panX: 0, panY: 0, zoom: 1 })).toEqual(
      [],
    );
  });

  it("projects world manaTrayPos through the camera", () => {
    const cam = { panX: 10, panY: 20, zoom: 2 };
    const trays = projectManaTrays(
      [{ player: 0, mana_pool: { ...emptyManaPool(), colored: [2, 0, 0, 0, 0] } }],
      0,
      2,
      cam,
    );
    expect(trays).toHaveLength(1);
    expect(trays[0]).toMatchObject({
      seat: 0,
      x: -8 * 2 + 10,
      y: 868 * 2 + 20,
      zoom: 2,
    });
    expect(trays[0].chips).toEqual([{ kind: "glyph", ms: "w", code: "W", amount: 2 }]);
  });
});
