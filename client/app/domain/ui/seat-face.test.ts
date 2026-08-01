import { describe, expect, it } from "vitest";
import { testHtml } from "~/test-html";
import { seatFace } from "./seat-face";

const h = testHtml<never>();

describe("seatFace", () => {
  it("uses the zero-based seat number for empty-username monograms", () => {
    const face = seatFace(h, { seat: 2, username: "", gravatarHash: "" });

    expect(JSON.stringify(face)).toContain('"text":"2"');
    expect(JSON.stringify(face)).not.toContain('"text":"3"');
  });
});
