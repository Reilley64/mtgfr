import { and, eq, isNull, lt, sql } from "drizzle-orm";
import type { EffectDrizzleQueryError } from "drizzle-orm/effect-core/errors";
import * as Effect from "effect/Effect";
import * as Result from "effect/Result";
import { lobbies, lobbySeats, tableRoutes } from "../../db/schema";
import { WebDb } from "../../server/db/client";
import type { LobbyView } from "./lobby-types";

const IDLE_LOBBY_MS = 30 * 60 * 1000;
const ROUTE_TTL_MS = 24 * 60 * 60 * 1000;

const CODE_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";

export function randomTableCode(): string {
  while (true) {
    const bytes = new Uint8Array(6);
    crypto.getRandomValues(bytes);
    let out = "";
    let hasLetter = false;
    for (const b of bytes) {
      const char = CODE_ALPHABET.charAt(b % CODE_ALPHABET.length);
      out += char;
      if (char >= "A" && char <= "Z") hasLetter = true;
    }
    if (hasLetter) return out;
  }
}

export type LobbySeatRow = {
  seat: number;
  userId: number;
  username: string;
  gravatarHash: string;
  deckId: number;
  deckName: string;
  ready: boolean;
};

export type LobbySnapshot = {
  tableId: string;
  hostUserId: number;
  startedAt: Date | null;
  seats: LobbySeatRow[];
};

export type LobbyMutation = { error?: string; snap?: LobbySnapshot };

function messageChain(err: unknown): string {
  let out = "";
  let current: unknown = err;
  for (let depth = 0; current != null && depth < 6; depth++) {
    out += current instanceof Error ? `${current.message} ` : `${String(current)} `;
    current = current instanceof Error ? current.cause : undefined;
  }
  return out;
}

function isUniqueViolation(err: unknown): boolean {
  // Postgres unique_violation (23505); effect-sql classifies it as a `UniqueViolation` reason.
  // Walk the cause chain (drizzle query error → SqlError → reason → driver error).
  const msg = messageChain(err);
  return msg.includes("23505") || /duplicate key|unique constraint|UniqueViolation/i.test(msg);
}

/** Visible for unit tests — collision detection must not swallow unrelated insert failures. */
export function createLobbyTreatsAsCollision(err: unknown): boolean {
  return isUniqueViolation(err);
}

export const createLobby = Effect.fn(function* (hostUserId: number) {
  const db = yield* WebDb;
  let lastError: unknown;
  for (let attempt = 0; attempt < 8; attempt++) {
    const tableId = randomTableCode();
    const inserted = yield* Effect.result(db.insert(lobbies).values({ tableId, hostUserId }));
    if (Result.isSuccess(inserted)) return tableId;
    lastError = inserted.failure;
    // primary-key collision on table_id — retry with a fresh code; re-raise anything else
    if (!isUniqueViolation(inserted.failure)) return yield* Effect.fail(inserted.failure);
  }
  return yield* Effect.fail(new Error("Could not mint a unique table code", { cause: lastError }));
});

export const loadLobby = Effect.fn(function* (tableId: string) {
  const db = yield* WebDb;
  const [lobby] = yield* db.select().from(lobbies).where(eq(lobbies.tableId, tableId)).limit(1);
  if (!lobby) return null;
  const seats = yield* db.select().from(lobbySeats).where(eq(lobbySeats.tableId, tableId));
  const snap: LobbySnapshot = {
    tableId: lobby.tableId,
    hostUserId: lobby.hostUserId,
    startedAt: lobby.startedAt,
    seats: seats.map((s) => ({
      seat: s.seat,
      userId: s.userId,
      username: s.username,
      gravatarHash: s.gravatarHash,
      deckId: s.deckId,
      deckName: s.deckName,
      ready: s.ready,
    })),
  };
  return snap;
});

export const touchLobby = Effect.fn(function* (tableId: string) {
  const db = yield* WebDb;
  yield* db.update(lobbies).set({ lastActivity: sql`now()` }).where(eq(lobbies.tableId, tableId));
});

export const joinLobby: (opts: {
  tableId: string;
  userId: number;
  username: string;
  gravatarHash: string;
  deckId: number;
  deckName: string;
}) => Effect.Effect<LobbyMutation, EffectDrizzleQueryError, WebDb> = Effect.fn(function* (opts: {
  tableId: string;
  userId: number;
  username: string;
  gravatarHash: string;
  deckId: number;
  deckName: string;
}) {
  const db = yield* WebDb;
  const snap = yield* loadLobby(opts.tableId);
  if (!snap) return { error: "UnknownTable" };
  if (snap.startedAt) return { error: "AlreadyStarted", snap };

  const existing = snap.seats.find((s) => s.userId === opts.userId);
  if (existing) {
    yield* db
      .update(lobbySeats)
      .set({
        deckId: opts.deckId,
        deckName: opts.deckName,
        username: opts.username,
        gravatarHash: opts.gravatarHash,
      })
      .where(and(eq(lobbySeats.tableId, opts.tableId), eq(lobbySeats.seat, existing.seat)));
    yield* touchLobby(opts.tableId);
    const updated = yield* loadLobby(opts.tableId);
    if (!updated) return { error: "UnknownTable" };
    return { snap: updated };
  }

  if (snap.seats.length >= 4) return { error: "TableFull", snap };

  const seat = snap.seats.length;
  const inserted = yield* Effect.result(
    db.insert(lobbySeats).values({
      tableId: opts.tableId,
      seat,
      userId: opts.userId,
      username: opts.username,
      gravatarHash: opts.gravatarHash,
      deckId: opts.deckId,
      deckName: opts.deckName,
      ready: false,
    }),
  );
  if (Result.isFailure(inserted)) {
    // Unique seat/user race — no surrounding transaction, so re-read and reconcile.
    const again = yield* loadLobby(opts.tableId);
    if (!again) return { error: "UnknownTable" };
    if (again.seats.some((s) => s.userId === opts.userId)) return { snap: again };
    return { error: "TableFull", snap: again };
  }
  yield* touchLobby(opts.tableId);
  const joined = yield* loadLobby(opts.tableId);
  if (!joined) return { error: "UnknownTable" };
  return { snap: joined };
});

