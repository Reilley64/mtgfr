import { describe, expect, it } from "vitest";
import type { LobbySnapshot } from "~/lib/lobbyStore";
import { randomTableCode, startError, toLobbyView } from "~/lib/lobbyStore";

describe("randomTableCode", () => {
  it("mints a 6-character code from the unambiguous alphabet", () => {
    const code = randomTableCode();
    expect(code).toMatch(/^[23456789ABCDEFGHJKMNPQRSTUVWXYZ]{6}$/);
  });

  it("varies across calls (not a fixed code)", () => {
    const codes = new Set(Array.from({ length: 20 }, () => randomTableCode()));
    expect(codes.size).toBeGreaterThan(1);
  });
});

function snapshot(overrides: Partial<LobbySnapshot> = {}): LobbySnapshot {
  return {
    tableId: "ABC123",
    hostUserId: 1,
    startedAt: null,
    seats: [],
    ...overrides,
  };
}

function seat(overrides: Partial<LobbySnapshot["seats"][number]> = {}): LobbySnapshot["seats"][number] {
  return {
    seat: 0,
    userId: 1,
    username: "host",
    deckId: 1,
    deckName: "Deck",
    ready: false,
    ...overrides,
  };
}

describe("startError", () => {
  it("blocks a non-host from starting", () => {
    const snap = snapshot({ seats: [seat({ userId: 1 }), seat({ seat: 1, userId: 2, ready: true })] });
    expect(startError(snap, 2)).toBe("NotHost");
  });

  it("blocks starting an already-started game", () => {
    const snap = snapshot({
      startedAt: new Date(),
      seats: [seat({ ready: true }), seat({ seat: 1, userId: 2, ready: true })],
    });
    expect(startError(snap, 1)).toBe("AlreadyStarted");
  });

  it("blocks starting with fewer than two seated players", () => {
    const snap = snapshot({ seats: [seat({ ready: true })] });
    expect(startError(snap, 1)).toBe("NeedTwoPlayers");
  });

  it("blocks starting until every seat is ready", () => {
    const snap = snapshot({ seats: [seat({ ready: true }), seat({ seat: 1, userId: 2, ready: false })] });
    expect(startError(snap, 1)).toBe("NotAllReady");
  });

  it("allows starting once the host has two-plus ready seats", () => {
    const snap = snapshot({ seats: [seat({ ready: true }), seat({ seat: 1, userId: 2, ready: true })] });
    expect(startError(snap, 1)).toBeNull();
  });
});

describe("toLobbyView", () => {
  it("fills all four seats, marking unclaimed ones and the caller's own seat", () => {
    const snap = snapshot({
      seats: [seat({ ready: true }), seat({ seat: 1, userId: 2, username: "guest", ready: false })],
    });
    const view = toLobbyView(snap, 2);

    expect(view.table_id).toBe("ABC123");
    expect(view.you).toBe(1);
    expect(view.started).toBe(false);
    expect(view.error).toBeNull();
    expect(view.seats).toHaveLength(4);
    expect(view.seats[0]).toMatchObject({ claimed: true, username: "host", is_host: true, is_you: false });
    expect(view.seats[1]).toMatchObject({
      claimed: true,
      username: "guest",
      is_host: false,
      is_you: true,
      ready: false,
    });
    expect(view.seats[2]).toMatchObject({
      claimed: false,
      username: null,
      deck_name: null,
      is_host: false,
      is_you: false,
    });
  });

  it("reports NotSeated as the start error for a viewer with no claimed seat", () => {
    const snap = snapshot({ seats: [seat({ ready: true })] });
    expect(toLobbyView(snap, 99).start_error).toBe("NotSeated");
    expect(toLobbyView(snap, null).start_error).toBe("NotSeated");
  });

  it("surfaces the host's own start_error gate once seated", () => {
    const snap = snapshot({ seats: [seat({ ready: true })] });
    expect(toLobbyView(snap, 1).start_error).toBe("NeedTwoPlayers");
  });

  it("carries the passed error through unchanged", () => {
    const snap = snapshot();
    expect(toLobbyView(snap, null, "TableFull").error).toBe("TableFull");
  });
});
