import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { CARD_W, ZONE } from "./geometry/layout";
import { STACK_CARD_W, stackFaceScreenOrigin, stackPeekFor, stackPresentation } from "./geometry/stackLayout";
import { FlightsSynced, HandActionActivated } from "./messages";
import { handFlightScale, spawnFlight, stackFlightScale, stepFlights } from "./motion/flights";
import { BOARD_VIEWPORT, initialBoardModel, syncBoardWithGame, updateBoard } from "./submodel";

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

  it("does not restart a local seed flight from the avatar after it reaches the stack", () => {
    const handId = 7;
    const spellId = 42;
    const bolt = spell(spellId, "Lightning Bolt");
    const board0 = { ...initialBoardModel(), viewport: { ...BOARD_VIEWPORT } };
    const face = restingStackFace(board0, 1, 0);
    const seeded = spawnFlight({
      id: handId,
      print: bolt.print ?? "",
      name: bolt.name,
      x: 400,
      y: 700,
      scale: 2,
      targetX: face.x,
      targetY: face.y,
      targetScale: stackFlightScale(board0.camera.zoom),
      kind: "stack",
      fromCardId: handId,
      hold: true,
    });

    // Mount clock reaches the stack face before the game sync arrives.
    let live = new Map([[handId, seeded]]);
    for (let i = 0; i < 40; i += 1) {
      live = stepFlights(live, 16, false).flights;
    }
    const parked = live.get(handId);
    if (parked == null) throw new Error("expected seed flight to park at the stack");
    expect(parked.x).toBeCloseTo(face.x, 0);
    expect(parked.y).toBeCloseTo(face.y, 0);
    expect(parked.phase).toBe("settled");
    expect(parked.hold).toBe(true);

    const fold = gameFold(state());
    const [afterSyncMsg] = updateBoard(
      {
        ...board0,
        flights: new Map([[handId, parked]]),
        handHidden: new Set([handId]),
        hideCardIds: new Set([handId]),
        ownedIds: new Set([handId]),
      },
      FlightsSynced({ flights: [parked], exitFx: [], now: 500 }),
      fold,
      null,
    );

    // Held seed must survive settle sync so stackEntrances can rebind it.
    expect(afterSyncMsg.flights.has(handId)).toBe(true);

    const afterGame = syncBoardWithGame(
      afterSyncMsg,
      gameFold(
        state({
          objects: [bolt],
          stack: [{ controller: 0, kind: "spell", label: testMessageRef("Lightning Bolt"), source: spellId }],
        }),
        {
          stackEntrances: new Map([[spellId, { from: handId, controller: 0 }]]),
        },
      ),
    );

    // Already on the resting face — hand off immediately (no second flying settle pulse).
    expect(afterGame.flights.size).toBe(0);
    expect(afterGame.hideCardIds.size).toBe(0);
    expect(afterGame.handHidden.has(handId)).toBe(false);
  });

  it("keeps parked stack seed screen size when sync fits the camera", () => {
    const handId = 7;
    const spellId = 42;
    const bolt = spell(spellId, "Lightning Bolt");
    const board0 = { ...initialBoardModel(), viewport: { ...BOARD_VIEWPORT }, cameraFitPlayers: 0 };
    const face = restingStackFace(board0, 1, 0);
    const targetScale = stackFlightScale(board0.camera.zoom);
    const parked = {
      ...spawnFlight({
        id: handId,
        print: bolt.print ?? "",
        name: bolt.name,
        x: face.x,
        y: face.y,
        scale: targetScale,
        targetX: face.x,
        targetY: face.y,
        targetScale,
        kind: "stack",
        fromCardId: handId,
        hold: true,
      }),
      phase: "settled" as const,
    };

    const afterGame = syncBoardWithGame(
      {
        ...board0,
        flights: new Map([[handId, parked]]),
        handHidden: new Set([handId]),
        hideCardIds: new Set([handId]),
        ownedIds: new Set([handId]),
      },
      gameFold(
        state({
          objects: [bolt],
          stack: [{ controller: 0, kind: "spell", label: testMessageRef("Lightning Bolt"), source: spellId }],
        }),
        {
          stackEntrances: new Map([[spellId, { from: handId, controller: 0 }]]),
        },
      ),
    );

    // Camera fit must not leave a second scale glide; hand off once size is preserved.
    expect(afterGame.camera.zoom).not.toBe(board0.camera.zoom);
    expect(afterGame.flights.size).toBe(0);
  });

  it("parks a battlefield seed at drop scale so land sync is the only glide", () => {
    const handId = 9;
    const land: ObjectView = {
      ...spell(handId, "Forest"),
      kind: { kind: "land", colors: [] },
      zone: ZONE.Hand,
      print: "forest-print",
    };
    const action = {
      id: 3,
      kind: "play_land" as const,
      label: testMessageRef("Play Forest"),
      needs_target: false,
      object: handId,
      section: "hand" as const,
    };
    const fold = gameFold(state({ objects: [land], actions: [action] }));
    const [afterPlay] = updateBoard(
      { ...initialBoardModel(), viewport: { ...BOARD_VIEWPORT } },
      HandActionActivated({ action, x: 500, y: 400 }),
      fold,
      "T1",
    );
    const flight = afterPlay.flights.get(handId);
    expect(flight).toBeDefined();
    expect(flight?.hold).toBe(true);
    expect(flight?.kind).toBe("battlefield");
    expect(flight?.scale).toBe(handFlightScale(afterPlay.camera.zoom));
    expect(flight?.targetScale).toBe(flight?.scale);
    expect(flight?.targetX).toBe(500);
    expect(flight?.targetY).toBe(400);
  });
});
