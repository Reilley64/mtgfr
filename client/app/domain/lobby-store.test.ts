import { eq } from "drizzle-orm";
import * as Effect from "effect/Effect";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { lobbies } from "../../db/schema";
import { WebDb, WebDbLive } from "../../server/db/client";
import {
  createLobby,
  joinLobby,
  type LobbySnapshot,
  loadLobby,
  randomTableCode,
  startError,
  toLobbyView,
} from "./lobby-store";

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

describe("randomTableCode", () => {
  beforeEach(() => {
    let draws = 0;
    vi.spyOn(globalThis.crypto, "getRandomValues").mockImplementation((array) => {
      const next = draws === 0 ? [0, 1, 2, 3, 4, 5] : [8, 9, 10, 11, 12, 13];
      draws += 1;
      if (array == null) return array;
      const bytes = new Uint8Array(array.buffer, array.byteOffset, array.byteLength);
      bytes.set(next);
      return array;
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("redraws when a generated table code would be all digits", () => {
    expect(randomTableCode()).toBe("ABCDEF");
    expect(globalThis.crypto.getRandomValues).toHaveBeenCalledTimes(2);
  });

  it("always includes at least one letter", () => {
    const code = randomTableCode();
    expect(code).toMatch(/[A-Z]/);
    expect(code).toHaveLength(6);
  });
});

// Client CI has no Postgres; run the round-trip only when WEB_DATABASE_URL is set
// (local Cloud / migrate environments). Projection coverage above still runs in CI.
describe.skipIf(!process.env.WEB_DATABASE_URL)("joinLobby gravatar persistence", () => {
  const run = <A, E>(op: Effect.Effect<A, E, WebDb>): Promise<A> =>
    Effect.runPromise(op.pipe(Effect.provide(WebDbLive)));

  const deleteLobby = Effect.fn(function* (id: string) {
    const db = yield* WebDb;
    yield* db.delete(lobbies).where(eq(lobbies.tableId, id));
  });

  let tableId: string | undefined;

  afterEach(async () => {
    if (!tableId) return;
    await run(deleteLobby(tableId));
    tableId = undefined;
  });

  it("loadLobby succeeds on an empty table (missing gravatar_hash 500s Host as Unreachable)", async () => {
    // Schema must come from edh-web-migrate (0002/0003 + Job assert), not app self-heal.
    // Without gravatar_hash, join/lobby GET 500 → client Unreachable.
    tableId = await run(createLobby(9000));
    await expect(run(loadLobby(tableId))).resolves.toMatchObject({
      tableId,
      hostUserId: 9000,
      seats: [],
    });
  });

  it("writes gravatarHash on insert/update and loadLobby/toLobbyView read it back", async () => {
    tableId = await run(createLobby(9001));

    const joined = await run(
      joinLobby({
        tableId,
        userId: 9001,
        username: "alice",
        gravatarHash: "hash-on-join",
        deckId: 1,
        deckName: "Test Deck",
      }),
    );
    expect(joined.snap?.seats[0]?.gravatarHash).toBe("hash-on-join");

    const loaded = await run(loadLobby(tableId));
    expect(loaded?.seats[0]?.gravatarHash).toBe("hash-on-join");
    if (loaded == null) throw new Error("expected lobby to load");
    expect(toLobbyView(loaded, 9001).seats[0]?.gravatar_hash).toBe("hash-on-join");

    const updated = await run(
      joinLobby({
        tableId,
        userId: 9001,
        username: "alice",
        gravatarHash: "hash-updated",
        deckId: 1,
        deckName: "Test Deck",
      }),
    );
    expect(updated.snap?.seats[0]?.gravatarHash).toBe("hash-updated");

    const reloaded = await run(loadLobby(tableId));
    expect(reloaded?.seats[0]?.gravatarHash).toBe("hash-updated");
    if (reloaded == null) throw new Error("expected lobby to reload");
    expect(toLobbyView(reloaded, 9001).seats[0]?.gravatar_hash).toBe("hash-updated");
  });
});
