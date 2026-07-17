// `/api/rpc` dispatcher tests with `grpcClient` mocked.

import { beforeEach, describe, expect, it, vi } from "vitest";

const calls: Record<string, unknown> = {};
const mockClient = {
  auth: {
    signup: vi.fn(async (req: unknown) => {
      calls.signup = req;
      return { me: { id: 1, email: "a@b.c", username: "a" }, sessionToken: "tok" };
    }),
    login: vi.fn(async (req: unknown) => {
      calls.login = req;
      return { me: { id: 1, email: "a@b.c", username: "a" }, sessionToken: "tok" };
    }),
    logout: vi.fn(async () => {}),
    getMe: vi.fn(async () => ({ id: 1, email: "a@b.c", username: "a" })),
  },
  decks: {
    create: vi.fn(async (req: unknown) => {
      calls.create = req;
      return { id: 1, name: "Deck" };
    }),
    list: vi.fn(async () => [{ id: 1, name: "Deck" }]),
    get: vi.fn(async (id: number) => ({ id, name: "Deck" })),
    update: vi.fn(async (id: number, req: unknown) => {
      calls.update = { id, req };
      return { id, name: "Deck" };
    }),
    delete: vi.fn(async () => {}),
  },
  cards: {
    catalog: vi.fn(async () => []),
    search: vi.fn(async (q: string, limit: number, offset: number) => {
      calls.search = { q, limit, offset };
      return [];
    }),
    lookup: vi.fn(async (ids: string[]) => {
      calls.lookup = ids;
      return [];
    }),
  },
  game: {
    submitIntent: vi.fn(async () => ({ accepted: true })),
    setYield: vi.fn(async () => ({ accepted: true })),
    setTurnYield: vi.fn(async () => ({ accepted: true })),
    setStackDwell: vi.fn(async () => ({ accepted: true })),
    stream: vi.fn(),
  },
  tables: { seed: vi.fn() },
};

class MockGrpcCallError extends Error {
  code: string;
  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

vi.mock("~/wire/grpcClient", () => ({
  grpcClient: () => mockClient,
  GrpcCallError: MockGrpcCallError,
  httpStatusOf: (code: string) => {
    if (code === "invalid_argument") return 422;
    if (code === "not_found") return 404;
    if (code === "unauthenticated") return 401;
    return 500;
  },
}));

const { dispatchRpc } = await import("~/wire/rpcServer");

const env = {
  sessionToken: "tok",
  defaultAddress: "127.0.0.1:50051",
  resolveTableAddress: vi.fn(async (tableId: string) => (tableId === "unknown" ? null : "pod:50051")),
};

beforeEach(() => {
  for (const key of Object.keys(calls)) delete calls[key];
});

describe("dispatchRpc", () => {
  it("404s an unknown group", async () => {
    const outcome = await dispatchRpc(["bogus"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 404 });
  });

  it("404s an unknown auth method", async () => {
    const outcome = await dispatchRpc(["auth", "bogus"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 404 });
  });

  it("routes auth/login and carries the minted session token back for the route to Set-Cookie", async () => {
    const outcome = await dispatchRpc(
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
    const outcome = await dispatchRpc(["auth", "logout"], "POST", undefined, new URLSearchParams(), env);
    expect(outcome).toEqual({ kind: "empty", status: 204, clearSession: true });
  });

  it("routes decks list (GET, no id) vs. create (POST, no id) by HTTP method", async () => {
    const list = await dispatchRpc(["decks"], "GET", undefined, new URLSearchParams(), env);
    expect(list).toMatchObject({ kind: "json", status: 200, body: [{ id: 1, name: "Deck" }] });

    await dispatchRpc(["decks"], "POST", { name: "Deck" }, new URLSearchParams(), env);
    expect(calls.create).toEqual({ name: "Deck" });
  });

  it("routes decks/:id get vs. update vs. delete by HTTP method", async () => {
    const got = await dispatchRpc(["decks", "5"], "GET", undefined, new URLSearchParams(), env);
    expect(got).toMatchObject({ kind: "json", status: 200, body: { id: 5, name: "Deck" } });

    await dispatchRpc(["decks", "5"], "PUT", { name: "Renamed" }, new URLSearchParams(), env);
    expect(calls.update).toEqual({ id: 5, req: { name: "Renamed" } });

    const deleted = await dispatchRpc(["decks", "5"], "DELETE", undefined, new URLSearchParams(), env);
    expect(deleted).toEqual({ kind: "empty", status: 204 });
  });

  it("reconstructs DeckError.problems from decks_svc.rs's folded 'illegal deck: a; b' status message", async () => {
    mockClient.decks.create.mockRejectedValueOnce(
      new MockGrpcCallError("invalid_argument", "illegal deck: Too many cards; Illegal commander"),
    );
    const outcome = await dispatchRpc(["decks"], "POST", { name: "Deck" }, new URLSearchParams(), env);
    expect(outcome).toEqual({
      kind: "json",
      status: 422,
      body: { problems: ["Too many cards", "Illegal commander"] },
    });
  });

  it("routes cards/search with q/limit/offset from the query string", async () => {
    const params = new URLSearchParams({ q: "goblin", limit: "10", offset: "20" });
    await dispatchRpc(["cards", "search"], "GET", undefined, params, env);
    expect(calls.search).toEqual({ q: "goblin", limit: 10, offset: 20 });
  });

  it("routes cards/lookup with every repeated ids param", async () => {
    const params = new URLSearchParams();
    params.append("ids", "a");
    params.append("ids", "b");
    await dispatchRpc(["cards", "lookup"], "GET", undefined, params, env);
    expect(calls.lookup).toEqual(["a", "b"]);
  });

  it("resolves the table's pod address for game calls and 404s an unresolvable table", async () => {
    const outcome = await dispatchRpc(
      ["game", "ABC123", "intent"],
      "POST",
      { table_id: "ABC123" },
      new URLSearchParams(),
      env,
    );
    expect(outcome).toMatchObject({ kind: "json", status: 200 });

    const unknown = await dispatchRpc(["game", "unknown", "intent"], "POST", {}, new URLSearchParams(), env);
    expect(unknown).toEqual({ kind: "empty", status: 404 });
  });

  it("streams game/:table/stream instead of returning json", async () => {
    async function* frames() {
      yield { frame: "heartbeat" as const };
    }
    mockClient.game.stream.mockReturnValueOnce(frames());
    const outcome = await dispatchRpc(["game", "ABC123", "stream"], "GET", undefined, new URLSearchParams(), env);
    expect(outcome.kind).toBe("stream");
  });
});
