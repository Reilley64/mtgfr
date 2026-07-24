import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { choiceDraftKey } from "~/choice";
import type { ActionView, ObjectView, VisibleState, WireCost } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { SubmitIntent } from "../../game/intents";
import { emptyCostPicks } from "../action/execution";
import { ZONE } from "../geometry/layout";
import {
  type Message,
  PromptCardToggled,
  PromptDamageSet,
  PromptNumberSet,
  PromptOrderDragEnded,
  PromptOrderRowClicked,
  PromptPartitionSet,
  PromptStringSet,
  PromptSubmitted,
  XDraftSet,
  XSubmitted,
} from "../messages";
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
    Scene.expect(Scene.testId("prompt-order-list")).toExist(),
    Scene.expect(Scene.testId("prompt-order-0")).toExist(),
    Scene.expect(Scene.testId("prompt-order-pick-0")).toHaveText("First"),
    Scene.expect(Scene.testId("prompt-order-up-0")).toExist(),
    Scene.expect(Scene.testId("prompt-order-down-0")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
  );
});

test("order_triggers rows are HTML5-draggable drop targets", () => {
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
    Scene.expect(Scene.selector('[data-testid="prompt-order-0"][draggable="true"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="prompt-order-1"][draggable="true"]')).toExist(),
  );
});

test("order_triggers click-to-place reorders then submits choose_order", () => {
  const s = state({
    pending_choice: {
      kind: "order_triggers",
      count: 3,
      labels: ["A", "B", "C"],
      player: 0,
      source: 1,
    },
  });
  const gf = gameFold(s);
  let board = updateBoard(initialBoardModel(), PromptOrderRowClicked({ pos: 0 }), gf, "T1")[0];
  expect(board.orderPickPos).toBe(0);
  board = updateBoard(board, PromptOrderRowClicked({ pos: 2 }), gf, "T1")[0];
  expect(board.orderPickPos).toBeNull();
  expect(board.promptDraft).toEqual({ kind: "order", order: [1, 2, 0] });
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({ kind: "choose_order", player: 0, order: [1, 2, 0] });
});

