import * as Effect from "effect/Effect";
import * as Layer from "effect/Layer";
import type { H3Event } from "nitro/h3";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { json, readJsonObject, runMetaGet, tableParam, unknownLobby, withLobbyAuth } from "./lobby-http";

const mocks = vi.hoisted(() => ({
  fetchMe: vi.fn(),
  grpcRequestEnv: vi.fn(),
  runTracedRequest: vi.fn(),
  sweepWebDb: vi.fn(),
}));

vi.mock("../app/domain/api-upstream-auth", () => ({
  fetchMe: mocks.fetchMe,
}));

vi.mock("../app/domain/lobby-store", () => ({
  sweepWebDb: mocks.sweepWebDb,
}));

vi.mock("../app/domain/otel", () => ({
  grpcRequestEnv: mocks.grpcRequestEnv,
  runTracedRequest: mocks.runTracedRequest,
}));

vi.mock("./db/client", () => ({
  WebDbLive: Layer.empty,
}));

const env = { sessionToken: "session-token" };
const me = { id: 42, email: "player@example.test", username: "Player" };

function eventWithBody(body: string): H3Event {
  return {
    req: { text: async () => body },
  } as unknown as H3Event;
}

function authEvent(): H3Event {
  return {
    req: new Request("http://test.local", { method: "POST" }),
    context: {},
  } as unknown as H3Event;
}

describe("lobby-http", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.fetchMe.mockReturnValue(Effect.succeed(me));
    mocks.grpcRequestEnv.mockReturnValue(Effect.succeed(env));
    mocks.runTracedRequest.mockImplementation((_traceparent, _spanName, body) => Effect.runPromise(body));
    mocks.sweepWebDb.mockReturnValue(Effect.void);
  });

  it("json sets content-type and status", async () => {
    const res = json({ ok: true }, 201);
    expect(res.status).toBe(201);
    expect(res.headers.get("content-type")).toBe("application/json");
    await expect(res.json()).resolves.toEqual({ ok: true });
  });

  it("tableParam reads event.context.params.table", () => {
    const event = {
      context: { params: { table: "ABC123" } },
    } as unknown as H3Event;
    expect(tableParam(event)).toBe("ABC123");
  });

  it("tableParam returns null when missing or empty", () => {
    expect(tableParam({ context: { params: {} } } as unknown as H3Event)).toBeNull();
    expect(tableParam({ context: {} } as unknown as H3Event)).toBeNull();
    expect(tableParam({ context: { params: { table: "" } } } as unknown as H3Event)).toBeNull();
  });

  it("readJsonObject parses valid JSON body", async () => {
    await expect(readJsonObject(eventWithBody('{"table_id":"T1","ready":true}'))).resolves.toEqual({
      table_id: "T1",
      ready: true,
    });
  });

  it("readJsonObject returns null on invalid JSON", async () => {
    await expect(readJsonObject(eventWithBody("not-json"))).resolves.toBeNull();
  });

  it.each(["[]", "123", "null"])("readJsonObject returns null for non-object JSON %s", async (body) => {
    await expect(readJsonObject(eventWithBody(body))).resolves.toBeNull();
  });

  it("unknownLobby returns empty snapshot with hostUserId 0", () => {
    expect(unknownLobby("T99")).toEqual({
      tableId: "T99",
      hostUserId: 0,
      startedAt: null,
      seats: [],
    });
  });

  it("withLobbyAuth returns Unauthorized when fetchMe returns null", async () => {
    mocks.fetchMe.mockReturnValueOnce(Effect.succeed(null));

    const res = await withLobbyAuth(authEvent(), "api test", () => Effect.succeed(json({ ok: true })));

    expect(res.status).toBe(401);
    await expect(res.text()).resolves.toBe("Unauthorized");
  });

  it("withLobbyAuth returns LobbyDb when the traced path throws", async () => {
    const message = "database offline";
    const res = await withLobbyAuth(authEvent(), "api test", () => Effect.fail(new Error(message)));

    expect(res.status).toBe(500);
    await expect(res.json()).resolves.toEqual({ error: "LobbyDb", message });
  });

  it("runMetaGet executes Effect response bodies", async () => {
    const res = await runMetaGet(authEvent(), "api meta test", () => Effect.succeed(json({ ok: true })));

    expect(res.status).toBe(200);
    await expect(res.json()).resolves.toEqual({ ok: true });
  });
});
