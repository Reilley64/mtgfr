import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { SubmitIntent } from "../../game/intents";
import { type Message, PromptCardToggled, PromptSubmitted } from "../messages";
import { type BoardModel, initialBoardModel, updateBoard } from "../submodel";
import { boardOverlays } from "./overlays";
import { resolveBoardOverlayMounts } from "./scene-helpers";

const h = html<Message>();

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const view = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

function state(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [
      {
        player: 0,
        username: "Alice",
        life: 40,
        hand_count: 5,
        library_count: 90,
        lost: false,
        commander_tax: 0,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function gameFold(s: VisibleState): GameFoldState {
  return {
    seq: 1,
    state: s,
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

function viewModel(s: VisibleState, board = initialBoardModel()): ViewModel {
  return { board, fold: gameFold(s), tableId: "T1" };
}

const sceneUpdate = (model: ViewModel, message: Message): readonly [ViewModel, ReadonlyArray<never>] => {
  const [board] = updateBoard(model.board, message, model.fold, model.tableId);
  return [{ ...model, board }, []];
};

function intentFromCommand(cmd: unknown): unknown {
  return (cmd as { args: { intent: unknown } }).args.intent;
}

test("discard prompt submit disabled until count cards picked", () => {
  const s = state({
    pending_choice: {
      kind: "discard",
      count: 2,
      player: 0,
      items: [
        { id: 10, label: "Card A" },
        { id: 11, label: "Card B" },
        { id: 12, label: "Card C" },
      ],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
    Scene.click(Scene.testId("prompt-card-10")),
    Scene.click(Scene.testId("prompt-card-11")),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
  );
});

test("discard prompt submit emits discard intent", () => {
  const s = state({
    pending_choice: {
      kind: "discard",
      count: 2,
      player: 0,
      items: [
        { id: 10, label: "Card A" },
        { id: 11, label: "Card B" },
      ],
    },
  });
  const gf = gameFold(s);
  let board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 10 }), gf, "T1")[0];
  board = updateBoard(board, PromptCardToggled({ id: 11 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
  expect(intentFromCommand(commands[0])).toEqual({ kind: "discard", player: 0, cards: [10, 11] });
});

test("order_triggers shows reorder controls and submit", () => {
  const s = state({
    pending_choice: {
      kind: "order_triggers",
      count: 2,
      labels: ["First", "Second"],
      player: 0,
      source: 1,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-order-0")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
  );
});

test("order_triggers submit emits choose_order intent", () => {
  const s = state({
    pending_choice: {
      kind: "order_triggers",
      count: 2,
      labels: ["A", "B"],
      player: 0,
      source: 1,
    },
  });
  const [, commands] = updateBoard(initialBoardModel(), PromptSubmitted(), gameFold(s), "T1");
  expect(intentFromCommand(commands[0])).toEqual({ kind: "choose_order", player: 0, order: [0, 1] });
});

test("scry prompt submit emits arrange_top intent", () => {
  const s = state({
    pending_choice: {
      kind: "scry",
      player: 0,
      items: [
        { id: 1, label: "Top" },
        { id: 2, label: "Mid" },
      ],
    },
  });
  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 1 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "arrange_top",
    player: 0,
    top: [1],
    bottom: [2],
  });
});

test("assign_combat_damage submit when damage sums to power", () => {
  const attacker: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 9,
    is_commander: false,
    kind: { kind: "creature", power: 4, toughness: 4 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Attacker",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 4,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 4,
    zone: 2,
  };
  const s = state({
    objects: [attacker],
    pending_choice: {
      kind: "assign_combat_damage",
      player: 0,
      source: 9,
      items: [{ id: 20, label: "Blocker" }],
    },
  });
  const [, commands] = updateBoard(initialBoardModel(), PromptSubmitted(), gameFold(s), "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "assign_damage",
    player: 0,
    assignment: [{ blocker: 20, amount: 4 }],
  });
});
