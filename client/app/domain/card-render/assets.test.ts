import { existsSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { FRAME_ASSETS, frameAssetUrl, loadCardFonts } from "./assets";

const PUBLIC_DIR = join(import.meta.dirname, "../../../public");

describe("card frame assets", () => {
  it("every manifest entry resolves to a vendored file", () => {
    const missing = Object.entries(FRAME_ASSETS)
      .filter(([, url]) => !existsSync(join(PUBLIC_DIR, url)))
      .map(([name]) => name);
    expect(missing).toEqual([]);
  });

  it("names a frame for every colour, plus multicolour, colourless, and land", () => {
    for (const key of ["w", "u", "b", "r", "g", "m", "c", "land"]) {
      expect(frameAssetUrl(`m15/${key}`)).toContain(key);
    }
  });

  it("gives a legendary land a crown but no power/toughness box", () => {
    expect(frameAssetUrl("m15/crown/land")).toBeTruthy();
    expect(() => frameAssetUrl("m15/pt/land")).toThrow();
  });

  it("ships both typefaces", () => {
    for (const font of ["/card-fonts/beleren-bold.ttf", "/card-fonts/mplantin.ttf"]) {
      expect(existsSync(join(PUBLIC_DIR, font))).toBe(true);
    }
  });

  it("resolves fonts without a FontFace implementation", async () => {
    await expect(loadCardFonts()).resolves.toBeUndefined();
  });
});
