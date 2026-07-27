// `/api/rpc` dispatcher tests with `grpcClient` mocked.

import * as Effect from "effect/Effect";
import * as Stream from "effect/Stream";
import { beforeEach, describe, expect, it, vi } from "vitest";

class MockGrpcCallError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

const calls: Record<string, unknown> = {};
const mockClient = {
  auth: {
    signup: vi.fn((req: unknown) =>
      Effect.succeed({
        me: { id: 1, email: "a@b.c", username: "a" },
        sessionToken: "tok",
      }).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.signup = req;
          }),
        ),
      ),
    ),
    login: vi.fn((req: unknown) =>
      Effect.succeed({
        me: { id: 1, email: "a@b.c", username: "a" },
        sessionToken: "tok",
      }).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.login = req;
          }),
        ),
      ),
    ),
    logout: vi.fn(() => Effect.void),
    getMe: vi.fn(() => Effect.succeed({ id: 1, email: "a@b.c", username: "a" })),
  },
  decks: {
    create: vi.fn(
      (req: unknown): Effect.Effect<{ id: number; name: string }, MockGrpcCallError> =>
        Effect.succeed({ id: 1, name: "Deck" }).pipe(
          Effect.tap(() =>
            Effect.sync(() => {
              calls.create = req;
            }),
          ),
        ),
    ),
    list: vi.fn(() => Effect.succeed([{ id: 1, name: "Deck" }])),
    get: vi.fn((id: number) => Effect.succeed({ id, name: "Deck" })),
    update: vi.fn((id: number, req: unknown) =>
      Effect.succeed({ id, name: "Deck" }).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.update = { id, req };
          }),
        ),
      ),
    ),
    delete: vi.fn(() => Effect.void),
  },
  cards: {
    catalog: vi.fn(() => Effect.succeed([])),
    search: vi.fn((q: string, limit: number, offset: number) =>
      Effect.succeed([]).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.search = { q, limit, offset };
          }),
        ),
      ),
    ),
    lookup: vi.fn((ids: string[]) =>
      Effect.succeed([]).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.lookup = ids;
          }),
        ),
      ),
    ),
  },
  ratings: {
    getLeaderboard: vi.fn((req: { limit: number; offset: number }) =>
      Effect.succeed({
        entries: [{ user_id: 7, username: "alice", rating: 1234, rank: 26 }],
        total: 99,
      }).pipe(
        Effect.tap(() =>
          Effect.sync(() => {
            calls.leaderboard = req;
          }),
        ),
      ),
    ),
  },
  game: {
    submitIntent: vi.fn(() => Effect.succeed({ accepted: true })),
    setYield: vi.fn(() => Effect.succeed({ accepted: true })),
    setTurnYield: vi.fn(() => Effect.succeed({ accepted: true })),
    setStackDwell: vi.fn(() => Effect.succeed({ accepted: true })),
    stream: vi.fn(),
  },
  tables: { seed: vi.fn() },
};

vi.mock("./grpcClient", () => ({
  grpcClient: () => mockClient,
  grpcClientFor: () => mockClient,
  GrpcCallError: MockGrpcCallError,
  httpStatusOf: (code: string) => {
    if (code === "invalid_argument") return 422;
    if (code === "not_found") return 404;
    if (code === "unauthenticated") return 401;
    return 500;
  },
}));

const { dispatchRpc } = await import("./rpcServer");

function runDispatchRpc(...args: Parameters<typeof dispatchRpc>) {
  return Effect.runPromise(dispatchRpc(...args));
}

const env = {
  sessionToken: "tok",
  traceparent: null as string | null,
  defaultAddress: "127.0.0.1:50051",
  resolveTableAddress: vi.fn(async (tableId: string) => (tableId === "unknown" ? null : "pod:50051")),
};

beforeEach(() => {
  for (const key of Object.keys(calls)) delete calls[key];
});

