import { afterEach, describe, expect, it, vi } from "vitest";
import type { ActionView, ObjectView } from "~/wire/types";
import * as tableAudio from "../../lib/tableAudio";
import type { GameFoldState } from "../game/fold";
import { SoundToggled } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function fold(objects: ObjectView[] = [], actions: ActionView[] = []): GameFoldState {
  return {
    seq: 1,
    state: {
      active_player: 0,
      can_act: true,
      combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
      objects,
      pending_choice: null,
      players: [
        {
          commander_tax: 0,
          hand_count: 0,
          library_count: 80,
          life: 40,
          lost: false,
          mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
          player: 0,
          username: "Alice",
        },
      ],
      priority: 0,
      stack: [],
      step: 3,
      viewer: 0,
      actions,
    },
    log: [],
    reject: null,
    provenance: {
      zoneMoves: new Map(),
      resolvedFromStack: new Set(),
      leftStackToPile: new Set(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false },
  };
}

describe("SoundToggled", () => {
  afterEach(() => {
    vi.restoreAllMocks();
    tableAudio.resetTableAudioForTests();
    tableAudio.setSoundEnabledForTests(null);
  });

  it("unlocks and plays unmute tick when turning sound on", () => {
    tableAudio.setSoundEnabledForTests(false);
    const board = { ...initialBoardModel(), soundOn: false };
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const tick = vi.spyOn(tableAudio, "playUnmuteTick");

    const [next] = updateBoard(board, SoundToggled(), fold(), "T1");
    expect(next.soundOn).toBe(true);
    expect(unlock).toHaveBeenCalledTimes(1);
    expect(tick).toHaveBeenCalledTimes(1);
  });

  it("does not unlock when turning sound off", () => {
    tableAudio.setSoundEnabledForTests(true);
    const board = { ...initialBoardModel(), soundOn: true };
    const unlock = vi.spyOn(tableAudio, "unlockTableAudio");
    const tick = vi.spyOn(tableAudio, "playUnmuteTick");

    const [next] = updateBoard(board, SoundToggled(), fold(), "T1");
    expect(next.soundOn).toBe(false);
    expect(unlock).not.toHaveBeenCalled();
    expect(tick).not.toHaveBeenCalled();
  });
});
