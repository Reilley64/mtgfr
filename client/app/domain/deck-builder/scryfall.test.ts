import * as Effect from "effect/Effect";
import { afterEach, describe, expect, it, vi } from "vitest";
import { buildImageUrl, parseRetryAfterMs, printSearchUrl, searchPrintPage } from "./scryfall";

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("parseRetryAfterMs", () => {
  it("reads integer seconds from Retry-After", () => {
    expect(parseRetryAfterMs("2")).toBe(2_000);
  });

  it("falls back to 30s when the header is missing or invalid", () => {
    expect(parseRetryAfterMs(null)).toBe(30_000);
    expect(parseRetryAfterMs("")).toBe(30_000);
    expect(parseRetryAfterMs("nope")).toBe(30_000);
  });

  it("clamps oversized delays", () => {
    expect(parseRetryAfterMs("120")).toBe(60_000);
  });

  it("parses HTTP-date Retry-After relative to now", () => {
    const now = Date.UTC(2026, 6, 27, 12, 0, 0);
    const when = new Date(now + 5_000).toUTCString();
    expect(parseRetryAfterMs(when, now)).toBe(5_000);
  });
});

describe("searchPrintPage User-Agent", () => {
  it("identifies as edh.reilley.dev/0.1", async () => {
    const fetchMock = vi.fn(
      async (_url: string, _init?: RequestInit) =>
        new Response(JSON.stringify({ data: [], has_more: false }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await Effect.runPromise(searchPrintPage(printSearchUrl("00000000-0000-0000-0000-000000000000")));

    expect(fetchMock).toHaveBeenCalled();
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit | undefined;
    const headers = init?.headers as Record<string, string>;
    expect(headers["User-Agent"]).toBe("edh.reilley.dev/0.1");
  });
});

describe("searchPrintPage 429 retry", () => {
  it("waits Retry-After then retries and succeeds", async () => {
    vi.useFakeTimers();
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ object: "error", code: "rate_limit" }), {
          status: 429,
          headers: { "Content-Type": "application/json", "Retry-After": "2" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(
          JSON.stringify({
            data: [
              {
                id: "print-1",
                set: "one",
                set_name: "Phyrexia: All Will Be One",
                collector_number: "91",
                released_at: "2023-02-03",
              },
            ],
            has_more: false,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        ),
      );
    vi.stubGlobal("fetch", fetchMock);

    const pending = Effect.runPromise(searchPrintPage(printSearchUrl("e4912bc3-bee9-4a2f-a13e-3a99018f8a65")));
    await vi.advanceTimersByTimeAsync(2_000);
    const page = await pending;

    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(page.prints).toEqual([
      {
        collector_number: "91",
        id: "print-1",
        released_at: "2023-02-03",
        set: "one",
        set_name: "Phyrexia: All Will Be One",
      },
    ]);
  });

  it("does not retry non-429 failures", async () => {
    const fetchMock = vi.fn(
      async () =>
        new Response(JSON.stringify({ object: "error" }), {
          status: 400,
          headers: { "Content-Type": "application/json" },
        }),
    );
    vi.stubGlobal("fetch", fetchMock);

    await expect(
      Effect.runPromise(searchPrintPage(printSearchUrl("e4912bc3-bee9-4a2f-a13e-3a99018f8a65"))),
    ).rejects.toThrow(/Scryfall print search failed \(400\)/);
    expect(fetchMock).toHaveBeenCalledTimes(1);
  });

  it("gives up after Retry-After retries are exhausted", async () => {
    vi.useFakeTimers();
    const fetchMock = vi.fn(
      async () =>
        new Response(JSON.stringify({ object: "error", code: "rate_limit" }), {
          status: 429,
          headers: { "Content-Type": "application/json", "Retry-After": "1" },
        }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const pending = Effect.runPromise(searchPrintPage(printSearchUrl("e4912bc3-bee9-4a2f-a13e-3a99018f8a65")));
    const expectation = expect(pending).rejects.toThrow(/Scryfall refused \(429\)/);
    await vi.advanceTimersByTimeAsync(1_000);
    await vi.advanceTimersByTimeAsync(1_000);
    await expectation;
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });
});

describe("searchPrintPage paging", () => {
  function pageResponse(body: unknown) {
    return new Response(JSON.stringify(body), { status: 200, headers: { "Content-Type": "application/json" } });
  }

  const print = {
    id: "print-1",
    set: "one",
    set_name: "Phyrexia: All Will Be One",
    collector_number: "91",
    released_at: "2023-02-03",
  };

  it("fetches only the page it was given and reports where the rest are", async () => {
    const fetchMock = vi.fn(async () =>
      pageResponse({ data: [print], has_more: true, next_page: "https://api.scryfall.com/next" }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const page = await Effect.runPromise(searchPrintPage(printSearchUrl("e4912bc3-bee9-4a2f-a13e-3a99018f8a65")));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(page.prints).toHaveLength(1);
    expect(page.nextPage).toBe("https://api.scryfall.com/next");
  });

  it("reports no next page on the last one", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => pageResponse({ data: [print], has_more: false })),
    );

    const page = await Effect.runPromise(searchPrintPage("https://api.scryfall.com/next"));

    expect(page.nextPage).toBeNull();
  });
});

describe("buildImageUrl", () => {
  const id = "abcd1234-5678-90ab-cdef-000000000001";

  it("names the size in the path, so a smaller size fetches a smaller image", () => {
    expect(buildImageUrl(id, "thumb", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/thumb/front/a/b/${id}.webp`,
    );
    expect(buildImageUrl(id, "display", "front", "https://cards.example.com")).toBe(
      `https://cards.example.com/display/front/a/b/${id}.webp`,
    );
  });

  it("keeps the face in the path and tolerates a trailing slash on the base", () => {
    expect(buildImageUrl(id, "art", "back", "https://cards.example.com/")).toBe(
      `https://cards.example.com/art/back/a/b/${id}.webp`,
    );
  });

  // Not api.scryfall.com/cards/{id}?format=image&version=… — that endpoint 302s instead of
  // serving bytes, and answers `version=display` with the `large` JPEG.
  it("falls back to Scryfall's image host at the same layout when cdnBase is empty", () => {
    expect(buildImageUrl(id, "display", "back", "")).toBe(`https://cards.scryfall.io/display/back/a/b/${id}.webp`);
  });

  it("returns empty string for empty print id", () => {
    expect(buildImageUrl("", "art", "front", "https://cards.example.com")).toBe("");
  });
});
