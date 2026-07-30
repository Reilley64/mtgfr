import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { buildImageUrl } from "../../client/app/domain/deck-builder/scryfall";
import worker from "./card-cdn.js";

const ID = "abcd1234-5678-90ab-cdef-000000000001";
const KEY = `large/front/a/b/${ID}.jpg`;
const SCRYFALL = `https://api.scryfall.com/cards/${ID}?format=image&version=large`;

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

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(200);
    expect(res.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect(res.headers.get("Content-Type")).toBe("image/jpeg");
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([1, 2, 3]));
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("fills from Scryfall on a miss, stores under the layout key, and serves the bytes", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response(new Uint8Array([7, 8, 9]), { status: 200 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

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

    await worker.fetch(get(`/art_crop/back/a/b/${ID}.jpg`), env);

    expect(fetchMock.mock.calls[0]?.[0]).toBe(
      `https://api.scryfall.com/cards/${ID}?format=image&version=art_crop&face=back`,
    );
    expect(env.CARDS.put.mock.calls[0]?.[0]).toBe(`art_crop/back/a/b/${ID}.jpg`);
  });

  it("404s a fan-out prefix that disagrees with the print id, before any outbound request", async () => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(`/large/front/f/f/${ID}.jpg`), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
    expect(env.CARDS.get).not.toHaveBeenCalled();
  });

  it.each([
    ["unknown size folder", `/normal/front/a/b/${ID}.jpg`],
    ["unknown face folder", `/large/side/a/b/${ID}.jpg`],
    // Fan-out chars `a`/`b` agree with the id's own first two chars, so this fails on the id
    // group's shape specifically — not on the fan-out guard, which a loosened id class would
    // still (coincidentally) pass through undetected.
    ["non-UUID print id", "/large/front/a/b/abzzzzzz-zzzz-zzzz-zzzz-zzzzzzzzzzzz.jpg"],
    ["wrong extension", `/large/front/a/b/${ID}.webp`],
    ["extra path segment", `/large/front/a/b/${ID}.jpg/extra`],
    ["bare root", "/"],
  ])("404s an off-layout path (%s) with no outbound request", async (_label, path) => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(path), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("405s a write method", async () => {
    const env = { CARDS: bucket() };

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`, "POST"), env);

    expect(res.status).toBe(405);
    expect(res.headers.get("Allow")).toBe("GET, HEAD");
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("redirects to Scryfall and stores nothing when the fill is rate-limited", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response("", { status: 429 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("redirects to Scryfall when the fill throws", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockRejectedValue(new Error("network down"));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("404s a print Scryfall does not have", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response("", { status: 404 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(404);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("still serves the fetched bytes when the R2 write fails", async () => {
    const env = { CARDS: bucket() };
    env.CARDS.put.mockRejectedValue(new Error("R2 unavailable"));
    fetchMock.mockResolvedValue(new Response(new Uint8Array([4]), { status: 200 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(200);
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([4]));
  });

  it("falls through to the fill when the R2 read fails", async () => {
    const env = { CARDS: bucket() };
    env.CARDS.get.mockRejectedValue(new Error("R2 unavailable"));
    fetchMock.mockResolvedValue(new Response(new Uint8Array([5, 6]), { status: 200 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(200);
    expect(new Uint8Array(await res.arrayBuffer())).toEqual(new Uint8Array([5, 6]));
  });

  it("redirects and stores nothing when the fill is a zero-length 200", async () => {
    const env = { CARDS: bucket() };
    fetchMock.mockResolvedValue(new Response(new Uint8Array([]), { status: 200 }));

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

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

    const res = await worker.fetch(get(`/large/front/a/b/${ID}.jpg`), env);

    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe(SCRYFALL);
    expect(env.CARDS.put).not.toHaveBeenCalled();
  });

  it("404s a fan-out prefix where only the second char disagrees with the print id", async () => {
    const env = { CARDS: bucket() };

    // ID starts "ab...": `a` matches id[0], but `f` does not match id[1] — isolates the
    // `b !== id[1]` half of the guard from the `a !== id[0]` half.
    const res = await worker.fetch(get(`/large/front/a/f/${ID}.jpg`), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("404s an uppercase-hex UUID rather than treating it as an alias of the lowercase key", async () => {
    const env = { CARDS: bucket() };

    // Print ids are canonically lowercase. Fan-out chars uppercased to match, so this only
    // fails on the regex's case sensitivity, not the fan-out guard.
    const res = await worker.fetch(get("/large/front/A/B/ABCD1234-5678-90AB-CDEF-000000000001.jpg"), env);

    expect(res.status).toBe(404);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

// The Worker cannot import shared code, so the layout string exists twice. This is the test
// that fails if the two copies drift. Exhaustive rather than representative: the invariant is
// that every folder `buildImageUrl` can emit is one `LAYOUT` accepts, so every `ImageSize` (which
// collapses to two folders) crossed with every `ImageFace` must be swept, not a sample of them.
describe("layout round-trip against buildImageUrl", () => {
  const sizes = ["small", "normal", "large", "png", "art_crop"] as const;
  const faces = ["front", "back"] as const;
  const cases = sizes.flatMap((size) => faces.map((face) => [size, face] as const));

  it.each(cases)("parses the URL buildImageUrl emits for %s/%s", async (size, face) => {
    const folder = size === "art_crop" ? "art_crop" : "large";
    const url = buildImageUrl(ID, size, face, "https://edh-images.reilley.dev");
    const env = { CARDS: bucket({ [`${folder}/${face}/a/b/${ID}.jpg`]: new Uint8Array([1]) }) };

    const res = await worker.fetch(new Request(url), env);

    expect(res.status).toBe(200);
    expect(fetchMock).not.toHaveBeenCalled();
  });
});
