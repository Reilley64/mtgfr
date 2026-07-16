import { afterEach, describe, expect, it, vi } from "vitest";

describe("imageUrlByPrint", () => {
  const printId = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it("returns empty for an empty print id", async () => {
    vi.stubEnv("VITE_CARD_CDN", "");
    const { imageUrlByPrint } = await import("~/lib/scryfall");
    expect(imageUrlByPrint("")).toBe("");
  });

  it("requests the back face from Scryfall when face is back (no CDN)", async () => {
    vi.stubEnv("VITE_CARD_CDN", "");
    const { imageUrlByPrint } = await import("~/lib/scryfall");
    const url = imageUrlByPrint(printId, "large", "back");
    expect(url).toContain("face=back");
    expect(url).toContain(printId);
  });

  it("uses the CDN back-face path when VITE_CARD_CDN is set", async () => {
    vi.stubEnv("VITE_CARD_CDN", "https://cdn.example");
    const { imageUrlByPrint } = await import("~/lib/scryfall");
    const url = imageUrlByPrint(printId, "large", "back");
    expect(url).toBe("https://cdn.example/large/back/a/a/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.webp");
  });

  it("omits face=back on the front face", async () => {
    vi.stubEnv("VITE_CARD_CDN", "");
    const { imageUrlByPrint } = await import("~/lib/scryfall");
    const url = imageUrlByPrint(printId, "large", "front");
    expect(url).not.toContain("face=back");
  });
});

describe("searchPrints", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetModules();
  });

  it("throws when Scryfall returns a non-OK status", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 429,
      }),
    );
    const { searchPrints } = await import("~/lib/scryfall");
    await expect(searchPrints("oracle-id")).rejects.toThrow("Scryfall print search failed (429)");
  });

  it("maps print metadata from a successful search page", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          data: [
            {
              id: "print-1",
              set: "mh3",
              set_name: "Modern Horizons 3",
              collector_number: "123",
              released_at: "2024-06-14",
            },
          ],
          has_more: false,
        }),
      }),
    );
    const { searchPrints } = await import("~/lib/scryfall");
    await expect(searchPrints("oracle-id")).resolves.toEqual([
      {
        id: "print-1",
        set: "mh3",
        set_name: "Modern Horizons 3",
        collector_number: "123",
        released_at: "2024-06-14",
      },
    ]);
  });
});
