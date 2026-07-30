import { expect, test } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import type { GameFoldState } from "../game/fold";
import { engagedIds } from "./engagement";
import { layout, ZONE } from "./geometry/layout";
import { BoardPointerDown, BoardPointerUp, CombatAttackerDropped } from "./messages";
import { type BoardModel, initialBoardModel, updateBoard } from "./submodel";

function creature(id: number, over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
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
