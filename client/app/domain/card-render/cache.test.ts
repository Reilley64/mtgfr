import { describe, expect, it, vi } from "vitest";
import { CardFaceCache, faceKey } from "./cache";
import type { FaceData } from "./frame";

function face(overrides: Partial<FaceData> = {}): FaceData {
  return {
    print: "abcdef01-2345-6789-abcd-ef0123456789",
    name: "Llanowar Elves",
    colors: [4],
    isLand: false,
    isToken: false,
    legendary: false,
    power: "1",
    toughness: "1",
    loyalty: "",
    typeLine: "",
    oracle: "",
    flavor: "",
    ...overrides,
  };
}

/** Stands in for `ImageCache`: every url is instantly ready and nothing ever fails. */
function readyImages() {
  return {
    get: vi.fn((_url: string) => ({ width: 626, height: 457 }) as unknown as HTMLImageElement | undefined),
    preload: vi.fn((_urls: Iterable<string>) => {}),
    isFailed: vi.fn((_url: string) => false),
    subscribe: vi.fn((_fn: () => void) => () => {}),
  };
}

/** Records each canvas handed out, so a test can count how many times a face was drawn. */
function stubCanvas() {
  const drawn: unknown[] = [];
  const make = vi.fn((w: number, h: number) => {
    drawn.push({ w, h });
    return {
      width: w,
      height: h,
      getContext: () => ({
        canvas: { width: w, height: h },
        save: () => {},
        restore: () => {},
        beginPath: () => {},
        translate: () => {},
        rotate: () => {},
        rect: () => {},
        clip: () => {},
        drawImage: () => {},
        fillText: () => {},
        measureText: () => ({ width: 10 }),
      }),
    } as unknown as OffscreenCanvas;
  });
  return { drawn, make };
}

