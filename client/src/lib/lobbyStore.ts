import { and, eq, isNull, lt, sql } from "drizzle-orm";
import type { WebDb } from "~/db/client";
import { lobbies, lobbySeats, tableRoutes } from "../../db/schema";

const IDLE_LOBBY_MS = 30 * 60 * 1000;
const ROUTE_TTL_MS = 24 * 60 * 60 * 1000;

const CODE_ALPHABET = "23456789ABCDEFGHJKMNPQRSTUVWXYZ";

export function randomTableCode(): string {
  let out = "";
  for (let i = 0; i < 6; i++) {
    out += CODE_ALPHABET[Math.floor(Math.random() * CODE_ALPHABET.length)]!;
  }
  return out;
}

export type LobbySeatRow = {
  seat: number;
  userId: number;
  username: string;
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

export async function createLobby(db: WebDb, hostUserId: number): Promise<string> {
  for (let attempt = 0; attempt < 8; attempt++) {
    const tableId = randomTableCode();
    try {
      await db.insert(lobbies).values({ tableId, hostUserId });
      return tableId;
    } catch {
      // collision — retry
    }
  }
  throw new Error("Could not mint a unique table code");
}

export async function loadLobby(db: WebDb, tableId: string): Promise<LobbySnapshot | null> {
  const [lobby] = await db.select().from(lobbies).where(eq(lobbies.tableId, tableId)).limit(1);
  if (!lobby) return null;
  const seats = await db.select().from(lobbySeats).where(eq(lobbySeats.tableId, tableId));
  return {
    tableId: lobby.tableId,
    hostUserId: lobby.hostUserId,
    startedAt: lobby.startedAt,
    seats: seats.map((s) => ({
      seat: s.seat,
      userId: s.userId,
      username: s.username,
      deckId: s.deckId,
      deckName: s.deckName,
      ready: s.ready,
    })),
  };
}

export async function touchLobby(db: WebDb, tableId: string): Promise<void> {
  await db.update(lobbies).set({ lastActivity: sql`now()` }).where(eq(lobbies.tableId, tableId));
}

export async function joinLobby(
  db: WebDb,
  opts: {
    tableId: string;
    userId: number;
    username: string;
    deckId: number;
    deckName: string;
  },
): Promise<{ error?: string; snap?: LobbySnapshot }> {
  const snap = await loadLobby(db, opts.tableId);
  if (!snap) return { error: "UnknownTable" };
  if (snap.startedAt) return { error: "AlreadyStarted", snap };

  const existing = snap.seats.find((s) => s.userId === opts.userId);
  if (existing) {
    await db
      .update(lobbySeats)
      .set({
        deckId: opts.deckId,
        deckName: opts.deckName,
        username: opts.username,
      })
      .where(and(eq(lobbySeats.tableId, opts.tableId), eq(lobbySeats.seat, existing.seat)));
    await touchLobby(db, opts.tableId);
    return { snap: (await loadLobby(db, opts.tableId))! };
  }

  if (snap.seats.length >= 4) return { error: "TableFull", snap };

  const seat = snap.seats.length;
  await db.insert(lobbySeats).values({
    tableId: opts.tableId,
    seat,
    userId: opts.userId,
    username: opts.username,
    deckId: opts.deckId,
    deckName: opts.deckName,
    ready: false,
  });
  await touchLobby(db, opts.tableId);
  return { snap: (await loadLobby(db, opts.tableId))! };
}

export async function setReady(
  db: WebDb,
  tableId: string,
  userId: number,
  ready: boolean,
): Promise<{ error?: string; snap?: LobbySnapshot }> {
  const snap = await loadLobby(db, tableId);
  if (!snap) return { error: "UnknownTable" };
  const seat = snap.seats.find((s) => s.userId === userId);
  if (!seat) return { error: "NotSeated", snap };
  await db
    .update(lobbySeats)
    .set({ ready })
    .where(and(eq(lobbySeats.tableId, tableId), eq(lobbySeats.seat, seat.seat)));
  await touchLobby(db, tableId);
  return { snap: (await loadLobby(db, tableId))! };
}

export function startError(snap: LobbySnapshot, userId: number): string | null {
  if (snap.hostUserId !== userId) return "NotHost";
  if (snap.startedAt) return "AlreadyStarted";
  if (snap.seats.length < 2) return "NeedTwoPlayers";
  if (!snap.seats.every((s) => s.ready)) return "NotAllReady";
  return null;
}

export async function markStarted(db: WebDb, tableId: string): Promise<void> {
  await db.update(lobbies).set({ startedAt: sql`now()` }).where(eq(lobbies.tableId, tableId));
}

export async function putTableRoute(db: WebDb, tableId: string, podDns: string): Promise<void> {
  const expiresAt = new Date(Date.now() + ROUTE_TTL_MS);
  await db
    .insert(tableRoutes)
    .values({ tableId, podDns, expiresAt })
    .onConflictDoUpdate({
      target: tableRoutes.tableId,
      set: { podDns, expiresAt, createdAt: sql`now()` },
    });
}

export async function lookupTableRoute(db: WebDb, tableId: string): Promise<string | null> {
  const [row] = await db.select().from(tableRoutes).where(eq(tableRoutes.tableId, tableId)).limit(1);
  if (!row) return null;
  if (row.expiresAt.getTime() < Date.now()) {
    await db.delete(tableRoutes).where(eq(tableRoutes.tableId, tableId));
    return null;
  }
  return row.podDns;
}

export async function deleteTableRoute(db: WebDb, tableId: string): Promise<void> {
  await db.delete(tableRoutes).where(eq(tableRoutes.tableId, tableId));
}

export async function sweepExpiredRoutes(db: WebDb): Promise<void> {
  await db.delete(tableRoutes).where(lt(tableRoutes.expiresAt, new Date()));
}

export async function sweepIdleLobbies(db: WebDb): Promise<void> {
  const cutoff = new Date(Date.now() - IDLE_LOBBY_MS);
  await db.delete(lobbies).where(and(isNull(lobbies.startedAt), lt(lobbies.lastActivity, cutoff)));
}

/** Opportunistic GC: idle lobbies + expired routes (safe to call on lobby traffic). */
export async function sweepWebDb(db: WebDb): Promise<void> {
  await sweepIdleLobbies(db);
  await sweepExpiredRoutes(db);
}

export function toLobbyView(
  snap: LobbySnapshot,
  userId: number | null,
  error?: string | null,
): {
  table_id: string;
  seats: Array<{
    player: number;
    claimed: boolean;
    username: string | null;
    deck_name: string | null;
    ready: boolean;
    is_host: boolean;
    is_you: boolean;
  }>;
  you: number | null;
  started: boolean;
  start_error: string | null;
  error: string | null;
} {
  const you = userId == null ? null : (snap.seats.find((s) => s.userId === userId)?.seat ?? null);
  const seats = Array.from({ length: 4 }, (_, i) => {
    const s = snap.seats.find((x) => x.seat === i);
    return {
      player: i,
      claimed: !!s,
      username: s?.username ?? null,
      deck_name: s?.deckName ?? null,
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
    start_error: userId == null ? "NotSeated" : you == null ? "NotSeated" : startError(snap, userId),
    error: error ?? null,
  };
}
