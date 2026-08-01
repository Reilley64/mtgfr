import { describe, expect, it } from "vitest";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { ZONE } from "./geometry/layout";
import { spawnFlight } from "./motion/flights";
import { type BoardModel, initialBoardModel, syncBoardWithGame } from "./submodel";

function player(overrides: Partial<PlayerView> = {}): PlayerView {
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
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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

function gameFold(
  visible: VisibleState,
  overrides: Partial<GameFoldState> = {},
  provenance: Partial<GameFoldState["provenance"]> = {},
): GameFoldState {
  return {
    seq: overrides.seq ?? 1,
    state: visible,
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
      ...provenance,
    },
    tableFeel: { land: false, stack: false, resolve: false, damage: false, destroy: false, exile: false },
    ...overrides,
  };
}

function creature(id: number, controller: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller,
    has_haste: false,
    id,
    is_commander: false,
    is_token: false,
    legendary: false,
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

describe("syncBoardWithGame exit FX", () => {
  it("BF to graveyard spawns destroy ExitFx, suppresses glide, and hides the card id", () => {
    const bearId = 22;
    const battlefieldBear = creature(bearId, 0, {
      name: "Grizzly Bears",
      print: "print-bear",
      zone: ZONE.Battlefield,
    });
    const graveyardBear = creature(bearId, 0, {
      name: "Grizzly Bears",
      print: "print-bear",
      zone: ZONE.Graveyard,
    });

    const seeded = syncBoardWithGame(initialBoardModel(), gameFold(state({ objects: [battlefieldBear] })));
    const after = syncBoardWithGame(
      seeded,
      gameFold(
        state({ objects: [graveyardBear] }),
        { seq: 2 },
        {
          zoneMoves: new Map([[bearId, bearId]]),
          battlefieldExits: new Map([[bearId, "graveyard"]]),
        },
      ),
    );

    expect(after.exitFx.get(bearId)?.kind).toBe("destroy");
    expect(after.flights.has(bearId)).toBe(false);
    expect(after.hideCardIds.has(bearId)).toBe(true);
  });

  it("BF to exile spawns exile ExitFx from the current flight pose and clears that flight", () => {
    const bearId = 33;
    const board: BoardModel = {
      ...initialBoardModel(),
      // Match fold player count so auto fitCamera does not remap flight scale mid-assertion.
      cameraFitPlayers: 2,
      flights: new Map([
        [
          bearId,
          {
            ...spawnFlight({
              id: bearId,
              kind: "battlefield",
              name: "Banished Bear",
              print: "print-exile",
              x: 144,
              y: 188,
              scale: 0.8,
              targetX: 300,
              targetY: 300,
              targetScale: 1,
            }),
            phase: "flying",
          },
        ],
      ]),
      hideCardIds: new Set([bearId]),
      ownedIds: new Set([bearId]),
    };
    const exiledBear = creature(bearId, 0, {
      name: "Banished Bear",
      print: "print-exile",
      zone: ZONE.Exile,
    });

    const after = syncBoardWithGame(
      board,
      gameFold(
        state({ objects: [exiledBear] }),
        { seq: 2 },
        {
          zoneMoves: new Map([[bearId, bearId]]),
          battlefieldExits: new Map([[bearId, "exile"]]),
        },
      ),
    );

    expect(after.exitFx.get(bearId)).toMatchObject({
      kind: "exile",
      x: 144,
      y: 188,
      scale: 0.8,
      print: "print-exile",
      name: "Banished Bear",
    });
    expect(after.flights.has(bearId)).toBe(false);
    expect(after.hideCardIds.has(bearId)).toBe(true);
  });

  it("BF exit rebinds to the prior battlefield id pose when provenance changes ids", () => {
    const priorBattlefieldId = 19;
    const exitId = 20;
    const board: BoardModel = {
      ...initialBoardModel(),
      lastBattlefieldPoses: new Map([
        [
          priorBattlefieldId,
          {
            x: 144,
            y: 188,
            scale: 0.8,
            print: "print-bear",
            name: "Grizzly Bears",
          },
        ],
      ]),
    };
    const graveyardBear = creature(exitId, 0, {
      id: exitId,
      name: "Grizzly Bears",
      print: "print-bear",
      zone: ZONE.Graveyard,
    });

    const after = syncBoardWithGame(
      board,
      gameFold(
        state({ objects: [graveyardBear] }),
        { seq: 2 },
        {
          zoneMoves: new Map([[exitId, priorBattlefieldId]]),
          battlefieldExits: new Map([[exitId, "graveyard"]]),
        },
      ),
    );

    expect(after.exitFx.get(exitId)).toMatchObject({
      kind: "destroy",
      x: 144,
      y: 188,
      scale: 0.8,
      print: "print-bear",
      name: "Grizzly Bears",
    });
    expect(after.flights.has(exitId)).toBe(false);
    expect(after.hideCardIds.has(exitId)).toBe(true);
  });

  it("non-battlefield graveyard moves still glide when battlefieldExits is empty", () => {
    const fromId = 90;
    const toId = 91;
    const handCard = creature(fromId, 0, {
      id: fromId,
      name: "Discarded Card",
      print: "print-discard",
      zone: ZONE.Hand,
    });
    const graveyardCard = creature(toId, 0, {
      id: toId,
      name: "Discarded Card",
      print: "print-discard",
      zone: ZONE.Graveyard,
    });

    const after = syncBoardWithGame(
      initialBoardModel(),
      gameFold(
        state({ objects: [handCard, graveyardCard] }),
        { seq: 2 },
        {
          zoneMoves: new Map([[toId, fromId]]),
        },
      ),
    );

    expect(after.exitFx.size).toBe(0);
    expect(after.flights.get(toId)?.kind).toBe("battlefield");
    expect(after.hideCardIds.has(toId)).toBe(true);
  });
});
