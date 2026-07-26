import * as Effect from "effect/Effect";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { makeClient } from "./client";
import type { LobbyHttpError } from "./errors";

function stubLocation(): void {
  vi.stubGlobal("location", { origin: "http://localhost", pathname: "/" });
}

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function recordingFetch(response: Response): { fetch: typeof fetch; calls: [URL, RequestInit | undefined][] } {
  const calls: [URL, RequestInit | undefined][] = [];
  const fetchImpl = ((url: URL, init?: RequestInit) => {
    calls.push([url, init]);
    return Promise.resolve(response.clone());
  }) as unknown as typeof fetch;
  return { fetch: fetchImpl, calls };
}

const unknownTable = {
  table_id: "GONE",
  seats: [],
  you: null,
  started: false,
  start_error: null,
  error: "UnknownTable",
};

beforeAll(stubLocation);

describe("lobby makeClient", () => {
  it("prepends /api and sends credentials include", async () => {
    const { fetch, calls } = recordingFetch(json(unknownTable));
    const client = makeClient(fetch);
    await Effect.runPromise(client.lobbyState("GONE"));
    expect(calls[0][0].pathname).toBe("/api/tables/GONE/lobby/v1");
    expect(calls[0][1]?.credentials).toBe("include");
  });

  it("succeeds with LobbyView body on HTTP 404 (UnknownTable)", async () => {
    const client = makeClient((() => Promise.resolve(json(unknownTable, 404))) as unknown as typeof fetch);
    const view = await Effect.runPromise(client.lobbyState("GONE"));
    expect(view.error).toBe("UnknownTable");
  });

  it("fails LobbyHttpError on 500 non-LobbyView JSON", async () => {
    const client = makeClient((() =>
      Promise.resolve(
        json({ error: true, statusCode: 500, message: "Server Error" }, 500),
      )) as unknown as typeof fetch);
    const err = await Effect.runPromise(client.lobbyState("ABC123").pipe(Effect.flip));
    expect(err._tag).toBe("LobbyHttpError");
    expect((err as LobbyHttpError).status).toBe(500);
  });

  it("fails LobbyDecodeError on 200 camelCase tableId", async () => {
    const client = makeClient((() =>
      Promise.resolve(
        json({ tableId: "ABC123", seats: [], you: null, started: false, error: null, start_error: null }, 200),
      )) as unknown as typeof fetch);
    const err = await Effect.runPromise(client.lobbyState("ABC123").pipe(Effect.flip));
    expect(err._tag).toBe("LobbyDecodeError");
  });

  it("posts join/ready/start with table in path", async () => {
    const { fetch, calls } = recordingFetch(json(unknownTable, 404));
    const client = makeClient(fetch);
    await Effect.runPromise(client.joinTable("ABC123", { deck_id: 7 }));
    await Effect.runPromise(client.readyUp("ABC123", { ready: true }));
    await Effect.runPromise(client.startGame("ABC123"));
    expect(calls[0][0].pathname).toBe("/api/tables/ABC123/join/v1");
    expect(calls[1][0].pathname).toBe("/api/tables/ABC123/ready/v1");
    expect(calls[2][0].pathname).toBe("/api/tables/ABC123/start/v1");
  });

  it("decodes apiMeta camelCase fields", async () => {
    const metaClient = makeClient((() =>
      Promise.resolve(
        json({ version: "1.2.3", faithful_count: 662, oracle_total: 28412 }),
      )) as unknown as typeof fetch);
    await expect(Effect.runPromise(metaClient.apiMeta())).resolves.toEqual({
      version: "1.2.3",
      faithfulCount: 662,
      oracleTotal: 28412,
    });
  });

  it("decodes coverageMeta rows and null oracle totals", async () => {
    const coverageClient = makeClient((() =>
      Promise.resolve(
        json({
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
        }),
      )) as unknown as typeof fetch);
    await expect(Effect.runPromise(coverageClient.coverageMeta())).resolves.toEqual({
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
