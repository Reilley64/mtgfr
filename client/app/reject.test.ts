// Regression: top-level IntentRejected must surface in `board.reject` so the priority-bar reject
// chrome (which reads `board.reject`, not `game.reject`) shows the failure to the player.

import { describe, expect, it } from "vitest";
import type { VisibleState } from "~/wire/types";
import { init } from "./init";
import { IntentAcked, IntentRejected } from "./messages";
import { emptyGameSlice, type Model } from "./model";
import { update } from "./update";

function state(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function modelWithGame(): Model {
  const [initial] = init();
  const game = { ...emptyGameSlice("T1"), state: state(), seq: 1 };
  return { ...initial, game };
}

describe("intent reject wiring", () => {
  it("IntentRejected sets board.reject so priority-bar chrome shows the reason", () => {
    const initial = modelWithGame();
    const [next] = update(initial, IntentRejected({ reason: "That's not your seat." }));
    expect(next.game?.board.reject).toBe("That's not your seat.");
    expect(next.game?.reject).toBe("That's not your seat.");
  });

  it("IntentAcked clears board.reject", () => {
    const seeded = modelWithGame();
    const game = seeded.game;
    if (game == null) throw new Error("test setup: game is null");
    seeded.game = { ...game, board: { ...game.board, reject: "prior" }, reject: "prior" };
    const [next] = update(seeded, IntentAcked());
    expect(next.game?.board.reject).toBeNull();
    expect(next.game?.reject).toBeNull();
  });
});
