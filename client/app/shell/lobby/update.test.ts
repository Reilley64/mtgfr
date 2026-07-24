import { afterEach, describe, expect, it, vi } from "vitest";
import * as tableAudio from "../../../lib/tableAudio";
import { RequestedLobbyHost, RequestedLobbyReady } from "./messages";
import { initialLobbySlice } from "./submodel";
import { ReadyLobby, update } from "./update";

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
