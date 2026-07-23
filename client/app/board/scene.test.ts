// Scene tests for board overlays: primary Next button visible when empty stack + your priority,
// and hand-bar tile activation dispatches SubmitIntent.

/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { beforeAll, expect, test } from "vitest";
import { choiceDraftKey } from "~/choice";
import { BindCardArt } from "~/ui/card-art";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { SetStackDwell, SubmitIntent } from "../game/intents";
import { emptyCostPicks } from "./action/execution";
import { worldToScreen } from "./geometry/camera";
import type { RenderCard } from "./geometry/layout";
import { avatarPos, layout, STEP, ZONE } from "./geometry/layout";
import { activationRadialOuterRadius, radialOverlayPlacement } from "./geometry/radial";
import { boardOverlays } from "./html/overlays";
import { resolveBoardCardArtMounts, resolveBoardOverlayMounts, resolveLiveBoardMounts } from "./html/scene-helpers";
import {
  BoardPointerUp,
  HandActionActivated,
  KeyboardEnterPressed,
  type Message,
  PendingChoiceAnswered,
  PromptSubmitted,
  RadialOptionPicked,
  StackDwellChanged,
  TargetChosen,
} from "./messages";
import { BOARD_VIEWPORT, type BoardModel, initialBoardModel, updateBoard } from "./submodel";
import { type BoardViewModel, view as boardView } from "./view";

const h = html<Message>();

beforeAll(() => {
  class MockImage {
    onload: (() => void) | null = null;
    onerror: (() => void) | null = null;
    src = "";
    addEventListener(type: string, fn: () => void): void {
      if (type === "load") this.onload = fn;
    }
  }
  // @ts-expect-error test stub
  globalThis.Image = MockImage;
});

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const view = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

const fullBoardView = Submodel.defineView<BoardViewModel, Message>(boardView);

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

function fullBoardModel(fold: GameFoldState): BoardViewModel {
  return { board: initialBoardModel(), fold, tableId: "T1", connected: true };
}

const update = (model: ViewModel, message: Message): readonly [ViewModel, ReadonlyArray<never>] => {
  const [board] = updateBoard(model.board, message, model.fold, model.tableId);
  return [{ ...model, board }, []];
};

function overlayScene(model: ViewModel, ...steps: readonly unknown[]) {
  Scene.scene<ViewModel, Message>({ update, view }, Scene.with(model), resolveBoardOverlayMounts(), ...(steps as []));
}

function staticOverlayScene(model: ViewModel, ...steps: readonly unknown[]) {
  Scene.scene<ViewModel, Message>({ update, view }, Scene.with(model), ...(steps as []));
}

function liveBoardScene(model: BoardViewModel, ...steps: readonly unknown[]) {
  const hint = !model.board.hintDismissed && !model.board.hintAutoHidden;
  Scene.scene<BoardViewModel, Message>(
    { update: (m) => [m, []], view: fullBoardView },
    Scene.with(model),
    resolveLiveBoardMounts({ hint }),
    ...(steps as []),
  );
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

  // Drop below threshold (y > viewport - HAND_BAR_H + HAND_PLAY_SLACK_PX) → ignore, no commands.
  const [nextModel, commands] = updateBoard(
    board,
    HandActionActivated({ action, x: 400, y: BOARD_VIEWPORT.height - 20 }),
    gameFold,
    "T1",
  );
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

function intentFromCommand(cmd: unknown): unknown {
  return (cmd as { args: { intent: unknown } }).args.intent;
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

test("pointer up on on-board pending choose_target submits choose_targets", () => {
  const bear = creature(22, 1, { name: "Grizzly Bears" });
  const board: BoardModel = {
    ...initialBoardModel(),
    pointer: { kind: "drag", card: renderStub(22), x: 100, y: 100, moved: false },
  };
  const gameFold = fold(
    state({
      objects: [bear],
      pending_choice: {
        kind: "choose_target",
        label: "Target creature",
        max: 1,
        optional: false,
        player: 0,
        source: 1,
        items: [{ id: 22, label: "Grizzly Bears" }],
      },
    }),
  );
  const [, commands] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_targets",
    player: 0,
    targets: [{ kind: "object", id: 22 }],
  });
});

