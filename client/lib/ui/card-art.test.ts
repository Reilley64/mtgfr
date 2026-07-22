import { describe, expect, it, vi } from "vitest";
import { ImageCache } from "../image-cache";
import { cardArtUrl } from "./card-art";

describe("cardArtUrl", () => {
  it("uses card back when print is empty", () => {
    expect(cardArtUrl("")).toMatch(/card-back/);
  });
  it("defaults to large front", () => {
    expect(cardArtUrl("abcd-print")).toContain("abcd-print");
  });
});

describe("ImageCache readiness for DOM art", () => {
  it("becomes ready after onload", async () => {
    let img!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    const cache = new ImageCache(
      () => {},
      () => {
        img = { src: "", onload: null, onerror: null };
        return img;
      },
    );
    expect(cache.get("https://example.test/a.webp")).toBeUndefined();
    expect(cache.isReady("https://example.test/a.webp")).toBe(false);
    img.onload?.();
    await vi.waitFor(() => expect(cache.isReady("https://example.test/a.webp")).toBe(true));
  });
});