test("order_triggers drag-end clears a cancelled pick", () => {
  const s = state({
    pending_choice: {
      kind: "order_triggers",
      count: 2,
      labels: ["A", "B"],
      player: 0,
      source: 1,
    },
  });
  const gf = gameFold(s);
  let board = updateBoard(initialBoardModel(), PromptOrderRowClicked({ pos: 0 }), gf, "T1")[0];
  expect(board.orderPickPos).toBe(0);
  board = updateBoard(board, PromptOrderDragEnded(), gf, "T1")[0];
  expect(board.orderPickPos).toBeNull();
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

test("scry prompt shows Top and Bottom lanes instead of a flat ordered grid", () => {
  const s = state({
    pending_choice: {
      kind: "scry",
      player: 0,
      items: [
        { id: 1, label: "Island" },
        { id: 2, label: "Forest" },
      ],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-arrange-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-top")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-bottom")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-bottom-label")).toHaveText("Bottom of library"),
    Scene.expect(Scene.testId("prompt-card-1")).toExist(),
    Scene.expect(Scene.testId("prompt-card-2")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
  );
});

test("surveil bottom lane is labeled Graveyard", () => {
  const s = state({
    pending_choice: {
      kind: "surveil",
      player: 0,
      items: [{ id: 3, label: "Swamp" }],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-arrange-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-arrange-bottom-label")).toHaveText("Graveyard"),
  );
});

test("scry card click moves from Bottom lane to Top lane", () => {
  const s = state({
    pending_choice: {
      kind: "scry",
      player: 0,
      items: [
        { id: 1, label: "Island" },
        { id: 2, label: "Forest" },
      ],
    },
  });
  const gf = gameFold(s);
  let board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 1 }), gf, "T1")[0];
  expect(board.promptDraft).toEqual({
    kind: "partition",
    buckets: { top: [1], bottom: [2] },
  });
  board = updateBoard(board, PromptCardToggled({ id: 1 }), gf, "T1")[0];
  expect(board.promptDraft).toEqual({
    kind: "partition",
    buckets: { top: [], bottom: [2, 1] },
  });
});

test("select_from_top Take lane click emits select_from_top intent", () => {
  const s = state({
    pending_choice: {
      kind: "select_from_top",
      up_to: 2,
      player: 0,
      items: [
        { id: 1, label: "A" },
        { id: 2, label: "B" },
      ],
    },
  });
  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 1 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "select_from_top",
    player: 0,
    cards: [1],
  });
});

test("distribute_top shows docked Hand Bottom Exile lanes", () => {
  const s = state({
    pending_choice: {
      kind: "distribute_top",
      player: 0,
      to_hand: 1,
      to_bottom: 1,
      to_exile_may_play: 1,
      items: [
        { id: 1, label: "A" },
        { id: 2, label: "B" },
        { id: 3, label: "C" },
      ],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-distribute-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-distribute-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-distribute-pool")).toExist(),
    Scene.expect(Scene.testId("prompt-distribute-hand")).toExist(),
    Scene.expect(Scene.testId("prompt-distribute-bottom")).toExist(),
    Scene.expect(Scene.testId("prompt-distribute-exile")).toExist(),
    Scene.expect(Scene.testId("prompt-partition-1-to_hand")).not.toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
  );
});

test("distribute_top card click cycles into Hand then Bottom", () => {
  const pending = {
    kind: "distribute_top" as const,
    player: 0,
    to_hand: 1,
    to_bottom: 1,
    to_exile_may_play: 1,
    items: [
      { id: 1, label: "A" },
      { id: 2, label: "B" },
      { id: 3, label: "C" },
    ],
  };
  const gf = gameFold(state({ pending_choice: pending }));
  let board = updateBoard(initialBoardModel(), PromptPartitionSet({ id: 1, bucket: "to_hand" }), gf, "T1")[0];
  expect(board.promptDraft).toEqual({
    kind: "partition",
    buckets: { to_hand: [1], to_bottom: [], to_exile_may_play: [] },
  });
  board = updateBoard(board, PromptPartitionSet({ id: 1, bucket: "to_bottom" }), gf, "T1")[0];
  expect(board.promptDraft).toEqual({
    kind: "partition",
    buckets: { to_hand: [], to_bottom: [1], to_exile_may_play: [] },
  });
});

test("partition_revealed shows docked Pile A and Pile B lanes", () => {
  const s = state({
    pending_choice: {
      kind: "partition_revealed",
      player: 0,
      source: 9,
      items: [
        { id: 1, label: "A" },
        { id: 2, label: "B" },
      ],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-partition-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-partition-lanes")).toExist(),
    Scene.expect(Scene.testId("prompt-partition-a")).toExist(),
    Scene.expect(Scene.testId("prompt-partition-b")).toExist(),
    Scene.expect(Scene.testId("prompt-card-1")).toExist(),
    Scene.expect(Scene.testId("prompt-card-2")).toExist(),
  );
});

test("partition_revealed card click moves into Pile A", () => {
  const gf = gameFold(
    state({
      pending_choice: {
        kind: "partition_revealed",
        player: 0,
        source: 9,
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
      },
    }),
  );
  const board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 1 }), gf, "T1")[0];
  expect(board.promptDraft).toEqual({ kind: "partition", buckets: { pile_a: [1] } });
});

test("choose_dredge shows Draw normally and disables Dredge until one pick", () => {
  const s = state({
    pending_choice: {
      kind: "choose_dredge",
      player: 0,
      items: [
        { id: 61, label: "Stinkweed Imp" },
        { id: 62, label: "Golgari Thug" },
      ],
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Draw normally"),
    Scene.click(Scene.testId("prompt-card-61")),
    Scene.expect(Scene.testId("prompt-submit")).toBeEnabled(),
  );
});

test("choose_dredge decline emits choose_dredge with null dredger", () => {
  const intents = clickPromptIntent(
    state({
      pending_choice: {
        kind: "choose_dredge",
        player: 0,
        items: [{ id: 61, label: "Stinkweed Imp" }],
      },
    }),
    Scene.click(Scene.testId("prompt-decline")),
  );
  expect(intents).toEqual([{ kind: "choose_dredge", player: 0, dredger: null }]);
});

test("optional on-board choose_target Decline emits empty choose_targets", () => {
  const intents = clickPromptIntent(
    state({
      objects: [
        {
          controller: 0,
          has_haste: false,
          id: 7,
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
        },
      ],
      pending_choice: {
        kind: "choose_target",
        label: "Target creature",
        max: 1,
        optional: true,
        player: 0,
        source: 1,
        items: [{ id: 7, label: "Bear" }],
      },
    }),
    Scene.click(Scene.testId("prompt-decline")),
  );
  expect(intents).toEqual([{ kind: "choose_targets", player: 0, targets: [] }]);
});

test("choose_dredge submit emits chosen dredger", () => {
  const s = state({
    pending_choice: {
      kind: "choose_dredge",
      player: 0,
      items: [{ id: 61, label: "Stinkweed Imp" }],
    },
  });
  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptCardToggled({ id: 61 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_dredge",
    player: 0,
    dredger: 61,
  });
});

test("revealed_card_to_battlefield_or_hand Battlefield submits revealed choice", () => {
  const s = state({
    pending_choice: {
      kind: "revealed_card_to_battlefield_or_hand",
      player: 0,
      item: { id: 17, label: "Beast" },
    },
  });
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-destination-battlefield")));
  expect(intents).toEqual([{ kind: "revealed_card_to_battlefield_or_hand", player: 0, choice: 17 }]);
});

test("revealed_card_to_battlefield_or_hand Hand puts the card in hand", () => {
  const s = state({
    pending_choice: {
      kind: "revealed_card_to_battlefield_or_hand",
      player: 0,
      item: { id: 17, label: "Beast" },
    },
  });
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-destination-hand")));
  expect(intents).toEqual([{ kind: "revealed_card_to_battlefield_or_hand", player: 0, choice: null }]);
});

test("search_library Choose submits selected card", () => {
  const s = state({
    pending_choice: {
      kind: "search_library",
      player: 0,
      items: [
        { id: 1, label: "Sol Ring" },
        { id: 2, label: "Forest" },
      ],
    },
  });
  const commands: unknown[] = [];
  const update = (model: ViewModel, message: Message): readonly [ViewModel, ReadonlyArray<never>] => {
    const [board, nextCommands] = updateBoard(model.board, message, model.fold, model.tableId);
    commands.push(...nextCommands);
    return [{ ...model, board }, []];
  };
  Scene.scene(
    { update, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.click(Scene.testId("prompt-card-1")),
    Scene.click(Scene.testId("prompt-submit")),
  );
  expect(commands.map(intentFromCommand)).toEqual([{ kind: "search_library", player: 0, choice: 1 }]);
});

test("search_library Fail to find declines", () => {
  const s = state({
    pending_choice: {
      kind: "search_library",
      player: 0,
      items: [{ id: 1, label: "Sol Ring" }],
    },
  });
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-decline")));
  expect(intents).toEqual([{ kind: "search_library", player: 0, choice: null }]);
});

test("opponent_chooses_revealed_to_graveyard card click submits choose_exiled", () => {
  const s = state({
    pending_choice: {
      kind: "opponent_chooses_revealed_to_graveyard",
      player: 0,
      source: 1,
      items: [
        { id: 21, label: "Island" },
        { id: 22, label: "Swamp" },
      ],
    },
  });
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-card-21")));
  expect(intents).toEqual([{ kind: "choose_exiled_with_card", player: 0, choice: 21 }]);
});

test("opponent_chooses_revealed_to_graveyard Choose none declines", () => {
  const s = state({
    pending_choice: {
      kind: "opponent_chooses_revealed_to_graveyard",
      player: 0,
      source: 1,
      items: [{ id: 21, label: "Island" }],
    },
  });
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-decline")));
  expect(intents).toEqual([{ kind: "choose_exiled_with_card", player: 0, choice: null }]);
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

test("assign_combat_damage stepper increments a blocker amount", () => {
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
  const pending = {
    kind: "assign_combat_damage" as const,
    player: 0,
    source: 9,
    items: [
      { id: 20, label: "Bear" },
      { id: 21, label: "Elf" },
    ],
  };
  const s = state({ objects: [attacker], pending_choice: pending });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(
      viewModel(s, {
        ...initialBoardModel(),
        pendingChoiceKey: choiceDraftKey(pending),
        promptDraft: { kind: "damage", amounts: { 20: 4, 21: 0 } },
      }),
    ),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-damage-20-value")).toHaveText("4"),
    Scene.expect(Scene.testId("prompt-damage-20-inc")).toBeDisabled(),
    Scene.click(Scene.testId("prompt-damage-20-dec")),
    Scene.expect(Scene.testId("prompt-damage-20-value")).toHaveText("3"),
    Scene.expect(Scene.testId("prompt-damage-assigned")).toHaveText("assigned 3 / 4"),
  );
});

test("trample assign_combat_damage submit allows under-assign overflow to defender", () => {
  const attacker: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 9,
    is_commander: false,
    keywords: ["trample"],
    kind: { kind: "creature", power: 5, toughness: 5 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Trampler",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 5,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 5,
    zone: 2,
  };
  const pending = {
    kind: "assign_combat_damage" as const,
    player: 0,
    source: 9,
    items: [
      { id: 20, label: "Bear" },
      { id: 21, label: "Elf" },
    ],
  };
  const s = state({
    objects: [attacker],
    pending_choice: pending,
  });
  const board: BoardModel = {
    ...initialBoardModel(),
    pendingChoiceKey: choiceDraftKey(pending),
    promptDraft: { kind: "damage", amounts: { 20: 2, 21: 0 } },
  };
  const [, commands] = updateBoard(board, PromptSubmitted(), gameFold(s), "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "assign_damage",
    player: 0,
    assignment: [
      { blocker: 20, amount: 2 },
      { blocker: 21, amount: 0 },
    ],
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

test("pay_cost prompt shows cost on Pay and Don't pay decline", () => {
  const s = state({
    pending_choice: {
      kind: "pay_cost",
      cost: { colored: [0, 0, 0, 1, 0], generic: 2 },
      label: "Create a Fungus Beast",
      player: 0,
      source: 1,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-pay-cost-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-pay")).toHaveText("Pay {2}{R}"),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Don't pay"),
  );
});

test("pay_echo_or_sacrifice decline is labeled Sacrifice", () => {
  const s = state({
    pending_choice: {
      kind: "pay_echo_or_sacrifice",
      cost: { colored: [], generic: 1 },
      player: 0,
      source: 9,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-pay")).toHaveText("Pay {1}"),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Sacrifice"),
  );
});

test("pay_or_counter decline is labeled Let it be countered", () => {
  const s = state({
    pending_choice: {
      kind: "pay_or_counter",
      cost: { colored: [0, 1, 0, 0, 0], generic: 0 },
      player: 0,
      spell: 3,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("prompt-pay")).toHaveText("Pay {U}"),
    Scene.expect(Scene.testId("prompt-decline")).toHaveText("Let it be countered"),
  );
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

test("choose_mode aim docks above the hand bar", () => {
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(
      viewModel(
        state({
          pending_choice: {
            kind: "choose_mode",
            labels: ["Mode A", "Mode B"],
            player: 0,
            source: 1,
          },
        }),
      ),
    ),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-mode-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
});

test("choose_target_players on-board aim shows pending-player-aim chrome", () => {
  const s = state({
    pending_choice: {
      kind: "choose_target_players",
      label: "Choose opponents",
      min: 1,
      max: 2,
      player: 0,
      source: 1,
      items: [
        { id: 0, label: "Bob", player: 1 },
        { id: 1, label: "Carol", player: 2 },
      ],
    },
    players: [
      {
        player: 0,
        username: "Alice",
        life: 40,
        hand_count: 7,
        library_count: 80,
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
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-player-aim")).toExist(),
    Scene.expect(Scene.testId("pending-player-count")).toHaveText("0 / 2 selected"),
    Scene.expect(Scene.testId("prompt-player-1")).not.toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
  );
});

test("choose_splitting_opponent on-board aim shows pending-player-aim chrome", () => {
  const s = state({
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
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-player-aim")).toExist(),
    Scene.expect(Scene.testId("prompt-player-2")).not.toExist(),
  );
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

test("choose_color prompt renders mana-font pips instead of letter labels", () => {
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(
      viewModel(
        state({
          pending_choice: {
            kind: "choose_color",
            player: 0,
            source: 1,
          },
        }),
      ),
    ),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-color-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-color-0")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-0")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-1")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-2")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-3")).toExist(),
    Scene.expect(Scene.testId("prompt-color-pip-4")).toExist(),
  );
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

test("choose_card_name prompt has placeholder and Names a typed card", () => {
  const s = state({
    pending_choice: {
      kind: "choose_card_name",
      player: 0,
      source: 5,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.placeholder("Card name")).toExist(),
    Scene.expect(Scene.testId("prompt-name-input")).toExist(),
    Scene.expect(Scene.testId("prompt-submit")).toBeDisabled(),
  );

  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptStringSet({ value: "Lightning Bolt" }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_card_name",
    player: 0,
    name: "Lightning Bolt",
  });
});

test("may_draw_up_to prompt emits choose_draw_count intent from UI", () => {
  const s = state({
    pending_choice: {
      kind: "may_draw_up_to",
      max: 3,
      player: 0,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-draw-count-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
  );
  const intents = clickPromptIntent(s, Scene.click(Scene.testId("prompt-number-2")));
  expect(intents).toEqual([{ kind: "choose_draw_count", player: 0, count: 2 }]);
});

test("pay_any_amount_of_mana uses a stepper and submits the draft amount", () => {
  const s = state({
    pending_choice: {
      kind: "pay_any_amount_of_mana",
      max: 12,
      player: 0,
      source: 7,
    },
  });
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("pending-join-forces-aim")).toExist(),
    Scene.expect(Scene.testId("pending-choice")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-number-value")).toHaveText("0"),
    Scene.expect(Scene.testId("prompt-number-0")).not.toExist(),
    Scene.expect(Scene.testId("prompt-number-dec")).toBeDisabled(),
    Scene.click(Scene.testId("prompt-number-inc")),
    Scene.click(Scene.testId("prompt-number-inc")),
    Scene.expect(Scene.testId("prompt-number-value")).toHaveText("2"),
  );
  const gf = gameFold(s);
  const board = updateBoard(initialBoardModel(), PromptNumberSet({ count: 2 }), gf, "T1")[0];
  const [, commands] = updateBoard(board, PromptSubmitted(), gf, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "pay_optional_cost",
    player: 0,
    pay: true,
    x: 2,
  });
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

function xCost(overrides: Partial<WireCost> = {}): WireCost {
  return {
    generic: 1,
    colored: [0, 0, 0, 0, 0],
    has_x: true,
    x_symbols: 1,
    ...overrides,
  };
}

function xAction(): ActionView {
  return {
    id: 12,
    kind: "cast",
    label: "Comet Storm",
    has_x: true,
    min_x: 0,
    max_x: 3,
    x_cost: xCost(),
    needs_target: false,
    section: "hand",
  };
}

test("XDraftSet clamps draftX into min/max", () => {
  const board = {
    ...initialBoardModel(),
    xPrompt: {
      action: xAction(),
      target: null,
      picks: emptyCostPicks(),
      modes: [],
      name: "Comet Storm",
      minX: 0,
      maxX: 3,
      draftX: 3,
      xCost: xCost(),
    },
  };
  const [next] = updateBoard(board, XDraftSet({ x: 99 }), gameFold(state()), "T1");
  expect(next.xPrompt?.draftX).toBe(3);
  const [low] = updateBoard(next, XDraftSet({ x: -5 }), gameFold(state()), "T1");
  expect(low.xPrompt?.draftX).toBe(0);
});

test("XSubmitted confirms draft X on the cast intent", () => {
  const [, commands] = updateBoard(
    {
      ...initialBoardModel(),
      xPrompt: {
        action: xAction(),
        target: null,
        picks: emptyCostPicks(),
        modes: [],
        name: "Comet Storm",
        minX: 0,
        maxX: 3,
        draftX: 2,
        xCost: xCost(),
      },
    },
    XSubmitted({ x: 2 }),
    gameFold(state()),
    "T1",
  );
  expect(commands.length).toBeGreaterThan(0);
  expect(intentFromCommand(commands[0])).toMatchObject({ x: 2 });
});

test("XSubmitted clamps out-of-range x before submit", () => {
  const [, commands] = updateBoard(
    {
      ...initialBoardModel(),
      xPrompt: {
        action: xAction(),
        target: null,
        picks: emptyCostPicks(),
        modes: [],
        name: "Comet Storm",
        minX: 0,
        maxX: 3,
        draftX: 3,
        xCost: xCost(),
      },
    },
    XSubmitted({ x: 99 }),
    gameFold(state()),
    "T1",
  );
  expect(intentFromCommand(commands[0])).toMatchObject({ x: 3 });
});

test("choose-X stepper dec updates value and preview via the view", () => {
  const s = state();
  const board: BoardModel = {
    ...initialBoardModel(),
    xPrompt: {
      action: xAction(),
      target: null,
      picks: emptyCostPicks(),
      modes: [],
      name: "Comet Storm",
      minX: 0,
      maxX: 3,
      draftX: 3,
      xCost: xCost(),
    },
  };
  Scene.scene(
    { update: sceneUpdate, view },
    Scene.with(viewModel(s, board)),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("x-prompt-value")).toHaveText("3"),
    Scene.expect(Scene.testId("x-prompt-inc")).toBeDisabled(),
    Scene.click(Scene.testId("x-prompt-dec")),
    Scene.expect(Scene.testId("x-prompt-value")).toHaveText("2"),
    Scene.expect(Scene.testId("x-prompt-inc")).toBeEnabled(),
    Scene.expect(Scene.testId("x-prompt-preview")).toHaveText("Pay {3}"),
  );
});
