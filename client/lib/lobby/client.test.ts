import { afterEach, describe, expect, it, vi } from "vitest";
import { createTable, joinTable, lobbyState } from "./client";

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

  it("returns null for non-LobbyView JSON so Foldkit does not crash on ReceivedLobbyView", async () => {
    // Nitro/server error bodies (and other non-lobby JSON) lack table_id. Casting them
    // as LobbyView used to reach ReceivedLobbyView({ view }) and throw Missing key at
    // ["view"]["table_id"].
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ error: true, statusCode: 500, message: "Server Error" }, 500)),
    );

    await expect(lobbyState("ABC123")).resolves.toBeNull();
    await expect(joinTable({ table_id: "ABC123", deck_id: 1 })).resolves.toBeNull();
  });

  it("returns null when lobby JSON uses camelCase tableId instead of table_id", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        json({ tableId: "ABC123", seats: [], you: null, started: false, error: null, start_error: null }, 200),
      ),
    );

    await expect(lobbyState("ABC123")).resolves.toBeNull();
  });

  it("returns null when create-table JSON is missing table_id", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ error: true, statusCode: 500 }, 500)),
    );

    await expect(createTable()).resolves.toBeNull();
  });
});
