/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { emptyCostPicks } from "../action/execution";
import { ZONE } from "../geometry/layout";
import { STACK_EXPAND_COUNT } from "../geometry/stackLayout";
import { type Message, StackCollapseClicked } from "../messages";
import { type BoardModel, initialBoardModel, updateBoard } from "../submodel";
import { boardOverlays } from "./overlays";
import { resolveBoardOverlayMounts } from "./scene-helpers";

const h = html<Message>();

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const overlayView = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

function player(): import("~/wire/types").PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
  };
}

function gameState(over: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [player(), { ...player(), player: 1, username: "Bob" }],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

function gameFold(state: VisibleState): GameFoldState {
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

function spellOnStack(
  sourceId: number,
  label: string,
  print: string,
): { stack: VisibleState["stack"]; objects: ObjectView[] } {
  const spell: ObjectView = {
    controller: 0,
    has_haste: false,
    id: sourceId,
    is_commander: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: label,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print,
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Stack,
  };
  return {
    objects: [spell],
    stack: [{ controller: 0, kind: "spell", label, source: sourceId }],
  };
}

test("stack overlay renders card art for spells on the stack", () => {
  const { objects, stack } = spellOnStack(42, "Lightning Bolt", "bolt-print");
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.selector("img")).toExist(),
  );
});

test("staged ghost appears on the stack during arrow targeting", () => {
  const handCard: ObjectView = {
    controller: 0,
    has_haste: false,
    id: 11,
    is_commander: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Shock",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "shock-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
  };
  const target: ObjectView = {
    controller: 1,
    has_haste: false,
    id: 22,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 1,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
  };
  const castAction: ActionView = {
    id: 9,
    kind: "cast",
    label: "Cast Shock",
    needs_target: true,
    object: handCard.id,
    section: "hand",
    targets: [{ kind: "object", id: 22 }],
  };
  const model: ViewModel = {
    board: {
      ...initialBoardModel(),
      staged: {
        card: handCard,
        action: castAction,
        picks: emptyCostPicks(),
        preferPick: false,
        playOrigin: { x: 0, y: 0 },
        playOriginScreen: { x: 0, y: 0 },
      },
    },
    fold: gameFold(gameState({ objects: [handCard, target] })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toExist(),
    Scene.expect(Scene.testId("stack-face-0")).toExist(),
    Scene.expect(Scene.testId("stack-staged-hint")).toContainText("Choose a target"),
    Scene.expect(Scene.selector("img")).toExist(),
  );
});

test("stack overlay hidden when stack is empty and nothing is staged", () => {
  const model: ViewModel = { board: initialBoardModel(), fold: gameFold(gameState()), tableId: "T1" };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-overlay")).toBeAbsent(),
  );
});

test("expand button appears for a tall stack and opens strip view", () => {
  const objects: ObjectView[] = [];
  const stack: VisibleState["stack"] = [];
  for (let i = 0; i < STACK_EXPAND_COUNT; i++) {
    const id = 100 + i;
    objects.push({
      controller: 0,
      has_haste: false,
      id,
      is_commander: false,
      kind: { kind: "instant" },
      mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
      marked_damage: 0,
      name: `Spell ${i}`,
      needs_target: false,
      owner: 0,
      plus_counters: 0,
      power: 0,
      print: `print-${i}`,
      summoning_sick: false,
      tapped: false,
      toughness: 0,
      zone: ZONE.Stack,
    });
    stack.push({ controller: 0, kind: "spell", label: `Spell ${i}`, source: id });
  }
  const model: ViewModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ objects, stack })),
    tableId: "T1",
  };
  Scene.scene(
    {
      update: (m, msg: Message) => {
        const [board] = updateBoard(m.board, msg, m.fold, m.tableId);
        return [{ ...m, board }, []];
      },
      view: overlayView,
    },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-expand")).toExist(),
    Scene.click(Scene.testId("stack-expand")),
    Scene.expect(Scene.testId("stack-overlay-expanded")).toExist(),
  );
});

test("StackCollapseClicked collapses expanded stack", () => {
  const board = { ...initialBoardModel(), stackExpand: true };
  const next = updateBoard(board, StackCollapseClicked(), gameFold(gameState()), "T1")[0];
  expect(next.stackExpand).toBe(false);
});

test("hold bar renders when stack_hold_remaining_ms is positive", () => {
  const { objects, stack } = spellOnStack(42, "Bolt", "bolt-print");
  const model: ViewModel = {
    board: { ...initialBoardModel(), stackHoldPeak: 2000 },
    fold: gameFold(gameState({ objects, stack, stack_hold_remaining_ms: 1500 })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("stack-hold-bar")).toExist(),
  );
});
