import { describe, expect, it } from "vitest";
import { type LobbySnapshot, startError, toLobbyView } from "./lobby-store";

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
        deckId: -1,
        deckName: "Silverquill Influence",
        ready: true,
      },
      {
        seat: 1,
        userId: 2,
        username: "bob",
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
          deckId: -1,
          deckName: "Silverquill Influence",
          ready: true,
        },
        {
          seat: 1,
          userId: 2,
          username: "bob",
          deckId: -2,
          deckName: "Prismari Artistry",
          ready: false,
        },
      ],
    });
    expect(toLobbyView(notReady, 1).start_error).toBe("NotAllReady");
  });
});

describe("startError", () => {
  it("does not treat started as a start_error code", () => {
    expect(startError(snap({ startedAt: new Date("2026-07-22T00:00:00Z") }), 1)).toBeNull();
  });
});
