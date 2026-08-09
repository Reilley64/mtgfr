import { describe, expect, it } from "vitest";
import { testMessageRef } from "~/i18n/testMessageRef";
import { emptyManaPool } from "~/manaPips";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import { stackTargetArrowEndpoints } from "../canvas/arrows";
import { combatArrowEndpoints } from "../canvas/combatArrowEndpoints";
import { hitAvatar, hitTest } from "./hit-test";
import {
  AVATAR_LABEL_BELOW,
  AVATAR_R,
  avatarPos,
  boardBounds,
  CARD_H,
  CARD_W,
  FLIGHT_CARD_H,
  FLIGHT_CARD_W,
  layout,
  layoutBoard,
  manaTrayPos,
  STEP,
  STEP_NAMES,
  seatBand,
  seatCell,
  seatSlot,
  ZONE,
} from "./layout";
import {
  MIN_CROWDED_PERMANENT_SIDE,
  PERMANENT_CLEARANCE,
  PERMANENT_STEP,
  permanentRowMetrics,
  tiltedPermanentExtent,
} from "./permanent-layout";

const TEST_GAP = 8;
const TEST_COL_X = -64;
const TEST_BATTLE_H = 3 * PERMANENT_STEP;
const TEST_BAND_STRIDE = TEST_BATTLE_H + TEST_GAP;
const TEST_ROW_W = 6 * PERMANENT_STEP + CARD_W;
const TEST_SEAT_RIGHT = TEST_ROW_W + TEST_GAP;
const TEST_SEAT_STRIDE_X = TEST_SEAT_RIGHT - TEST_COL_X + PERMANENT_STEP;
const TEST_BAND_W = TEST_SEAT_RIGHT - TEST_COL_X + TEST_GAP;
const TEST_CARD_Y_IN_ROW = (PERMANENT_STEP - CARD_H) / 2;

describe("card tile shape", () => {
  it("rests permanents in a square tile so a four-seat board reads at a glance", () => {
    expect(CARD_W).toBe(CARD_H);
  });

  it("keeps a card in motion card-shaped — a flight does not morph mid-air", () => {
    expect(FLIGHT_CARD_H / FLIGHT_CARD_W).toBeCloseTo(134 / 96, 2);
  });
});

function mkObject(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 0,
    is_commander: false,
    is_token: false,
    legendary: false,
    kind: { kind: "creature", power: 0, toughness: 0 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Object",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 0,
    summoning_sick: false,
    tapped: false,
    toughness: 0,
    zone: ZONE.Battlefield,
    ...overrides,
  };
}

function mkPlayer(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 0,
    library_count: 0,
    life: 40,
    lost: false,
    mana_pool: emptyManaPool(),
    player: 0,
    ...overrides,
  };
}

function mkState(overrides: Partial<VisibleState> = {}): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [], blocked_attackers: [] },
    objects: [],
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 0,
    viewer: 0,
    ...overrides,
  };
}

function depthFixture(): VisibleState {
  const objects = [0, 1].flatMap((controller) => [
    mkObject({
      id: controller * 10 + 1,
      controller,
      owner: controller,
      name: "Forest",
      kind: { kind: "land", colors: [4] },
    }),
    mkObject({ id: controller * 10 + 2, controller, owner: controller, name: "Bear" }),
    mkObject({
      id: controller * 10 + 3,
      controller,
      owner: controller,
      name: "Sol Ring",
      kind: { kind: "artifact" },
    }),
  ]);
  return mkState({ players: [mkPlayer({ player: 0 }), mkPlayer({ player: 1 })], objects });
}

function crowdedCreatureFixture(count: number): VisibleState {
  return mkState({
    players: [mkPlayer({ player: 0 })],
    objects: [
      ...Array.from({ length: count }, (_, index) => mkObject({ id: index + 1, name: `Unique Bear ${index}` })),
      mkObject({ id: 1000, name: "Forest", kind: { kind: "land", colors: [4] } }),
    ],
  });
}

describe("seatSlot", () => {
  it("is viewer-relative", () => {
    expect(seatSlot(2, 2, 4)).toBe(0);
    expect(seatSlot(3, 2, 4)).toBe(1);
  });

  it("falls back to seat order for a spectator", () => {
    expect(seatSlot(2, 255, 4)).toBe(2);
  });

  it("clamps count to avoid NaN", () => {
    expect(seatSlot(0, 0, 0)).toBe(0);
    expect(seatSlot(1, 0, 0)).toBe(0);
  });
});

describe("seatCell", () => {
  it("rotates the table so the viewer sits bottom-left", () => {
    expect(seatCell(2, 2, 4)).toEqual({ col: 0, row: 1 });
    expect(seatCell(3, 2, 4)).toEqual({ col: 0, row: 0 });
  });

  it("gives a spectator four distinct quadrants in seat order", () => {
    // The spectator sentinel viewer (255) sits at no seat, so seats keep their own order
    // instead of every band collapsing into the bottom-left cell.
    const cells = [0, 1, 2, 3].map((seat) => seatCell(seat, 255, 4));
    expect(cells).toEqual([
      { col: 0, row: 1 },
      { col: 0, row: 0 },
      { col: 1, row: 1 },
      { col: 1, row: 0 },
    ]);
  });
});

