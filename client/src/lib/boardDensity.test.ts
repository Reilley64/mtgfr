import { describe, expect, it } from "vitest";
import type { RenderCard } from "~/layout";
import { CARD_H, CARD_W, seatBand, ZONE } from "~/layout";
import { withBoardDensity, withClusterFan, withHoverRaise } from "~/lib/boardDensity";

const card = (over: Partial<RenderCard> = {}): RenderCard => ({
  id: 1,
  x: 0,
  y: 0,
  w: CARD_W,
  h: CARD_H,
  name: "c",
  pt: "",
  tapped: false,
  counters: 0,
  markedDamage: 0,
  faceDown: false,
  zone: ZONE.Battlefield,
  controller: 0,
  owner: 0,
  kind: "creature",
  tapsForMana: false,
  summoningSick: false,
  hasHaste: false,
  keywords: [],
  goaded: false,
  isCommander: false,
  prepared: false,
  pile: 0,
  cluster: 0,
  clusterMembers: [],
  attachedTo: null,
  ...over,
});

describe("withHoverRaise", () => {
  it("moves the hovered card to the end (top for paint/hit)", () => {
    const cards = [card({ id: 1, x: 0 }), card({ id: 2, x: 112 }), card({ id: 3, x: 224 })];
    expect(withHoverRaise(cards, 2).map((c) => c.id)).toEqual([1, 3, 2]);
  });

  it("raises an attachment stack with its host via attachedTo", () => {
    const host = card({ id: 10, x: 200, y: 100 });
    const aura = card({ id: 11, x: 200, y: 100 - 26.8, kind: "enchantment", attachedTo: 10 });
    const other = card({ id: 12, x: 400, y: 100 });
    const cards = [aura, host, other];
    expect(withHoverRaise(cards, 10).map((c) => c.id)).toEqual([12, 11, 10]);
  });

  it("does not raise a same-x neighbour that is not an attachment", () => {
    // Packed creature at x=0 next to a noncreature also at x=0 — must not ride along.
    const ring = card({ id: 1, x: 0, y: 0, kind: "artifact" });
    const bear = card({ id: 2, x: 0, y: 150, kind: "creature" });
    expect(withHoverRaise([ring, bear], 2).map((c) => c.id)).toEqual([1, 2]);
    expect(withHoverRaise([ring, bear], 1).map((c) => c.id)).toEqual([2, 1]);
  });

  it("is a no-op when hoverId is null or missing", () => {
    const cards = [card({ id: 1 }), card({ id: 2 })];
    expect(withHoverRaise(cards, null)).toEqual(cards);
    expect(withHoverRaise(cards, 99).map((c) => c.id)).toEqual([1, 2]);
  });
});

describe("withClusterFan", () => {
  it("replaces a cluster face with one card per member, fanned in an arc above the rest", () => {
    const cluster = card({
      id: 10,
      x: 400,
      y: 100,
      cluster: 3,
      clusterMembers: [10, 11, 12],
      name: "Saproling",
    });
    const before = card({ id: 1, x: 0, y: 100 });
    const after = card({ id: 2, x: 800, y: 100 });
    const out = withClusterFan([before, cluster, after], 10);
    expect(out.map((c) => c.id)).toEqual([1, 2, 10, 11, 12]);
    const fan = out.slice(2);
    expect(fan.every((c) => c.cluster === 0)).toBe(true);
    expect(fan[1].x).toBeGreaterThan(fan[0].x);
    expect(fan[2].x).toBeGreaterThan(fan[1].x);
    const mid = (fan[0].x + fan[2].x) / 2;
    expect(mid).toBeCloseTo(cluster.x, 5);
    // Arc: outer cards sink and tilt; the middle stays flat at the slot y.
    expect(fan[1].y).toBe(cluster.y);
    expect(fan[1].fanAngle ?? 0).toBe(0);
    expect(fan[0].y).toBeGreaterThan(cluster.y);
    expect(fan[2].y).toBeGreaterThan(cluster.y);
    expect(fan[0].fanAngle!).toBeLessThan(0);
    expect(fan[2].fanAngle!).toBeGreaterThan(0);
  });

  it("places an outer fan member away from the cluster slot center", () => {
    const cluster = card({
      id: 10,
      x: 400,
      y: 100,
      cluster: 3,
      clusterMembers: [10, 11, 12],
    });
    // Middle index stays on the slot x; an outer member shifts horizontally.
    const outer = withClusterFan([cluster], 10).find((c) => c.id === 12)!;
    expect(outer.x + outer.w / 2).not.toBeCloseTo(cluster.x + cluster.w / 2);
  });

  it("clamps the fan inside the controller's seat band", () => {
    const band = seatBand(0, 0, 2);
    // Park the cluster at the left edge so an unclamped fan would spill past band.x.
    const cluster = card({
      id: 10,
      x: band.x,
      y: band.y + 40,
      controller: 0,
      cluster: 5,
      clusterMembers: [10, 11, 12, 13, 14],
    });
    const fan = withClusterFan([cluster], 10, { viewer: 0, playerCount: 2 });
    for (const c of fan) {
      expect(c.x).toBeGreaterThanOrEqual(band.x - 0.01);
      expect(c.x + c.w).toBeLessThanOrEqual(band.x + band.w + 0.01);
      expect(c.y).toBeGreaterThanOrEqual(band.y - 0.01);
      expect(c.y + c.h).toBeLessThanOrEqual(band.y + band.h + 0.01);
    }
  });

  it("is a no-op when the id is not a cluster", () => {
    const cards = [card({ id: 1, cluster: 0 })];
    expect(withClusterFan(cards, 1)).toEqual(cards);
  });
});

describe("withBoardDensity", () => {
  it("raises the selected id over hover so a picked fan card stays in front", () => {
    const cluster = card({
      id: 10,
      x: 400,
      y: 100,
      cluster: 3,
      clusterMembers: [10, 11, 12],
    });
    const out = withBoardDensity([cluster], {
      hoverId: 10,
      fannedClusterId: 10,
      raiseId: 11,
    });
    expect(out.map((c) => c.id)).toEqual([10, 12, 11]);
  });

  it("lifts the whole fan above other permanents, with the hovered member on top", () => {
    const cluster = card({
      id: 10,
      x: 400,
      y: 100,
      cluster: 3,
      clusterMembers: [10, 11, 12],
    });
    const neighbour = card({ id: 99, x: 600, y: 100 });
    const out = withBoardDensity([cluster, neighbour], {
      hoverId: 11,
      fannedClusterId: 10,
    });
    expect(out.map((c) => c.id)).toEqual([99, 10, 12, 11]);
  });

  it("lifts the whole fan above other permanents even with no member hover", () => {
    const cluster = card({
      id: 10,
      x: 400,
      y: 100,
      cluster: 3,
      clusterMembers: [10, 11, 12],
    });
    const neighbour = card({ id: 99, x: 600, y: 100 });
    const out = withBoardDensity([cluster, neighbour], {
      hoverId: null,
      fannedClusterId: 10,
    });
    expect(out.map((c) => c.id)).toEqual([99, 10, 11, 12]);
  });
});