describe("CardFaceCache", () => {
  it("serves the drawn face and notifies subscribers when it is ready", () => {
    const { make } = stubCanvas();
    const cache = new CardFaceCache(readyImages(), make);
    const listener = vi.fn();
    cache.subscribe(listener);

    expect(cache.get(face(), "permanent")).toBeUndefined();
    cache.request(face(), "permanent");

    expect(cache.get(face(), "permanent")).toBeDefined();
    expect(listener).toHaveBeenCalled();
  });

  it("draws a face once however many times it is requested", () => {
    const { drawn, make } = stubCanvas();
    const cache = new CardFaceCache(readyImages(), make);

    cache.request(face(), "permanent");
    cache.request(face(), "permanent");
    cache.request(face(), "permanent");

    expect(drawn).toHaveLength(1);
  });

  it("evicts old full faces before their bitmap bytes exceed the cache budget", () => {
    const cache = new CardFaceCache(readyImages(), stubCanvas().make);
    const faces = Array.from({ length: 33 }, (_, index) => face({ print: `print-${index}`, name: `Card ${index}` }));

    for (const card of faces) cache.request(card, "full");

    // 33 × 745 × 1040 × 4 bytes is just over a 96 MiB budget. A count-only cap of 240 keeps
    // every one and can grow to roughly 740 MiB before evicting anything.
    expect(cache.get(faces[0], "full")).toBeUndefined();
    const last = faces.at(-1);
    if (last == null) throw new Error("expected a final face fixture");
    expect(cache.get(last, "full")).toBeDefined();
  });

  it("keeps a busy four-seat board of permanent faces inside the bitmap budget", () => {
    const cache = new CardFaceCache(readyImages(), stubCanvas().make);
    const permanents = Array.from({ length: 90 }, (_, index) =>
      face({ print: `permanent-${index}`, name: `Permanent ${index}` }),
    );

    for (const permanent of permanents) cache.request(permanent, "permanent");

    expect(cache.get(permanents[0], "permanent")).toBeDefined();
  });

  it("redraws when the creature's power changes", () => {
    const { drawn, make } = stubCanvas();
    const cache = new CardFaceCache(readyImages(), make);

    cache.request(face(), "permanent");
    cache.request(face({ power: "3", toughness: "3" }), "permanent");

    expect(drawn).toHaveLength(2);
  });

  it("holds the face back until the frame asset has loaded, then draws on the image callback", () => {
    const images = readyImages();
    let notify = () => {};
    images.subscribe.mockImplementation((fn: () => void) => {
      notify = fn;
      return () => {};
    });
    images.get.mockReturnValue(undefined);
    const { make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");
    expect(cache.get(face(), "permanent")).toBeUndefined();

    images.get.mockReturnValue({ width: 626, height: 457 } as unknown as HTMLImageElement);
    notify();

    expect(cache.get(face(), "permanent")).toBeDefined();
  });

  it("holds the face back until the P/T plate lands — only the art earns a redraw", () => {
    const images = readyImages();
    let notify = () => {};
    images.subscribe.mockImplementation((fn: () => void) => {
      notify = fn;
      return () => {};
    });
    // The frame is in; the plate is its own request and still in flight.
    images.get.mockImplementation((url: string) =>
      url.includes("/pt/") ? undefined : ({ width: 750, height: 1050 } as unknown as HTMLImageElement),
    );
    const { make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");
    expect(cache.get(face(), "permanent")).toBeUndefined();

    images.get.mockReturnValue({ width: 750, height: 1050 } as unknown as HTMLImageElement);
    notify();

    expect(cache.get(face(), "permanent")).toBeDefined();
  });

  it("draws without the plate once its load has failed, rather than never drawing", () => {
    const images = readyImages();
    images.get.mockImplementation((url: string) =>
      url.includes("/pt/") ? undefined : ({ width: 750, height: 1050 } as unknown as HTMLImageElement),
    );
    images.isFailed.mockImplementation((url: string) => url.includes("/pt/"));
    const { make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");

    expect(cache.get(face(), "permanent")).toBeDefined();
  });

  it("draws the frame with no art rather than waiting for art that may never load", () => {
    const images = readyImages();
    // Frame assets resolve; the CDN art url does not.
    images.get.mockImplementation((url: string) =>
      url.includes("card-frames") ? ({ width: 750, height: 1050 } as unknown as HTMLImageElement) : undefined,
    );
    const { make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");

    expect(cache.get(face(), "permanent")).toBeDefined();
  });

  it("redraws the art-less face once the art lands", () => {
    const images = readyImages();
    let notify = () => {};
    images.subscribe.mockImplementation((fn: () => void) => {
      notify = fn;
      return () => {};
    });
    const frameOnly = (url: string) =>
      url.includes("card-frames") ? ({ width: 750, height: 1050 } as unknown as HTMLImageElement) : undefined;
    images.get.mockImplementation(frameOnly);
    const { drawn, make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");
    expect(drawn).toHaveLength(1);

    // The frame is a local asset and the art is a CDN round trip, so on a cold load the frame
    // always wins the race. If the art arriving did not redraw, every card would stay art-less.
    images.get.mockImplementation(() => ({ width: 626, height: 457 }) as unknown as HTMLImageElement);
    notify();

    expect(drawn).toHaveLength(2);
  });

  it("stops waiting on art the CDN could not serve", () => {
    const images = readyImages();
    let notify = () => {};
    images.subscribe.mockImplementation((fn: () => void) => {
      notify = fn;
      return () => {};
    });
    images.get.mockImplementation((url: string) =>
      url.includes("card-frames") ? ({ width: 750, height: 1050 } as unknown as HTMLImageElement) : undefined,
    );
    images.isFailed.mockImplementation((url: string) => !url.includes("card-frames"));
    const { drawn, make } = stubCanvas();
    const cache = new CardFaceCache(images, make);

    cache.request(face(), "permanent");
    notify();
    notify();

    expect(drawn).toHaveLength(1);
  });

  it("asks the CDN for the art size the art window actually draws", () => {
    const images = readyImages();
    const cache = new CardFaceCache(images, stubCanvas().make);

    cache.request(face(), "permanent");

    expect(images.preload).toHaveBeenCalledWith(expect.arrayContaining([expect.stringContaining("/art/")]));
  });

  it("preloads urls as a list — a bare string would preload one character per letter", () => {
    const images = readyImages();
    const cache = new CardFaceCache(images, stubCanvas().make);

    cache.request(face(), "permanent");

    for (const [urls] of images.preload.mock.calls) {
      expect(Array.isArray(urls)).toBe(true);
    }
  });

  it("asks for no art at all for a token with no printing", () => {
    const images = readyImages();
    const cache = new CardFaceCache(images, stubCanvas().make);

    cache.request(face({ print: "", isToken: true }), "permanent");

    const asked = images.preload.mock.calls.flatMap(([urls]) => [...urls]);
    expect(asked.every((url) => url.startsWith("/card-frames/"))).toBe(true);
  });
});

describe("faceKey", () => {
  it("keys a printing separately per variant", () => {
    expect(faceKey(face(), "permanent")).not.toBe(faceKey(face(), "full"));
  });

  it("keys a buffed creature separately from its printed self", () => {
    expect(faceKey(face(), "permanent")).not.toBe(faceKey(face({ power: "3" }), "permanent"));
  });

  it("keys a token separately from the card it copies", () => {
    expect(faceKey(face(), "permanent")).not.toBe(faceKey(face({ isToken: true }), "permanent"));
  });

  it("keys a card that changed colour separately — it draws in a different frame", () => {
    expect(faceKey(face(), "permanent")).not.toBe(faceKey(face({ colors: [2] }), "permanent"));
  });

  it("keys on every field of FaceData, so a new one cannot serve a stale face", () => {
    const base = face();
    const changed: FaceData = {
      print: "ffffffff-2345-6789-abcd-ef0123456789",
      name: "Grizzly Bears",
      colors: [1],
      isLand: true,
      isToken: true,
      legendary: true,
      power: "9",
      toughness: "9",
      loyalty: "9",
      typeLine: "Land",
      oracle: "Flying",
      flavor: "It watches.",
    };
    for (const field of Object.keys(base) as Array<keyof FaceData>) {
      expect(faceKey({ ...base, [field]: changed[field] }, "permanent"), `faceKey ignores ${field}`).not.toBe(
        faceKey(base, "permanent"),
      );
    }
  });
});

describe("CardFaceCache.clear", () => {
  it("redraws every face — the card typefaces land after the first frames are on screen", () => {
    const { drawn, make } = stubCanvas();
    const cache = new CardFaceCache(readyImages(), make);
    const listener = vi.fn();
    cache.subscribe(listener);

    cache.request(face(), "permanent");
    cache.clear();
    expect(cache.get(face(), "permanent")).toBeUndefined();
    expect(listener).toHaveBeenCalled();

    cache.request(face(), "permanent");
    expect(drawn).toHaveLength(2);
  });
});
