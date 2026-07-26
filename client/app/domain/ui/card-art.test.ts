/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it } from "vitest";
import { ImageCache } from "../image-cache";
import { cardArtUrl, syncCardArtHost } from "./card-art";

async function waitUntil(predicate: () => boolean, timeoutMs = 1000): Promise<void> {
  const start = Date.now();
  while (!predicate()) {
    if (Date.now() - start > timeoutMs) throw new Error("waitUntil timed out");
    await new Promise((r) => setTimeout(r, 5));
  }
}

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
    await waitUntil(() => cache.isReady("https://example.test/a.webp"));
  });
});

describe("syncCardArtHost", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("repaints when data-art-url changes (hover preview print swap)", async () => {
    let lastImg!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    const cache = new ImageCache(
      () => {},
      () => {
        lastImg = { src: "", onload: null, onerror: null };
        return lastImg;
      },
    );

    const host = document.createElement("div");
    host.dataset.artUrl = "https://example.test/a.webp";
    host.dataset.artAlt = "A";
    host.dataset.artClass = "art";
    document.body.append(host);

    syncCardArtHost(host, cache);
    expect(host.querySelector("[aria-hidden='true']")).toBeTruthy();
    lastImg.onload?.();
    await waitUntil(() => cache.isReady("https://example.test/a.webp"));
    syncCardArtHost(host, cache);
    expect(host.querySelector("img")?.getAttribute("src")).toBe("https://example.test/a.webp");

    host.dataset.artUrl = "https://example.test/b.webp";
    host.dataset.artAlt = "B";
    syncCardArtHost(host, cache);
    expect(host.querySelector("[aria-hidden='true']")).toBeTruthy();
    lastImg.onload?.();
    await waitUntil(() => cache.isReady("https://example.test/b.webp"));
    syncCardArtHost(host, cache);
    expect(host.querySelector("img")?.getAttribute("src")).toBe("https://example.test/b.webp");
    expect(host.querySelector("img")?.getAttribute("alt")).toBe("B");
  });
});

describe("syncCardArtHost art_crop CDN fallback", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("swaps to data-art-fallback after primary load failure", async () => {
    let lastImg!: { src: string; onload: (() => void) | null; onerror: (() => void) | null };
    const cache = new ImageCache(
      () => {},
      () => {
        lastImg = { src: "", onload: null, onerror: null };
        return lastImg;
      },
    );

    const host = document.createElement("div");
    host.dataset.artUrl = "https://cards.example.com/art_crop/front/a/b/abcd.webp";
    host.dataset.artFallback = "https://api.scryfall.com/cards/abcd?format=image&version=art_crop";
    host.dataset.artAlt = "Commander";
    host.dataset.artClass = "art";
    document.body.append(host);

    syncCardArtHost(host, cache);
    expect(host.querySelector("[aria-hidden='true']")).toBeTruthy();
    lastImg.onerror?.();
    await waitUntil(
      () => cache.isFailed(host.dataset.artUrl ?? "nope") || host.dataset.artUrl?.includes("scryfall") === true,
    );
    syncCardArtHost(host, cache);
    expect(host.dataset.artUrl).toContain("api.scryfall.com");
    expect(host.dataset.artFallback ?? "").toBe("");
    lastImg.onload?.();
    await waitUntil(() => cache.isReady(host.dataset.artUrl ?? ""));
    syncCardArtHost(host, cache);
    expect(host.querySelector("img")?.getAttribute("src")).toContain("api.scryfall.com");
  });
});