export const setReady: (
  tableId: string,
  userId: number,
  ready: boolean,
) => Effect.Effect<LobbyMutation, EffectDrizzleQueryError, WebDb> = Effect.fn(function* (
  tableId: string,
  userId: number,
  ready: boolean,
) {
  const db = yield* WebDb;
  const snap = yield* loadLobby(tableId);
  if (!snap) return { error: "UnknownTable" };
  const seat = snap.seats.find((s) => s.userId === userId);
  if (!seat) return { error: "NotSeated", snap };
  yield* db
    .update(lobbySeats)
    .set({ ready })
    .where(and(eq(lobbySeats.tableId, tableId), eq(lobbySeats.seat, seat.seat)));
  yield* touchLobby(tableId);
  const updated = yield* loadLobby(tableId);
  if (!updated) return { error: "UnknownTable" };
  return { snap: updated };
});

export function startError(snap: LobbySnapshot, userId: number): string | null {
  if (snap.hostUserId !== userId) return "NotHost";
  if (!snap.seats.some((s) => s.userId === userId)) return "NotSeated";
  if (snap.seats.length < 2) return "NeedTwoPlayers";
  if (!snap.seats.every((s) => s.ready)) return "NotAllReady";
  return null;
}

export const markStarted = Effect.fn(function* (tableId: string) {
  const db = yield* WebDb;
  yield* db.update(lobbies).set({ startedAt: sql`now()` }).where(eq(lobbies.tableId, tableId));
});

export const putTableRoute = Effect.fn(function* (tableId: string, podDns: string) {
  const db = yield* WebDb;
  const expiresAt = new Date(Date.now() + ROUTE_TTL_MS);
  yield* db
    .insert(tableRoutes)
    .values({ tableId, podDns, expiresAt })
    .onConflictDoUpdate({
      target: tableRoutes.tableId,
      set: { podDns, expiresAt, createdAt: sql`now()` },
    });
});

/** Route then mark started; roll back the route if mark fails (no surrounding transaction). */
export const commitStart = Effect.fn(function* (tableId: string, podDns: string) {
  yield* putTableRoute(tableId, podDns);
  const marked = yield* Effect.result(markStarted(tableId));
  if (Result.isSuccess(marked)) return;
  yield* deleteTableRoute(tableId);
  return yield* Effect.fail(marked.failure);
});

export const lookupTableRoute = Effect.fn(function* (tableId: string) {
  const db = yield* WebDb;
  const [row] = yield* db.select().from(tableRoutes).where(eq(tableRoutes.tableId, tableId)).limit(1);
  if (!row) return null;
  if (row.expiresAt.getTime() < Date.now()) {
    yield* db.delete(tableRoutes).where(eq(tableRoutes.tableId, tableId));
    return null;
  }
  const expiresAt = new Date(Date.now() + ROUTE_TTL_MS);
  yield* db.update(tableRoutes).set({ expiresAt }).where(eq(tableRoutes.tableId, tableId));
  return row.podDns;
});

export const deleteTableRoute = Effect.fn(function* (tableId: string) {
  const db = yield* WebDb;
  yield* db.delete(tableRoutes).where(eq(tableRoutes.tableId, tableId));
});

export const sweepExpiredRoutes = Effect.fn(function* () {
  const db = yield* WebDb;
  yield* db.delete(tableRoutes).where(lt(tableRoutes.expiresAt, new Date()));
});

export const sweepIdleLobbies = Effect.fn(function* () {
  const db = yield* WebDb;
  const cutoff = new Date(Date.now() - IDLE_LOBBY_MS);
  yield* db.delete(lobbies).where(and(isNull(lobbies.startedAt), lt(lobbies.lastActivity, cutoff)));
});

export const sweepWebDb = Effect.fn(function* () {
  yield* sweepIdleLobbies();
  yield* sweepExpiredRoutes();
});

export function toLobbyView(snap: LobbySnapshot, userId: number | null, error?: string | null): LobbyView {
  const you = userId == null ? null : (snap.seats.find((s) => s.userId === userId)?.seat ?? null);
  const seats = Array.from({ length: 4 }, (_, i) => {
    const s = snap.seats.find((x) => x.seat === i);
    return {
      player: i,
      claimed: !!s,
      username: s?.username ?? null,
      gravatar_hash: s?.gravatarHash ?? null,
      deck_name: s?.deckName ?? null,
      deck_id: s?.deckId ?? null,
      ready: s?.ready ?? false,
      is_host: !!s && s.userId === snap.hostUserId,
      is_you: you === i,
    };
  });
  return {
    table_id: snap.tableId,
    seats,
    you,
    started: snap.startedAt != null,
    start_error: snap.startedAt != null ? null : userId == null || you == null ? "NotSeated" : startError(snap, userId),
    error: error ?? null,
  };
}
