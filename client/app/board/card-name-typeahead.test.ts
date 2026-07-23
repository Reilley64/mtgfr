import { expect, test } from "vitest";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { PromptStringSet } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function state(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: { kind: "choose_card_name", player: 0, source: 1 },
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
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
    ...overrides,
  };
}

function fold(visible: VisibleState = state()): GameFoldState {
  return {
    seq: 1,
    state: visible,
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

test("PromptStringSet searches catalog names once the query is long enough", () => {
  const game = fold();
  const board = {
    ...initialBoardModel(),
    promptDraft: { kind: "string" as const, value: "" },
    pendingChoiceKey: "choose_card_name",
  };
  const [short, shortCmds] = updateBoard(board, PromptStringSet({ value: "S" }), game, "T1");
  expect(short.promptDraft).toEqual({ kind: "string", value: "S" });
  expect(short.cardNameSuggestions).toBeNull();
  expect(shortCmds).toEqual([]);

  const [ready, readyCmds] = updateBoard(short, PromptStringSet({ value: "Sol" }), game, "T1");
  expect(ready.promptDraft).toEqual({ kind: "string", value: "Sol" });
  expect((readyCmds[0] as { name?: string } | undefined)?.name).toBe("SearchCardNames");
});
