import { describe, expect, it, vi } from "vitest";
import { assertSafeProxyTarget, fetchProxyCardArt, PROXY_ART_MAX_BYTES } from "./proxy-fetch";

describe("assertSafeProxyTarget", () => {
  it("allows public https hosts", () => {
    expect(assertSafeProxyTarget("https://cdn.example.com/a.png").host).toBe("cdn.example.com");
  });

  it("rejects non-https, private, and credentialed URLs", () => {
    expect(() => assertSafeProxyTarget("http://example.com/a.png")).toThrow();
    expect(() => assertSafeProxyTarget("https://127.0.0.1/a.png")).toThrow();
    expect(() => assertSafeProxyTarget("https://[::1]/a.png")).toThrow();
    expect(() => assertSafeProxyTarget("https://user:pass@example.com/a.png")).toThrow();
    expect(() => assertSafeProxyTarget("https://169.254.169.254/latest/meta-data")).toThrow();
    expect(() => assertSafeProxyTarget("https://metadata.google.internal/computeMetadata/v1")).toThrow();
  });
});

describe("fetchProxyCardArt", () => {
  it("rejects hosts that resolve to private addresses", async () => {
    const fetchImpl = vi.fn();

    await expect(
      fetchProxyCardArt("https://cdn.example.com/a.png", {
        fetchImpl,
        lookupHost: vi.fn(async () => [{ address: "10.0.0.5", family: 4 }]),
      }),
    ).resolves.toEqual({ ok: false, status: 400 });

    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("fetches an allowed image without forwarding cookies", async () => {
    const body = Uint8Array.from([1, 2, 3, 4]);
    const fetchImpl = vi.fn(async (target: URL, init?: RequestInit & { lookup?: unknown }) => {
      expect(target.hostname).toBe("cdn.example.com");
      expect(init?.redirect).toBe("manual");
      expect(init?.headers).toEqual({ accept: "image/*" });
      expect(init?.lookup).toBeTypeOf("function");
      return new Response(body, {
        status: 200,
        headers: { "content-type": "image/png", "content-length": String(body.byteLength) },
      });
    });

    await expect(
      fetchProxyCardArt("https://cdn.example.com/a.png", {
        fetchImpl,
        lookupHost: vi.fn(async () => [{ address: "198.51.100.10", family: 4 }]),
      }),
    ).resolves.toEqual({
      ok: true,
      body,
      contentType: "image/png",
    });
  });

  it("pins the upstream request lookup to the vetted DNS result", async () => {
    const body = Uint8Array.from([9, 8, 7]);
    const resolvedAddresses = [{ address: "198.51.100.10", family: 4 }] as const;
    const fetchImpl = vi.fn(async (_target: URL, init?: RequestInit & { lookup?: unknown }) => {
      expect(init?.lookup).toBeTypeOf("function");
      const addresses = await new Promise<unknown>((resolve, reject) => {
        const lookup = init?.lookup;
        if (typeof lookup !== "function") {
          reject(new Error("missing lookup"));
          return;
        }
        lookup(
          "cdn.example.com",
          { all: true, verbatim: true },
          (error: unknown, result: unknown) => {
            if (error) {
              reject(error);
              return;
            }
            resolve(result);
          },
        );
      });
      expect(addresses).toEqual(resolvedAddresses);
      return new Response(body, {
        status: 200,
        headers: { "content-type": "image/png", "content-length": String(body.byteLength) },
      });
    });

    await expect(
      fetchProxyCardArt("https://cdn.example.com/a.png", {
        fetchImpl,
        lookupHost: vi.fn(async () => [...resolvedAddresses]),
      }),
    ).resolves.toEqual({
      ok: true,
      body,
      contentType: "image/png",
    });
  });

  it("rejects non-image responses", async () => {
    await expect(
      fetchProxyCardArt("https://cdn.example.com/a.png", {
        fetchImpl: vi.fn(async () => new Response("nope", { status: 200, headers: { "content-type": "text/html" } })),
        lookupHost: vi.fn(async () => [{ address: "198.51.100.10", family: 4 }]),
      }),
    ).resolves.toEqual({ ok: false, status: 502 });
  });

  it("rejects oversized image responses", async () => {
    const body = new Uint8Array(PROXY_ART_MAX_BYTES + 1);

    await expect(
      fetchProxyCardArt("https://cdn.example.com/a.png", {
        fetchImpl: vi.fn(
          async () =>
            new Response(body, {
              status: 200,
              headers: { "content-type": "image/webp", "content-length": String(body.byteLength) },
            }),
        ),
        lookupHost: vi.fn(async () => [{ address: "198.51.100.10", family: 4 }]),
      }),
    ).resolves.toEqual({ ok: false, status: 502 });
  });
});
