import { describe, expect, it } from "vitest";
import { spawnFlight } from "../motion/flights";
import { mergeFlightPoses, restingPaintChanged, restingPaintSnapshot } from "./flight-frame";

const baseResting = {
  width: 1440,
  height: 900,
  camera: { panX: 0, panY: 0, zoom: 1 },
  cards: [{ id: 1 }],
  viewer: 0,
  players: [],
  priority: 0,
  combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
  stagedAttackers: [],
  stagedBlocks: [],
  hideCardIds: new Set<number>(),
  targetObjects: new Set<number>(),
  targetPlayers: new Set<number>(),
  aimFrom: null,
  cursor: { x: 0, y: 0 },
  combatDragFrom: null,
  combatDragStroke: null,
  paymentPreviewIds: new Set<number>(),
  actions: undefined as undefined,
};

describe("restingPaintChanged", () => {
  it("is false when only flights would differ (snapshot omits flights)", () => {
    const a = restingPaintSnapshot({ ...baseResting /* snapshot factory ignores flights */ } as never);
    const b = restingPaintSnapshot({ ...baseResting } as never);
    expect(restingPaintChanged(a, b)).toBe(false);
  });

  it("is true when hideCardIds or camera changes", () => {
    const a = restingPaintSnapshot({ ...baseResting, hideCardIds: new Set([1]) } as never);
    const b = restingPaintSnapshot({ ...baseResting, hideCardIds: new Set() } as never);
    expect(restingPaintChanged(a, b)).toBe(true);
  });

  it("is true when only fanAngle changes on a card", () => {
    const flat = restingPaintSnapshot({ ...baseResting, cards: [{ id: 1, fanAngle: 0 }] } as never);
    const fanned = restingPaintSnapshot({ ...baseResting, cards: [{ id: 1, fanAngle: 0.12 }] } as never);
    expect(restingPaintChanged(flat, fanned)).toBe(true);
  });

  it("is true when only commander_damage changes on a player", () => {
    const before = restingPaintSnapshot({
      ...baseResting,
      players: [{ player: 0, life: 40, lost: false, username: "Alice", hand_count: 7 }],
    } as never);
    const after = restingPaintSnapshot({
      ...baseResting,
      players: [
        {
          player: 0,
          life: 40,
          lost: false,
          username: "Alice",
          hand_count: 7,
          commander_damage: [{ from: 1, amount: 14 }],
        },
      ],
    } as never);
    expect(restingPaintChanged(before, after)).toBe(true);
  });
});

describe("mergeFlightPoses", () => {
  it("keeps live x/y/scale when id and targets match", () => {
    const incoming = [
      spawnFlight({
        id: 7,
        print: "p",
        name: "Bolt",
        x: 0,
        y: 0,
        scale: 1,
        targetX: 100,
        targetY: 200,
        targetScale: 1,
        kind: "battlefield",
      }),
    ];
    const live = [{ ...incoming[0], x: 40, y: 80, scale: 1, phase: "flying" as const }];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({ id: 7, x: 40, y: 80, targetX: 100, targetY: 200 });
  });

  it("keeps the live pose when target retargets", () => {
    const live = [
      spawnFlight({
        id: 7,
        print: "p",
        name: "Bolt",
        x: 40,
        y: 80,
        scale: 1,
        targetX: 100,
        targetY: 200,
        targetScale: 1,
        kind: "battlefield",
      }),
    ];
    const incoming = [{ ...live[0], targetX: 300, targetY: 400, x: 0, y: 0, scale: 0.5, phase: "settled" as const }];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({
      targetX: 300,
      targetY: 400,
      x: 40,
      y: 80,
      scale: 1,
      phase: live[0].phase,
    });
  });
});
