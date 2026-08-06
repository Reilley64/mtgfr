import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { engagedIds } from "./engagement";
import { layout, ZONE } from "./geometry/layout";
import { selectedRadialOptions } from "./html/activation-menu";
import {
  BoardPointerDown,
  BoardPointerUp,
  CombatAttackerDropped,
  RadialOptionPicked,
  ShiftDown,
  ShiftUp,
} from "./messages";
import { type BoardModel, initialBoardModel, updateBoard } from "./submodel";

function creature(id: number, over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Bear",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: ZONE.Battlefield,
    ...over,
  };
}

function state(over: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
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
        username: "Alice",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

function fold(over: Partial<VisibleState> = {}): GameFoldState {
  return {
    seq: 1,
    state: state(over),
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

/** 6 unique bears + 4 identical Saprolings in one seat: the Saprolings must cluster. */
function crowdedObjects(): ObjectView[] {
  const bears = Array.from({ length: 6 }, (_, i) =>
    creature(i + 1, { name: `Bear ${i}`, kind: { kind: "creature", power: 2, toughness: 2 } }),
  );
  const saprolings = [10, 11, 12, 13].map((id) =>
    creature(id, {
      name: "Saproling",
      kind: { kind: "creature", power: 1, toughness: 1 },
      power: 1,
      toughness: 1,
      // ponytail: tapping-for-mana is a cheap way to make the click-to-select path always legal
      // (canSelectPermanent's `activate`-action check needs a matching ActionView otherwise), and
      // it must be uniform across all four copies or it becomes part of the cluster key itself.
      taps_for_mana: true,
    }),
  );
  return [...bears, ...saprolings];
}

const declareAttackers: ActionView = {
  id: 1,
  kind: "declare_attackers",
  label: testMessageRef("Declare attackers"),
  needs_target: false,
  section: "combat",
  declare_for: [0],
};

const twoSeats: VisibleState["players"] = [
  {
    commander_tax: 0,
    hand_count: 0,
    library_count: 40,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
  },
  {
    commander_tax: 0,
    hand_count: 0,
    library_count: 40,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 1,
    username: "Bob",
  },
];

// Documents `combatDropModel`'s staging-list merge: two distinct wire ids dropped in sequence
// produce two distinct `combatAttackers` entries, not one overwriting the other. Renamed from
// "two attack drops on one cluster declare two distinct attackers" — it dispatches
// `CombatAttackerDropped` with attacker ids given directly as message payloads, so it never
// touches `cardAt`/`cardsFor`/`layout()`; it does not exercise cluster hit-testing at all (see the
// pointer-driven test below for that).
test("two distinct dropped attacker ids each keep their own combatAttackers entry", () => {
  const gameFold = fold({ objects: crowdedObjects(), actions: [declareAttackers], step: 5, players: twoSeats });
  const board: BoardModel = initialBoardModel();

  const [afterFirst] = updateBoard(board, CombatAttackerDropped({ attackerId: 10, defenderSeat: 1 }), gameFold, "T1");
  const [afterSecond] = updateBoard(
    afterFirst,
    CombatAttackerDropped({ attackerId: 11, defenderSeat: 1 }),
    gameFold,
    "T1",
  );

  expect(afterSecond.combatAttackers.map((a) => a.attacker).sort()).toEqual([10, 11]);
});

// Shift commits the whole pile. The drop message carries one attacker id; `combatDropModel` has to
// resolve that id back to its cluster face through `cardsFor` → `layout()` and stage every member.
test("a shift-held drop sends the whole Saproling cluster at the defender", () => {
  const gameFold = fold({ objects: crowdedObjects(), actions: [declareAttackers], step: 5, players: twoSeats });
  const [held] = updateBoard(initialBoardModel(), ShiftDown(), gameFold, "T1");
  expect(held.shiftDown).toBe(true);

  const [next] = updateBoard(held, CombatAttackerDropped({ attackerId: 10, defenderSeat: 1 }), gameFold, "T1");

  expect(next.combatAttackers.map((a) => a.attacker).sort()).toEqual([10, 11, 12, 13]);
  expect(next.combatAttackers.every((a) => a.defender === 1)).toBe(true);
});

test("releasing shift goes back to one attacker per drop", () => {
  const gameFold = fold({ objects: crowdedObjects(), actions: [declareAttackers], step: 5, players: twoSeats });
  const [held] = updateBoard(initialBoardModel(), ShiftDown(), gameFold, "T1");
  const [released] = updateBoard(held, ShiftUp(), gameFold, "T1");
  expect(released.shiftDown).toBe(false);

  const [next] = updateBoard(released, CombatAttackerDropped({ attackerId: 10, defenderSeat: 1 }), gameFold, "T1");

  expect(next.combatAttackers.map((a) => a.attacker)).toEqual([10]);
});

test("clicking the residual cluster tile after a copy is staged reaches the next free copy", () => {
  const gameFold = fold({ objects: crowdedObjects(), players: twoSeats });
  const visible = gameFold.state as VisibleState;
  // Saproling 10 is already staged as an attacker (local UI staging — no drop message needed here;
  // engagement.ts reads it straight off `BoardModel.combatAttackers`).
  const board: BoardModel = { ...initialBoardModel(), combatAttackers: [{ attacker: 10, defender: 1 }] };

  // The cards this scene *should* render once engagement is wired: id 10 gets pulled out of the
  // Saproling cluster (it's committed), leaving the residual cluster's face as the next free copy.
  const engaged = engagedIds(visible, board);
  const cards = layout(visible, visible.viewer, engaged);
  const residualCluster = cards.find((c) => c.cluster > 1 && c.clusterMembers.includes(11));
  if (residualCluster == null) throw new Error("expected a residual Saproling cluster containing id 11");
  expect(residualCluster.id).toBe(11);

  // The default camera is the identity transform (zoom 1, no pan), so a card's world top-left is
  // also its screen position — click its center.
  const x = residualCluster.x + residualCluster.w / 2;
  const y = residualCluster.y + residualCluster.h / 2;

  const [afterDown] = updateBoard(board, BoardPointerDown({ x, y }), gameFold, "T1");
  const [afterUp] = updateBoard(afterDown, BoardPointerUp({ x, y }), gameFold, "T1");

  // A real click on that screen point must select the next free copy (11), not the one already
  // attacking (10) — the click reaches `cardAt` → `cardsFor` → `layout()`, so this fails if that
  // path ever stops feeding it the engaged set.
  expect(afterUp.selectedId).toBe(11);
});

// A prompt draft toggles `picked` by object id, so clicking the cluster face twice used to
// deselect the first copy instead of choosing a second — the second click must resolve to the
// next distinct Saproling, not the first one's own id.
//
// Drives real pointer messages, twice, the way "clicking the residual cluster tile after a copy
// is staged reaches the next free copy" does — through `cardAt` → `cardsFor` → `layout()` — rather
// than dispatching `TargetChosen` with hardcoded ids (that reducer path calls
// `togglePendingObjectAimPick` directly off the id already carried on the message, so it never
// touches cluster hit-testing and stays green with the fix reverted).
//
// Each click re-reads the board at the point the cluster face is *now*, not the point it was
// before the previous click: after the first pick, the engaged copy gets pulled out of the
// cluster and the row repacks around one more slot, so the residual face's screen position shifts.
// A real player looks at the tile, clicks it, looks again, clicks it again — they track the tile,
// not a fixed pixel — so re-deriving the point per click is the honest simulation of that click,
// not a workaround for the assertion.
test("two target picks on one cluster select two distinct copies instead of toggling", () => {
  const pendingChoice = {
    kind: "choose_target" as const,
    label: testMessageRef("Choose two creatures"),
    items: [10, 11, 12, 13].map((id) => ({ id, label: "Saproling" })),
    min: 2,
    max: 2,
    player: 0,
    source: 50,
  };
  const gameFold = fold({ objects: crowdedObjects(), pending_choice: pendingChoice });
  const visible = gameFold.state as VisibleState;
  const board: BoardModel = initialBoardModel();

  // Before any pick, all four Saprolings are one cluster faced by the lowest id.
  const initialCluster = layout(visible, visible.viewer, engagedIds(visible, board)).find(
    (c) => c.cluster > 1 && c.clusterMembers.includes(10),
  );
  if (initialCluster == null) throw new Error("expected the full Saproling cluster before any pick");
  expect(initialCluster.id).toBe(10);
  const firstClick = { x: initialCluster.x + initialCluster.w / 2, y: initialCluster.y + initialCluster.h / 2 };

  const [afterFirstDown] = updateBoard(board, BoardPointerDown(firstClick), gameFold, "T1");
  const [afterFirst] = updateBoard(afterFirstDown, BoardPointerUp(firstClick), gameFold, "T1");
  expect(afterFirst.promptDraft).toMatchObject({ kind: "card-pick", picked: [10] });

  // Picking 10 engages it, so it splits out of the cluster and the residual face becomes 11 — at
  // whatever screen point the repacked row now puts it.
  const residualCluster = layout(visible, visible.viewer, engagedIds(visible, afterFirst)).find(
    (c) => c.cluster > 1 && c.clusterMembers.includes(11),
  );
  if (residualCluster == null) throw new Error("expected a residual Saproling cluster containing id 11");
  expect(residualCluster.id).toBe(11);
  const secondClick = { x: residualCluster.x + residualCluster.w / 2, y: residualCluster.y + residualCluster.h / 2 };

  const [afterSecondDown] = updateBoard(afterFirst, BoardPointerDown(secondClick), gameFold, "T1");
  const [afterSecond] = updateBoard(afterSecondDown, BoardPointerUp(secondClick), gameFold, "T1");

  expect(afterSecond.promptDraft).toMatchObject({ kind: "card-pick", picked: [10, 11] });
});

// A cluster offers one row per ability, and that row carries the `ActionView` of whichever copy can
// still pay for it — which is not the face once the face has spent its ability. The staged card is
// what the aim leg then treats as the ability's source, so it has to be the acting copy: stage the
// face instead and the arrow points from the wrong permanent and self-referential aim reads the
// wrong id.
//
// `commitRadialIndex` is the only place that resolves it. Asserting on `planRunAction` would not
// discriminate — it falls back to `action.object` only when `card` is null, so a wrongly-resolved
// face card wins there silently.
test("activating a cluster ability stages the copy that can act, not the cluster face", () => {
  // Saproling 10 (the face) already spent this ability, so the engine lists it only for copy 11.
  const pump: ActionView = {
    id: 77,
    kind: "activate",
    label: testMessageRef("Pump"),
    needs_target: true,
    object: 11,
    section: "battlefield",
    targets: [{ kind: "object", id: 1 }],
  };
  const gameFold = fold({ objects: crowdedObjects(), actions: [pump] });
  const visible = gameFold.state as VisibleState;
  const board: BoardModel = { ...initialBoardModel(), selectedId: 10 };

  const index = selectedRadialOptions(board, visible).findIndex((o) => o.kind === "action" && o.action.id === pump.id);
  if (index < 0) throw new Error("expected the cluster menu to offer copy 11's ability");

  const [after] = updateBoard(board, RadialOptionPicked({ index }), gameFold, "T1");

  expect(after.staged?.card.id).toBe(11);
});
