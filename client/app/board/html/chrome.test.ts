/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { beforeAll, test } from "vitest";
import { SPECTATOR_VIEWER } from "~/spectator";
import { BindCardArt } from "~/ui/card-art";
import type { ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { ZONE } from "../geometry/layout";
import type { Message } from "../messages";
import { ArtLoaded, PriorityElapsed } from "../messages";
import { type BoardModel, initialBoardModel } from "../submodel";
import { type BoardViewModel, view as boardView } from "../view";
import { MountPriorityWatch } from "./audio-mount";
import { boardOverlays } from "./overlays";
import { resolveBoardOverlayMounts, resolveLiveBoardMounts } from "./scene-helpers";

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

function player(seat: number, lost = false): import("~/wire/types").PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: seat,
    username: seat === 0 ? "Alice" : "Bob",
  };
}

function card(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "land", colors: [] },
    mana_cost: { generic: 0, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Forest",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "forest-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Hand,
    ...overrides,
  };
}

function gameState(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [player(0), player(1)],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...overrides,
  };
}

function gameFold(state: VisibleState | null = gameState()): GameFoldState {
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

type OverlayModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const overlayView = Submodel.defineView<OverlayModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log);
});

const fullBoardView = Submodel.defineView<BoardViewModel, Message>(boardView);

function boardModel(fold: GameFoldState, connected = true): BoardViewModel {
  return {
    board: { ...initialBoardModel(), soundOn: true },
    fold,
    tableId: "T1",
    connected,
  };
}

test("active player sees hand, priority bar, concede, and hint chrome", () => {
  const model: OverlayModel = {
    board: initialBoardModel(),
    fold: gameFold(),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("hand-bar")).toExist(),
    Scene.expect(Scene.testId("board-primary")).toExist(),
    Scene.expect(Scene.testId("board-concede")).toExist(),
    Scene.expect(Scene.testId("board-hint")).toExist(),
    Scene.expect(Scene.testId("board-legend-toggle")).toExist(),
  );
});

test("mulliganing undecided seat sees overlay and hides hand bar", () => {
  const state = gameState({
    mulliganing: true,
    objects: [card(1)],
    players: [
      {
        ...player(0),
        hand_kept: false,
        can_mulligan: true,
        mulligans_taken: 0,
      },
      {
        ...player(1),
        hand_kept: false,
        can_mulligan: true,
        mulligans_taken: 0,
      },
    ],
  });
  const model: OverlayModel = {
    board: {
      ...initialBoardModel(),
      inspectPin: { name: "Forest", prepared: false, cardId: "forest-card", print: "forest-print" },
    },
    fold: gameFold(state),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    Scene.Mount.resolveAll([MountPriorityWatch(), PriorityElapsed({ seconds: 0 })], [BindCardArt, ArtLoaded()]),
    Scene.expect(Scene.testId("mulligan-overlay")).toExist(),
    Scene.expect(Scene.testId("mulligan-keep")).toExist(),
    Scene.expect(Scene.testId("mulligan-take")).toExist(),
    Scene.expect(Scene.testId("mulligan-face-1")).toExist(),
    Scene.expect(Scene.testId("hand-bar")).not.toExist(),
    Scene.expect(Scene.testId("mulligan-bar")).not.toExist(),
    Scene.expect(Scene.testId("board-primary")).not.toExist(),
    Scene.expect(Scene.testId("board-concede")).toExist(),
    Scene.expect(Scene.testId("inspect-overlay")).not.toExist(),
  );
});

test("mulligan take is disabled when can_mulligan is false", () => {
  const state = gameState({
    mulliganing: true,
    objects: [card(1)],
    players: [
      {
        ...player(0),
        hand_kept: false,
        can_mulligan: false,
        mulligans_taken: 6,
      },
      {
        ...player(1),
        hand_kept: false,
        can_mulligan: true,
        mulligans_taken: 0,
      },
    ],
  });
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with({
      board: initialBoardModel(),
      fold: gameFold(state),
      tableId: "T1",
    }),
    Scene.Mount.resolveAll([MountPriorityWatch(), PriorityElapsed({ seconds: 0 })], [BindCardArt, ArtLoaded()]),
    Scene.expect(Scene.testId("mulligan-overlay")).toExist(),
    Scene.expect(Scene.testId("mulligan-take")).toBeDisabled(),
  );
});

