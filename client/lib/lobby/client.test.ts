import { afterEach, describe, expect, it, vi } from "vitest";
import { lobbyState } from "./client";

const unknownTable = {
  table_id: "GONE",
  commander_damage_enabled: true,
  seats: [],
  you: null,
  started: false,
  start_error: null,
  error: "UnknownTable",
};

function json(body: unknown, status: number): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

describe("lobby client", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("preserves structured lobby error bodies on non-2xx responses", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json(unknownTable, 404)),
    );

    await expect(lobbyState("GONE")).resolves.toEqual(unknownTable);
  });
});
