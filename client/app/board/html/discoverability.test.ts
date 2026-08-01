/**
 * @vitest-environment happy-dom
 */
import { Submodel } from "foldkit";
import { Scene } from "foldkit/test";
import { expect, test } from "vitest";
import { testHtml } from "~/test-html";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../../game/fold";
import { HintDismissed, LegendToggled, type Message } from "../messages";
import { type BoardModel, initialBoardModel, updateBoard } from "../submodel";
import { discoverabilityView, HINT_DISMISSED_KEY } from "./discoverability";
import { boardOverlays } from "./overlays";
import { resolveBoardOverlayMounts } from "./scene-helpers";

// Node 26 defines a `localStorage` global that reads `undefined` without `--localstorage-file`,
// and that shadows the one happy-dom installs. Give the hint's persistence a real store either way.
if (typeof localStorage === "undefined") {
  const store = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => void store.set(k, v),
      removeItem: (k: string) => void store.delete(k),
    },
  });
}

const h = testHtml<Message>();

type ViewModel = { board: BoardModel; fold: GameFoldState; tableId: string };

const overlayView = Submodel.defineView<ViewModel, Message>((model) => {
  if (model.fold.state == null) return h.div([], []);
  return boardOverlays(model.board, model.fold.state, model.tableId, model.fold.log, h);
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

function gameState(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: null,
    players: [player(), { ...player(), player: 1, username: "Bob" }],
    priority: 0,
    stack: [],
    step: 0,
    viewer: 0,
  };
}

function gameFold(): GameFoldState {
  return {
    seq: 1,
    state: gameState(),
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

const discoverView = Submodel.defineView<{ board: BoardModel }, never>((m) =>
  discoverabilityView(m.board, gameState(), h),
);

test("discoverability shows hint strip for seated players", () => {
  localStorage.removeItem(HINT_DISMISSED_KEY);
  const board = initialBoardModel();
  Scene.scene(
    { update: (m) => [m, []], view: overlayView },
    Scene.given({ board, fold: gameFold(), tableId: "t1" }),
    resolveBoardOverlayMounts(),
    Scene.expect(Scene.selector('[data-testid="board-hint"]')).toExist(),
    Scene.expect(Scene.selector('[data-testid="board-legend-toggle"]')).toExist(),
  );
});

test("dismissing hint hides strip and persists to localStorage", () => {
  localStorage.removeItem(HINT_DISMISSED_KEY);
  let board = initialBoardModel();
  [board] = updateBoard(board, HintDismissed(), gameFold(), "t1");
  expect(board.hintDismissed).toBe(true);
  expect(localStorage.getItem(HINT_DISMISSED_KEY)).toBe("1");

  Scene.scene(
    { update: (m) => [m, []], view: discoverView },
    Scene.given({ board }),
    Scene.expect(Scene.selector('[data-testid="board-hint"]')).not.toExist(),
    Scene.expect(Scene.selector('[data-testid="board-legend-toggle"]')).toExist(),
  );
});

test("legend toggle opens and closes the legend panel", () => {
  let board = { ...initialBoardModel(), hintDismissed: true };
  [board] = updateBoard(board, LegendToggled(), gameFold(), "t1");
  expect(board.legendOpen).toBe(true);

  Scene.scene(
    { update: (m) => [m, []], view: discoverView },
    Scene.given({ board }),
    Scene.expect(Scene.selector('[data-testid="board-legend"]')).toExist(),
    Scene.expect(Scene.text("Board legend")).toExist(),
  );

  [board] = updateBoard(board, LegendToggled(), gameFold(), "t1");
  expect(board.legendOpen).toBe(false);
});
