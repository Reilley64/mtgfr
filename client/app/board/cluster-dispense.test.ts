import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { ZONE } from "./geometry/layout";
import { CombatAttackerDropped } from "./messages";
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
      battlefieldExits: new Map(),
      tokenCreators: new Map(),
      landPlayFrom: new Map(),
      zonePileEntrances: new Map(),
      stackEntrances: new Map(),
      priorStackObjectIds: new Set(),
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false, destroy: false, exile: false },
  };
}

/** 6 unique bears + 4 identical Saprolings in one seat: the Saprolings must cluster. */
function crowdedObjects(): ObjectView[] {
  const bears = Array.from({ length: 6 }, (_, i) =>
    creature(i + 1, { name: `Bear ${i}`, kind: { kind: "creature", power: 2, toughness: 2 } }),
  );
  const saprolings = [10, 11, 12, 13].map((id) =>
    creature(id, { name: "Saproling", kind: { kind: "creature", power: 1, toughness: 1 }, power: 1, toughness: 1 }),
  );
  return [...bears, ...saprolings];
}

const declareAttackers: ActionView = {
  id: 1,
  kind: "declare_attackers",
  label: testMessageRef("Declare attackers"),
  needs_target: false,
  section: "combat",
  declare_for: [0],
};

test("two attack drops on one cluster declare two distinct attackers", () => {
  const gameFold = fold({
    objects: crowdedObjects(),
    actions: [declareAttackers],
    step: 5,
    players: [
      {
        commander_tax: 0,
        hand_count: 0,
        library_count: 40,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
        username: "Alice",
      },
      {
        commander_tax: 0,
        hand_count: 0,
        library_count: 40,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 1,
        username: "Bob",
      },
    ],
  });
  const board: BoardModel = initialBoardModel();

  const [afterFirst] = updateBoard(board, CombatAttackerDropped({ attackerId: 10, defenderSeat: 1 }), gameFold, "T1");
  const [afterSecond] = updateBoard(
    afterFirst,
    CombatAttackerDropped({ attackerId: 11, defenderSeat: 1 }),
    gameFold,
    "T1",
  );

  expect(afterSecond.combatAttackers.map((a) => a.attacker).sort()).toEqual([10, 11]);
});
