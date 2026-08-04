import { describe, expect, it } from "vitest";
import { compareRegion, faceFromScryfall } from "./card-render-diff.mjs";

const SIZE = { w: 4, h: 4 };

function buffer(fill: (x: number, y: number) => [number, number, number]): Uint8ClampedArray {
  const pixels = new Uint8ClampedArray(SIZE.w * SIZE.h * 4);
  for (let y = 0; y < SIZE.h; y += 1) {
    for (let x = 0; x < SIZE.w; x += 1) {
      const [r, g, b] = fill(x, y);
      const i = (y * SIZE.w + x) * 4;
      pixels.set([r, g, b, 255], i);
    }
  }
  return pixels;
}

describe("compareRegion", () => {
  it("scores identical pixels as a perfect match", () => {
    const pixels = buffer((x, y) => [x * 20, y * 20, 7]);

    expect(compareRegion(pixels, pixels, SIZE, { x: 0, y: 0, ...SIZE })).toEqual({
      match: 100,
      near: 100,
      pixels: 16,
    });
  });

  it("counts a shade off as near but not a perfect match", () => {
    const white = buffer(() => [255, 255, 255]);
    const dimmed = buffer(() => [245, 255, 255]);

    const score = compareRegion(white, dimmed, SIZE, { x: 0, y: 0, ...SIZE });

    expect(score.near).toBe(100);
    expect(score.match).toBeCloseTo(100 - (100 * 10) / (3 * 255), 5);
  });

  it("scores black against white as no match at all", () => {
    const white = buffer(() => [255, 255, 255]);
    const black = buffer(() => [0, 0, 0]);

    expect(compareRegion(white, black, SIZE, { x: 0, y: 0, ...SIZE })).toEqual({
      match: 0,
      near: 0,
      pixels: 16,
    });
  });

  it("only reads the rectangle it is given, clipped to the image", () => {
    const white = buffer(() => [255, 255, 255]);
    const half = buffer((x) => (x < 2 ? [255, 255, 255] : [0, 0, 0]));

    // The left half agrees; asking about it must not see the black half, even past the edge.
    expect(compareRegion(white, half, SIZE, { x: 0, y: 0, w: 2, h: 99 })).toEqual({
      match: 100,
      near: 100,
      pixels: 8,
    });
  });
});

describe("faceFromScryfall", () => {
  it("reads a legendary creature printing into renderer inputs", () => {
    const face = faceFromScryfall({
      id: "print-id",
      name: "Kaalia of the Vast",
      type_line: "Legendary Creature — Human Cleric",
      colors: ["W", "B", "R"],
      power: "2",
      toughness: "2",
      oracle_text: "Flying",
      flavor_text: "I'll have my revenge.",
      set: "mh3",
    });

    expect(face).toEqual({
      print: "print-id",
      name: "Kaalia of the Vast",
      colors: [0, 2, 3],
      isLand: false,
      isToken: false,
      legendary: true,
      power: "2",
      toughness: "2",
      loyalty: "",
      typeLine: "Legendary Creature — Human Cleric",
      oracle: "Flying",
      flavor: "I'll have my revenge.",
    });
  });

  it("reads a land printing that prints no flavor", () => {
    const face = faceFromScryfall({ id: "forest", name: "Forest", type_line: "Basic Land — Forest" });

    expect(face.isLand).toBe(true);
    expect(face.legendary).toBe(false);
    expect(face.flavor).toBe("");
  });
});
