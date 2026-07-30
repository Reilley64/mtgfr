import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildImageUrl } from "../../client/app/domain/deck-builder/scryfall";
import worker from "./card-cdn.js";

const ID = "abcd1234-5678-90ab-cdef-000000000001";
const KEY = `display/front/a/b/${ID}.webp`;
const SCRYFALL = `https://cards.scryfall.io/display/front/a/b/${ID}.webp`;

type Stored = { body: Uint8Array };

/** Minimal stand-in for the R2 binding: the Worker only ever calls get/put. */
function bucket(seed: Record<string, Uint8Array> = {}) {
  const store = new Map<string, Stored>(Object.entries(seed).map(([k, v]) => [k, { body: v }]));
  return {
    store,
    get: vi.fn(async (key: string) => store.get(key) ?? null),
    put: vi.fn(async (key: string, body: ArrayBuffer) => {
      store.set(key, { body: new Uint8Array(body) });
    }),
  };
}

function get(path: string, method = "GET"): Request {
  return new Request(`https://edh-images.reilley.dev${path}`, { method });
}

let fetchMock: ReturnType<typeof vi.fn>;

beforeEach(() => {
  fetchMock = vi.fn();
  vi.stubGlobal("fetch", fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("card CDN worker", () => {
  it("serves stored bytes without touching Scryfall", async () => {
    const env = { CARDS: bucket({ [KEY]: new Uint8Array([1, 2, 3]) }) };

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(200);
    expect(res.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect(res.headers.get("Content-Type")).toBe("image/webp");
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([1, 2, 3]));
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("fills from Scryfall on a miss, stores under the layout key, and serves the bytes", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response(new Uint8Array([7, 8, 9]), { status: 200 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(200);
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([7, 8, 9]));
    expect(fetchMock.mock.calls[0]?.[0]).toBe(SCRYFALL);
    expect(env.CARDS.put).toHaveBeenCalledTimes(1);
    expect(env.CARDS.put.mock.calls[0]?.[0]).toBe(KEY);
    expect(env.CARDS.store.get(KEY)?.body).toEqual(new Uint8Array([7, 8, 9]));
  });

  it("asks Scryfall for the back face of a DFC", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response(new Uint8Array([1]), { status: 200 }));

    await worker.fetch(get(`/art/back/a/b/${ID}.webp`), env);

    expect(fetchMock.mock.calls[0]?.[0]).toBe(`https://cards.scryfall.io/art/back/a/b/${ID}.webp`);
    expect(env.CARDS.put.mock.calls[0]?.[0]).toBe(`art/back/a/b/${ID}.webp`);
  });

  it("404s a fan-out prefix that disagrees with the print id, before any outbound request", async () => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(`/display/front/f/f/${ID}.webp`), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(env.CARDS.get).not.toHaveBeenCalled();
  });

  it.each([
    ["unknown size folder", `/normal/front/a/b/${ID}.webp`],
    ["unknown face folder", `/display/side/a/b/${ID}.webp`],
    ["retired JPEG folder", `/large/front/a/b/${ID}.webp`],
    // Fan-out chars `a`/`b` agree with the id's own first two chars, so this fails on the id
    // group's shape specifically — not on the fan-out guard, which a loosened id class would
    // still (coincidentally) pass through undetected.
    ["non-UUID print id", "/display/front/a/b/abzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz.webp"],
    ["wrong extension", `/display/front/a/b/${ID}.jpg`],
    ["extra path segment", `/display/front/a/b/${ID}.webp/extra`],
    ["bare root", "/"],
  ])("404s an off-layout path (%s) with no outbound request", async (_label, path) => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(path), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("405s a write method", async () => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`, "POST"), env);

    expect(res.status).toBe(405);
    expect(res.headers.get("Allow")).toBe("GET, HEAD");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("redirects to Scryfall and stores nothing when the fill is rate-limited", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response("", { status: 429 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("redirects to Scryfall and stores nothing when the fill itself is a redirect", async () => {
    // The exact shape of the bug this guards: api.scryfall.com/cards/{id}?format=image redirects
    // rather than returning bytes, so `!filled.ok` must still catch a 3xx from upstream, not
    // just 429/5xx — a Worker that trusted a 2xx-shaped mock here would have shipped with every
    // fill silently failing.
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response("", { status: 302, headers: { Location: "https://example.com/x" } }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("redirects to Scryfall when the fill throws", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockRejectedValue(new Error("network down"));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("404s a print Scryfall does not have", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response("", { status: 404 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(404);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("still serves the fetched bytes when the R2 write fails", async () => {
    const env = { CARDS: bucket() };
    env.CARDS.put.mockRejectedValue(new Error("R2 unavailable"));
    fetchMock.mockResolvedValue(new Response(new Uint8Array([4]), { status: 200 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(200);
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([4]));
  });

  it("falls through to the fill when the R2 read fails", async () => {
    const env = { CARDS: bucket() };
    env.CARDS.get.mockRejectedValue(new Error("R2 unavailable"));
    fetchMock.mockResolvedValue(new Response(new Uint8Array([5, 6]), { status: 200 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(200);
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([5, 6]));
  });

  it("redirects and stores nothing when the fill is a zero-length 200", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response(new Uint8Array([]), { status: 200 }));

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("redirects and stores nothing when the fill body fails to read mid-stream", async () => {
    const env = { CARDS: bucket() };
    // A same-shape stand-in for a Response whose body stream errors after headers arrive
    // (e.g. an upstream connection reset) — `Response` itself has no way to construct that.
    fetchMock.mockResolvedValue({
      ok: true,
      status: 200,
      arrayBuffer: () => Promise.reject(new Error("ECONNRESET")),
    });

    const res = await worker.fetch(get(`/display/front/a/b/${ID}.webp`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("404s a fan-out prefix where only the second char disagrees with the print id", async () => {
    const env = { CARDS: bucket() };

    // ID starts "ab...": `a` matches id[0], but `f` does not match id[1] — isolates the
    // `b !== id[1]` half of the guard from the `a !== id[0]` half.
    const res = await worker.fetch(get(`/display/front/a/f/${ID}.webp`), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("404s an uppercase-hex UUID rather than treating it as an alias of the lowercase key", async () => {
    const env = { CARDS: bucket() };

    // Print ids are canonically lowercase. Fan-out chars uppercased to match, so this only
    // fails on the regex's case sensitivity, not the fan-out guard.
    const res = await worker.fetch(get("/display/front/A/B/ABCD1234-5678-90AB-CDEF-000000000001.webp"), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

// The Worker cannot import shared code, so the layout exists twice — here and in `LAYOUT`. This is
// the test that fails if the two copies drift. Exhaustive rather than representative: the invariant
// is that every URL `buildImageUrl` can emit is one `LAYOUT` accepts, so every `ImageSize` crossed
// with every `ImageFace` must be swept, not a sample of them.
//
// The expected key is read back out of the URL rather than restated here. Restating it would let a
// wrong layout agree with itself and pass — which is exactly how the `api.scryfall.com` upstream
// shipped broken past a green suite.
describe("layout round-trip against buildImageUrl", () => {
  const sizes = ["thumb", "grid", "display", "art", "crop"] as const;
  const faces = ["front", "back"] as const;
  const cases = sizes.flatMap((size) => faces.map((face) => [size, face] as const));

  it.each(cases)("parses the URL buildImageUrl emits for %s/%s", async (size, face) => {
    const url = buildImageUrl(ID, size, face, "https://edh-images.reilley.dev");
    const key = new URL(url).pathname.slice(1);
    const env = { CARDS: bucket({ [key]: new Uint8Array([1]) }) };

    const res = await worker.fetch(new Request(url), env);

    expect(res.status).toBe(200);
    expect(res.headers.get("Content-Type")).toBe("image/webp");
    // A hit must come from R2, so the key the Worker derived matched the one it was given.
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("emits size and face as path segments, not query params", () => {
    const url = buildImageUrl(ID, "display", "back", "https://edh-images.reilley.dev");

    expect(url).toBe(`https://edh-images.reilley.dev/display/back/a/b/${ID}.webp`);
  });
});
