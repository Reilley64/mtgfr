import { eq } from "drizzle-orm";
import { afterEach, describe, expect, it } from "vitest";
import { lobbies } from "../db/schema";
import { createWebDb } from "../server/db/client";
import { createLobby, joinLobby, type LobbySnapshot, loadLobby, startError, toLobbyView } from "./lobby-store";

function snap(overrides: Partial<LobbySnapshot> = {}): LobbySnapshot {
  return {
    tableId: "ABC123",
    hostUserId: 1,
    startedAt: null,
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

  afterEach(async () => {
    if (!tableId || db == null) return;
    await db.delete(lobbies).where(eq(lobbies.tableId, tableId));
    tableId = undefined;
  });

  it("loadLobby succeeds on an empty table (missing gravatar_hash 500s Host as Unreachable)", async () => {
    // Prod 3.100.2: POST /tables/v1 ok, then join/lobby GET 500 when lobby_seats lacks
    // gravatar_hash — client decode maps that Nitro body to Unreachable.
    db = createWebDb();
    tableId = await createLobby(db, 9000);
    await expect(loadLobby(db, tableId)).resolves.toMatchObject({
      tableId,
      hostUserId: 9000,
      seats: [],
    });
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
});
