import { describe, expect, it } from "vitest";
import type { Camera } from "./camera";
import { type CardRect, hitAvatar, hitTest } from "./hit-test";

const identity: Camera = { panX: 0, panY: 0, zoom: 1 };
const cards: CardRect[] = [
  { id: 1, x: 0, y: 0, w: 100, h: 140 },
  { id: 2, x: 50, y: 50, w: 100, h: 140 }, // overlaps card 1, drawn on top
];

describe("hitTest", () => {
  it("returns null when the point misses every card", () => {
    expect(hitTest(identity, 500, 500, cards)).toBeNull();
  });

  it("returns the card under a point that hits only one", () => {
    expect(hitTest(identity, 10, 10, cards)).toBe(1);
  });

  it("returns the topmost (last-drawn) card in an overlap", () => {
    // (60, 60) is inside both card 1 and card 2; card 2 is drawn later.
    expect(hitTest(identity, 60, 60, cards)).toBe(2);
  });

  it("accounts for pan and zoom via the shared camera", () => {
    // Zoom 2x, pan (200,100): world (10,10) draws at screen (220,120).
    const cam: Camera = { panX: 200, panY: 100, zoom: 2 };
    expect(hitTest(cam, 220, 120, cards)).toBe(1);
    // The same screen point misses if the camera isn't applied.
    expect(hitTest(identity, 220, 120, cards)).toBeNull();
  });

  it("keeps a card's footprint where it is drawn, tapped or not", () => {
    // A tapped permanent tilts a few degrees about its center rather than turning a quarter turn,
    // so the upright rect is still where you click it.
    const square: CardRect[] = [{ id: 1, x: 0, y: 0, w: 96, h: 96 }];
    expect(hitTest(identity, 90, 90, square)).toBe(1);
    expect(hitTest(identity, 110, 48, square)).toBeNull();
  });
});

describe("hitAvatar", () => {
  const avatars = { 0: { x: 100, y: 100 }, 1: { x: 300, y: 100 } };

  it("returns null when no avatar is hit", () => {
    expect(hitAvatar(identity, 500, 500, avatars)).toBeNull();
  });

  it("returns the seat of an avatar inside the circle", () => {
    // Avatar 0 at (100, 100) with radius AVATAR_R (40): (100, 100) is the center
    expect(hitAvatar(identity, 100, 100, avatars)).toBe(0);
    // (110, 100) is 10 pixels away, well within the radius
    expect(hitAvatar(identity, 110, 100, avatars)).toBe(0);
    // (130, 100) is 30 pixels away, still within 40
    expect(hitAvatar(identity, 130, 100, avatars)).toBe(0);
  });

  it("returns null when the point is outside the avatar circle", () => {
    // Avatar 0 at (100, 100) with radius 40: (200, 100) is 100 pixels away, outside
    expect(hitAvatar(identity, 200, 100, avatars)).toBeNull();
  });

  it("scales the radius with zoom", () => {
    // Zoom 2x: a screen point that maps to world distance 30 should hit (30 < 40)
    const cam: Camera = { panX: 0, panY: 0, zoom: 2 };
    // Avatar 0 is at world (100, 100). At zoom 2 with no pan, screen (200, 200) = world (100, 100).
    expect(hitAvatar(cam, 200, 200, avatars)).toBe(0);
    // Screen (220, 200) = world (110, 100), which is 10 pixels away in world space, well within 40.
    expect(hitAvatar(cam, 220, 200, avatars)).toBe(0);
    // Screen (290, 200) = world (145, 100), which is 45 pixels away, outside the 40-pixel radius.
    expect(hitAvatar(cam, 290, 200, avatars)).toBeNull();
  });

  it("returns the first avatar hit when multiple overlap (iteration order)", () => {
    // Both avatars are at y=100; avatar 0 at x=100, avatar 1 at x=300, each with radius 40.
    // Screen point (200, 100) is 100 away from avatar 0, outside. 100 away from avatar 1, also outside.
    expect(hitAvatar(identity, 200, 100, avatars)).toBeNull();
    // Screen point (130, 100) is 30 away from avatar 0 (hit), 170 away from avatar 1.
    expect(hitAvatar(identity, 130, 100, avatars)).toBe(0);
  });
});
