import { describe, expect, it } from "vitest";
import { ImageCache } from "./image-cache";

async function waitUntil(predicate: () => boolean, timeoutMs = 1000): Promise<void> {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error("waitUntil timed out");
    await new Promise((r) => setTimeout(r, 5));
  }
}

describe("ImageCache failures", () => {
  it("marks url failed and notifies subscribers on onerror", async () => {
    let img!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    let ticks = 0;
    const cache = new ImageCache(
      () => {
        ticks += 1;
      },
      () => {
        img = { src: "", onload: null, onerror: null };
        return img;
      },
    );
    let subTicks = 0;
    cache.subscribe(() => {
      subTicks += 1;
    });

    cache.get("https://example.test/missing.webp");
    expect(cache.isFailed("https://example.test/missing.webp")).toBe(false);
    img.onerror?.();
    await waitUntil(() => cache.isFailed("https://example.test/missing.webp"));
    expect(cache.isReady("https://example.test/missing.webp")).toBe(false);
    expect(ticks).toBeGreaterThan(0);
    expect(subTicks).toBeGreaterThan(0);
  });
});
