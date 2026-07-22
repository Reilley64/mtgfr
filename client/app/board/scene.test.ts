// Scene tests for board overlays: primary Next button visible when empty stack + your priority,
// and hand-bar tile activation dispatches SubmitIntent.

import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { SetStackDwell, SubmitIntent } from "../game/intents";
import { emptyCostPicks } from "./action/execution";
import type { RenderCard } from "./geometry/layout";
import { avatarPos, STEP, ZONE } from "./geometry/layout";
import { boardOverlays } from "./html/overlays";
import { resolveBoardOverlayMounts } from "./html/scene-helpers";
import {
  BoardPointerUp,
  HandActionActivated,
  type Message,
  PendingChoiceAnswered,
  RadialOptionPicked,
  StackDwellChanged,
} from "./messages";
import { type BoardModel, initialBoardModel, updateBoard } from "./submodel";

const h = html<Message>();

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const view = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

function player(overrides: Partial<import("~/wire/types").PlayerView> = {}): import("~/wire/types").PlayerView {
  return {
    commander_tax: 0,
    hand_count: 1,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...overrides,
  };
}

function state(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [player(), player({ player: 1, username: "Bob" })],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function fold(state: VisibleState | null): GameFoldState {
  return {
    seq: 1,
    state,
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

function viewModel(fold: GameFoldState): ViewModel {
  return { board: initialBoardModel(), fold, tableId: "T1" };
}

const update = (model: ViewModel, message: Message): readonly [ViewModel, ReadonlyArray<never>] => {
  const [board] = updateBoard(model.board, message, model.fold, model.tableId);
  return [{ ...model, board }, []];
};

function overlayScene(model: ViewModel, ...steps: readonly unknown[]) {
  Scene.scene<ViewModel, Message>({ update, view }, Scene.with(model), resolveBoardOverlayMounts(), ...(steps as []));
}

test("primary Next visible when empty stack + your priority", () => {
  const model = viewModel(fold(state()));
  overlayScene(
    model,
    Scene.expect(Scene.testId("board-primary")).toExist(),
    Scene.expect(Scene.testId("board-primary")).toContainText("Next"),
    Scene.expect(Scene.testId("board-primary")).toBeEnabled(),
  );
});

test("primary Next disabled when not your priority", () => {
  const model = viewModel(fold(state({ priority: 1, can_act: false })));
  overlayScene(model, Scene.expect(Scene.testId("board-primary")).toBeDisabled());
});

test("hand tile activates when clicked (below hand-bar threshold)", () => {
  const object: ObjectView = {
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
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
  };
  const action: ActionView = {
    id: 7,
    kind: "cast",
    label: "Cast Lightning Bolt",
    needs_target: false,
    object: 42,
    section: "hand",
  };
  const model = viewModel(fold(state({ objects: [object], actions: [action] })));

  Scene.scene<ViewModel, Message>(
    { update: (m, msg) => [update(m, msg)[0], []] as const, view },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("hand-card-42")).toExist(),
  );
});

test("hand-drop planner ignores release below the hand-bar threshold", () => {
  const object: ObjectView = {
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
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
  };
  const action: ActionView = {
    id: 7,
    kind: "cast",
    label: "Cast",
    needs_target: false,
    object: 42,
    section: "hand",
  };
  const gameFold = fold(state({ objects: [object], actions: [action] }));
  const board = initialBoardModel();

  // Drop below threshold (y > viewport - HAND_BAR_H) → ignore, no commands.
  const [nextModel, commands] = updateBoard(board, HandActionActivated({ action, x: 400, y: 900 }), gameFold, "T1");
  expect(commands).toEqual([]);
  expect(nextModel).toEqual(board);

  // Drop above threshold → emits SubmitIntent (untargeted cast fires immediately).
  const [, commandsAbove] = updateBoard(board, HandActionActivated({ action, x: 400, y: 200 }), gameFold, "T1");
  expect(commandsAbove).toHaveLength(1);
  expect(commandsAbove[0]?.name).toBe(SubmitIntent.name);
});

test("stack owns Resolve card, hides primary pass", () => {
  const model = viewModel(
    fold(
      state({
        stack: [{ controller: 1, kind: "spell", label: "Lightning Bolt", source: 99 }],
      }),
    ),
  );
  overlayScene(
    model,
    Scene.expect(Scene.testId("board-primary")).toBeAbsent(),
    Scene.expect(Scene.testId("board-pass")).toExist(),
  );
});

// ── Targeting / combat / prompt / handHidden / dwell wiring ────────────────────────

function creature(id: number, controller: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    marked_damage: 0,
    name: `C${id}`,
    needs_target: false,
    owner: controller,
    plus_counters: 0,
    power: 2,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function renderStub(id: number): RenderCard {
  return {
    id,
    x: 0,
    y: 0,
    w: 96,
    h: 134,
    name: `C${id}`,
    cardId: "",
    print: "",
    pt: "2/2",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: false,
    zone: ZONE.Battlefield,
    controller: 0,
    owner: 0,
    kind: "creature",
    tapsForMana: false,
    summoningSick: false,
    hasHaste: true,
    keywords: [],
    goaded: false,
    isCommander: false,
    prepared: false,
    pile: 0,
    cluster: 0,
    clusterMembers: [],
  };
}

test("pointer up on legal staged target emits SubmitIntent (target completion)", () => {
  const attacker = creature(11, 0);
  const target = creature(22, 1, { name: "Grizzly Bears" });
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: "Cast Bolt",
    needs_target: true,
    object: attacker.id,
    section: "hand",
    targets: [{ kind: "object", id: 22 }],
  };
  const board: BoardModel = {
    ...initialBoardModel(),
    staged: {
      card: attacker,
      action: castAction,
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
    pointer: { kind: "drag", card: renderStub(22), x: 100, y: 100, moved: false },
  };
  const gameFold = fold(state({ objects: [attacker, target] }));
  const [nextBoard, commands] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");
  expect(nextBoard.staged).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
});

test("pointer up on non-target while staged clears drag without submitting", () => {
  const attacker = creature(11, 0);
  const other = creature(99, 1);
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: "Cast Bolt",
    needs_target: true,
    object: attacker.id,
    section: "hand",
    targets: [{ kind: "object", id: 22 }],
  };
  const board: BoardModel = {
    ...initialBoardModel(),
    staged: {
      card: attacker,
      action: castAction,
      picks: emptyCostPicks(),
      preferPick: false,
      playOrigin: { x: 0, y: 0 },
      playOriginScreen: { x: 0, y: 0 },
    },
    pointer: { kind: "drag", card: renderStub(other.id), x: 100, y: 100, moved: false },
  };
  const gameFold = fold(state({ objects: [attacker, other] }));
  const [, commands] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");
  expect(commands).toEqual([]);
});

test("pointer combat drop on opponent life orb stages an attacker", () => {
  const myCreature = creature(30, 0, { has_haste: true });
  const gameFold = fold(
    state({
      step: STEP.DeclareAttackers,
      objects: [myCreature],
      combat: {
        attackers: [],
        blocks: [],
        attackers_declared: false,
        blockers_declared: [],
      },
      actions: [
        { id: 1, kind: "declare_attackers", label: "Attack", needs_target: false, section: "combat" } as ActionView,
      ],
    }),
  );
  const opponentAvatar = avatarPos(1, 0, 2);
  const board: BoardModel = {
    ...initialBoardModel(),
    pointer: { kind: "drag", card: renderStub(myCreature.id), x: 0, y: 0, moved: true },
  };
  const [nextBoard, commands] = updateBoard(
    board,
    BoardPointerUp({ x: opponentAvatar.x, y: opponentAvatar.y }),
    gameFold,
    "T1",
  );
  expect(commands).toEqual([]);
  expect(nextBoard.combatAttackers).toEqual([{ attacker: myCreature.id, defender: 1 }]);
  expect(nextBoard.pointer).toEqual({ kind: "idle" });
});

test("PendingChoiceAnswered folds into a SubmitIntent command", () => {
  const board = initialBoardModel();
  const gameFold = fold(state());
  const [, commands] = updateBoard(
    board,
    PendingChoiceAnswered({ intent: { kind: "answer_may", player: 0, yes: true } }),
    gameFold,
    "T1",
  );
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
});

test("prompt Yes/No buttons appear for may_yes_no pending choice", () => {
  const model = viewModel(
    fold(
      state({
        pending_choice: { kind: "may_yes_no", label: "Cast?", player: 0, source: 1 },
      }),
    ),
  );
  overlayScene(
    model,
    Scene.expect(Scene.testId("prompt-yes")).toExist(),
    Scene.expect(Scene.testId("prompt-no")).toExist(),
  );
});

test("handHidden suppresses a hand tile that would otherwise render", () => {
  const object = creature(77, 0, { zone: ZONE.Hand });
  const action: ActionView = {
    id: 4,
    kind: "cast",
    label: "Cast",
    needs_target: false,
    object: 77,
    section: "hand",
  };
  const base = viewModel(fold(state({ objects: [object], actions: [action] })));
  // Tile exists by default.
  overlayScene(base, Scene.expect(Scene.testId("hand-card-77")).toExist());
  // With id in handHidden, tile is gone.
  const hidden: ViewModel = { ...base, board: { ...base.board, handHidden: new Set([77]) } };
  overlayScene(hidden, Scene.expect(Scene.testId("hand-card-77")).toBeAbsent());
});

test("selected permanent with tap-for-mana shows a single-option activation radial", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const base = viewModel(fold(state({ objects: [land], can_act: true })));
  const selected: ViewModel = { ...base, board: { ...base.board, selectedId: 5 } };
  overlayScene(
    selected,
    Scene.expect(Scene.testId("activation-radial")).toExist(),
    Scene.expect(Scene.testId("radial-wedge-tap_for_mana")).toExist(),
  );
});

test("RadialOptionPicked tap_for_mana submits tap_for_mana intent", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const board: BoardModel = { ...initialBoardModel(), selectedId: 5 };
  const gameFold = fold(state({ objects: [land], can_act: true }));
  const [next, commands] = updateBoard(board, RadialOptionPicked({ index: 0 }), gameFold, "T1");
  expect(next.selectedId).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
});

