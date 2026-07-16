import { describe, expect, it } from "vitest";
import { imageUrlByPrint } from "~/lib/scryfall";

describe("imageUrlByPrint", () => {
  it("returns empty for an empty print id", () => {
    expect(imageUrlByPrint("")).toBe("");
  });

  it("requests the back face from Scryfall when face is back (no CDN)", () => {
    const url = imageUrlByPrint("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "large", "back");
    expect(url).toContain("face=back");
    expect(url).toContain("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
  });

  it("omits face=back on the front face", () => {
    const url = imageUrlByPrint("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee", "large", "front");
    expect(url).not.toContain("face=back");
  });
});
