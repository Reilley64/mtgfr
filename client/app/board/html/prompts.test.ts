import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { SubmitIntent } from "../../game/intents";
import { type Message, PromptCardToggled, PromptDamageSet, PromptSubmitted } from "../messages";
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

function clickPromptIntent(s: VisibleState, click: ReturnType<typeof Scene.click>): unknown[] {
  const commands: unknown[] = [];
  const update = (model: ViewModel, message: Message): readonly [ViewModel, ReadonlyArray<never>] => {
    const [board, nextCommands] = updateBoard(model.board, message, model.fold, model.tableId);
    commands.push(...nextCommands);
    return [{ ...model, board }, []];
  };
  Scene.scene({ update, view }, Scene.with(viewModel(s)), resolveBoardOverlayMounts(), click);
  return commands.map(intentFromCommand);
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

test("may_yes_no prompt emits answer_may intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "may_yes_no",
        label: "Draw a card?",
        player: 0,
        source: 1,
      },
    }),
    Scene.click(Scene.testId("prompt-yes")),
  );
  expect(intents).toEqual([{ kind: "answer_may", player: 0, yes: true }]);
});

test("pay_cost prompt emits pay_optional_cost intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "pay_cost",
        cost: { colored: [], generic: 2 },
        label: "Pay 2?",
        player: 0,
        source: 1,
      },
    }),
    Scene.click(Scene.testId("prompt-pay")),
  );
  expect(intents).toEqual([{ kind: "pay_optional_cost", player: 0, pay: true }]);
});

test("choose_mode prompt emits choose_mode intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_mode",
        labels: ["Mode A", "Mode B"],
        player: 0,
        source: 1,
      },
    }),
    Scene.click(Scene.testId("prompt-mode-1")),
  );
  expect(intents).toEqual([{ kind: "choose_mode", player: 0, mode: 1 }]);
});

test("choose_splitting_opponent prompt emits player target intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_splitting_opponent",
        items: [
          { id: 0, label: "Player 2", player: 1 },
          { id: 0, label: "Player 3", player: 2 },
        ],
        label: "Choose an opponent",
        player: 0,
        source: 1,
      },
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
        {
          player: 1,
          username: "Bob",
          life: 40,
          hand_count: 5,
          library_count: 90,
          lost: false,
          commander_tax: 0,
          mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        },
        {
          player: 2,
          username: "Carol",
          life: 40,
          hand_count: 5,
          library_count: 90,
          lost: false,
          commander_tax: 0,
          mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        },
      ],
    }),
    Scene.click(Scene.testId("prompt-player-2")),
  );
  expect(intents).toEqual([
    {
      kind: "choose_targets",
      player: 0,
      targets: [{ kind: "player", player: 2 }],
    },
  ]);
});

test("divide_spell_damage submit emits divide intent when assignments match total", () => {
  const s = state({
    pending_choice: {
      kind: "divide_spell_damage",
      items: [
        { id: 21, label: "Target A" },
        { id: 22, label: "Target B" },
      ],
      player: 0,
      spell: 99,
      total: 3,
    },
  });
  const gf = gameFold(s);
  let board = updateBoard(initialBoardModel(), PromptDamageSet({ id: 0, amount: 2 }), gf, "T1")[0];
  board = updateBoard(board, PromptDamageSet({ id: 1, amount: 1 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "divide_spell_damage",
    player: 0,
    assignment: [
      { amount: 2, target: { kind: "object", id: 21 } },
      { amount: 1, target: { kind: "object", id: 22 } },
    ],
  });
});

test("choose_pile_for_hand prompt emits choose_opponent_pile intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_pile_for_hand",
        pile_a: [{ id: 1, label: "Pile A card" }],
        pile_b: [{ id: 2, label: "Pile B card" }],
        player: 0,
        source: 8,
      },
    }),
    Scene.click(Scene.testId("prompt-pile-1")),
  );
  expect(intents).toEqual([{ kind: "choose_opponent_pile", player: 0, pile: 1 }]);
});

test("partition_revealed submit emits choose_sacrifices intent", () => {
  const s = state({
    pending_choice: {
      kind: "partition_revealed",
      items: [
        { id: 6, label: "A" },
        { id: 7, label: "B" },
      ],
      player: 1,
      source: 8,
    },
  });
  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 6 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_sacrifices",
    player: 1,
    sacrifices: [6],
  });
});

test("choose_color prompt emits choose_color intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_color",
        player: 0,
        source: 1,
      },
    }),
    Scene.click(Scene.testId("prompt-color-4")),
  );
  expect(intents).toEqual([{ kind: "choose_color", player: 0, color: 4 }]);
});

test("choose_creature_type prompt emits choose_creature_type intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_creature_type",
        options: ["Wizard", "Cleric"],
        player: 0,
        source: 1,
      },
    }),
    Scene.click(Scene.testId("prompt-string-1")),
  );
  expect(intents).toEqual([{ kind: "choose_creature_type", player: 0, subtype: "Cleric" }]);
});

test("may_draw_up_to prompt emits choose_draw_count intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "may_draw_up_to",
        max: 3,
        player: 0,
      },
    }),
    Scene.click(Scene.testId("prompt-number-2")),
  );
  expect(intents).toEqual([{ kind: "choose_draw_count", player: 0, count: 2 }]);
});

test("choose_countered_spell_destination prompt emits choose_top_or_bottom intent from UI", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_countered_spell_destination",
        player: 0,
        spell: 5,
      },
    }),
    Scene.click(Scene.testId("prompt-destination-top")),
  );
  expect(intents).toEqual([{ kind: "choose_top_or_bottom", player: 0, top: true }]);
});
