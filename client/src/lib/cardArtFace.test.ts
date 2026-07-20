import { describe, expect, it } from "vitest";
import { cardArtFaceTag } from "~/lib/cardArtFace";

describe("cardArtFaceTag", () => {
  it("uses img for empty URLs so CardArt can keep rest props (style, pointer handlers)", () => {
    expect(cardArtFaceTag("")).toBe("img");
  });

  it("uses img for loaded art URLs", () => {
    expect(cardArtFaceTag("https://cdn.example/large/front/a/b/id.webp")).toBe("img");
  });
});
