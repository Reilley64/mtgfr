/**
 * @vitest-environment happy-dom
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { BLANK_FACE, type FaceData } from "../card-render/frame";
import { syncCardFaceHost } from "./card-face";

function face(overrides: Partial<FaceData> = {}): FaceData {
  return { ...BLANK_FACE, print: "p", name: "Llanowar Elves", colors: [4], ...overrides };
}

/** Stands in for `CardFaceCache`: `ready` decides whether the face has been drawn yet. */
function stubCache(ready: boolean) {
  return {
    get: vi.fn(() => (ready ? ({ width: 745, height: 1040 } as unknown as CanvasImageSource) : undefined)),
    request: vi.fn(),
  };
}

function host(overrides: Partial<FaceData> = {}): HTMLElement {
  const element = document.createElement("div");
  element.dataset.face = JSON.stringify(face(overrides));
  element.dataset.faceVariant = "full";
  element.dataset.faceW = "100";
  element.dataset.faceH = "140";
  element.dataset.faceClass = "rounded-game";
  element.dataset.faceAlt = "Llanowar Elves";
  document.body.append(element);
  return element;
}

describe("syncCardFaceHost", () => {
  afterEach(() => {
    document.body.replaceChildren();
  });

  it("paints the drawn face into a canvas the size of its box", () => {
    const cache = stubCache(true);
    const element = host();

    syncCardFaceHost(element, cache, 2);

    const canvas = element.querySelector("canvas");
    expect(canvas?.getAttribute("aria-label")).toBe("Llanowar Elves");
    // Bitmap in device pixels, box in CSS pixels — a 2x display draws the face at 2x and scales down.
    expect(canvas?.width).toBe(200);
    expect(canvas?.height).toBe(280);
    expect(canvas?.style.width).toBe("100px");
  });

  it("asks the cache to draw a face it does not have, and shows a skeleton meanwhile", () => {
    const cache = stubCache(false);
    const element = host();

    syncCardFaceHost(element, cache);

    expect(cache.request).toHaveBeenCalledWith(face(), "full");
    expect(element.querySelector("canvas")).toBeNull();
    expect(element.querySelector("[aria-hidden='true']")?.className).toContain("animate-skeleton");
  });

  it("paints the face `request` drew on the spot rather than leaving a skeleton up", () => {
    // The real cache draws inside `request` when the frame and art are already loaded, and tells
    // its listeners so — before this host has subscribed. Missing that left the commander in hand
    // showing a skeleton for the rest of the game.
    let drawn = false;
    const cache = {
      get: vi.fn(() => (drawn ? ({ width: 745, height: 1040 } as unknown as CanvasImageSource) : undefined)),
      request: vi.fn(() => {
        drawn = true;
      }),
    };

    syncCardFaceHost(host(), cache, 1);

    expect(document.querySelector("canvas")).not.toBeNull();
    expect(document.querySelector(".animate-skeleton")).toBeNull();
  });

  it("repaints when the card changes — a hand tile is reused as cards come and go", () => {
    const cache = stubCache(true);
    const element = host();

    syncCardFaceHost(element, cache, 1);
    element.dataset.face = JSON.stringify(face({ name: "Grizzly Bears" }));
    syncCardFaceHost(element, cache, 1);

    expect(element.querySelectorAll("canvas")).toHaveLength(1);
    expect(element.querySelector("canvas")?.getAttribute("aria-label")).toBe("Grizzly Bears");
  });
});
