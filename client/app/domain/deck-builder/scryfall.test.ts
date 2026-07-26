import { afterEach, describe, expect, it, vi } from "vitest";
import { artCropFallbackUrl, buildImageUrl, scryfallImageUrl, searchPrints } from "./scryfall";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("searchPrints User-Agent", () => {
  it("identifies as edh.reilley.dev/0.1", async () => {
    const fetchMock = vi.fn(
      async (_url: string, _init?: RequestInit) =>
        new Response(JSON.stringify({ data: [], has_more: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await searchPrints("00000000-0000-0000-0000-000000000000");

    expect(fetchMock).toHaveBeenCalled();
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit | undefined;
    const headers = init?.headers as Record<string, string>;
    expect(headers["User-Agent"]).toBe("edh.reilley.dev/0.1");
  });
});

describe("buildImageUrl", () => {
  const id = "abcd1234-5678-90ab-cdef-000000000001";

  it("uses Scryfall version=art_crop when cdnBase is empty", () => {
    expect(buildImageUrl(id, "art_crop", "front", "")).toBe(
      `https://api.scryfall.com/cards/${id}?format=image&version=art_crop`,
    );
  });

  it("uses CDN art_crop folder when cdnBase is set", () => {
    expect(buildImageUrl(id, "art_crop", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/art_crop/front/a/b/${id}.webp`,
    );
  });

  it("maps non-art_crop sizes to CDN large folder when cdnBase is set", () => {
    expect(buildImageUrl(id, "large", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/large/front/a/b/${id}.webp`,
    );
    expect(buildImageUrl(id, "small", "back", "https://cards.example.com/")).toBe(
      `https://cards.example.com/large/back/a/b/${id}.webp`,
    );
  });

  it("adds face=back on Scryfall URLs", () => {
    expect(buildImageUrl(id, "art_crop", "back", "")).toBe(
      `https://api.scryfall.com/cards/${id}?format=image&version=art_crop&face=back`,
    );
  });

  it("returns empty string for empty print id", () => {
    expect(buildImageUrl("", "art_crop", "front", "https://cards.example.com")).toBe("");
  });
});

describe("scryfallImageUrl", () => {
  it("ignores CDN and always builds Scryfall", () => {
    const id = "ffff0000-0000-0000-0000-000000000001";
    expect(scryfallImageUrl(id, "art_crop")).toContain("version=art_crop");
    expect(scryfallImageUrl(id, "art_crop")).toContain("api.scryfall.com");
  });
});

describe("artCropFallbackUrl", () => {
  it("returns null when module CDN is unset (default vitest)", () => {
    expect(artCropFallbackUrl("abcd1234-5678-90ab-cdef-000000000001")).toBeNull();
  });

  it("returns Scryfall art_crop when a cdnBase is provided", () => {
    const id = "abcd1234-5678-90ab-cdef-000000000001";
    expect(artCropFallbackUrl(id, "front", "https://cards.example.com")).toBe(
      `https://api.scryfall.com/cards/${id}?format=image&version=art_crop`,
    );
  });

  it("returns null when cdnBase is empty", () => {
    expect(artCropFallbackUrl("abcd1234-5678-90ab-cdef-000000000001", "front", "")).toBeNull();
  });
});