test("pointer up on multi on-board choose_target accumulates picks until Confirm", () => {
  const a = creature(1, 1, { name: "A" });
  const b = creature(2, 1, { name: "B" });
  const pending = {
    kind: "choose_target" as const,
    label: "Target creatures",
    max: 2,
    optional: false,
    player: 0,
    source: 1,
    items: [
      { id: 1, label: "A" },
      { id: 2, label: "B" },
    ],
  };
  const gameFold = fold(state({ objects: [a, b], pending_choice: pending }));
  let board: BoardModel = {
    ...initialBoardModel(),
    pointer: { kind: "drag", card: renderStub(1), x: 100, y: 100, moved: false },
  };
  [board] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");
  expect(board.promptDraft).toEqual({ kind: "card-pick", picked: [1], filter: "" });
  board = {
    ...board,
    pointer: { kind: "drag", card: renderStub(2), x: 100, y: 100, moved: false },
  };
  const [next, commands] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");
  expect(commands).toEqual([]);
  expect(next.promptDraft).toEqual({ kind: "card-pick", picked: [1, 2], filter: "" });
  const [, submitCmds] = updateBoard(next, PromptSubmitted(), gameFold, "T1");
  expect(intentFromCommand(submitCmds[0])).toEqual({
    kind: "choose_targets",
    player: 0,
    targets: [
      { kind: "object", id: 1 },
      { kind: "object", id: 2 },
    ],
  });
});

test("Enter confirms multi on-board choose_target when draft is ready", () => {
  const a = creature(1, 1, { name: "A" });
  const b = creature(2, 1, { name: "B" });
  const pending = {
    kind: "choose_target" as const,
    label: "Target creatures",
    max: 2,
    optional: false,
    player: 0,
    source: 1,
    items: [
      { id: 1, label: "A" },
      { id: 2, label: "B" },
    ],
  };
  const gameFold = fold(state({ objects: [a, b], pending_choice: pending }));
  const board: BoardModel = {
    ...initialBoardModel(),
    promptDraft: { kind: "card-pick", picked: [1, 2], filter: "" },
    pendingChoiceKey: choiceDraftKey(pending),
  };
  const [, commands] = updateBoard(board, KeyboardEnterPressed(), gameFold, "T1");
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_targets",
    player: 0,
    targets: [
      { kind: "object", id: 1 },
      { kind: "object", id: 2 },
    ],
  });
});

test("TargetChosen accumulates multi on-board stack targets until Confirm", () => {
  const spellA = creature(40, 0, { name: "Spell A", zone: ZONE.Stack });
  const spellB = creature(41, 0, { name: "Spell B", zone: ZONE.Stack });
  const pending = {
    kind: "choose_target" as const,
    label: "Target spells",
    max: 2,
    optional: false,
    player: 0,
    source: 1,
    items: [
      { id: 40, label: "Spell A" },
      { id: 41, label: "Spell B" },
    ],
  };
  const gameFold = fold(
    state({
      objects: [spellA, spellB],
      stack: [
        { controller: 0, kind: "spell", label: "Spell A", source: 40 },
        { controller: 0, kind: "spell", label: "Spell B", source: 41 },
      ],
      pending_choice: pending,
    }),
  );
  let board: BoardModel = initialBoardModel();
  let commands: ReturnType<typeof updateBoard>[1];
  [board, commands] = updateBoard(board, TargetChosen({ target: { kind: "object", id: 40 } }), gameFold, "T1");
  expect(commands).toEqual([]);
  expect(board.promptDraft).toEqual({ kind: "card-pick", picked: [40], filter: "" });
  [board, commands] = updateBoard(board, TargetChosen({ target: { kind: "object", id: 41 } }), gameFold, "T1");
  expect(commands).toEqual([]);
  expect(board.promptDraft).toEqual({ kind: "card-pick", picked: [40, 41], filter: "" });
  [, commands] = updateBoard(board, PromptSubmitted(), gameFold, "T1");
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "choose_targets",
    player: 0,
    targets: [
      { kind: "object", id: 40 },
      { kind: "object", id: 41 },
    ],
  });
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