describe("dispatchRpc", () => {
  it("404s an unknown group", async () => {
    const outcome = await runDispatchRpc(["bogus"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 404 });
  });

  it("404s an unknown auth method", async () => {
    const outcome = await runDispatchRpc(["auth", "bogus"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 404 });
  });

  it("routes auth/login and carries the minted session token back for the route to Set-Cookie", async () => {
    const outcome = await runDispatchRpc(
      ["auth", "login"],
      "POST",
      { email: "a@b.c", password: "pw" },
      new URLSearchParams(),
      env,
    );
    expect(outcome.kind).toBe("json");
    expect(outcome).toMatchObject({ status: 200, setSessionToken: "tok" });
    expect(calls.login).toEqual({ email: "a@b.c", password: "pw" });
  });

  it("routes auth/logout and signals the route to clear the cookie", async () => {
    const outcome = await runDispatchRpc(["auth", "logout"], "POST", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 204, clearSession: true });
  });

  it("routes decks list (GET, no id) vs. create (POST, no id) by HTTP method", async () => {
    const list = await runDispatchRpc(["decks"], "GET", undefined, new URLSearchParams(), env);
    expect(list).toMatchObject({ kind: "json", status: 200, body: [{ id: 1, name: "Deck" }] });

    await runDispatchRpc(["decks"], "POST", { name: "Deck" }, new URLSearchParams(), env);
    expect(calls.create).toEqual({ name: "Deck" });
  });

  it("routes decks/:id get vs. update vs. delete by HTTP method", async () => {
    const got = await runDispatchRpc(["decks", "5"], "GET", undefined, new URLSearchParams(), env);
    expect(got).toMatchObject({ kind: "json", status: 200, body: { id: 5, name: "Deck" } });

    await runDispatchRpc(["decks", "5"], "PUT", { name: "Renamed" }, new URLSearchParams(), env);
    expect(calls.update).toEqual({ id: 5, req: { name: "Renamed" } });

    const deleted = await runDispatchRpc(["decks", "5"], "DELETE", undefined, new URLSearchParams(), env);
    expect(deleted).toEqual({ kind: "empty", status: 204 });
  });

  it("reconstructs DeckError.problems from decks_svc.rs's folded 'illegal deck: a; b' status message", async () => {
    mockClient.decks.create.mockReturnValueOnce(
      Effect.fail(new MockGrpcCallError("invalid_argument", "illegal deck: Too many cards; Illegal commander")),
    );
    const outcome = await runDispatchRpc(["decks"], "POST", { name: "Deck" }, new URLSearchParams(), env);
    expect(outcome).toEqual({
      kind: "json",
      status: 422,
      body: { problems: ["Too many cards", "Illegal commander"] },
    });
  });

  it("routes cards/search with q/limit/offset from the query string", async () => {
    const params = new URLSearchParams({ q: "goblin", limit: "10", offset: "20" });
    await runDispatchRpc(["cards", "search"], "GET", undefined, params, env);
    expect(calls.search).toEqual({ q: "goblin", limit: 10, offset: 20 });
  });

  it("routes cards/lookup with every repeated ids param", async () => {
    const params = new URLSearchParams();
    params.append("ids", "a");
    params.append("ids", "b");
    await runDispatchRpc(["cards", "lookup"], "GET", undefined, params, env);
    expect(calls.lookup).toEqual(["a", "b"]);
  });

  it("routes ratings/leaderboard with limit/offset from the query string", async () => {
    const params = new URLSearchParams({ limit: "25", offset: "25" });
    const outcome = await runDispatchRpc(["ratings", "leaderboard"], "GET", undefined, params, env);
    expect(outcome).toEqual({
      kind: "json",
      status: 200,
      body: {
        entries: [{ user_id: 7, username: "alice", rating: 1234, rank: 26 }],
        total: 99,
      },
    });
    expect(calls.leaderboard).toEqual({ limit: 25, offset: 25 });
  });

  it("405s ratings/leaderboard for non-GET methods", async () => {
    const outcome = await runDispatchRpc(["ratings", "leaderboard"], "POST", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 405 });
  });

  it("400s ratings/leaderboard for invalid limit/offset query params", async () => {
    for (const params of [
      new URLSearchParams({ limit: "-1" }),
      new URLSearchParams({ limit: "abc" }),
      new URLSearchParams({ limit: "10.5" }),
      new URLSearchParams({ offset: "-5" }),
      new URLSearchParams({ limit: "10", offset: "NaN" }),
      new URLSearchParams({ limit: "4294967296" }),
      new URLSearchParams({ offset: "4294967296" }),
    ]) {
      const outcome = await runDispatchRpc(["ratings", "leaderboard"], "GET", undefined, params, env);
      expect(outcome).toEqual({ kind: "json", status: 400, body: { error: "BadQuery" } });
    }
    expect(calls.leaderboard).toBeUndefined();
  });

  it("defaults ratings/leaderboard limit/offset to 0 when query params are missing", async () => {
    await runDispatchRpc(["ratings", "leaderboard"], "GET", undefined, new URLSearchParams(), env);
    expect(calls.leaderboard).toEqual({ limit: 0, offset: 0 });
  });

  it("resolves the table's pod address for game calls and 404s an unresolvable table", async () => {
    const outcome = await runDispatchRpc(
      ["game", "ABC123", "intent"],
      "POST",
      { table_id: "ABC123" },
      new URLSearchParams(),
      env,
    );
    expect(outcome).toMatchObject({ kind: "json", status: 200 });

    const unknown = await runDispatchRpc(["game", "unknown", "intent"], "POST", {}, new URLSearchParams(), env);
    expect(unknown).toEqual({ kind: "empty", status: 404 });
  });

  it("streams game/:table/stream instead of returning json", async () => {
    mockClient.game.stream.mockReturnValueOnce(Stream.make({ frame: "heartbeat" }));
    const outcome = await runDispatchRpc(["game", "ABC123", "stream"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome.kind).toBe("stream");
  });
});
