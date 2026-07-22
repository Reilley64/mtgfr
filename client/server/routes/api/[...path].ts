// BFF lobby + meta surface. Auth/decks/cards/game go through `/api/rpc/**`.

import * as Effect from "effect/Effect";
import {
  defineEventHandler,
  getCookie,
  getMethod,
  getRequestHeader,
  getRouterParam,
  type H3Event,
  readRawBody,
} from "nitro/h3";
import { normalizePublicApiPath } from "../../../lib/api-upstream";
import { fetchApiVersion, fetchDeckName, fetchMe, seedGame } from "../../../lib/api-upstream-auth";
import {
  commitStart,
  createLobby,
  deleteTableRoute,
  joinLobby,
  type LobbySnapshot,
  loadLobby,
  setReady,
  startError,
  sweepWebDb,
  toLobbyView,
} from "../../../lib/lobby-store";
import { grpcRequestEnv, runTracedRequest } from "../../../lib/otel";
import type { GrpcRequestEnv } from "../../../lib/wire/grpcClient";
import { createWebDb } from "../../db/client";

/** BFF session cookie — cookies terminate here; downstream calls use gRPC metadata. */
const SESSION_COOKIE = "session";

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function webDb() {
  return createWebDb();
}

function unknownLobby(tableId: string): LobbySnapshot {
  return { tableId, hostUserId: 0, startedAt: null, seats: [] };
}

function tableIdFromPath(path: string): string | null {
  const tableId = path.split("/")[1];
  return tableId && tableId.length > 0 ? tableId : null;
}

async function jsonBody(event: H3Event): Promise<Record<string, unknown>> {
  const raw = await readRawBody(event, "utf8");
  return JSON.parse(raw ?? "") as Record<string, unknown>;
}

async function handleLobby(event: H3Event, path: string, env: GrpcRequestEnv): Promise<Response | null> {
  const method = getMethod(event);

  if (method === "GET" && path === "meta/health/v1") {
    return json({ ok: true });
  }

  if (method === "GET" && path === "meta/version/v1") {
    const version = (await fetchApiVersion()) ?? "unknown";
    return json({ version });
  }

  const routeDelete = method === "DELETE" && /^tables\/[^/]+\/route\/v1$/.test(path);
  const isCreate = method === "POST" && path === "tables/v1";
  const isJoin = method === "POST" && path === "tables/join/v1";
  const isReady = method === "POST" && path === "tables/ready/v1";
  const isStart = method === "POST" && path === "tables/start/v1";
  const lobbyGet = method === "GET" && /^tables\/[^/]+\/lobby\/v1$/.test(path);

  if (!routeDelete && !isCreate && !isJoin && !isReady && !isStart && !lobbyGet) return null;

  const me = await fetchMe(env);
  if (!me) return new Response("Unauthorized", { status: 401 });

  const db = webDb();
  await sweepWebDb(db);

  if (routeDelete) {
    const tableId = tableIdFromPath(path);
    if (!tableId) return new Response("Not Found", { status: 404 });
    const snap = await loadLobby(db, tableId);
    if (snap && !snap.seats.some((s) => s.userId === me.id) && snap.hostUserId !== me.id) {
      return new Response("Forbidden", { status: 403 });
    }
    await deleteTableRoute(db, tableId);
    return new Response(null, { status: 204 });
  }

  if (isCreate) {
    const tableId = await createLobby(db, me.id);
    return json({ table_id: tableId });
  }

  if (lobbyGet) {
    const tableId = tableIdFromPath(path);
    if (!tableId) return new Response("Not Found", { status: 404 });
    const snap = await loadLobby(db, tableId);
    if (!snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    return json(toLobbyView(snap, me.id));
  }

  let body: Record<string, unknown>;
  try {
    body = await jsonBody(event);
  } catch {
    return json({ error: "BadJson" }, 400);
  }

  if (isJoin) {
    const tableId = String(body.table_id ?? "");
    const deckId = Number(body.deck_id);
    const deckName = await fetchDeckName(env, deckId);
    if (!deckName) {
      const snap = await loadLobby(db, tableId);
      if (!snap) {
        return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
      }
      return json(toLobbyView(snap, me.id, "UnknownDeck"));
    }
    const result = await joinLobby(db, {
      tableId,
      userId: me.id,
      username: me.username,
      deckId,
      deckName,
    });
    if (!result.snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, result.error), 404);
    }
    return json(toLobbyView(result.snap, me.id, result.error));
  }

  if (isReady) {
    const tableId = String(body.table_id ?? "");
    const ready = Boolean(body.ready);
    const result = await setReady(db, tableId, me.id, ready);
    if (!result.snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, result.error), 404);
    }
    return json(toLobbyView(result.snap, me.id, result.error));
  }

  if (isStart) {
    const tableId = String(body.table_id ?? "");
    const snap = await loadLobby(db, tableId);
    if (!snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    if (snap.startedAt) {
      return json(toLobbyView(snap, me.id));
    }
    const err = startError(snap, me.id);
    if (err) return json(toLobbyView(snap, me.id, err));

    const seeded = await seedGame(env, {
      table_id: tableId,
      host_user_id: snap.hostUserId,
      seats: snap.seats
        .slice()
        .sort((a, b) => a.seat - b.seat)
        .map((s) => ({
          user_id: s.userId,
          username: s.username,
          deck_id: s.deckId,
        })),
    });
    if (!seeded.ok) {
      return json(toLobbyView(snap, me.id, seeded.status === 503 ? "Draining" : "SeedFailed"));
    }
    try {
      await commitStart(db, tableId, seeded.data.pod_dns);
    } catch {
      return json(toLobbyView(snap, me.id, "SeedFailed"));
    }
    const started = await loadLobby(db, tableId);
    if (!started) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    return json(toLobbyView(started, me.id));
  }

  return null;
}

/**
 * Lobby/meta under the request span. Unnamed `Effect.fn` — stack traces without a
 * second named span; the edge opens `api <path>` via `runTracedRequest` / `withSpan`.
 */
const handleLobbyTraced = Effect.fn(function* (event: H3Event, path: string) {
  yield* Effect.annotateCurrentSpan({
    "http.method": getMethod(event),
    "http.route": path,
  });
  const sessionToken = getCookie(event, SESSION_COOKIE) ?? null;
  const env = yield* grpcRequestEnv(sessionToken);
  return yield* Effect.tryPromise({
    try: () => handleLobby(event, path, env),
    catch: (err) => (err instanceof Error ? err : new Error(String(err))),
  });
});

/** Nitro handler methods for lobby/meta. */
const ALLOWED_METHODS = new Set(["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]);

async function forward(event: H3Event) {
  const method = getMethod(event);
  if (!ALLOWED_METHODS.has(method)) {
    return new Response("Method Not Allowed", { status: 405 });
  }

  const path = normalizePublicApiPath(getRouterParam(event, "path") ?? "");
  if (path === null) {
    return new Response("Not Found", { status: 404 });
  }

  const lobby = await runTracedRequest(
    getRequestHeader(event, "traceparent") ?? null,
    `api ${path}`,
    handleLobbyTraced(event, path),
  );
  return lobby ?? new Response("Not Found", { status: 404 });
}

export default defineEventHandler(forward);