describe("seatBand", () => {
  it("puts the viewer's own band at the bottom-left of a 4-player table", () => {
    expect(seatBand(0, 0, 4)).toEqual({
      x: TEST_COL_X - TEST_GAP,
      y: TEST_BAND_STRIDE - TEST_GAP,
      w: TEST_BAND_W,
      h: TEST_BATTLE_H + TEST_GAP,
    });
  });

  it("puts the seat after the viewer directly in front (top-left)", () => {
    expect(seatBand(1, 0, 4)).toEqual({
      x: TEST_COL_X - TEST_GAP,
      y: -TEST_GAP,
      w: TEST_BAND_W,
      h: TEST_BATTLE_H + TEST_GAP,
    });
  });

  it("puts the next seat to the side (bottom-right) and the last diagonal (top-right)", () => {
    expect(seatBand(2, 0, 4)).toMatchObject({
      x: TEST_SEAT_STRIDE_X + TEST_COL_X - TEST_GAP,
      y: TEST_BAND_STRIDE - TEST_GAP,
    });
    expect(seatBand(3, 0, 4)).toMatchObject({ x: TEST_SEAT_STRIDE_X + TEST_COL_X - TEST_GAP, y: -TEST_GAP });
  });

  it("assigns quadrants by turn order regardless of which seat is the viewer", () => {
    // Viewer is seat 2: turn order after them is 3 (front), 0 (side), 1 (diagonal).
    expect(seatBand(2, 2, 4)).toMatchObject({ x: TEST_COL_X - TEST_GAP, y: TEST_BAND_STRIDE - TEST_GAP });
    expect(seatBand(3, 2, 4)).toMatchObject({ x: TEST_COL_X - TEST_GAP, y: -TEST_GAP });
    expect(seatBand(0, 2, 4)).toMatchObject({
      x: TEST_SEAT_STRIDE_X + TEST_COL_X - TEST_GAP,
      y: TEST_BAND_STRIDE - TEST_GAP,
    });
    expect(seatBand(1, 2, 4)).toMatchObject({ x: TEST_SEAT_STRIDE_X + TEST_COL_X - TEST_GAP, y: -TEST_GAP });
  });
});

describe("avatarPos", () => {
  it("sits below the viewer's own bottom-left band", () => {
    const band = seatBand(0, 0, 4);
    expect(avatarPos(0, 0, 4)).toEqual({
      x: band.x + band.w / 2,
      y: TEST_BAND_STRIDE + TEST_BATTLE_H + AVATAR_R + TEST_GAP,
    });
  });

  it("sits above the flipped front seat's band", () => {
    const band = seatBand(1, 0, 4);
    expect(avatarPos(1, 0, 4)).toEqual({ x: band.x + band.w / 2, y: -AVATAR_R - TEST_GAP });
  });

  it("sits below the upright side seat and above the flipped diagonal", () => {
    expect(avatarPos(2, 0, 4)).toEqual({
      x: seatBand(2, 0, 4).x + TEST_BAND_W / 2,
      y: TEST_BAND_STRIDE + TEST_BATTLE_H + AVATAR_R + TEST_GAP,
    });
    expect(avatarPos(3, 0, 4)).toEqual({
      x: seatBand(3, 0, 4).x + TEST_BAND_W / 2,
      y: -AVATAR_R - TEST_GAP,
    });
  });
});

describe("manaTrayPos", () => {
  // Past the zone column (COL_X + COL_W + GAP), just outside the seat band on the outer edge.
  it("sits under the zone column below the viewer's upright band", () => {
    const band = seatBand(0, 0, 4);
    const tray = manaTrayPos(0, 0, 4);
    expect(tray).toEqual({ x: -8, y: band.y + band.h + TEST_GAP });
    expect(tray.x).toBeLessThan(band.x + band.w / 2);
    expect(tray.y).toBeGreaterThan(band.y + band.h);
  });

  it("sits under the zone column above the flipped front seat's band", () => {
    const band = seatBand(1, 0, 4);
    const tray = manaTrayPos(1, 0, 4);
    expect(tray).toEqual({ x: -8, y: -16 });
    expect(tray.y).toBeLessThan(band.y);
  });

  it("keeps the same seat-relative offset for side and diagonal", () => {
    expect(manaTrayPos(2, 0, 4)).toEqual({
      x: TEST_SEAT_STRIDE_X - 8,
      y: seatBand(2, 0, 4).y + seatBand(2, 0, 4).h + TEST_GAP,
    });
    expect(manaTrayPos(3, 0, 4)).toEqual({ x: TEST_SEAT_STRIDE_X - 8, y: -16 });
  });
});

describe("boardBounds", () => {
  // A 2-player table is a single (left) column; 3 and 4 players both span both columns, so their
  // bounds match (the 3p table just leaves the diagonal cell empty).
  it("fits a 2-player table (one column)", () => {
    expect(boardBounds(2)).toEqual({
      minX: TEST_COL_X - TEST_GAP,
      minY: -128,
      maxX: TEST_SEAT_RIGHT,
      maxY: TEST_BAND_STRIDE + TEST_BATTLE_H + AVATAR_R + TEST_GAP + AVATAR_LABEL_BELOW,
    });
  });

  it("fits a 3-player table (both columns, no diagonal)", () => {
    expect(boardBounds(3)).toEqual({
      minX: TEST_COL_X - TEST_GAP,
      minY: -128,
      maxX: TEST_SEAT_STRIDE_X + TEST_SEAT_RIGHT,
      maxY: TEST_BAND_STRIDE + TEST_BATTLE_H + AVATAR_R + TEST_GAP + AVATAR_LABEL_BELOW,
    });
  });

  it("fits a 4-player table (full 2×2)", () => {
    expect(boardBounds(4)).toEqual(boardBounds(3));
  });

  it("reserves label space on the outer side of flipped and upright seats", () => {
    const bounds = boardBounds(2);
    const front = avatarPos(1, 0, 2);
    const viewer = avatarPos(0, 0, 2);

    expect(bounds.minY).toBe(front.y - AVATAR_LABEL_BELOW);
    expect(bounds.maxY).toBe(viewer.y + AVATAR_LABEL_BELOW);
  });
});

