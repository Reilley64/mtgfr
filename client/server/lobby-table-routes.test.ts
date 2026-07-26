import * as Effect from "effect/Effect";
import type { H3Event } from "nitro/h3";
import { beforeEach, describe, expect, it, vi } from "vitest";
import joinHandler from "./routes/api/tables/[table]/join/v1.post";
import lobbyHandler from "./routes/api/tables/[table]/lobby/v1.get";
import readyHandler from "./routes/api/tables/[table]/ready/v1.post";
import routeDeleteHandler from "./routes/api/tables/[table]/route/v1.delete";
import startHandler from "./routes/api/tables/[table]/start/v1.post";
import createHandler from "./routes/api/tables/v1.post";

const mocks = vi.hoisted(() => ({
  commitStart: vi.fn(),
  createLobby: vi.fn(),
  deleteTableRoute: vi.fn(),
  fetchDeckName: vi.fn(),
  gravatarHash: vi.fn(),
  joinLobby: vi.fn(),
  loadLobby: vi.fn(),
  seedGame: vi.fn(),
  setReady: vi.fn(),
  startError: vi.fn(),
  withLobbyAuth: vi.fn(),
}));

vi.mock("./lobby-http", () => ({
  json: (data: unknown, status = 200) =>
    new Response(JSON.stringify(data), {
      status,
      headers: { "content-type": "application/json" },
    }),
  readJsonObject: async (event: H3Event) => {
    try {
      return JSON.parse(await event.req.text()) as Record<string, unknown>;
    } catch {
      return null;
    }
  },
  tableParam: (event: H3Event) => {
    const table = event.context?.params?.table;
    return typeof table === "string" && table.length > 0 ? table : null;
  },
  unknownLobby: (tableId: string) => ({ tableId, hostUserId: 0, startedAt: null, seats: [] }),
  withLobbyAuth: mocks.withLobbyAuth,
}));

vi.mock("../app/domain/api-upstream-auth", () => ({
  fetchDeckName: mocks.fetchDeckName,
  seedGame: mocks.seedGame,
}));

vi.mock("../app/domain/gravatar", () => ({
  gravatarHash: mocks.gravatarHash,
}));

vi.mock("../app/domain/lobby-store", () => ({
  commitStart: mocks.commitStart,
  createLobby: mocks.createLobby,
  deleteTableRoute: mocks.deleteTableRoute,
  joinLobby: mocks.joinLobby,
  loadLobby: mocks.loadLobby,
  setReady: mocks.setReady,
  startError: mocks.startError,
  toLobbyView: (snap: { tableId: string; startedAt: Date | null }, _userId: number, error?: string | null) => ({
    table_id: snap.tableId,
    seats: [],
    you: null,
    started: snap.startedAt !== null,
    start_error: null,
    error: error ?? null,
  }),
}));

const db = { kind: "db" };
const env = { sessionToken: "session-token" };
const me = { id: 42, email: "player@example.test", username: "Player" };

function event(table: string | null, body: Record<string, unknown> = {}): H3Event {
  return {
    req: new Request("http://test.local", {
      method: "POST",
      body: JSON.stringify(body),
    }),
    context: { params: table === null ? {} : { table } },
  } as unknown as H3Event;
}

describe("lobby table route files", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.withLobbyAuth.mockImplementation(async (_event, _span, fn) => fn({ me, env, db }));
    mocks.createLobby.mockResolvedValue("NEWTBL");
    mocks.fetchDeckName.mockReturnValue(Effect.succeed("Mock Deck"));
    mocks.gravatarHash.mockResolvedValue("avatar-hash");
    mocks.joinLobby.mockResolvedValue({ snap: { tableId: "PATHID", hostUserId: me.id, startedAt: null, seats: [] } });
    mocks.setReady.mockResolvedValue({ snap: { tableId: "PATHID", hostUserId: me.id, startedAt: null, seats: [] } });
    mocks.loadLobby.mockResolvedValue({
      tableId: "PATHID",
      hostUserId: me.id,
      startedAt: null,
      seats: [
        {
          seat: 0,
          userId: me.id,
          username: me.username,
          gravatarHash: "avatar-hash",
          deckId: 7,
          deckName: "Mock Deck",
          ready: true,
        },
        {
          seat: 1,
          userId: 99,
          username: "Friend",
          gravatarHash: "friend-hash",
          deckId: 8,
          deckName: "Other Deck",
          ready: true,
        },
      ],
    });
    mocks.startError.mockReturnValue(null);
    mocks.seedGame.mockReturnValue(Effect.succeed({ ok: true, data: { pod_dns: "pod.local" } }));
  });

  it("exports all table handlers", () => {
    expect(createHandler).toBeTypeOf("function");
    expect(lobbyHandler).toBeTypeOf("function");
    expect(routeDeleteHandler).toBeTypeOf("function");
    expect(joinHandler).toBeTypeOf("function");
    expect(readyHandler).toBeTypeOf("function");
    expect(startHandler).toBeTypeOf("function");
  });

  it("uses the path table id for join, ready, and start", async () => {
    await joinHandler(event("PATHID", { table_id: "BODYID", deck_id: 7 }));
    await readyHandler(event("PATHID", { table_id: "BODYID", ready: true }));
    await startHandler(event("PATHID", { table_id: "BODYID" }));

    expect(mocks.joinLobby).toHaveBeenCalledWith(
      db,
      expect.objectContaining({ tableId: "PATHID", userId: me.id, deckId: 7 }),
    );
    expect(mocks.setReady).toHaveBeenCalledWith(db, "PATHID", me.id, true);
    expect(mocks.seedGame).toHaveBeenCalledWith(env, expect.objectContaining({ table_id: "PATHID" }));
    expect(mocks.commitStart).toHaveBeenCalledWith(db, "PATHID", "pod.local");
  });
});
