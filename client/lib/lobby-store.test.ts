import { eq } from "drizzle-orm";
import { afterEach, describe, expect, it } from "vitest";
import { lobbies } from "../db/schema";
import { createWebDb } from "../server/db/client";
import {
  createLobby,
  joinLobby,
  type LobbySnapshot,
  loadLobby,
  markStarted,
  setCommanderDamageEnabled,
  startError,
  toLobbyView,
} from "./lobby-store";

function snap(overrides: Partial<LobbySnapshot> = {}): LobbySnapshot {
  return {
    tableId: "ABC123",
    hostUserId: 1,
    startedAt: null,
    commanderDamageEnabled: true,
    seats: [
      {
        seat: 0,
        userId: 1,
        username: "alice",
        gravatarHash: "abc",
        deckId: -1,
        deckName: "Silverquill Influence",
        ready: true,
      },
      {
        seat: 1,
        userId: 2,
        username: "bob",
        gravatarHash: "def",
        deckId: -2,
        deckName: "Prismari Artistry",
        ready: true,
      },
    ],
    ...overrides,
  };
}

describe("toLobbyView", () => {
  it("projects a started lobby with start_error null", () => {
    const view = toLobbyView(snap({ startedAt: new Date("2026-07-22T00:00:00Z") }), 1);
    expect(view.started).toBe(true);
    expect(view.start_error).toBeNull();
    expect(view.error).toBeNull();
  });

  it("still reports pre-start gates when not started", () => {
    const notReady = snap({
      seats: [
        {
          seat: 0,
          userId: 1,
          username: "alice",
          gravatarHash: "abc",
          deckId: -1,
          deckName: "Silverquill Influence",
          ready: true,
        },
        {
          seat: 1,
          userId: 2,
          username: "bob",
          gravatarHash: "def",
          deckId: -2,
          deckName: "Prismari Artistry",
          ready: false,
        },
      ],
    });
    expect(toLobbyView(notReady, 1).start_error).toBe("NotAllReady");
  });

  it("projects gravatar_hash on claimed seats", () => {
    const view = toLobbyView(snap(), 1);

    expect(view.seats[0]?.gravatar_hash).toBe("abc");
  });

  it("projects commander_damage_enabled on LobbyView", () => {
    expect(toLobbyView(snap(), 1).commander_damage_enabled).toBe(true);
    expect(toLobbyView(snap({ commanderDamageEnabled: false }), 1).commander_damage_enabled).toBe(false);
  });
});

describe("startError", () => {
  it("does not treat started as a start_error code", () => {
    expect(startError(snap({ startedAt: new Date("2026-07-22T00:00:00Z") }), 1)).toBeNull();
  });
});

