import * as Effect from "effect/Effect";
import { afterEach, describe, expect, it, vi } from "vitest";
import { client as lobbyHttpClient } from "../../domain/lobby/client";
import { LobbyNotFound } from "../../domain/lobby/errors";
import * as tableAudio from "../../domain/tableAudio";
import { LobbyClient } from "../../resources";
import {
  LobbyRequestFailed,
  RequestedLobbyCancelJoin,
  RequestedLobbyHost,
  RequestedLobbyOpenJoin,
  RequestedLobbyReady,
} from "./messages";
import { initialLobbySlice } from "./submodel";
import { CreateLobbyTable, JoinLobbyTable, ReadyLobby, update } from "./update";

describe("RequestedLobbyReady audio unlock", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    tableAudio.resetTableAudioForTests();
  });

  it("unlocks table audio synchronously on Ready", () => {
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const model = { ...initialLobbySlice(), tableId: "ABC123" };
    const [next, commands] = update(model, RequestedLobbyReady({ ready: true }), [1]);

    expect(unlock).toHaveBeenCalledTimes(1);
    expect(next.submitting).toBe(true);
    expect(commands).toHaveLength(1);
    expect(commands[0]?.name).toBe(ReadyLobby.name);
  });

  it("does not unlock when tableId is missing", () => {
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    update(initialLobbySlice(), RequestedLobbyReady({ ready: true }), [1]);
    expect(unlock).not.toHaveBeenCalled();
  });
});

describe("RequestedLobbyHost deck selection", () => {
  it("does not fall back to the first loaded deck", () => {
    const [next, commands] = update(initialLobbySlice(), RequestedLobbyHost(), [7]);

    expect(next.error).toBe("Pick a deck to bring first.");
    expect(next.selectedDeckId).toBeNull();
    expect(commands).toHaveLength(0);
  });
});

describe("lobby entryMode", () => {
  it("defaults to choose", () => {
    expect(initialLobbySlice().entryMode).toBe("choose");
  });

  it("open join switches to join mode without submitting", () => {
    const [next, commands] = update(initialLobbySlice(), RequestedLobbyOpenJoin(), []);
    expect(next.entryMode).toBe("join");
    expect(commands).toHaveLength(0);
  });

  it("cancel join returns to choose and clears code + error", () => {
    const model = {
      ...initialLobbySlice(),
      entryMode: "join" as const,
      code: "ABC123",
      error: "UnknownTable",
    };
    const [next, commands] = update(model, RequestedLobbyCancelJoin(), []);
    expect(next.entryMode).toBe("choose");
    expect(next.code).toBe("");
    expect(next.error).toBeNull();
    expect(commands).toHaveLength(0);
  });

  it("host still creates a table when a deck is selected", () => {
    const model = { ...initialLobbySlice(), selectedDeckId: 7, entryMode: "choose" as const };
    const [next, commands] = update(model, RequestedLobbyHost(), [7]);
    expect(next.submitting).toBe(true);
    expect(commands).toHaveLength(1);
    expect(commands[0]?.name).toBe(CreateLobbyTable.name);
  });
});

describe("lobby commands", () => {
  it("maps a missing lobby table to UnknownTable", async () => {
    const failingClient = {
      ...lobbyHttpClient,
      joinTable: () => Effect.fail(new LobbyNotFound()),
    };

    const message = await Effect.runPromise(
      JoinLobbyTable({ tableId: "GONE", deckId: 7 }).effect.pipe(Effect.provideService(LobbyClient, failingClient)),
    );

    expect(message).toEqual(LobbyRequestFailed({ message: "UnknownTable" }));
  });
});
