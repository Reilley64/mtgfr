import { describe, expect, it } from "vitest";
import { cardArtFaceTag, imageFaceAfterLoadError } from "~/lib/cardArtFace";

describe("cardArtFaceTag", () => {
  it("uses img for empty URLs so CardArt can keep rest props (style, pointer handlers)", () => {
    expect(cardArtFaceTag("")).toBe("img");
  });

  it("uses img for loaded art URLs", () => {
    expect(cardArtFaceTag("https://cdn.example/large/front/a/b/id.webp")).toBe("img");
  });
});

describe("imageFaceAfterLoadError", () => {
  it("falls back from a missing back face to front (prepare/flip DFCs)", () => {
    expect(imageFaceAfterLoadError("back")).toBe("front");
  });

  it("stays on front when front art itself fails (no retry loop)", () => {
    expect(imageFaceAfterLoadError("front")).toBe("front");
  });
});
