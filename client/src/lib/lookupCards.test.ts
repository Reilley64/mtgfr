import * as Effect from "effect/Effect";
import { describe, expect, it, vi } from "vitest";
import { LOOKUP_CHUNK, lookupCardsByIds } from "~/lib/lookupCards";

vi.mock("~/effect/client", () => ({
  client: {
    lookupCards: vi.fn(),
  },
}));

import { client } from "~/effect/client";

describe("lookupCardsByIds", () => {
  it("returns [] without calling the wire for an empty id list", async () => {
    const result = await Effect.runPromise(lookupCardsByIds([]));
    expect(result).toEqual([]);
    expect(client.lookupCards).not.toHaveBeenCalled();
  });

  it("chunks long id lists so each request stays under LOOKUP_CHUNK", async () => {
    const ids = Array.from({ length: LOOKUP_CHUNK + 3 }, (_, i) => `id-${i}`);
    vi.mocked(client.lookupCards).mockImplementation((chunkIds) => Effect.succeed(chunkIds.map((id) => ({ id }) as never)));
    const result = await Effect.runPromise(lookupCardsByIds(ids));
    expect(client.lookupCards).toHaveBeenCalledTimes(2);
    expect(vi.mocked(client.lookupCards).mock.calls[0]?.[0]).toHaveLength(LOOKUP_CHUNK);
    expect(vi.mocked(client.lookupCards).mock.calls[1]?.[0]).toHaveLength(3);
    expect(result.map((c) => c.id)).toEqual(ids);
  });
});
