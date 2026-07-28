// Regression: top-level IntentRejected must surface in `board.reject` so the priority-bar reject
// chrome (which reads `board.reject`, not `game.reject`) shows the failure to the player.

import { describe, expect, it } from "vitest";
import type { VisibleState } from "~/wire/types";
import { spawnFlight } from "./board/motion/flights";
import { IntentAcked, IntentRejected } from "./game/messages";
import { init } from "./init";
import { GotGameMessage } from "./messages";
import { emptyGameSlice, type Model } from "./model";
import { update } from "./update";

function state(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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
    const [next] = update(initial, GotGameMessage({ message: IntentRejected({ reason: "That's not your seat." }) }));
    expect(next.game?.board.reject).toBe("That's not your seat.");
    expect(next.game?.reject).toBe("That's not your seat.");
  });

  it("IntentRejected clears promptSubmitInFlight so a frozen prompt can be retried", () => {
    const seeded = modelWithGame();
    const game = seeded.game;
    if (game == null) throw new Error("test setup: game is null");
    seeded.game = { ...game, board: { ...game.board, promptSubmitInFlight: true, promptSubmitSeq: 4 } };
    const [next] = update(seeded, GotGameMessage({ message: IntentRejected({ reason: "Not your turn." }) }));
    expect(next.game?.board.promptSubmitInFlight).toBe(false);
    expect(next.game?.board.promptSubmitSeq).toBeNull();
    expect(next.game?.board.reject).toBe("Not your turn.");
  });

  it("IntentRejected clears combat confirm latches so a rejected goad declare can be retried", () => {
    const seeded = modelWithGame();
    const game = seeded.game;
    if (game == null) throw new Error("test setup: game is null");
    seeded.game = {
      ...game,
      board: { ...game.board, attackersConfirmed: true, blockersConfirmed: true },
    };
    const [next] = update(seeded, GotGameMessage({ message: IntentRejected({ reason: "Illegal declaration." }) }));
    expect(next.game?.board.attackersConfirmed).toBe(false);
    expect(next.game?.board.blockersConfirmed).toBe(false);
  });

  it("IntentRejected drops the optimistic seed so the card returns to hand instead of ghosting", () => {
    const seeded = modelWithGame();
    const game = seeded.game;
    if (game == null) throw new Error("test setup: game is null");
    const held = spawnFlight({
      id: 7,
      print: "forest-print",
      name: "Forest",
      x: 500,
      y: 400,
      scale: 0.5,
      targetX: 600,
      targetY: 300,
      targetScale: 1,
      kind: "battlefield",
      fromCardId: 7,
      hold: true,
    });
    seeded.game = {
      ...game,
      board: { ...game.board, flights: new Map([[7, held]]), handHidden: new Set([7]), hideCardIds: new Set([7]) },
    };

    const [next] = update(seeded, GotGameMessage({ message: IntentRejected({ reason: "Not your turn." }) }));

    expect([...(next.game?.board.flights.keys() ?? [])]).toEqual([]);
    expect([...(next.game?.board.handHidden ?? [])]).toEqual([]);
    expect([...(next.game?.board.hideCardIds ?? [])]).toEqual([]);
  });

  it("IntentAcked clears board.reject", () => {
    const seeded = modelWithGame();
    const game = seeded.game;
    if (game == null) throw new Error("test setup: game is null");
    seeded.game = { ...game, board: { ...game.board, reject: "prior" }, reject: "prior" };
    const [next] = update(seeded, GotGameMessage({ message: IntentAcked() }));
    expect(next.game?.board.reject).toBeNull();
    expect(next.game?.reject).toBeNull();
  });
});
