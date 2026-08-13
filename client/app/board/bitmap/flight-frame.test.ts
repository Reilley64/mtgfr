import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import type { PlayerView } from "~/wire/types";
import { BLANK_FACE } from "../../domain/card-render/frame";
import { type RenderCard, ZONE } from "../geometry/layout";
import { spawnFlight } from "../motion/flights";
import { mergeFlightPoses, restingPaintChanged, restingPaintSnapshot } from "./flight-frame";
import type { BitmapFrame } from "./mount";

type RestingFrame = Omit<BitmapFrame, "flights" | "exitFx" | "dragGhost">;

function card(overrides: Partial<RenderCard> = {}): RenderCard {
  return {
    cardId: "card",
    cluster: 0,
    clusterMembers: [],
    controller: 0,
    counters: 0,
    faceDown: false,
    goaded: false,
    face: BLANK_FACE,
    h: 134,
    hasHaste: false,
    id: 1,
    isCommander: false,
    keywords: [],
    kind: "creature",
    markedDamage: 0,
    name: "Grizzly Bears",
    owner: 0,
    pile: 0,
    prepared: false,
    print: "resting-print",
    pt: "2/2",
    summoningSick: false,
    tapped: false,
    tapsForMana: false,
    w: 96,
    x: 10,
    y: 20,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function player(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: 0,
    username: "Alice",
    ...overrides,
  };
}

function restingFrame(overrides: Partial<RestingFrame> = {}): RestingFrame {
  return {
    width: 1440,
    height: 900,
    dpr: 1,
    camera: { panX: 0, panY: 0, zoom: 1 },
    cards: [card()],
    hoveredAttachmentId: null,
    viewer: 0,
    players: [],
    priority: 0,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    stagedAttackers: [],
    stagedBlocks: [],
    hideCardIds: new Set(),
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
    ...overrides,
  };
}

describe("restingPaintChanged", () => {
  it("is false when only flights would differ (snapshot omits flights)", () => {
    const a = restingPaintSnapshot(restingFrame());
    const b = restingPaintSnapshot(restingFrame());
    expect(restingPaintChanged(a, b)).toBe(false);
  });

  it("is true when hideCardIds or camera changes", () => {
    const a = restingPaintSnapshot(restingFrame({ hideCardIds: new Set([1]) }));
    const b = restingPaintSnapshot(restingFrame({ hideCardIds: new Set() }));
    expect(restingPaintChanged(a, b)).toBe(true);
  });

  it("is true when only the hovered attachment changes", () => {
    const before = restingPaintSnapshot(restingFrame({ hoveredAttachmentId: null }));
    const after = restingPaintSnapshot(restingFrame({ hoveredAttachmentId: 7 }));

    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is false when only attachment hover animation progress changes", () => {
    const before = restingPaintSnapshot(
      restingFrame({ hoveredAttachmentId: 7, attachmentHoverProgress: new Map([[7, 0]]) }),
    );
    const after = restingPaintSnapshot(
      restingFrame({ hoveredAttachmentId: 7, attachmentHoverProgress: new Map([[7, 0.5]]) }),
    );

    expect(restingPaintChanged(before, after)).toBe(false);
  });

  it("is true when only an overflow-shifted avatar position changes", () => {
    const before = restingPaintSnapshot(restingFrame({ avatarPositions: { 0: { x: 200, y: 300 } } }));
    const after = restingPaintSnapshot(restingFrame({ avatarPositions: { 0: { x: 900, y: 300 } } }));
    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is true when only a card's tap rotation changes", () => {
    const upright = restingPaintSnapshot(restingFrame({ cards: [card({ tapFrac: 0 })] }));
    const tapped = restingPaintSnapshot(restingFrame({ cards: [card({ tapFrac: 0.5 })] }));
    expect(restingPaintChanged(upright, tapped)).toBe(true);
  });

  it("is true when only a card's rendered frame changes", () => {
    const face = {
      print: "p",
      name: "Painter's Servant",
      colors: [0],
      isLand: false,
      isToken: false,
      legendary: false,
      power: "1",
      toughness: "3",
      loyalty: "",
      typeLine: "Artifact Creature — Scarecrow",
      oracle: "",
      flavor: "",
    };
    const before = restingPaintSnapshot(restingFrame({ cards: [card({ face })] }));
    const after = restingPaintSnapshot(restingFrame({ cards: [card({ face: { ...face, colors: [1] } })] }));
    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is true when two stack entries share a source but have different lossless entry identity", () => {
    const source = 9;
    const first = BigInt(Number.MAX_SAFE_INTEGER) + 1n;
    const second = first + 1n;
    const before = restingPaintSnapshot({
      ...restingFrame(),
      stack: [{ controller: 0, entry_id: first, kind: "ability", label: { key: "card.name", params: [] }, source }],
    } as never);
    const after = restingPaintSnapshot({
      ...restingFrame(),
      stack: [{ controller: 0, entry_id: second, kind: "ability", label: { key: "card.name", params: [] }, source }],
    } as never);

    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is true when only stack declared targets change", () => {
    const before = restingPaintSnapshot({
      ...restingFrame(),
      stack: [{ entry_id: 1n, controller: 0, kind: "spell", label: { key: "card.name", params: [] }, source: 9 }],
    } as never);
    const after = restingPaintSnapshot({
      ...restingFrame(),
      stack: [
        {
          controller: 0,
          entry_id: 1n,
          kind: "spell",
          label: { key: "card.name", params: [] },
          source: 9,
          target: { kind: "object", id: 1 },
        },
      ],
    } as never);
    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is true when only commander_damage changes on a player", () => {
    const before = restingPaintSnapshot(restingFrame({ players: [player()] }));
    const after = restingPaintSnapshot(
      restingFrame({ players: [player({ commander_damage: [{ from: 1, amount: 14 }] })] }),
    );
    expect(restingPaintChanged(before, after)).toBe(true);
  });

  it("is true when only gravatar_hash changes on a player", () => {
    const before = restingPaintSnapshot(restingFrame({ players: [player({ gravatar_hash: "" })] }));
    const after = restingPaintSnapshot(restingFrame({ players: [player({ gravatar_hash: "abc123" })] }));

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
      phase: "flying",
    });
  });

  it("releases a parked settled flight when authority marks it flying again", () => {
    const live = [
      {
        ...spawnFlight({
          id: 7,
          print: "p",
          name: "Bolt",
          x: 40,
          y: 80,
          scale: 1,
          targetX: 100,
          targetY: 200,
          targetScale: 1,
          kind: "stack",
          hold: true,
        }),
        phase: "settled" as const,
      },
    ];
    const incoming = [{ ...live[0], hold: false, phase: "flying" as const, targetX: 120, targetY: 210 }];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({
      x: 40,
      y: 80,
      phase: "flying",
      hold: false,
      targetX: 120,
    });
  });

  it("keeps the live pose across landPlayFrom id rebind instead of restarting from the stale seed", () => {
    // Mount has advanced the hand-id seed; model still holds the spawn pose and land sync
    // rebinds to the permanent id. Matching only by id drops the live pose and the card
    // jumps back toward the hand — the every-time land double animation.
    const live = [
      {
        ...spawnFlight({
          id: 102,
          print: "forest",
          name: "Forest",
          x: 400,
          y: 200,
          scale: 2.5,
          targetX: 737,
          targetY: 565,
          targetScale: 1,
          kind: "battlefield",
          fromCardId: 102,
          hold: true,
        }),
        x: 680,
        y: 480,
        scale: 1.3,
        phase: "flying" as const,
      },
    ];
    const incoming = [
      {
        ...live[0],
        id: 214,
        fromCardId: 102,
        // Stale model spawn pose — must not win over the live Mount pose.
        x: 400,
        y: 200,
        scale: 2.5,
      },
    ];
    expect(mergeFlightPoses(live, incoming)[0]).toMatchObject({
      id: 214,
      fromCardId: 102,
      x: 680,
      y: 480,
      scale: 1.3,
      targetX: 737,
      targetY: 565,
      phase: "flying",
    });
  });
});
