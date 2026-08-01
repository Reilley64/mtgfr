import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { applyPublishedFrame, type BitmapFrame, bitmapFrameNeedsRaf, tickFlightClock } from "./bitmap/mount";
import { layout, ZONE } from "./geometry/layout";
import { FlightsSynced, HandActionActivated } from "./messages";
import { BOARD_VIEWPORT, initialBoardModel, syncBoardWithGame, updateBoard } from "./submodel";

type BoardModel = ReturnType<typeof initialBoardModel>;

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
  seq: number,
  visible: VisibleState,
  provenance: Partial<GameFoldState["provenance"]> = {},
): GameFoldState {
  return {
    seq,
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

function forest(id: number, zone: number): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    is_token: false,
    legendary: false,
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
    zone,
  };
}

/**
 * The board view's `publishBitmapFrame` payload, trimmed to the fields the flight clock reads.
 * Everything else only feeds resting paint, which this sweep does not assert on.
 */
function frameOf(model: BoardModel, fold: GameFoldState): BitmapFrame {
  const visible = fold.state as VisibleState;
  return {
    width: model.viewport.width,
    height: model.viewport.height,
    dpr: model.dpr,
    camera: model.camera,
    cards: layout(visible, visible.viewer),
    viewer: visible.viewer,
    players: visible.players,
    priority: visible.priority,
    combat: visible.combat,
    stagedAttackers: [],
    stagedBlocks: [],
    stack: visible.stack,
    flights: [...model.flights.values()],
    exitFx: [...model.exitFx.values()],
    hideCardIds: model.hideCardIds,
    targetObjects: new Set(),
    pickedObjects: new Set(),
    assignAmounts: new Map(),
    targetPlayers: new Set(),
    pickedPlayers: new Set(),
    aimFrom: null,
    cursor: { x: 0, y: 0 },
    combatDragFrom: null,
    combatDragStroke: null,
    paymentPreviewIds: new Set(),
    actions: visible.actions,
  };
}

type Sim = {
  model: BoardModel;
  clock: Parameters<typeof applyPublishedFrame>[0];
  frame: BitmapFrame | null;
  now: number;
};

function newSim(model: BoardModel): Sim {
  return {
    model,
    clock: { liveFlights: [], liveExitFx: [], liveDragGhost: null, lastRestingSnapshot: null },
    frame: null,
    now: 0,
  };
}

/** One render pass: the view publishes the model's flights and the Mount merges live poses. */
function publish(sim: Sim, fold: GameFoldState): Sim {
  const published = applyPublishedFrame(sim.clock, frameOf(sim.model, fold));
  const model =
    published.sync == null ? sim.model : updateBoard(sim.model, FlightsSynced(published.sync), fold, "T1")[0];
  return { ...sim, model, clock: published.state, frame: published.frame };
}

/** One rAF tick, plus the render pass that follows the message it may dispatch. */
function tick(sim: Sim, fold: GameFoldState): Sim {
  if (sim.frame == null) return publish(sim, fold);
  const now = sim.now + 16;
  const ticked = tickFlightClock(sim.clock, sim.frame, now, 16, false);
  const model = ticked.sync == null ? sim.model : updateBoard(sim.model, FlightsSynced(ticked.sync), fold, "T1")[0];
  return publish({ model, clock: ticked.state, frame: ticked.frame, now }, fold);
}

/** Run the clock until `kickRaf` would stop scheduling frames — the board is then visually frozen. */
function runToQuiescence(sim: Sim, fold: GameFoldState): Sim {
  let current = publish(sim, fold);
  for (let frames = 0; frames < 400; frames += 1) {
    if (!bitmapFrameNeedsRaf(current.frame)) return current;
    current = tick(current, fold);
  }
  throw new Error("flight clock never quiesced");
}

function runFrames(sim: Sim, fold: GameFoldState, frames: number): Sim {
  let current = publish(sim, fold);
  for (let frame = 0; frame < frames; frame += 1) {
    if (!bitmapFrameNeedsRaf(current.frame)) return current;
    current = tick(current, fold);
  }
  return current;
}

/** Authority folding the way `mergeGameFold` does: fold in, then sync the board against it. */
function deliver(sim: Sim, fold: GameFoldState): Sim {
  return { ...sim, model: syncBoardWithGame(sim.model, fold) };
}

function bolt(id: number, zone: number): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "instant" },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Lightning Bolt",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    print: "bolt-print",
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone,
  };
}

const HAND_ID = 9;
const PERMANENT_ID = 90;

const playLand = {
  id: 3,
  kind: "play_land" as const,
  label: testMessageRef("Play Forest"),
  needs_target: false,
  object: HAND_ID,
  section: "hand" as const,
};

const inHand = gameFold(1, state({ objects: [forest(HAND_ID, ZONE.Hand)], actions: [playLand] }));

const castBolt = {
  id: 4,
  kind: "cast" as const,
  label: testMessageRef("Cast Lightning Bolt"),
  needs_target: false,
  object: HAND_ID,
  section: "hand" as const,
};