describe("layout", () => {
  it("layers every seat from its controller avatar toward table center", () => {
    const cards = layout(depthFixture(), 0).filter((card) => card.zone === ZONE.Battlefield);
    for (const controller of [0, 1]) {
      const owned = cards.filter((card) => card.controller === controller);
      expect(owned.map((card) => card.kind)).toEqual(["land", "creature", "artifact"]);
    }
  });

  it("shrinks only an overcrowded row and keeps its tilted cards separated", () => {
    const uncrowded = layout(crowdedCreatureFixture(7), 0);
    const cards = layout(crowdedCreatureFixture(12), 0).filter((card) => card.zone === ZONE.Battlefield);
    const creatures = cards.filter((card) => card.kind === "creature");
    const land = cards.find((card) => card.kind === "land");
    const uncrowdedLand = uncrowded.find((card) => card.kind === "land");
    expect(new Set(creatures.map((card) => card.w)).size).toBe(1);
    expect(creatures[0]?.w).toBeLessThan(CARD_W);
    expect(land).toMatchObject({ w: CARD_W, h: CARD_H });
    expect(land?.y).toBe(uncrowdedLand?.y);
    for (let index = 1; index < creatures.length; index += 1) {
      expect(
        creatures[index].x - creatures[index - 1].x - tiltedPermanentExtent(creatures[index].w),
      ).toBeGreaterThanOrEqual(PERMANENT_CLEARANCE - 1e-9);
    }
  });

  it("keeps full-size maximum-tilt rows vertically separated", () => {
    const rows = layout(depthFixture(), 0)
      .filter((card) => card.controller === 0 && card.zone === ZONE.Battlefield)
      .sort((left, right) => left.y - right.y);
    for (let index = 1; index < rows.length; index += 1) {
      expect(rows[index].y - rows[index - 1].y - tiltedPermanentExtent(CARD_H)).toBeGreaterThanOrEqual(
        PERMANENT_CLEARANCE - 1e-9,
      );
    }
  });

  it("does not change permanent size when it becomes tapped", () => {
    const upright = layout(crowdedCreatureFixture(7), 0).find((card) => card.kind === "creature");
    const tappedState = crowdedCreatureFixture(7);
    tappedState.objects = tappedState.objects.map((object) => ({ ...object, tapped: true }));
    const tapped = layout(tappedState, 0).find((card) => card.kind === "creature");
    expect(upright).toBeDefined();
    expect(tapped).toBeDefined();
    if (upright == null || tapped == null) throw new Error("missing permanent fixture");
    expect(tapped).toMatchObject({ w: upright.w, h: upright.h });
  });

  it("widens content bounds and shifts the next table column for a minimum-size row", () => {
    const crowded = Array.from({ length: 59 }, (_, index) => mkObject({ id: index + 1, name: `Unique Bear ${index}` }));
    const sideSeat = mkObject({ id: 1000, controller: 2, owner: 2, name: "Side Bear" });
    const state = mkState({
      players: [0, 1, 2, 3].map((player) => mkPlayer({ player })),
      objects: [...crowded, sideSeat],
    });

    const board = layoutBoard(state, 0);
    const crowdedCards = board.cards.filter((card) => card.controller === 0 && card.kind === "creature");
    const leftBand = board.seatBands.get(0);
    const rightBand = board.seatBands.get(2);

    expect(crowdedCards).toHaveLength(59);
    expect(crowdedCards.every((card) => card.w === MIN_CROWDED_PERMANENT_SIDE)).toBe(true);
    expect(board.bounds.maxX).toBeGreaterThan(boardBounds(4).maxX);
    expect(leftBand).toBeDefined();
    expect(rightBand).toBeDefined();
    if (leftBand == null || rightBand == null) throw new Error("missing seat bands");
    expect(rightBand.x).toBeGreaterThanOrEqual(leftBand.x + leftBand.w);
  });

  it("uses overflow-shifted avatars for hits and combat and stack arrow endpoints", () => {
    const crowded = Array.from({ length: 59 }, (_, index) => mkObject({ id: index + 1, name: `Unique Bear ${index}` }));
    const state = mkState({
      players: [0, 1, 2, 3].map((player) => mkPlayer({ player })),
      objects: crowded,
    });
    const board = layoutBoard(state, 0);
    const shifted = board.avatarPositions[2];
    const oldStatic = avatarPos(2, 0, 4);
    expect(shifted).toBeDefined();
    if (shifted == null) throw new Error("missing shifted avatar");
    expect(shifted).not.toEqual(oldStatic);

    const identity = { panX: 0, panY: 0, zoom: 1 };
    expect(hitAvatar(identity, shifted.x, shifted.y, { 2: shifted })).toBe(2);
    expect(hitAvatar(identity, oldStatic.x, oldStatic.y, { 2: shifted })).toBeNull();

    const combat = combatArrowEndpoints({
      camera: identity,
      cards: board.cards,
      avatars: board.avatarPositions,
      attackers: [{ attacker: 1, defender: 2 }],
      blocks: [],
      blockersDeclared: [],
      blockedAttackers: [],
    });
    expect(combat[0]?.to).toEqual(shifted);

    const stack = stackTargetArrowEndpoints({
      viewport: { width: 1440, height: 900 },
      stack: [
        {
          controller: 0,
          kind: "spell",
          label: testMessageRef("card.name"),
          source: 100,
          target: { kind: "player", player: 2 },
        },
      ],
      cards: board.cards,
      avatars: board.avatarPositions,
      camera: identity,
    });
    expect(stack[0]?.to).toEqual(shifted);
  });

  // Board layout collisions (foldkit remaining-bugs task 9): zone-column faces are half-size art;
  // combat chrome (P/T) on those faces shares the art AABB. Prefer face-only in the column —
  // P/T belongs on battlefield permanents (and inspect), not on command/GY/exile miniatures.
  it("omits P/T chrome on zone-column cards for a 2-player fixture", () => {
    const state = mkState({
      viewer: 0,
      players: [mkPlayer({ player: 0, library_count: 30 }), mkPlayer({ player: 1, library_count: 25 })],
      objects: [
        mkObject({
          id: 1,
          name: "Grizzly Bears",
          controller: 0,
          owner: 0,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({
          id: 3,
          name: "Atraxa, Praetors' Voice",
          controller: 0,
          owner: 0,
          zone: ZONE.Command,
          is_commander: true,
          is_token: false,
          legendary: false,
          kind: { kind: "creature", power: 4, toughness: 4 },
          power: 4,
          toughness: 4,
        }),
        mkObject({
          id: 4,
          name: "Dead Bear",
          controller: 0,
          owner: 0,
          zone: ZONE.Graveyard,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({
          id: 5,
          name: "Opposing Bear",
          controller: 1,
          owner: 1,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 3, toughness: 3 },
          power: 3,
          toughness: 3,
        }),
        mkObject({
          id: 6,
          name: "Opposing Commander",
          controller: 1,
          owner: 1,
          zone: ZONE.Command,
          is_commander: true,
          is_token: false,
          legendary: false,
          kind: { kind: "creature", power: 5, toughness: 5 },
          power: 5,
          toughness: 5,
        }),
      ],
    });

    const cards = layout(state, 0);
    const byId = new Map(cards.map((c) => [c.id, c]));

    expect(byId.get(3)?.pt).toBe("");
    expect(byId.get(4)?.pt).toBe("");
    expect(byId.get(6)?.pt).toBe("");
    // Battlefield creatures keep combat chrome.
    expect(byId.get(1)?.pt).toBe("2/2");
    expect(byId.get(5)?.pt).toBe("3/3");
  });

  it("keeps zone-column cards and avatars free of AABB collisions on a 2-player table", () => {
    const state = mkState({
      viewer: 0,
      players: [mkPlayer({ player: 0, library_count: 30 }), mkPlayer({ player: 1, library_count: 25 })],
      objects: [
        mkObject({
          id: 3,
          name: "Atraxa, Praetors' Voice",
          controller: 0,
          owner: 0,
          zone: ZONE.Command,
          is_commander: true,
          is_token: false,
          legendary: false,
        }),
        mkObject({ id: 4, name: "Doom Blade", controller: 0, owner: 0, zone: ZONE.Graveyard }),
        mkObject({ id: 7, name: "Exiled Spell", controller: 0, owner: 0, zone: ZONE.Exile }),
        mkObject({
          id: 1,
          name: "Grizzly Bears",
          controller: 0,
          owner: 0,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({
          id: 6,
          name: "Opposing Commander",
          controller: 1,
          owner: 1,
          zone: ZONE.Command,
          is_commander: true,
          is_token: false,
          legendary: false,
        }),
        mkObject({
          id: 5,
          name: "Opposing Bear",
          controller: 1,
          owner: 1,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 3, toughness: 3 },
          power: 3,
          toughness: 3,
        }),
      ],
    });

    const cards = layout(state, 0);
    const boxes = cards.map((c) => ({ id: c.id, x: c.x, y: c.y, r: c.x + c.w, b: c.y + c.h }));
    for (let i = 0; i < boxes.length; i++) {
      for (let j = i + 1; j < boxes.length; j++) {
        const a = boxes[i];
        const b = boxes[j];
        const overlap = a.x < b.r && a.r > b.x && a.y < b.b && a.b > b.y;
        expect(overlap, `cards ${a.id} and ${b.id} overlap`).toBe(false);
      }
    }

    // Layer-2 avatar clear bands: packing must not cover the life-orb disk.
    for (const seat of [0, 1]) {
      const a = avatarPos(seat, 0, 2);
      for (const c of cards) {
        const cx = Math.max(c.x, Math.min(a.x, c.x + c.w));
        const cy = Math.max(c.y, Math.min(a.y, c.y + c.h));
        const d = Math.hypot(cx - a.x, cy - a.y);
        expect(d, `card ${c.id} intersects avatar ${seat}`).toBeGreaterThanOrEqual(AVATAR_R);
      }
    }

    // Seat mats stay landscape (not tall/narrow portrait strips).
    const band = seatBand(0, 0, 2);
    expect(band.w / band.h).toBeGreaterThanOrEqual(1.5);
    expect(band.w / band.h).toBeLessThanOrEqual(2.5);
  });

  it("positions a 2-player table: viewer's board upright, opponent's flipped", () => {
    const state = mkState({
      viewer: 0,
      players: [mkPlayer({ player: 0, library_count: 30 }), mkPlayer({ player: 1, library_count: 25 })],
      objects: [
        mkObject({
          id: 1,
          name: "Grizzly Bears",
          controller: 0,
          owner: 0,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({
          id: 2,
          name: "Forest",
          controller: 0,
          owner: 0,
          zone: ZONE.Battlefield,
          kind: { kind: "land", colors: [4] },
        }),
        mkObject({
          id: 3,
          name: "Atraxa, Praetors' Voice",
          controller: 0,
          owner: 0,
          zone: ZONE.Command,
          is_commander: true,
          is_token: false,
          legendary: false,
        }),
        mkObject({ id: 4, name: "Doom Blade", controller: 0, owner: 0, zone: ZONE.Graveyard }),
        mkObject({
          id: 5,
          name: "Opposing Bear",
          controller: 1,
          owner: 1,
          zone: ZONE.Battlefield,
          kind: { kind: "creature", power: 3, toughness: 3 },
          power: 3,
          toughness: 3,
        }),
      ],
    });

    const cards = layout(state, 0);
    const byId = new Map(cards.map((c) => [c.id, c]));

    const singleCardX = 3 * PERMANENT_STEP;
    expect(byId.get(1)).toMatchObject({
      x: singleCardX,
      y: TEST_BAND_STRIDE + PERMANENT_STEP + TEST_CARD_Y_IN_ROW,
      w: CARD_W,
      h: CARD_H,
      zone: ZONE.Battlefield,
    });
    expect(byId.get(2)).toMatchObject({
      x: singleCardX,
      y: TEST_BAND_STRIDE + 2 * PERMANENT_STEP + TEST_CARD_Y_IN_ROW,
      w: CARD_W,
      h: CARD_H,
    });
    // Zone column top -> bottom for the viewer: commander, deck (no exile), graveyard.
    const columnStride = TEST_BATTLE_H / 4;
    expect(byId.get(3)).toMatchObject({ x: TEST_COL_X, y: TEST_BAND_STRIDE, w: 48, h: 67, pile: 0 });
    expect(byId.get(4)).toMatchObject({
      x: TEST_COL_X,
      y: TEST_BAND_STRIDE + 3 * columnStride,
      w: 48,
      h: 67,
      pile: 1,
      zone: ZONE.Graveyard,
    });

    expect(byId.get(5)).toMatchObject({
      x: singleCardX,
      y: PERMANENT_STEP + TEST_CARD_Y_IN_ROW,
      w: CARD_W,
      h: CARD_H,
    });

    // Opponent's library placeholder is the only zone-column card (synthetic id -1 - owner = -2),
    // and it lands at the flipped column's second slot (deck is index 1 once reversed).
    const opponentDeck = cards.find((c) => c.id === -2);
    expect(opponentDeck).toMatchObject({ x: TEST_COL_X, y: columnStride, w: 48, h: 67, pile: 25, faceDown: true });

    // Viewer's own library placeholder is the third slot (index 2) in the unreversed column.
    const viewerDeck = cards.find((c) => c.id === -1);
    expect(viewerDeck).toMatchObject({
      x: TEST_COL_X,
      y: TEST_BAND_STRIDE + 2 * columnStride,
      w: 48,
      h: 67,
      pile: 30,
      faceDown: true,
    });

    expect(cards).toHaveLength(7);
  });

  it("shows the revealed top card of a library on the deck slot", () => {
    // Field of Dreams — "Players play with the top card of their libraries revealed." The server
    // itemizes exactly that one card per seat, so the deck slot wears its face instead of a back,
    // while still counting the whole library.
    const state = mkState({
      players: [mkPlayer({ player: 0, library_count: 30 }), mkPlayer({ player: 1, library_count: 25 })],
      objects: [mkObject({ id: 7, name: "Shock", owner: 1, controller: 1, zone: ZONE.Library, print: "abc" })],
    });

    const cards = layout(state, 0);

    const opponentDeck = cards.find((c) => c.zone === ZONE.Library && c.owner === 1);
    expect(opponentDeck).toMatchObject({ name: "Shock", print: "abc", faceDown: false, pile: 25 });
    const viewerDeck = cards.find((c) => c.zone === ZONE.Library && c.owner === 0);
    expect(viewerDeck).toMatchObject({ name: "Library", faceDown: true, pile: 30 });
  });

  it("omits the library slot when the library is empty", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0, library_count: 0 })],
      objects: [],
    });
    const cards = layout(state, 0);
    expect(cards.find((c) => c.faceDown && c.zone === ZONE.Library)).toBeUndefined();
  });

  it("splits Noncreature / Creatures / Lands into three reserved rows", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        mkObject({ id: 1, name: "Sol Ring", kind: { kind: "artifact" } }),
        mkObject({ id: 2, name: "Rhystic Study", kind: { kind: "enchantment" } }),
        mkObject({
          id: 3,
          name: "Bear",
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({ id: 4, name: "Forest", kind: { kind: "land", colors: [4] } }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    expect(byId.get(1)?.y).toBe(TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW);
    expect(byId.get(2)?.y).toBe(TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW);
    expect(byId.get(3)?.y).toBe(TEST_BAND_STRIDE + PERMANENT_STEP + TEST_CARD_Y_IN_ROW);
    expect(byId.get(4)?.y).toBe(TEST_BAND_STRIDE + 2 * PERMANENT_STEP + TEST_CARD_Y_IN_ROW);
  });

  it("left-aligns artifacts then enchantments; right-aligns planeswalkers", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        mkObject({ id: 1, name: "Sol Ring", kind: { kind: "artifact" } }),
        mkObject({ id: 2, name: "Arcane Signet", kind: { kind: "artifact" } }),
        mkObject({ id: 3, name: "Rhystic Study", kind: { kind: "enchantment" } }),
        mkObject({ id: 4, name: "Chandra", kind: { kind: "planeswalker", loyalty: 4 } }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    expect(byId.get(1)?.x).toBe(0);
    expect(byId.get(2)?.x).toBe(PERMANENT_STEP);
    expect(byId.get(3)?.x).toBe(2 * PERMANENT_STEP);
    expect(byId.get(4)?.x).toBe(6 * PERMANENT_STEP);
  });

  it("paints planeswalker loyalty in the P/T badge, falling back to WireKind when live loyalty is absent", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        // No ObjectView.loyalty — badge must use printed starting loyalty from kind.
        mkObject({ id: 1, name: "Chandra", kind: { kind: "planeswalker", loyalty: 4 } }),
        mkObject({
          id: 2,
          name: "Quintorius",
          kind: { kind: "planeswalker", loyalty: 5 },
          loyalty: 6,
        }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    expect(byId.get(1)?.pt).toBe("4");
    expect(byId.get(2)?.pt).toBe("6");
  });

  it("packs Noncreature left/right inside the seat when left + PWs exceed SEAT_COLS", () => {
    const left = Array.from({ length: 7 }, (_, i) =>
      mkObject({ id: i + 1, name: `Rock ${i}`, kind: { kind: "artifact" } }),
    );
    const pws = Array.from({ length: 4 }, (_, i) =>
      mkObject({ id: 100 + i, name: `Walker ${i}`, kind: { kind: "planeswalker", loyalty: 3 } }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [...left, ...pws],
    });
    const cards = layout(state, 0);
    const xs = cards.filter((c) => c.zone === ZONE.Battlefield).map((c) => c.x);
    expect(Math.min(...xs)).toBe(0);
    const rightmost = cards
      .filter((card) => card.zone === ZONE.Battlefield)
      .reduce((max, card) => Math.max(max, card.x + card.w), -Infinity);
    const metrics = permanentRowMetrics(left.length + pws.length, TEST_ROW_W);
    expect(rightmost + tiltedPermanentExtent(metrics.side) - metrics.side).toBeCloseTo(TEST_ROW_W);
    expect(new Set(xs).size).toBe(xs.length);
  });

  it("packs Creatures inside the seat when the row exceeds SEAT_COLS", () => {
    const creatures = Array.from({ length: 12 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: creatures,
    });
    const xs = layout(state, 0)
      .filter((c) => c.zone === ZONE.Battlefield)
      .map((c) => c.x)
      .sort((a, b) => a - b);
    expect(xs[0]).toBe(0);
    const metrics = permanentRowMetrics(creatures.length, TEST_ROW_W);
    expect(xs[xs.length - 1]).toBeCloseTo(TEST_ROW_W - tiltedPermanentExtent(metrics.side));
    const step = metrics.step;
    for (let i = 0; i < xs.length; i++) {
      expect(xs[i]).toBeCloseTo(i * step, 5);
    }
  });

  it("does not cluster identical permanents when the row still fits at full spacing", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: Array.from({ length: 4 }, (_, i) =>
        mkObject({
          id: i + 1,
          name: "Saproling",
          kind: { kind: "creature", power: 1, toughness: 1 },
          power: 1,
          toughness: 1,
        }),
      ),
    });
    const bf = layout(state, 0).filter((c) => c.zone === ZONE.Battlefield);
    expect(bf).toHaveLength(4);
    expect(bf.every((c) => c.cluster === 0)).toBe(true);
  });

  it("clusters all identical groups when the row overflows, using lowest id as the face", () => {
    // 6 unique + 4 identical Saprolings = 10 raw → overflow → Saprolings collapse to 1 → 7 slots, full spacing.
    const uniques = Array.from({ length: 6 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const saprolings = [10, 11, 12, 13].map((id) =>
      mkObject({
        id,
        name: "Saproling",
        kind: { kind: "creature", power: 1, toughness: 1 },
        power: 1,
        toughness: 1,
      }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [...uniques, ...saprolings],
    });
    const bf = layout(state, 0).filter((c) => c.zone === ZONE.Battlefield && c.kind === "creature");
    expect(bf).toHaveLength(7); // 6 bears + 1 cluster
    const cluster = bf.find((c) => c.cluster > 1);
    expect(cluster).toBeDefined();
    if (!cluster) return;
    expect(cluster).toMatchObject({ id: 10, cluster: 4, name: "Saproling" });
    expect(cluster.clusterMembers).toEqual([10, 11, 12, 13]);
    // 7 slots fit at full spacing — center-out, no pack to edges.
    const xs = bf.map((c) => c.x).sort((a, b) => a - b);
    expect(xs[0]).toBe(0);
    expect(xs[xs.length - 1]).toBe(6 * PERMANENT_STEP);
  });

  it("splits an engaged permanent out of its cluster and leaves the next free copy as the face", () => {
    // 6 unique + 4 identical Saprolings = 10 raw → overflow → Saprolings would collapse to 1 slot.
    const uniques = Array.from({ length: 6 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const saprolings = [10, 11, 12, 13].map((id) =>
      mkObject({
        id,
        name: "Saproling",
        kind: { kind: "creature", power: 1, toughness: 1 },
        power: 1,
        toughness: 1,
      }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [...uniques, ...saprolings],
    });

    const bf = layout(state, 0, new Set([10])).filter((c) => c.zone === ZONE.Battlefield);

    expect(bf).toHaveLength(8); // 6 bears + the split Saproling + a 3-member cluster
    const split = bf.find((c) => c.id === 10);
    expect(split).toMatchObject({ cluster: 0, clusterMembers: [] });
    const cluster = bf.find((c) => c.cluster > 1);
    expect(cluster).toMatchObject({ id: 11, cluster: 3 });
    expect(cluster?.clusterMembers).toEqual([11, 12, 13]);
  });

  it("gives every engaged copy its own slot rather than re-merging them", () => {
    const uniques = Array.from({ length: 6 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const saprolings = [10, 11, 12, 13].map((id) =>
      mkObject({
        id,
        name: "Saproling",
        kind: { kind: "creature", power: 1, toughness: 1 },
        power: 1,
        toughness: 1,
      }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [...uniques, ...saprolings],
    });

    const bf = layout(state, 0, new Set([10, 11])).filter((c) => c.zone === ZONE.Battlefield);

    expect(bf).toHaveLength(9); // 6 bears + 2 split Saprolings + a 2-member cluster
    expect(bf.filter((c) => c.id === 10 || c.id === 11).every((c) => c.cluster === 0)).toBe(true);
    expect(bf.find((c) => c.cluster > 1)).toMatchObject({ id: 12, cluster: 2 });
  });

  it("ignores engaged ids on a row that never clustered", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: Array.from({ length: 4 }, (_, i) =>
        mkObject({
          id: i + 1,
          name: "Saproling",
          kind: { kind: "creature", power: 1, toughness: 1 },
          power: 1,
          toughness: 1,
        }),
      ),
    });

    const bf = layout(state, 0, new Set([2])).filter((c) => c.zone === ZONE.Battlefield);

    expect(bf).toHaveLength(4);
    expect(bf.every((c) => c.cluster === 0)).toBe(true);
  });

  it("does not cluster a permanent that has an attachment stack", () => {
    // 10 creatures: 4 plain Saprolings + 1 Saproling with equipment + 5 unique = overflow.
    // Plain Saprolings cluster; equipped one stays separate.
    const plains = [1, 2, 3, 4].map((id) =>
      mkObject({
        id,
        name: "Saproling",
        kind: { kind: "creature", power: 1, toughness: 1 },
        power: 1,
        toughness: 1,
      }),
    );
    const equipped = mkObject({
      id: 5,
      name: "Saproling",
      kind: { kind: "creature", power: 1, toughness: 1 },
      power: 1,
      toughness: 1,
    });
    const sword = mkObject({ id: 50, name: "Bonesplitter", kind: { kind: "artifact" }, attached_to: 5 });
    const fillers = Array.from({ length: 5 }, (_, i) =>
      mkObject({
        id: 20 + i,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [...plains, equipped, sword, ...fillers],
    });
    const bf = layout(state, 0).filter((c) => c.zone === ZONE.Battlefield && c.kind === "creature");
    // 4 plains → 1 cluster, equipped alone, 5 bears → 7 faces
    expect(bf).toHaveLength(7);
    const cluster = bf.find((c) => c.cluster === 4);
    expect(cluster).toBeDefined();
    if (!cluster) return;
    expect(cluster.id).toBe(1);
    expect(bf.find((c) => c.id === 5)).toMatchObject({ cluster: 0 });
  });

  it("packs after clustering when slots still exceed SEAT_COLS", () => {
    // 12 distinct creatures → no clusters → pack to band.
    const creatures = Array.from({ length: 12 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const bf = layout(mkState({ players: [mkPlayer({ player: 0 })], objects: creatures }), 0).filter(
      (c) => c.zone === ZONE.Battlefield,
    );
    expect(bf.every((c) => c.cluster === 0)).toBe(true);
    expect(Math.min(...bf.map((c) => c.x))).toBe(0);
    const metrics = permanentRowMetrics(creatures.length, TEST_ROW_W);
    const rightmost = Math.max(...bf.map((c) => c.x + c.w));
    expect(rightmost + tiltedPermanentExtent(metrics.side) - metrics.side).toBeCloseTo(TEST_ROW_W);
  });

  it("clusters when keywords arrive in different order", () => {
    const uniques = Array.from({ length: 6 }, (_, i) =>
      mkObject({
        id: i + 1,
        name: `Bear ${i}`,
        kind: { kind: "creature", power: 2, toughness: 2 },
        power: 2,
        toughness: 2,
      }),
    );
    const a = mkObject({
      id: 10,
      name: "Saproling",
      kind: { kind: "creature", power: 1, toughness: 1 },
      power: 1,
      toughness: 1,
      keywords: ["trample", "haste"],
    });
    const b = mkObject({
      id: 11,
      name: "Saproling",
      kind: { kind: "creature", power: 1, toughness: 1 },
      power: 1,
      toughness: 1,
      keywords: ["haste", "trample"],
    });
    const bf = layout(mkState({ players: [mkPlayer({ player: 0 })], objects: [...uniques, a, b] }), 0).filter(
      (c) => c.zone === ZONE.Battlefield && c.kind === "creature",
    );
    expect(bf).toHaveLength(7);
    expect(bf.find((c) => c.cluster === 2)).toMatchObject({ id: 10, clusterMembers: [10, 11] });
  });

  it("sizes an attached permanent with its crowded host and keeps the host topmost", () => {
    const hostObject = mkObject({ id: 50, name: "Equipped Bear" });
    const equipmentObject = mkObject({
      id: 51,
      name: "Bonesplitter",
      kind: { kind: "artifact" },
      attached_to: hostObject.id,
    });
    const fillers = Array.from({ length: 11 }, (_, index) => mkObject({ id: index + 1, name: `Unique Bear ${index}` }));
    const cards = layout(
      mkState({ players: [mkPlayer({ player: 0 })], objects: [hostObject, equipmentObject, ...fillers] }),
      0,
    );
    const host = cards.find((card) => card.id === hostObject.id);
    const equip = cards.find((card) => card.id === equipmentObject.id);
    expect(host).toBeDefined();
    expect(equip).toBeDefined();
    if (host == null || equip == null) throw new Error("missing attachment stack");

    expect(equip).toMatchObject({ w: host.w, h: host.h });
    expect(equip.x).toBe(host.x);
    expect(Math.abs(equip.y - host.y)).toBeCloseTo(host.h * 0.2);
    expect(cards.findIndex((card) => card.id === equip.id)).toBe(cards.findIndex((card) => card.id === host.id) - 1);
    const overlapX = host.x + host.w / 2;
    const overlapY = host.y + host.h * 0.1;
    expect(hitTest({ panX: 0, panY: 0, zoom: 1 }, overlapX, overlapY, cards)).toBe(host.id);
  });

  it("keeps a cross-controller Aura in its host's shifted placement column", () => {
    const host = mkObject({
      id: 1000,
      name: "Side Bear",
      controller: 2,
      owner: 2,
      kind: { kind: "creature", power: 2, toughness: 2 },
      power: 2,
      toughness: 2,
    });
    const aura = mkObject({
      id: 1001,
      name: "Pacifism",
      controller: 0,
      owner: 0,
      kind: { kind: "enchantment" },
      attached_to: host.id,
    });
    const state = mkState({
      players: [0, 1, 2, 3].map((player) => mkPlayer({ player })),
      objects: [
        host,
        aura,
        ...Array.from({ length: 59 }, (_, index) => mkObject({ id: index + 1, name: `Unique Bear ${index}` })),
      ],
    });
    const cards = layout(state, 0);
    const byId = new Map(cards.map((c) => [c.id, c]));
    const hostCard = byId.get(host.id);
    const auraCard = byId.get(aura.id);
    expect(hostCard).toBeDefined();
    expect(auraCard).toBeDefined();
    if (hostCard == null || auraCard == null) throw new Error("missing cross-controller stack");
    expect(hostCard.x).toBeGreaterThan(boardBounds(4).maxX);
    expect(auraCard).toMatchObject({
      x: hostCard.x,
      y: hostCard.y - hostCard.h * 0.2,
      w: hostCard.w,
      h: hostCard.h,
    });
    expect(cards.findIndex((card) => card.id === aura.id)).toBe(cards.findIndex((card) => card.id === host.id) - 1);
  });

  it("allocates selectable stack depths across crowded sibling and nested attachments", () => {
    const root = mkObject({
      id: 1000,
      name: "Crowded Side Bear",
      controller: 2,
      owner: 2,
      kind: { kind: "creature", power: 2, toughness: 2 },
      power: 2,
      toughness: 2,
    });
    const equipment = mkObject({
      id: 1001,
      name: "Bonesplitter",
      controller: 0,
      owner: 0,
      kind: { kind: "artifact" },
      attached_to: root.id,
    });
    const aura = mkObject({
      id: 1002,
      name: "Artifact Ward",
      controller: 1,
      owner: 1,
      kind: { kind: "enchantment" },
      attached_to: equipment.id,
    });
    const shield = mkObject({
      id: 1005,
      name: "Shield of the Realm",
      controller: 3,
      owner: 3,
      kind: { kind: "artifact" },
      attached_to: root.id,
    });
    const cycleA = mkObject({
      id: 1003,
      name: "Looping Equipment",
      controller: 3,
      owner: 3,
      kind: { kind: "artifact" },
      attached_to: 1004,
    });
    const cycleB = mkObject({
      id: 1004,
      name: "Looping Aura",
      controller: 3,
      owner: 3,
      kind: { kind: "enchantment" },
      attached_to: cycleA.id,
    });
    const state = mkState({
      players: [0, 1, 2, 3].map((player) => mkPlayer({ player })),
      objects: [
        root,
        equipment,
        aura,
        shield,
        cycleA,
        cycleB,
        ...Array.from({ length: 59 }, (_, index) => mkObject({ id: index + 1, name: `Left Column Bear ${index}` })),
        ...Array.from({ length: 11 }, (_, index) =>
          mkObject({
            id: 2000 + index,
            controller: 2,
            owner: 2,
            name: `Side Column Bear ${index}`,
          }),
        ),
      ],
    });

    const cards = layout(state, 0);
    const rootCard = cards.find((card) => card.id === root.id);
    const equipmentCard = cards.find((card) => card.id === equipment.id);
    const auraCard = cards.find((card) => card.id === aura.id);
    const shieldCard = cards.find((card) => card.id === shield.id);
    expect(rootCard).toBeDefined();
    expect(equipmentCard).toBeDefined();
    expect(auraCard).toBeDefined();
    expect(shieldCard).toBeDefined();
    if (rootCard == null || equipmentCard == null || auraCard == null || shieldCard == null) {
      throw new Error("missing attachment subtree");
    }

    const depthStep = rootCard.h * 0.2;
    expect(rootCard.w).toBeLessThan(CARD_W);
    expect(rootCard.x).toBeGreaterThan(boardBounds(4).maxX);
    expect(equipmentCard).toMatchObject({
      x: rootCard.x,
      y: rootCard.y - 2 * depthStep,
      w: rootCard.w,
      h: rootCard.h,
    });
    expect(auraCard).toMatchObject({
      x: equipmentCard.x,
      y: rootCard.y - 3 * depthStep,
      w: equipmentCard.w,
      h: equipmentCard.h,
    });
    expect(shieldCard).toMatchObject({
      x: rootCard.x,
      y: rootCard.y - depthStep,
      w: rootCard.w,
      h: rootCard.h,
    });
    expect(new Set([auraCard.y, equipmentCard.y, shieldCard.y, rootCard.y]).size).toBe(4);
    expect(cards.findIndex((card) => card.id === aura.id)).toBe(
      cards.findIndex((card) => card.id === equipment.id) - 1,
    );
    expect(cards.findIndex((card) => card.id === equipment.id)).toBe(
      cards.findIndex((card) => card.id === shield.id) - 1,
    );
    expect(cards.findIndex((card) => card.id === shield.id)).toBe(cards.findIndex((card) => card.id === root.id) - 1);
    const identity = { panX: 0, panY: 0, zoom: 1 };
    const hitX = rootCard.x + rootCard.w / 2;
    expect(hitTest(identity, hitX, auraCard.y + depthStep / 2, cards)).toBe(aura.id);
    expect(hitTest(identity, hitX, equipmentCard.y + depthStep / 2, cards)).toBe(equipment.id);
    expect(hitTest(identity, hitX, shieldCard.y + depthStep / 2, cards)).toBe(shield.id);
    expect(hitTest(identity, hitX, rootCard.y + rootCard.h * 0.1, cards)).toBe(root.id);
    expect(cards.find((card) => card.id === cycleA.id)).toMatchObject({ w: CARD_W, h: CARD_H });
    expect(cards.find((card) => card.id === cycleB.id)).toMatchObject({ w: CARD_W, h: CARD_H });
  });

  it("renders a donated permanent under its controller's row, not its owner's (Zedruu, CR 800.4a)", () => {
    // Viewer (P0) donated a bear to P1: P0 still owns it (CR 108.3) but P1 controls it, so it must
    // render in P1's flipped creature row — not P0's — grouped by controller, badged by owner.
    const state = mkState({
      players: [mkPlayer({ player: 0 }), mkPlayer({ player: 1 })],
      objects: [
        mkObject({
          id: 1,
          name: "Donated Bear",
          owner: 0,
          controller: 1,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
      ],
    });
    const bear = layout(state, 0).find((c) => c.id === 1);
    expect(bear).toMatchObject({ y: PERMANENT_STEP + TEST_CARD_Y_IN_ROW, owner: 0, controller: 1 });
  });

  it("falls back to the Noncreature row when attached_to points at a missing host", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        mkObject({ id: 2, name: "Bonesplitter", kind: { kind: "artifact" }, attached_to: 999 }),
        mkObject({ id: 3, name: "Sol Ring", kind: { kind: "artifact" } }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    expect(byId.get(2)).toMatchObject({ x: 0, y: TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW });
    expect(byId.get(3)).toMatchObject({ x: PERMANENT_STEP, y: TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW });
  });

  it("puts unexpected WireKinds in the Noncreature left block", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        mkObject({ id: 1, name: "Weird Spell", kind: { kind: "instant" } }),
        mkObject({ id: 2, name: "Sol Ring", kind: { kind: "artifact" } }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    // Artifacts rank before other leftover kinds; both on Noncreature.
    expect(byId.get(2)).toMatchObject({ x: 0, y: TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW });
    expect(byId.get(1)).toMatchObject({ x: PERMANENT_STEP, y: TEST_BAND_STRIDE + TEST_CARD_Y_IN_ROW });
  });

  it("flips Noncreature to the centerward edge for a top-row opponent", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 }), mkPlayer({ player: 1 })],
      objects: [
        mkObject({
          id: 1,
          name: "Sol Ring",
          controller: 1,
          owner: 1,
          kind: { kind: "artifact" },
        }),
        mkObject({
          id: 2,
          name: "Forest",
          controller: 1,
          owner: 1,
          kind: { kind: "land", colors: [4] },
        }),
      ],
    });
    const byId = new Map(layout(state, 0).map((c) => [c.id, c]));
    expect(byId.get(1)?.y).toBe(2 * PERMANENT_STEP + TEST_CARD_Y_IN_ROW);
    expect(byId.get(2)?.y).toBe(TEST_CARD_Y_IN_ROW);
  });
});

describe("STEP constants", () => {
  it("has all steps defined in order", () => {
    const steps = Object.values(STEP);
    expect(steps.length).toBe(13);
    // Values should be 0, 1, 2, ..., 12 in order
    for (let i = 0; i < steps.length; i++) {
      expect(steps[i]).toBe(i);
    }
  });

  it("matches the order of STEP_NAMES", () => {
    expect(STEP.Untap).toBe(0);
    expect(STEP_NAMES[0]).toBe("Untap");
    expect(STEP.Upkeep).toBe(1);
    expect(STEP_NAMES[1]).toBe("Upkeep");
    expect(STEP.Draw).toBe(2);
    expect(STEP_NAMES[2]).toBe("Draw");
    expect(STEP.Main1).toBe(3);
    expect(STEP_NAMES[3]).toBe("Main 1");
    expect(STEP.BeginCombat).toBe(4);
    expect(STEP_NAMES[4]).toBe("Begin Combat");
    expect(STEP.DeclareAttackers).toBe(5);
    expect(STEP_NAMES[5]).toBe("Declare Attackers");
    expect(STEP.DeclareBlockers).toBe(6);
    expect(STEP_NAMES[6]).toBe("Declare Blockers");
    expect(STEP.FirstStrikeCombatDamage).toBe(7);
    expect(STEP_NAMES[7]).toBe("First Strike Damage");
    expect(STEP.CombatDamage).toBe(8);
    expect(STEP_NAMES[8]).toBe("Combat Damage");
    expect(STEP.EndCombat).toBe(9);
    expect(STEP_NAMES[9]).toBe("End Combat");
    expect(STEP.Main2).toBe(10);
    expect(STEP_NAMES[10]).toBe("Main 2");
    expect(STEP.End).toBe(11);
    expect(STEP_NAMES[11]).toBe("End");
    expect(STEP.Cleanup).toBe(12);
    expect(STEP_NAMES[12]).toBe("Cleanup");
  });
});