test("mulligan kept seat sees waiting banner and hand bar", () => {
  const state = gameState({
    mulliganing: true,
    players: [
      {
        ...player(0),
        hand_kept: true,
        can_mulligan: false,
        mulligans_taken: 0,
      },
      {
        ...player(1),
        hand_kept: false,
        can_mulligan: true,
        mulligans_taken: 0,
      },
    ],
  });
  Scene.scene(
    {
      update: (m) => [m, []],
      view: overlayView,
    },
    Scene.with({
      board: initialBoardModel(),
      fold: gameFold(state),
      tableId: "T1",
    }),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("mulligan-overlay")).not.toExist(),
    Scene.expect(Scene.testId("mulligan-waiting")).toExist(),
    Scene.expect(Scene.testId("mulligan-waiting")).toContainText("Waiting for Bob to choose."),
    Scene.expect(Scene.testId("mulligan-keep")).not.toExist(),
    Scene.expect(Scene.testId("hand-bar")).toExist(),
  );
});

test("declare attackers shows combat staging coach for the active seat", () => {
  const state = gameState({
    step: 5,
    active_player: 0,
    priority: 0,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
  });
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with({
      board: initialBoardModel(),
      fold: gameFold(state),
      tableId: "T1",
    }),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("board-combat-coach")).toContainText("Drag a creature onto an opponent to attack"),
  );
});

test("spectator hides hand, priority bar, concede, and discoverability chrome", () => {
  const model: OverlayModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ viewer: SPECTATOR_VIEWER })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    Scene.expect(Scene.testId("hand-bar")).not.toExist(),
    Scene.expect(Scene.testId("board-primary")).not.toExist(),
    Scene.expect(Scene.testId("board-concede")).not.toExist(),
    Scene.expect(Scene.testId("board-hint")).not.toExist(),
    Scene.expect(Scene.testId("board-legend-toggle")).not.toExist(),
    Scene.expect(Scene.testId("board-spectating")).toExist(),
    Scene.expect(Scene.text("Spectating")).toExist(),
  );
});

test("eliminated player hides action chrome like a spectator", () => {
  const model: OverlayModel = {
    board: initialBoardModel(),
    fold: gameFold(
      gameState({
        viewer: 0,
        players: [player(0, true), player(1)],
      }),
    ),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    Scene.expect(Scene.testId("hand-bar")).not.toExist(),
    Scene.expect(Scene.testId("board-primary")).not.toExist(),
    Scene.expect(Scene.testId("board-concede")).not.toExist(),
    Scene.expect(Scene.testId("board-hint")).not.toExist(),
  );
});

test("connecting empty state shows centered connecting hud", () => {
  Scene.scene(
    { update: (m) => [m, []], view: fullBoardView },
    Scene.with(boardModel(gameFold(null))),
    Scene.expect(Scene.testId("board-connecting")).toExist(),
    Scene.expect(Scene.text("Connecting to the table…")).toExist(),
    Scene.expect(Scene.testId("board-reconnecting")).not.toExist(),
  );
});

test("reconnect banner appears only when disconnected with live state", () => {
  Scene.scene(
    { update: (m) => [m, []], view: fullBoardView },
    Scene.with(boardModel(gameFold(), false)),
    resolveLiveBoardMounts(),
    Scene.expect(Scene.testId("board-reconnecting")).toExist(),
    Scene.expect(Scene.text("Reconnecting…")).toExist(),
    Scene.expect(Scene.testId("board-status")).not.toExist(),
  );
});

test("connected board does not show reconnect banner or status pill", () => {
  Scene.scene(
    { update: (m) => [m, []], view: fullBoardView },
    Scene.with(boardModel(gameFold(), true)),
    resolveLiveBoardMounts(),
    Scene.expect(Scene.testId("board-reconnecting")).not.toExist(),
    Scene.expect(Scene.testId("board-status")).not.toExist(),
  );
});

test("sound toggle is visible for spectators", () => {
  const model: OverlayModel = {
    board: initialBoardModel(),
    fold: gameFold(gameState({ viewer: SPECTATOR_VIEWER })),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    Scene.expect(Scene.testId("board-sound-toggle")).toExist(),
  );
});