test("StackDwellChanged emits a SetStackDwell command", () => {
  const board = initialBoardModel();
  const gameFold = fold(state());
  const [, commands] = updateBoard(board, StackDwellChanged({ dwelling: true }), gameFold, "T1");
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SetStackDwell.name);
});

test("mana tray renders when a player has mana in pool", () => {
  const model = viewModel(
    fold(
      state({
        players: [
          player({
            mana_pool: { any: 0, colored: [2, 0, 0, 0, 0], colorless: 0, either: [], of_colors: [] },
          }),
          player({ player: 1, username: "Bob" }),
        ],
      }),
    ),
  );
  overlayScene(
    model,
    Scene.expect(Scene.testId("mana-tray")).toExist(),
    Scene.expect(Scene.selector('[data-mana-tray-seat="0"]')).toExist(),
  );
});

test("mana tray hidden when all pools are empty", () => {
  const model = viewModel(fold(state()));
  overlayScene(model, Scene.expect(Scene.testId("mana-tray")).toBeAbsent());
});

test("log panel shows last lines with AUTO chip", () => {
  const model = viewModel({
    ...fold(state()),
    log: [
      { seq: 1, text: "Alice draws a card", auto: true },
      { seq: 2, text: "Bob casts Lightning Bolt" },
    ],
  });
  overlayScene(
    model,
    Scene.expect(Scene.testId("board-log")).toExist(),
    Scene.expect(Scene.testId("board-log")).toContainText("AUTO"),
    Scene.expect(Scene.testId("board-log")).toContainText("Lightning Bolt"),
  );
});

test("log panel hidden when log is empty", () => {
  const model = viewModel(fold(state()));
  overlayScene(model, Scene.expect(Scene.testId("board-log")).toBeAbsent());
});
