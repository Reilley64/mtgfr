import { Story } from "foldkit";
import { expect, test } from "vitest";
import type { VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import type { Message } from "./messages";
import { BoardPointerDown, BoardPointerMove, FlightsSynced } from "./messages";
import { spawnFlight } from "./motion/flights";
import { type BoardModel, initialBoardModel, syncBoardWithGame, updateBoard } from "./submodel";

function state(): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [],
    pending_choice: null,
    players: [
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 0,
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function gameFold(): GameFoldState {
  return {
    seq: 1,
    state: state(),
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

test("pointer down on empty felt enters pan phase", () => {
  const fold = gameFold();

  Story.story(
    (model: BoardModel, message: Message) => updateBoard(model, message, fold, null),
    Story.with(initialBoardModel()),
    Story.message(BoardPointerDown({ x: 12, y: 18 })),
    Story.model((model) => {
      expect(model.pointer).toEqual({ kind: "pan", x: 12, y: 18 });
    }),
  );
});

test("FlightsSynced stores still-flying poses and hides the source card", () => {
  const fold = gameFold();
  const flight = {
    ...spawnFlight({
      id: 1,
      kind: "battlefield",
      name: "Grizzly Bears",
      print: "print-id",
      scale: 0.8,
      targetScale: 1,
      targetX: 100,
      targetY: 0,
      x: 40,
      y: 12,
      fromCardId: 9,
    }),
    phase: "flying" as const,
  };

  Story.story(
    (board: BoardModel, message: Message) => updateBoard(board, message, fold, null),
    Story.with(initialBoardModel()),
    Story.message(FlightsSynced({ flights: [flight], now: 200 })),
    Story.model((board) => {
      expect(board.flights.get(1)).toEqual(flight);
      expect(board.handHidden.has(9)).toBe(true);
      expect(board.hideCardIds).toEqual(new Set([1]));
      expect(board.ownedIds).toEqual(new Set([1]));
      expect(board.lastFlightFrame).toBe(200);
    }),
  );
});

test("FlightsSynced clears hidden cards when flights disappear", () => {
  const fold = gameFold();
  const model: BoardModel = {
    ...initialBoardModel(),
    flights: new Map([
      [
        1,
        spawnFlight({
          id: 1,
          kind: "battlefield",
          name: "Grizzly Bears",
          print: "print-id",
          scale: 1,
          targetScale: 1,
          targetX: 100,
          targetY: 0,
          x: 0,
          y: 0,
          fromCardId: 9,
        }),
      ],
    ]),
    handHidden: new Set([9]),
    hideCardIds: new Set([1]),
    ownedIds: new Set([1]),
    lastFlightFrame: 100,
  };

  Story.story(
    (board: BoardModel, message: Message) => updateBoard(board, message, fold, null),
    Story.with(model),
    Story.message(FlightsSynced({ flights: [], now: 200 })),
    Story.model((board) => {
      expect(board.flights.size).toBe(0);
      expect(board.handHidden.size).toBe(0);
      expect(board.hideCardIds.size).toBe(0);
      expect(board.ownedIds.size).toBe(0);
      expect(board.lastFlightFrame).toBeNull();
    }),
  );
});

test("FlightsSynced keeps flyers and drops settled entries in one payload", () => {
  const fold = gameFold();
  const flyer = {
    ...spawnFlight({
      id: 1,
      kind: "battlefield",
      name: "Grizzly Bears",
      print: "print-a",
      scale: 0.9,
      targetScale: 1,
      targetX: 120,
      targetY: 40,
      x: 50,
      y: 20,
      fromCardId: 9,
    }),
    phase: "flying" as const,
  };
  const settled = {
    ...spawnFlight({
      id: 2,
      kind: "battlefield",
      name: "Shock",
      print: "print-b",
      scale: 1,
      targetScale: 1,
      targetX: 200,
      targetY: 80,
      x: 200,
      y: 80,
      fromCardId: 11,
    }),
    phase: "settled" as const,
  };
  const model: BoardModel = {
    ...initialBoardModel(),
    flights: new Map([
      [1, flyer],
      [2, { ...settled, phase: "flying" }],
    ]),
    handHidden: new Set([9, 11]),
    hideCardIds: new Set([1, 2]),
    ownedIds: new Set([1, 2]),
    lastFlightFrame: 50,
  };

  Story.story(
    (board: BoardModel, message: Message) => updateBoard(board, message, fold, null),
    Story.with(model),
    Story.message(FlightsSynced({ flights: [flyer, settled], now: 90 })),
    Story.model((board) => {
      expect(board.flights.get(1)).toEqual(flyer);
      expect(board.flights.has(2)).toBe(false);
      expect(board.handHidden).toEqual(new Set([9]));
      expect(board.hideCardIds).toEqual(new Set([1]));
      expect(board.ownedIds).toEqual(new Set([1]));
      expect(board.lastFlightFrame).toBe(90);
    }),
  );
});

test("syncBoardWithGame keeps a user-panned camera across game syncs", () => {
  const fold = gameFold();
  const fitted = syncBoardWithGame(initialBoardModel(), fold);

  const [panned] = updateBoard(fitted, BoardPointerDown({ x: 100, y: 100 }), fold, null);
  const [moved] = updateBoard(panned, BoardPointerMove({ x: 160, y: 140 }), fold, null);

  expect(moved.camera).not.toEqual(fitted.camera);
  expect(moved.camera).toEqual({
    panX: fitted.camera.panX + 60,
    panY: fitted.camera.panY + 40,
    zoom: fitted.camera.zoom,
  });

  // A later delta / action must not re-fit and wipe the pan.
  const afterAction = syncBoardWithGame(moved, { ...fold, seq: fold.seq + 1 });
  expect(afterAction.camera).toEqual(moved.camera);
});

test("syncBoardWithGame clears staged attackers/blocks when the step advances", () => {
  const initialFold = gameFold();
  const board: BoardModel = {
    ...initialBoardModel(),
    combatAttackers: [{ attacker: 42, defender: 1 }],
    combatBlocks: [{ blocker: 7, attacker: 42 }],
    attackersConfirmed: true,
    blockersConfirmed: true,
    priorStep: initialFold.state?.step ?? null,
  };

  // Same step → staging preserved.
  const same = syncBoardWithGame(board, initialFold);
  expect(same.combatAttackers).toHaveLength(1);
  expect(same.combatBlocks).toHaveLength(1);

  // Step advances → staging cleared.
  const nextFold: GameFoldState = {
    ...initialFold,
    state: initialFold.state == null ? null : { ...initialFold.state, step: (initialFold.state.step ?? 0) + 1 },
  };
  const advanced = syncBoardWithGame(board, nextFold);
  expect(advanced.combatAttackers).toEqual([]);
  expect(advanced.combatBlocks).toEqual([]);
  expect(advanced.attackersConfirmed).toBe(false);
  expect(advanced.blockersConfirmed).toBe(false);
});
