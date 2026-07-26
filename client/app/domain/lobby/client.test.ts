import { afterEach, describe, expect, it, vi } from "vitest";
import { apiMeta, coverageMeta, createTable, joinTable, lobbyState, readyUp, startGame } from "./client";

const unknownTable = {
  table_id: "GONE",
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
    await expect(joinTable("ABC123", { deck_id: 1 })).resolves.toBeNull();
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

  it("posts join/ready/start with table id in the path and without table_id in the body", async () => {
    const fetchMock = vi.fn(async () => json(unknownTable, 404));
    vi.stubGlobal("fetch", fetchMock);

    await joinTable("ABC123", { deck_id: 7 });
    await readyUp("ABC123", { ready: true });
    await startGame("ABC123");

    expect(fetchMock).toHaveBeenNthCalledWith(
      1,
      "/api/tables/ABC123/join/v1",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ deck_id: 7 }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      2,
      "/api/tables/ABC123/ready/v1",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ ready: true }),
      }),
    );
    expect(fetchMock).toHaveBeenNthCalledWith(
      3,
      "/api/tables/ABC123/start/v1",
      expect.objectContaining({
        method: "POST",
        body: "{}",
      }),
    );
  });

  it("returns null when create-table JSON is missing table_id", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ error: true, statusCode: 500 }, 500)),
    );

    await expect(createTable()).resolves.toBeNull();
  });

  it("decodes meta version coverage fields when present", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => json({ version: "1.2.3", faithful_count: 662, oracle_total: 28412 }, 200)),
    );

    await expect(apiMeta()).resolves.toEqual({
      version: "1.2.3",
      faithfulCount: 662,
      oracleTotal: 28412,
    });
  });

  it("decodes set coverage rows and null oracle totals", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () =>
        json(
          {
            faithful_count: 662,
            oracle_total: 28412,
            sets: [
              {
                code: "soc",
                name: "Secrets of Strixhaven Commander",
                released_at: "2026-04-01",
                faithful: 10,
                oracle_total: 400,
              },
              {
                code: "scn",
                name: "Set Without Oracle Rows",
                released_at: null,
                faithful: 0,
                oracle_total: null,
              },
            ],
          },
          200,
        ),
      ),
    );

    await expect(coverageMeta()).resolves.toEqual({
      faithfulCount: 662,
      oracleTotal: 28412,
      sets: [
        {
          code: "soc",
          name: "Secrets of Strixhaven Commander",
          releasedAt: "2026-04-01",
          faithful: 10,
          oracleTotal: 400,
        },
        {
          code: "scn",
          name: "Set Without Oracle Rows",
          releasedAt: null,
          faithful: 0,
          oracleTotal: null,
        },
      ],
    });
  });
});