const boltInHand = gameFold(1, state({ objects: [bolt(HAND_ID, ZONE.Hand)], actions: [castBolt] }));

const SPELL_ID = 42;
const stackEntry = {
  controller: 0,
  kind: "spell" as const,
  label: testMessageRef("Lightning Bolt"),
  source: SPELL_ID,
};

const onStack = gameFold(2, state({ objects: [bolt(SPELL_ID, ZONE.Stack)], stack: [stackEntry], actions: [] }), {
  stackEntrances: new Map([[SPELL_ID, { controller: 0, from: HAND_ID }]]),
});

const onStackNoProvenance = gameFold(
  2,
  state({ objects: [bolt(SPELL_ID, ZONE.Stack)], stack: [stackEntry], actions: [] }),
);

function afterCastingTheBolt(dropX: number, dropY: number): BoardModel {
  const [model] = updateBoard(
    { ...initialBoardModel(), viewport: { ...BOARD_VIEWPORT } },
    HandActionActivated({ action: castBolt, x: dropX, y: dropY }),
    boltInHand,
    "T1",
  );
  return model;
}

/** The delta that resolves the play: the land is a battlefield permanent under a fresh id. */
const onBattlefield = gameFold(2, state({ objects: [forest(PERMANENT_ID, ZONE.Battlefield)], actions: [] }), {
  landPlayFrom: new Map([[PERMANENT_ID, HAND_ID]]),
});

/** Same board reached by a snapshot (reconnect / resync): `applySnapshotPure` clears provenance. */
const onBattlefieldNoProvenance = gameFold(
  2,
  state({ objects: [forest(PERMANENT_ID, ZONE.Battlefield)], actions: [] }),
);

function afterPlayingTheLand(dropX: number, dropY: number): BoardModel {
  const [model] = updateBoard(
    { ...initialBoardModel(), viewport: { ...BOARD_VIEWPORT } },
    HandActionActivated({ action: playLand, x: dropX, y: dropY }),
    inHand,
    "T1",
  );
  return model;
}

// The drop point decides whether the seed parks near the real slot (straight handoff) or far from
// it (retarget with hold retained) — both have to end with the card released.
const DROP_POINTS = [
  { label: "over the lands row", x: 640, y: 620 },
  { label: "mid board", x: 500, y: 400 },
  { label: "far corner", x: 120, y: 120 },
];

describe("a played card is never left stuck at the end of its flight", () => {
  for (const drop of DROP_POINTS) {
    // Authority can land on any frame of the seed's glide; none of them may strand it.
    for (const framesBeforeAuthority of [0, 1, 2, 4, 8, 20]) {
      it(`releases a land dropped ${drop.label} when authority arrives after ${framesBeforeAuthority} frames`, () => {
        const seeded = runFrames(newSim(afterPlayingTheLand(drop.x, drop.y)), inHand, framesBeforeAuthority);
        const settled = runToQuiescence(deliver(seeded, onBattlefield), onBattlefield);

        expect([...settled.model.flights.keys()]).toEqual([]);
        expect([...settled.model.handHidden]).toEqual([]);
        expect([...settled.model.hideCardIds]).toEqual([]);
      });

      it(`releases a land dropped ${drop.label} when a snapshot lands after ${framesBeforeAuthority} frames`, () => {
        const seeded = runFrames(newSim(afterPlayingTheLand(drop.x, drop.y)), inHand, framesBeforeAuthority);
        const settled = runToQuiescence(deliver(seeded, onBattlefieldNoProvenance), onBattlefieldNoProvenance);

        expect([...settled.model.flights.keys()]).toEqual([]);
        expect([...settled.model.handHidden]).toEqual([]);
        expect([...settled.model.hideCardIds]).toEqual([]);
      });

      it(`releases a spell cast ${drop.label} when authority arrives after ${framesBeforeAuthority} frames`, () => {
        const seeded = runFrames(newSim(afterCastingTheBolt(drop.x, drop.y)), boltInHand, framesBeforeAuthority);
        const settled = runToQuiescence(deliver(seeded, onStack), onStack);

        expect([...settled.model.flights.keys()]).toEqual([]);
        expect([...settled.model.handHidden]).toEqual([]);
        expect([...settled.model.hideCardIds]).toEqual([]);
      });

      it(`releases a spell cast ${drop.label} when a snapshot lands after ${framesBeforeAuthority} frames`, () => {
        const seeded = runFrames(newSim(afterCastingTheBolt(drop.x, drop.y)), boltInHand, framesBeforeAuthority);
        const settled = runToQuiescence(deliver(seeded, onStackNoProvenance), onStackNoProvenance);

        expect([...settled.model.flights.keys()]).toEqual([]);
        expect([...settled.model.handHidden]).toEqual([]);
        expect([...settled.model.hideCardIds]).toEqual([]);
      });
    }
  }
});
