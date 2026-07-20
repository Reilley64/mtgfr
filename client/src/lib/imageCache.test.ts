// No DOM in node — the injected `makeImage` factory stands in for `new Image()`.

import { describe, expect, it } from "vitest";
import { ImageCache } from "~/lib/imageCache";

interface FakeImage {
  crossOrigin: string;
  src: string;
  onload: (() => void) | null;
  onerror: (() => void) | null;
}

function fakeImageFactory(): { make: () => FakeImage; images: FakeImage[] } {
  const images: FakeImage[] = [];
  const make = (): FakeImage => {
    const img: FakeImage = { crossOrigin: "", src: "", onload: null, onerror: null };
    images.push(img);
    return img;
  };
  return { make, images };
}

// The load resolves on a real fiber inside the shared runtime, not synchronously inside
// `onload()` — give it a tick to run before asserting.
const flush = (): Promise<void> => new Promise((resolve) => setTimeout(resolve, 0));

describe("ImageCache", () => {
  it("is undefined before load, the element after onload fires, onLoad called exactly once", async () => {
    const { make, images } = fakeImageFactory();
    let onLoadCalls = 0;
    const cache = new ImageCache(() => onLoadCalls++, make);

    expect(cache.get("a.png")).toBeUndefined();

    images[0].onload?.();
    await flush();

    expect(cache.get("a.png")).toBe(images[0]);
    expect(onLoadCalls).toBe(1);
  });

  it("dedupes: two get()s for the same URL create one image", () => {
    const { make, images } = fakeImageFactory();
    const cache = new ImageCache(() => {}, make);

    cache.get("a.png");
    cache.get("a.png");

    expect(images.length).toBe(1);
  });

  it("onerror: get() keeps returning undefined, no throw, no onLoad call", async () => {
    const { make, images } = fakeImageFactory();
    let onLoadCalls = 0;
    const cache = new ImageCache(() => onLoadCalls++, make);

    cache.get("a.png");
    expect(() => images[0].onerror?.()).not.toThrow();
    await flush();

    expect(cache.get("a.png")).toBeUndefined();
    expect(onLoadCalls).toBe(0);
  });

  it("preload starts a load for each non-empty URL and dedupes with get", () => {
    const { make, images } = fakeImageFactory();
    const cache = new ImageCache(() => {}, make);

    cache.preload(["a.png", "", "b.png", "a.png"]);

    expect(images.map((img) => img.src)).toEqual(["a.png", "b.png"]);
  });

  it("subscribe listeners fire once per successful load", async () => {
    const { make, images } = fakeImageFactory();
    const cache = new ImageCache(() => {}, make);
    let hits = 0;
    const unsub = cache.subscribe(() => {
      hits += 1;
    });

    cache.get("a.png");
    images[0].onload?.();
    await flush();
    expect(hits).toBe(1);

    unsub();
    cache.get("b.png");
    images[1].onload?.();
    await flush();
    expect(hits).toBe(1);
  });
});