test("may_yes_no prompt mounts only for the awaited seat", () => {
  const pendingChoice = { kind: "may_yes_no", label: "Cast?", player: 0, source: 1 } as const;
  overlayScene(
    viewModel(fold(state({ pending_choice: pendingChoice, viewer: 0 }))),
    Scene.expect(Scene.testId("prompt-yes")).toExist(),
    Scene.expect(Scene.testId("prompt-no")).toExist(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toBeAbsent(),
  );
  overlayScene(
    viewModel(fold(state({ pending_choice: pendingChoice, viewer: 1 }))),
    Scene.expect(Scene.testId("prompt-yes")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-no")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toHaveText("Waiting for Alice…"),
  );
  staticOverlayScene(
    viewModel(fold(state({ pending_choice: pendingChoice, viewer: 255 }))),
    Scene.expect(Scene.testId("prompt-yes")).toBeAbsent(),
    Scene.expect(Scene.testId("prompt-no")).toBeAbsent(),
    Scene.expect(Scene.testId("pending-choice-waiting")).toHaveText("Waiting for Alice…"),
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

test("selected tapped mana source keeps its disabled tap-for-mana wedge visible", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    tapped: true,
    toughness: 0,
  });
  const base = viewModel(fold(state({ objects: [land], can_act: true })));
  const selected: ViewModel = { ...base, board: { ...base.board, selectedId: 5 } };
  overlayScene(
    selected,
    Scene.expect(Scene.testId("activation-radial")).toExist(),
    Scene.expect(Scene.testId("radial-wedge-tap_for_mana")).toExist(),
    Scene.expect(Scene.selector('[data-testid="radial-wedge-tap_for_mana"] path')).toHaveClass("cursor-not-allowed"),
  );
});

test("activation radial svg is centered on the selected card screen center", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const gameFold = fold(state({ objects: [land], can_act: true }));
  const board: BoardModel = { ...initialBoardModel(), selectedId: 5 };
  const visible = gameFold.state;
  expect(visible).not.toBeNull();
  if (visible == null) return;
  const card = layout(visible, visible.viewer).find((c) => c.id === land.id);
  expect(card).toBeDefined();
  if (card == null) return;
  const center = worldToScreen(board.camera, card.x + card.w / 2, card.y + card.h / 2);
  const size = activationRadialOuterRadius(board.camera.zoom) * 2 + 8;
  const place = radialOverlayPlacement(center, size, board.viewport);
  const selected: ViewModel = { board, fold: gameFold, tableId: "T1" };

  overlayScene(
    selected,
    Scene.expect(Scene.selector("svg")).toHaveStyle("left", place.left),
    Scene.expect(Scene.selector("svg")).toHaveStyle("top", place.top),
    Scene.expect(Scene.selector("svg")).toHaveStyle("width", place.width),
    Scene.expect(Scene.selector("svg")).toHaveStyle("height", place.height),
    Scene.expect(Scene.selector("svg")).toHaveStyle("transform", place.transform),
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

test("RadialOptionPicked ignores a disabled tap_for_mana option", () => {
  const land = creature(5, 0, {
    name: "Forest",
    kind: { kind: "land", colors: [1, 0, 0, 0, 0] },
    taps_for_mana: true,
    power: 0,
    toughness: 0,
  });
  const board: BoardModel = { ...initialBoardModel(), selectedId: 5 };
  const gameFold = fold(state({ objects: [land], can_act: false }));
  const [next, commands] = updateBoard(board, RadialOptionPicked({ index: 0 }), gameFold, "T1");
  expect(next.selectedId).toBe(5);
  expect(commands).toEqual([]);
});

test("pointer click does not select a permanent with no radial option", () => {
  const vanilla = creature(5, 0, {
    name: "Vanilla Land",
    kind: { kind: "land", colors: [] },
    power: 0,
    toughness: 0,
  });
  const board: BoardModel = {
    ...initialBoardModel(),
    pointer: {
      kind: "drag",
      card: { ...renderStub(vanilla.id), kind: "land", tapsForMana: false },
      x: 100,
      y: 100,
      moved: false,
    },
  };
  const gameFold = fold(state({ objects: [vanilla], actions: [], can_act: true }));

  const [next, commands] = updateBoard(board, BoardPointerUp({ x: 100, y: 100 }), gameFold, "T1");

  expect(next.selectedId).toBeNull();
  expect(commands).toEqual([]);
});

test("RadialOptionPicked submits the selected activate action id", () => {
  const seer = creature(5, 0, { name: "Viscera Seer" });
  const action: ActionView = {
    id: 91,
    kind: "activate",
    label: "Scry 1",
    needs_target: false,
    object: seer.id,
    section: "battlefield",
  };
  const board: BoardModel = { ...initialBoardModel(), selectedId: seer.id };
  const gameFold = fold(state({ objects: [seer], actions: [action], can_act: true }));

  const [next, commands] = updateBoard(board, RadialOptionPicked({ index: 0 }), gameFold, "T1");

  expect(next.selectedId).toBeNull();
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SubmitIntent.name);
  expect(intentFromCommand(commands[0])).toEqual({
    kind: "take_action",
    player: 0,
    id: 91,
    target: null,
    x: 0,
    modes: [],
    sacrifice: [],
    discard_cost: [],
    graveyard_exile: [],
  });
});

test("RadialOptionPicked opens sacrifice picker before submitting a payable activate", () => {
  const seer = creature(5, 0, { name: "Viscera Seer" });
  const fodder = creature(6, 0, { name: "Goat" });
  const action: ActionView = {
    id: 92,
    kind: "activate",
    label: "Sacrifice a creature: Scry 1",
    needs_target: false,
    object: seer.id,
    sacrifice_choices: [fodder.id],
    section: "battlefield",
  };
  const board: BoardModel = { ...initialBoardModel(), selectedId: seer.id };
  const gameFold = fold(state({ objects: [seer, fodder], actions: [action], can_act: true }));

  const [next, commands] = updateBoard(board, RadialOptionPicked({ index: 0 }), gameFold, "T1");

  expect(next.selectedId).toBeNull();
  expect(next.sacrificePick?.action).toBe(action);
  expect(next.sacrificePick?.card?.id).toBe(seer.id);
  expect(commands).toEqual([]);
});

test("StackDwellChanged emits a SetStackDwell command", () => {
  const board = initialBoardModel();
  const gameFold = fold(state());
  const [, commands] = updateBoard(board, StackDwellChanged({ dwelling: true }), gameFold, "T1");
  expect(commands).toHaveLength(1);
  expect(commands[0]?.name).toBe(SetStackDwell.name);
});

test("mana tray renders when a player has mana in pool", () => {
  const model = fullBoardModel(
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
  liveBoardScene(
    model,
    Scene.expect(Scene.testId("mana-tray")).toExist(),
    Scene.expect(Scene.selector('[data-mana-tray-seat="0"]')).toExist(),
  );
});

test("mana tray hidden when all pools are empty", () => {
  const model = fullBoardModel(fold(state()));
  liveBoardScene(model, Scene.expect(Scene.testId("mana-tray")).toBeAbsent());
});

test("boardOverlays does not host the battlefield mana tray", () => {
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

test("board hosts keyboard and audio mounts on separate elements", () => {
  // Foldkit keeps only the last OnMount insert hook per DOM node. Stacking
  // keyboard + audio (+ hint) on <main> silently drops Alt inspect (and can
  // drop table audio). Each mount must own its own host element.
  liveBoardScene(
    fullBoardModel(fold(state())),
    Scene.expect(Scene.testId("board-keyboard-mount")).toExist(),
    Scene.expect(Scene.testId("board-audio-mount")).toExist(),
    Scene.expect(Scene.testId("board-hint-mount")).toExist(),
  );

  const hintGone = fullBoardModel(fold(state()));
  liveBoardScene(
    { ...hintGone, board: { ...hintGone.board, hintDismissed: true } },
    Scene.expect(Scene.testId("board-keyboard-mount")).toExist(),
    Scene.expect(Scene.testId("board-audio-mount")).toExist(),
    Scene.expect(Scene.testId("board-hint-mount")).toBeAbsent(),
  );
});

test("inspect overlay docks left with backdrop when pinned", () => {
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      inspectPin: { name: "Sol Ring", prepared: false, print: "sol-ring-print" },
      inspectCard: {
        approximates: null,
        back: null,
        color_identity: [],
        cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
        default_print: "sol-ring-print",
        id: "sol-ring",
        keywords: [],
        kind: { kind: "artifact" },
        legendary: false,
        name: "Sol Ring",
        oracle: "{T}: Add {C}.",
        otags: [],
        set: "soc",
        subtypes: [],
        summary: "Mana rock",
      },
    },
    fold: fold(state()),
    tableId: "T1",
  };

  overlayScene(
    model,
    resolveBoardCardArtMounts(),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("bg-black/55"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("fixed"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("inset-0"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("items-center"),
    Scene.expect(Scene.testId("inspect-overlay")).toHaveClass("z-[100]"),
    Scene.expect(Scene.testId("inspect-overlay")).not.toHaveClass("items-start"),
    Scene.expect(Scene.testId("inspect-overlay")).not.toHaveClass("top-(--y)"),
    Scene.expect(Scene.testId("inspect-overlay")).not.toHaveClass("left-(--x)"),
    Scene.expect(Scene.testId("inspect-overlay")).toContainText(": Add ."),
    Scene.expect(Scene.testId("inspect-overlay")).not.toContainText("Close"),
    Scene.expect(Scene.selector('[aria-label="{C}"]')).toExist(),
  );
});

test("inspect Flip keeps the dock open while backdrop click dismisses it", () => {
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      inspectPin: { name: "Front Face", prepared: false, print: "front-print" },
      inspectCard: {
        approximates: null,
        back: { approximates: null, name: "Back Face", oracle: "Back oracle." },
        color_identity: [],
        cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
        default_print: "front-print",
        id: "double-faced",
        keywords: [],
        kind: { kind: "artifact" },
        legendary: false,
        name: "Front Face",
        oracle: "Front oracle.",
        otags: [],
        set: "soc",
        subtypes: [],
        summary: "DFC",
      },
    },
    fold: fold(state()),
    tableId: "T1",
  };

  overlayScene(
    model,
    resolveBoardCardArtMounts(),
    Scene.click(Scene.selector('[title^="Flip"]')),
    Scene.expect(Scene.testId("inspect-overlay")).toExist(),
    Scene.expect(Scene.testId("inspect-overlay")).toContainText("Back oracle."),
    Scene.click(Scene.testId("inspect-overlay-backdrop")),
    Scene.expect(Scene.testId("inspect-overlay")).toBeAbsent(),
    Scene.Mount.expectEnded(BindCardArt),
  );
});
