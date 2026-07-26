import { describe, expect, it } from "vitest";
import { proxiedCardArtUrl, resolveCardFaceUrls } from "./proxy-url";

describe("proxiedCardArtUrl", () => {
  it("builds encoded proxy URL", () => {
    const remoteUrl = "https://example.com/a path/image.png?x=1&y=2";
    expect(proxiedCardArtUrl(remoteUrl)).toBe(`/api/card-art/proxy?url=${encodeURIComponent(remoteUrl)}`);
  });
});

describe("resolveCardFaceUrls", () => {
  const print = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";
  const proxyArtUrl = "https://example.com/a.png";

  it("prefers proxy with print fallback on front face", () => {
    const result = resolveCardFaceUrls({
      print,
      proxyArtUrl,
      size: "large",
      face: "front",
    });

    expect(result.url).toBe(`/api/card-art/proxy?url=${encodeURIComponent(proxyArtUrl)}`);
    expect(result.fallback).toContain(print);
  });

  it("ignores proxy on back face", () => {
    const result = resolveCardFaceUrls({
      print,
      proxyArtUrl,
      face: "back",
    });

    expect(result.url).not.toContain("/api/card-art/proxy");
    expect(result.url).toContain(print);
    expect(result.fallback).toBeNull();
  });

  it("preserves art crop CDN fallback when no proxy is set", () => {
    const result = resolveCardFaceUrls({
      print,
      size: "art_crop",
      face: "front",
    });

    expect(result.url).toContain(print);
    expect(result.fallback === null || result.fallback.includes(print)).toBe(true);
  });
});
