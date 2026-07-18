import { describe, expect, it } from "vitest";
import { contentLengthTooLarge, FARO_MAX_BODY_BYTES, readBodyCapped, upstreamUrl } from "~/lib/faroCollect";

describe("faro collect helpers", () => {
  it("upstreamUrl requires a non-empty FARO_COLLECT_UPSTREAM", () => {
    expect(upstreamUrl({})).toBeNull();
    expect(upstreamUrl({ FARO_COLLECT_UPSTREAM: "  " })).toBeNull();
    expect(upstreamUrl({ FARO_COLLECT_UPSTREAM: " http://alloy:12347/collect " })).toBe("http://alloy:12347/collect");
  });

  it("contentLengthTooLarge rejects oversize declared lengths", () => {
    expect(contentLengthTooLarge(null)).toBe(false);
    expect(contentLengthTooLarge("100")).toBe(false);
    expect(contentLengthTooLarge(String(FARO_MAX_BODY_BYTES + 1))).toBe(true);
    expect(contentLengthTooLarge("nope")).toBe(false);
  });

  it("readBodyCapped returns 413 when the body exceeds the cap", async () => {
    const over = new Uint8Array(64);
    const req = new Request("http://local/api/faro/collect", {
      method: "POST",
      body: over,
      headers: { "content-length": "64" },
    });
    const result = await readBodyCapped(req, 32);
    expect(result).toEqual({ ok: false, status: 413 });
  });

  it("readBodyCapped accepts a small body", async () => {
    const req = new Request("http://local/api/faro/collect", {
      method: "POST",
      body: new Uint8Array([1, 2, 3]),
    });
    const result = await readBodyCapped(req, 32);
    expect(result.ok).toBe(true);
    if (result.ok) expect(result.body.byteLength).toBe(3);
  });
});