// Client CI has no Postgres; run the round-trip only when WEB_DATABASE_URL is set
// (local Cloud / migrate environments). Projection coverage above still runs in CI.
describe.skipIf(!process.env.WEB_DATABASE_URL)("joinLobby gravatar persistence", () => {
  let db: ReturnType<typeof createWebDb>;
  let tableId: string | undefined;

  function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null;
  }

  function isLobbyRow(value: unknown, id: string): value is Record<string, unknown> {
    return isRecord(value) && value.tableId === id;
  }

  function startAfterFirstLobbyRead(
    baseDb: ReturnType<typeof createWebDb>,
    id: string,
  ): ReturnType<typeof createWebDb> {
    let armed = true;

    function wrapLobbyQueryBuilder<T extends object>(builder: T): T {
      return new Proxy(builder, {
        get(target, prop, receiver) {
          const value = Reflect.get(target, prop, receiver);
          if (typeof value !== "function") return value;

          return (...args: unknown[]) => {
            const result = Reflect.apply(value, target, args);
            if (prop !== "limit") {
              return isRecord(result) ? wrapLobbyQueryBuilder(result) : result;
            }

            return Promise.resolve(result).then(async (rows: unknown) => {
              if (!armed || !Array.isArray(rows)) return rows;
              armed = false;
              await markStarted(baseDb, id);
              return rows.map((row) => (isLobbyRow(row, id) ? { ...row, startedAt: null } : row));
            });
          };
        },
      });
    }

    return new Proxy(baseDb, {
      get(target, prop, receiver) {
        const value = Reflect.get(target, prop, receiver);
        if (prop !== "select" || typeof value !== "function") return value;

        return (...args: unknown[]) => {
          const selectBuilder = Reflect.apply(value, target, args);
          if (!isRecord(selectBuilder)) return selectBuilder;

          return new Proxy(selectBuilder, {
            get(selectTarget, selectProp, selectReceiver) {
              const selectValue = Reflect.get(selectTarget, selectProp, selectReceiver);
              if (selectProp !== "from" || typeof selectValue !== "function") return selectValue;

              return (table: unknown) => {
                const fromBuilder = Reflect.apply(selectValue, selectTarget, [table]);
                if (!isRecord(fromBuilder)) return fromBuilder;
                return table === lobbies ? wrapLobbyQueryBuilder(fromBuilder) : fromBuilder;
              };
            },
          });
        };
      },
    });
  }

  afterEach(async () => {
    if (!tableId || db == null) return;
    await db.delete(lobbies).where(eq(lobbies.tableId, tableId));
    tableId = undefined;
  });

  it("writes gravatarHash on insert/update and loadLobby/toLobbyView read it back", async () => {
    db = createWebDb();
    tableId = await createLobby(db, 9001);

    const joined = await joinLobby(db, {
      tableId,
      userId: 9001,
      username: "alice",
      gravatarHash: "hash-on-join",
      deckId: 1,
      deckName: "Test Deck",
    });
    expect(joined.snap?.seats[0]?.gravatarHash).toBe("hash-on-join");

    const loaded = await loadLobby(db, tableId);
    expect(loaded?.seats[0]?.gravatarHash).toBe("hash-on-join");
    if (loaded == null) throw new Error("expected lobby to load");
    expect(toLobbyView(loaded, 9001).seats[0]?.gravatar_hash).toBe("hash-on-join");

    const updated = await joinLobby(db, {
      tableId,
      userId: 9001,
      username: "alice",
      gravatarHash: "hash-updated",
      deckId: 1,
      deckName: "Test Deck",
    });
    expect(updated.snap?.seats[0]?.gravatarHash).toBe("hash-updated");

    const reloaded = await loadLobby(db, tableId);
    expect(reloaded?.seats[0]?.gravatarHash).toBe("hash-updated");
    if (reloaded == null) throw new Error("expected lobby to reload");
    expect(toLobbyView(reloaded, 9001).seats[0]?.gravatar_hash).toBe("hash-updated");
  });

  it("host can flip commander damage; non-host and started are rejected", async () => {
    db = createWebDb();
    tableId = await createLobby(db, 1);
    const loaded = await loadLobby(db, tableId);
    expect(loaded?.commanderDamageEnabled).toBe(true);

    const asHost = await setCommanderDamageEnabled(db, tableId, 1, false);
    expect(asHost.error).toBeUndefined();
    expect(asHost.snap?.commanderDamageEnabled).toBe(false);

    await joinLobby(db, {
      tableId,
      userId: 2,
      username: "bob",
      gravatarHash: "x",
      deckId: -2,
      deckName: "D",
    });
    const asGuest = await setCommanderDamageEnabled(db, tableId, 2, true);
    expect(asGuest.error).toBe("NotHost");

    await markStarted(db, tableId);
    const afterStart = await setCommanderDamageEnabled(db, tableId, 1, true);
    expect(afterStart.error).toBe("AlreadyStarted");
  });

  it("rejects commander damage changes when the table starts after the first read", async () => {
    db = createWebDb();
    tableId = await createLobby(db, 1);

    const result = await setCommanderDamageEnabled(startAfterFirstLobbyRead(db, tableId), tableId, 1, false);

    expect(result.error).toBe("AlreadyStarted");
    expect(result.snap?.startedAt).toBeInstanceOf(Date);
    expect(result.snap?.commanderDamageEnabled).toBe(true);
  });
});
