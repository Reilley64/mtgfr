import { expect, test } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { SubmitIntent } from "../game/intents";
import { emptyCostPicks } from "./action/execution";
import { ZONE } from "./geometry/layout";
import { promptsView } from "./html/prompts";
import { TargetChosen } from "./messages";
import { type BoardModel, initialBoardModel, updateBoard } from "./submodel";

function creature(id: number, over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...over,
  };
}

function state(over: Partial<VisibleState> = {}): VisibleState {
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
        username: "Alice",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

function fold(over: Partial<VisibleState> = {}): GameFoldState {
  return {
    seq: 1,
    state: state(over),
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

test("off-board staged target opens the target-pick prompt", () => {
  const corpse = creature(9, { zone: ZONE.Graveyard, name: "Corpse", print: "print-9" });
  const spell = creature(5, { name: "Reanimate", kind: { kind: "sorcery" } });
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: "Reanimate",
    needs_target: true,
    object: spell.id,
    section: "hand",
    targets: [{ kind: "object", id: 9 }],
  };
  const board: BoardModel = {
    ...initialBoardModel(),
    staged: {
      card: spell,
      action: castAction,
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
  };
  const view = promptsView(board, state({ objects: [spell, corpse] }), "T1");
  expect(view).not.toBeNull();
});

test("TargetChosen from the pick dialog submits take_action for off-board targets", () => {
  const corpse = creature(9, { zone: ZONE.Graveyard, name: "Corpse" });
  const spell = creature(5, { name: "Reanimate", kind: { kind: "sorcery" } });
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: "Reanimate",
    needs_target: true,
    object: spell.id,
    section: "hand",
    targets: [{ kind: "object", id: 9 }],
  };
  const board: BoardModel = {
    ...initialBoardModel(),
    staged: {
      card: spell,
      action: castAction,
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
  };
  const gameFold = fold({ objects: [spell, corpse] });
  const [nextBoard, commands] = updateBoard(board, TargetChosen({ target: { kind: "object", id: 9 } }), gameFold, "T1");
  expect(nextBoard.staged).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
});
