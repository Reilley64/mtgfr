/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { html } from "foldkit/html";
import { Scene } from "foldkit/test";
import { beforeAll, test } from "vitest";
import { SPECTATOR_VIEWER } from "~/spectator";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import type { Message } from "../messages";
import { type BoardModel, initialBoardModel } from "../submodel";
import { type BoardViewModel, view as boardView } from "../view";
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

test("mulliganing seat sees mulligan bar and hides priority chrome", () => {
  const state = gameState({
    mulliganing: true,
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
    board: initialBoardModel(),
    fold: gameFold(state),
    tableId: "T1",
  };
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.with(model),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.testId("hand-bar")).toExist(),
    Scene.expect(Scene.testId("mulligan-bar")).toExist(),
    Scene.expect(Scene.testId("mulligan-keep")).toExist(),
    Scene.expect(Scene.testId("board-primary")).not.toExist(),
    Scene.expect(Scene.testId("board-concede")).toExist(),
  );
});

test("mulligan bar waiting status names undecided seats after local keep", () => {
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
    Scene.expect(Scene.testId("mulligan-bar")).toContainText("Waiting for Bob to choose."),
    Scene.expect(Scene.testId("mulligan-keep")).not.toExist(),
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
