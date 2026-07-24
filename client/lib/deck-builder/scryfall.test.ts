import { afterEach, describe, expect, it, vi } from "vitest";
import { searchPrints } from "./scryfall";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("searchPrints User-Agent", () => {
  it("identifies as edh.reilley.dev/0.1", async () => {
    const fetchMock = vi.fn(async () =>
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
