import { Story } from "foldkit";
import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import type { Message } from "./messages";
import {
  BoardCameraZoomed,
  BoardPointerDown,
  BoardPointerMove,
  CombatAttackerDropped,
  CombatBlockerDropped,
  FlightsSynced,
} from "./messages";
import { spawnExitFx } from "./motion/exit-fx";
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

function gameFold(over: Partial<VisibleState> = {}): GameFoldState {
  return {
    seq: 1,
    state: { ...state(), ...over },
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

function creature(id: number, controller: number): ObjectView {
  return {
    controller,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 2 },
    marked_damage: 0,
    name: "Grizzly Bears",
    needs_target: false,
    owner: controller,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: 2, // battlefield
  };
}

/** A combat declaration as the engine projects it: `declare_for` names the seats it covers. */
function declareAction(kind: "declare_attackers" | "declare_blockers", declare_for: number[]): ActionView {
  return { id: 1, kind, label: testMessageRef(kind), needs_target: false, section: "combat", declare_for };
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
    Story.message(FlightsSynced({ flights: [flight], exitFx: [], now: 200 })),
    Story.model((board) => {
      expect(board.flights.get(1)).toEqual(flight);
      expect(board.handHidden.has(9)).toBe(true);
      expect(board.hideCardIds).toEqual(new Set([1]));
      expect(board.ownedIds).toEqual(new Set([1]));
      expect(board.lastFlightFrame).toBe(200);
    }),
  );
});

test("FlightsSynced keeps exit FX ids hidden even without flights", () => {
  const fold = gameFold();
  const fx = spawnExitFx({
    id: 7,
    kind: "destroy",
    name: "Grizzly Bears",
    print: "print-id",
    x: 80,
    y: 60,
    scale: 1,
  });

  Story.story(
    (board: BoardModel, message: Message) => updateBoard(board, message, fold, null),
    Story.with(initialBoardModel()),
    Story.message(FlightsSynced({ flights: [], exitFx: [fx], now: 200 })),
    Story.model((board) => {
      expect(board.exitFx.get(7)).toEqual(fx);
      expect(board.hideCardIds).toEqual(new Set([7]));
      expect(board.ownedIds.size).toBe(0);
      expect(board.lastFlightFrame).toBe(200);
    }),
  );
});

test("FlightsSynced keeps flyers and exit FX ids hidden together", () => {
  const fold = gameFold();
  const flight = {
    ...spawnFlight({
      id: 1,
      kind: "battlefield",
      name: "Grizzly Bears",
      print: "print-flight",
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
  const fx = spawnExitFx({
    id: 7,
    kind: "destroy",
    name: "Exit Bear",
    print: "print-fx",
    x: 80,
    y: 60,
    scale: 1,
  });

  Story.story(
    (board: BoardModel, message: Message) => updateBoard(board, message, fold, null),
    Story.with(initialBoardModel()),
    Story.message(FlightsSynced({ flights: [flight], exitFx: [fx], now: 200 })),
    Story.model((board) => {
      expect(board.flights.get(1)).toEqual(flight);
      expect(board.exitFx.get(7)).toEqual(fx);
      expect(board.hideCardIds).toEqual(new Set([1, 7]));
      expect(board.handHidden).toEqual(new Set([9]));
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
    Story.message(FlightsSynced({ flights: [], exitFx: [], now: 200 })),
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
    Story.message(FlightsSynced({ flights: [flyer, settled], exitFx: [], now: 90 })),
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

test("wheel or pinch zoom marks the camera user-moved and blocks later refit", () => {
  const fold = gameFold();
  const fitted = syncBoardWithGame(initialBoardModel(), fold);

  const [zoomed] = updateBoard(fitted, BoardCameraZoomed({ x: 720, y: 420, factor: 1.25 }), fold, null);

  expect(zoomed.cameraUserMoved).toBe(true);
  expect(zoomed.camera.zoom).toBeCloseTo(fitted.camera.zoom * 1.25);

  const morePlayers = {
    ...fold,
    seq: fold.seq + 1,
    state: {
      ...state(),
      players: [
        ...state().players,
        {
          commander_tax: 0,
          hand_count: 7,
          library_count: 80,
          life: 40,
          lost: false,
          mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
          player: 1,
        },
      ],
    },
  };
  const afterSync = syncBoardWithGame(zoomed, morePlayers);
  expect(afterSync.camera).toEqual(zoomed.camera);
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

// Master Warcraft: the engine hands seat 0 the *active player's* attack declaration, so dropping
// seat 1's creature onto seat 2's avatar has to stage it — the creature is not seat 0's own.
test("a moved attack declaration stages the creatures of the seat it covers", () => {
  const fold = gameFold({
    active_player: 1,
    step: 5, // declare attackers
    objects: [creature(7, 1)],
    actions: [declareAction("declare_attackers", [1])],
  });

  Story.story(
    (model: BoardModel, message: Message) => updateBoard(model, message, fold, null),
    Story.with(initialBoardModel()),
    Story.message(CombatAttackerDropped({ attackerId: 7, defenderSeat: 2 })),
    Story.model((model) => {
      expect(model.combatAttackers).toEqual([{ attacker: 7, defender: 2 }]);
    }),
  );
});

// The block half is where the covered seats bite: seat 0 is not being attacked at all, so without
// the engine's `declare_for` the drop is rejected as "nobody is attacking you".
test("a moved block declaration stages blocks for the attacked seat, not the declarer", () => {
  const attacked = gameFold({
    active_player: 2,
    step: 6, // declare blockers
    objects: [creature(7, 2), creature(8, 1)],
    combat: {
      attackers: [{ attacker: 7, defender: 1 }],
      blocks: [],
      attackers_declared: true,
      blockers_declared: [],
    },
    actions: [declareAction("declare_blockers", [1])],
  });

  Story.story(
    (model: BoardModel, message: Message) => updateBoard(model, message, attacked, null),
    Story.with(initialBoardModel()),
    Story.message(CombatBlockerDropped({ attackerId: 7, blockerId: 8 })),
    Story.model((model) => {
      expect(model.combatBlocks).toEqual([{ blocker: 8, attacker: 7 }]);
    }),
  );
});
