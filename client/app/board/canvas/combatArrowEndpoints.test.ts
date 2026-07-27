import { describe, expect, it } from "vitest";
import type { RenderCard } from "../geometry/layout";
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
