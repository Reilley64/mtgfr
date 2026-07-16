import type { APIEvent } from "@solidjs/start/server";
import { getRequestHeader, getRequestURL, proxyRequest } from "vinxi/http";
import { createWebDb } from "~/db/client";
import { normalizePublicApiPath, tableIdFromGamePath, upstreamFromPodDns } from "~/lib/apiUpstream";
import { apiUpstream, fetchApiVersion, fetchDeckName, fetchMe, seedGame } from "~/lib/apiUpstreamAuth";
import {
  commitStart,
  createLobby,
  deleteTableRoute,
  joinLobby,
  type LobbySnapshot,
  loadLobby,
  lookupTableRoute,
  setReady,
  startError,
  sweepWebDb,
  toLobbyView,
} from "~/lib/lobbyStore";

function json(data: unknown, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function webDb() {
  return createWebDb(process.env.WEB_DATABASE_URL);
}

function unknownLobby(tableId: string): LobbySnapshot {
  return { tableId, hostUserId: 0, startedAt: null, seats: [] };
}

async function handleLobby(event: APIEvent, path: string): Promise<Response | null> {
  const method = event.request.method;
  const cookie = getRequestHeader(event.nativeEvent, "cookie") ?? null;

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

  if (!process.env.WEB_DATABASE_URL) {
    return json({ error: "WebDbNotConfigured" }, 503);
  }

  const me = await fetchMe(cookie);
  if (!me) return new Response("Unauthorized", { status: 401 });

  const db = webDb();
  await sweepWebDb(db);

  if (routeDelete) {
    const tableId = path.split("/")[1]!;
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
    const tableId = path.split("/")[1]!;
    const snap = await loadLobby(db, tableId);
    if (!snap) {
      return json(toLobbyView(unknownLobby(tableId), me.id, "UnknownTable"), 404);
    }
    return json(toLobbyView(snap, me.id));
  }

  let body: Record<string, unknown>;
  try {
    body = (await event.request.json()) as Record<string, unknown>;
  } catch {
    return json({ error: "BadJson" }, 400);
  }

  if (isJoin) {
    const tableId = String(body.table_id ?? "");
    const deckId = Number(body.deck_id);
    const deckName = await fetchDeckName(cookie, deckId);
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
    const err = startError(snap, me.id);
    if (err) return json(toLobbyView(snap, me.id, err));

    const seeded = await seedGame(cookie, {
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
    const started = (await loadLobby(db, tableId))!;
    return json(toLobbyView(started, me.id));
  }

  return null;
}

async function resolveGameUpstream(path: string): Promise<string | null> {
  const tableId = tableIdFromGamePath(path);
  if (!tableId) return null;
  if (!process.env.WEB_DATABASE_URL) return apiUpstream();
  const db = webDb();
  const pod = await lookupTableRoute(db, tableId);
  if (!pod) return null;
  return upstreamFromPodDns(pod);
}

async function forward(event: APIEvent) {
  const path = normalizePublicApiPath(event.params.path ?? "");
  if (path === null) {
    return new Response("Not Found", { status: 404 });
  }

  const lobby = await handleLobby(event, path);
  if (lobby) return lobby;

  const search = getRequestURL(event.nativeEvent).search;
  if (tableIdFromGamePath(path)) {
    const gameBase = await resolveGameUpstream(path);
    if (!gameBase) return new Response("UnknownTable", { status: 404 });
    return proxyRequest(event.nativeEvent, `${gameBase}/${path}${search}`);
  }

  return proxyRequest(event.nativeEvent, `${apiUpstream()}/${path}${search}`);
}

export const GET = forward;
export const HEAD = forward;
export const POST = forward;
export const PUT = forward;
export const PATCH = forward;
export const DELETE = forward;
export const OPTIONS = forward;
