import { describe, expect, it, test } from "vitest";
import type { ObjectView, VisibleState } from "~/wire/types";
import { BLANK_FACE } from "../../domain/card-render/frame";
import { engagedIds } from "../engagement";
import { fitCamera } from "../geometry/interaction";
import { layout, type RenderCard, ZONE } from "../geometry/layout";
import { initialBoardModel } from "../submodel";
import { avatarScreenPositions } from "./avatars";
import { allBlockersDeclared, combatArrowEndpoints } from "./combatArrowEndpoints";

function card(id: number, over: Partial<RenderCard> = {}): RenderCard {
  return {
    id,
    x: 100,
    y: 200,
    w: 96,
    h: 134,
    name: "Bear",
    cardId: "",
    print: "",
    pt: "2/2",
    tapped: false,
    counters: 0,
    markedDamage: 0,
    faceDown: false,
    zone: 1,
    controller: 1,
    owner: 1,
    kind: "creature",
    tapsForMana: false,
    summoningSick: false,
    hasHaste: false,
    keywords: [],
    goaded: false,
    face: BLANK_FACE,
    isCommander: false,
    prepared: false,
    pile: 0,
    cluster: 0,
    clusterMembers: [],
    ...over,
  };
}

describe("allBlockersDeclared", () => {
  it("requires every distinct defender seat to have declared", () => {
    expect(
      allBlockersDeclared(
        [
          { attacker: 1, defender: 1 },
          { attacker: 2, defender: 2 },
        ],
        [1],
      ),
    ).toBe(false);
    expect(
      allBlockersDeclared(
        [
          { attacker: 1, defender: 1 },
          { attacker: 2, defender: 2 },
        ],
        [1, 2],
      ),
    ).toBe(true);
  });
});

describe("combatArrowEndpoints", () => {
  it("keeps attacker-to-defender arrows and block arrows before every defender declares blockers", () => {
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 }), card(11, { x: 0, y: 200 }), card(20, { x: 200, y: 0 })],
      avatars: { 1: { x: 100, y: 40 }, 2: { x: 500, y: 40 } },
      attackers: [
        { attacker: 10, defender: 1 },
        { attacker: 11, defender: 2 },
      ],
      blocks: [{ blocker: 20, attacker: 10 }],
      blockersDeclared: [1],
      blockedAttackers: [10],
    });

    expect(endpoints).toEqual([
      { from: { x: 48, y: 67 }, to: { x: 100, y: 40 }, kind: "attack" },
      { from: { x: 48, y: 267 }, to: { x: 500, y: 40 }, kind: "attack" },
      { from: { x: 248, y: 67 }, to: { x: 48, y: 67 }, kind: "block" },
    ]);
  });

  it("targets an attacked planeswalker card before every defender declares blockers", () => {
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 }), card(30, { x: 300, y: 100, kind: "planeswalker" })],
      avatars: { 1: { x: 100, y: 40 } },
      attackers: [{ attacker: 10, defender: 1, defender_planeswalker: 30 }],
      blocks: [],
      blockersDeclared: [],
      blockedAttackers: [],
    });

    expect(endpoints).toEqual([{ from: { x: 48, y: 67 }, to: { x: 348, y: 167 }, kind: "attack" }]);
  });

  it("retargets blocked attackers to living blockers after every defender declares blockers", () => {
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 }), card(20, { x: 200, y: 0 })],
      avatars: { 1: { x: 100, y: 40 } },
      attackers: [{ attacker: 10, defender: 1 }],
      blocks: [{ blocker: 20, attacker: 10 }],
      blockersDeclared: [1],
      blockedAttackers: [10],
    });

    expect(endpoints).toEqual([{ from: { x: 48, y: 67 }, to: { x: 248, y: 67 }, kind: "attack" }]);
    expect(endpoints[0]?.to).not.toEqual({ x: 100, y: 40 });
  });

  it("omits blocked attackers when block record references a blocker absent from cards", () => {
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 })],
      avatars: { 1: { x: 100, y: 40 } },
      attackers: [{ attacker: 10, defender: 1 }],
      blocks: [{ blocker: 20, attacker: 10 }],
      blockersDeclared: [1],
      blockedAttackers: [10],
    });

    expect(endpoints).toEqual([]);
  });

  it("keeps unblocked attackers pointed at their defender after every defender declares blockers", () => {
    const avatar = { x: 100, y: 40 };
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 })],
      avatars: { 1: avatar },
      attackers: [{ attacker: 10, defender: 1 }],
      blocks: [],
      blockersDeclared: [1],
      blockedAttackers: [],
    });

    expect(endpoints).toEqual([{ from: { x: 48, y: 67 }, to: avatar, kind: "attack" }]);
  });

  it("stays in pre-declare mode until all attacked defenders have declared blockers", () => {
    const endpoints = combatArrowEndpoints({
      camera: { panX: 0, panY: 0, zoom: 1 },
      cards: [card(10, { x: 0, y: 0 }), card(11, { x: 0, y: 200 }), card(20, { x: 200, y: 0 })],
      avatars: { 1: { x: 100, y: 40 }, 2: { x: 500, y: 40 } },
      attackers: [
        { attacker: 10, defender: 1 },
        { attacker: 11, defender: 2 },
      ],
      blocks: [{ blocker: 20, attacker: 10 }],
      blockersDeclared: [1],
      blockedAttackers: [10],
    });

    expect(endpoints.some((endpoint) => endpoint.kind === "block")).toBe(true);
    expect(endpoints).toContainEqual({ from: { x: 248, y: 67 }, to: { x: 48, y: 67 }, kind: "block" });
  });
});

// Regression coverage for a real `layout()`-built card list, not the `card()` fixture above:
// `combatArrowEndpoints` used to look up an attacker through the layout id map and silently
// `continue` when it was absent — a declared attacker inside a permanent cluster drew no arrow.
function creature(id: number, controller: number, over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller,
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
    owner: controller,
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
      {
        commander_tax: 0,
        hand_count: 7,
        library_count: 80,
        life: 40,
        lost: false,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
        player: 1,
        username: "Bob",
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
    ...over,
  };
}

test("an attacker inside a permanent cluster still draws its combat arrow", () => {
  const bears = Array.from({ length: 6 }, (_, i) => creature(i + 1, 0, { name: `Bear ${i}` }));
  const saprolings = [10, 11, 12, 13].map((id) =>
    creature(id, 0, { name: "Saproling", power: 1, toughness: 1, kind: { kind: "creature", power: 1, toughness: 1 } }),
  );
  const attacking = state({
    objects: [...bears, ...saprolings],
    // Attacker is 12, not the cluster's lowest id (10) — the lowest id is always the collapsed
    // cluster's face regardless of engagement, so an attacker id that already happens to be the
    // face wouldn't discriminate a broken engaged-set wire-up from a working one.
    combat: {
      attackers: [{ attacker: 12, defender: 1 }],
      blocks: [],
      attackers_declared: true,
      blockers_declared: [],
      blocked_attackers: [],
    },
  });

  const endpoints = combatArrowEndpoints({
    camera: fitCamera({ x: 1600, y: 900 }, 2, 0),
    cards: layout(attacking, 0, engagedIds(attacking, initialBoardModel())),
    avatars: avatarScreenPositions(attacking.players, 0, 2, fitCamera({ x: 1600, y: 900 }, 2, 0)),
    attackers: attacking.combat.attackers,
    blocks: [],
    blockersDeclared: [],
    blockedAttackers: [],
  });

  expect(endpoints.filter((e) => e.kind === "attack")).toHaveLength(1);
});
