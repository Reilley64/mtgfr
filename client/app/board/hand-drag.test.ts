import { describe, expect, it } from "vitest";
import type { ActionView, ObjectView } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { ZONE } from "./geometry/layout";
import { HandActionHovered, HandDragEnded, HandDragMoved, HandDragStarted } from "./messages";
import { initialBoardModel, updateBoard } from "./submodel";

function fold(objects: ObjectView[], actions: ActionView[]): GameFoldState {
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
          hand_count: 1,
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

const bolt: ObjectView = {
  controller: 0,
  has_haste: false,
  id: 42,
  is_commander: false,
  kind: { kind: "instant" },
  mana_cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
  marked_damage: 0,
  name: "Lightning Bolt",
  needs_target: false,
  owner: 0,
  plus_counters: 0,
  power: 0,
  print: "bolt-print",
  summoning_sick: false,
  tapped: false,
  toughness: 0,
  zone: ZONE.Hand,
};

const castAction: ActionView = {
  id: 7,
  kind: "cast",
  label: "Cast Lightning Bolt",
  needs_target: false,
  object: 42,
  section: "hand",
  auto_tap: [5],
};

describe("hand drag submodel", () => {
  it("tracks an in-flight drag ghost position", () => {
    const board = initialBoardModel();
    const [started] = updateBoard(
      board,
      HandDragStarted({
        action: castAction,
        name: "Lightning Bolt",
        print: "bolt-print",
        manaCost: bolt.mana_cost,
        kind: "instant",
        x: 100,
        y: 200,
      }),
      fold([bolt], [castAction]),
      "T1",
    );
    expect(started.handDrag).toMatchObject({ x: 100, y: 200, name: "Lightning Bolt" });

    const [moved] = updateBoard(started, HandDragMoved({ x: 150, y: 250 }), fold([bolt], [castAction]), "T1");
    expect(moved.handDrag?.x).toBe(150);
    expect(moved.handDrag?.y).toBe(250);
  });

  it("plays the card when drag ends above the hand-bar threshold", () => {
    const board = initialBoardModel();
    const [dragging] = updateBoard(
      board,
      HandDragStarted({
        action: castAction,
        name: "Lightning Bolt",
        print: "bolt-print",
        manaCost: bolt.mana_cost,
        kind: "instant",
        x: 100,
        y: 800,
      }),
      fold([bolt], [castAction]),
      "T1",
    );
    const [, commands] = updateBoard(dragging, HandDragEnded({ x: 400, y: 200 }), fold([bolt], [castAction]), "T1");
    expect(commands).toHaveLength(1);
  });

  it("ignores drag end below the hand-bar threshold", () => {
    const board = initialBoardModel();
    const [dragging] = updateBoard(
      board,
      HandDragStarted({
        action: castAction,
        name: "Lightning Bolt",
        print: "bolt-print",
        manaCost: bolt.mana_cost,
        kind: "instant",
        x: 100,
        y: 800,
      }),
      fold([bolt], [castAction]),
      "T1",
    );
    const [next, commands] = updateBoard(dragging, HandDragEnded({ x: 400, y: 900 }), fold([bolt], [castAction]), "T1");
    expect(commands).toEqual([]);
    expect(next.handDrag).toBeNull();
  });

  it("stores hovered action id for auto-tap preview", () => {
    const board = initialBoardModel();
    const [hovered] = updateBoard(board, HandActionHovered({ actionId: 7 }), fold([bolt], [castAction]), "T1");
    expect(hovered.hoverActionId).toBe(7);
    const [cleared] = updateBoard(hovered, HandActionHovered({ actionId: null }), fold([bolt], [castAction]), "T1");
    expect(cleared.hoverActionId).toBeNull();
  });
});
