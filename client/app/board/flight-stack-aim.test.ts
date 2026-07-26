import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { CARD_W, ZONE } from "./geometry/layout";
import { STACK_CARD_W, stackFaceScreenOrigin, stackPeekFor, stackPresentation } from "./geometry/stackLayout";
import { spawnFlight, stackFlightScale } from "./motion/flights";
import { BOARD_VIEWPORT, initialBoardModel, syncBoardWithGame } from "./submodel";

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

function gameFold(visible: VisibleState, provenance: Partial<GameFoldState["provenance"]> = {}): GameFoldState {
  return {
    seq: 1,
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
  };
}

function spell(id: number, name: string): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: `${name}-print`,
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Stack,
  };
}

function restingStackFace(model: ReturnType<typeof initialBoardModel>, count: number, row: number) {
  const presentation = stackPresentation({
    count,
    expandedOpen: model.stackExpand,
    viewportW: model.viewport.width,
    viewportH: model.viewport.height,
  });
  return stackFaceScreenOrigin({
    presentation,
    viewportW: model.viewport.width,
    viewportH: model.viewport.height,
    count,
    row,
    peek: presentation === "pile" ? stackPeekFor(count, model.viewport.height) : undefined,
  });
}

describe("stack flight settle handoff", () => {
  it("sizes stack flights to the resting HTML stack face width", () => {
    const zoom = 1;
    expect(stackFlightScale(zoom)).toBe(STACK_CARD_W / (CARD_W * zoom));
    expect(STACK_CARD_W).toBe(180);
  });

  it("retargets stack entrance flights to the resting stack face center", () => {
    const spellId = 42;
    const fromHand = 7;
    const bolt = spell(spellId, "Lightning Bolt");
    const board = {
      ...initialBoardModel(),
      viewport: { ...BOARD_VIEWPORT },
      flights: new Map([
        [
          spellId,
          spawnFlight({
            id: spellId,
            print: bolt.print ?? "",
            name: bolt.name,
            x: 400,
            y: 700,
            scale: 2,
            targetX: BOARD_VIEWPORT.width - 160,
            targetY: BOARD_VIEWPORT.height / 2,
            targetScale: 112 / CARD_W,
            kind: "stack",
            fromCardId: fromHand,
          }),
        ],
      ]),
      handHidden: new Set([fromHand]),
      hideCardIds: new Set([spellId]),
      ownedIds: new Set([spellId]),
    };

    const after = syncBoardWithGame(
      board,
      gameFold(
        state({
          objects: [bolt],
          stack: [{ controller: 0, kind: "spell", label: testMessageRef("Lightning Bolt"), source: spellId }],
        }),
        {
          stackEntrances: new Map([[spellId, { from: fromHand, controller: 0 }]]),
        },
      ),
    );

    const flight = after.flights.get(spellId);
    expect(flight).toBeDefined();
    const face = restingStackFace(after, 1, 0);
    expect(flight?.targetX).toBe(face.x);
    expect(flight?.targetY).toBe(face.y);
    expect(flight?.targetScale).toBe(stackFlightScale(after.camera.zoom));
  });

  it("aims a multi-card pile flight at that spell's resting face, not viewport mid-right", () => {
    const bottomId = 10;
    const topId = 11;
    const bottom = spell(bottomId, "Counterspell");
    const top = spell(topId, "Lightning Bolt");
    const board = {
      ...initialBoardModel(),
      viewport: { ...BOARD_VIEWPORT },
      flights: new Map([
        [
          topId,
          spawnFlight({
            id: topId,
            print: top.print ?? "",
            name: top.name,
            x: 400,
            y: 700,
            scale: 2,
            targetX: 0,
            targetY: 0,
            targetScale: 1,
            kind: "stack",
            fromCardId: 8,
          }),
        ],
      ]),
      handHidden: new Set([8]),
      hideCardIds: new Set([topId]),
      ownedIds: new Set([topId]),
    };

    const after = syncBoardWithGame(
      board,
      gameFold(
        state({
          objects: [bottom, top],
          stack: [
            { controller: 0, kind: "spell", label: testMessageRef("Counterspell"), source: bottomId },
            { controller: 0, kind: "spell", label: testMessageRef("Lightning Bolt"), source: topId },
          ],
        }),
        {
          stackEntrances: new Map([[topId, { from: 8, controller: 0 }]]),
        },
      ),
    );

    const flight = after.flights.get(topId);
    expect(flight).toBeDefined();
    const face = restingStackFace(after, 2, 1);
    expect(flight?.targetX).toBe(face.x);
    expect(flight?.targetY).toBe(face.y);
    // Stale hardcoded mid-right aim must not remain the settle target.
    expect(flight?.targetX).not.toBe(BOARD_VIEWPORT.width - 160);
    expect(flight?.targetY).not.toBe(BOARD_VIEWPORT.height / 2);
  });
});
