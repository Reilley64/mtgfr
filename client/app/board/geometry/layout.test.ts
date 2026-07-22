import { describe, expect, it } from "vitest";
import { emptyManaPool } from "~/manaPips";
import type { ObjectView, PlayerView, VisibleState } from "~/wire/types";
import {
  AVATAR_R,
  avatarPos,
  boardBounds,
  layout,
  manaTrayPos,
  STEP,
  STEP_NAMES,
  seatBand,
  ZONE,
} from "./layout";

// Geometry constants mirrored from layout.ts (CARD_W=96, CARD_H=134, GAP=8, AVATAR_R=40):
// STEP=104, ROW_H=142, BATTLE_H=426, BAND_GAP=8, BAND_STRIDE=434, COL_X=-64, COL_STRIDE=106.5.
// Quadrant grid: SEAT_COLS=7, SEAT_STRIDE_X=896 (column 1 origin), BAND_W=800.
// ATTACH_OFFSET = CARD_H * 0.2 = 26.8.

function mkObject(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 0,
    is_commander: false,
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
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
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

describe("seatBand", () => {
  // 2×2 quadrant: you bottom-left (col 0, row 1 → band y 426); front above you (col 0, row 0 →
  // y -8); side beside you (col 1, row 1 → x 824); diagonal top-right (col 1, row 0).
  it("puts the viewer's own band at the bottom-left of a 4-player table", () => {
    expect(seatBand(0, 0, 4)).toEqual({ x: -72, y: 426, w: 800, h: 434 });
  });

  it("puts the seat after the viewer directly in front (top-left)", () => {
    expect(seatBand(1, 0, 4)).toEqual({ x: -72, y: -8, w: 800, h: 434 });
  });

  it("puts the next seat to the side (bottom-right) and the last diagonal (top-right)", () => {
    expect(seatBand(2, 0, 4)).toMatchObject({ x: 824, y: 426 }); // side, beside you
    expect(seatBand(3, 0, 4)).toMatchObject({ x: 824, y: -8 }); // diagonal
  });

  it("assigns quadrants by turn order regardless of which seat is the viewer", () => {
    // Viewer is seat 2: turn order after them is 3 (front), 0 (side), 1 (diagonal).
    expect(seatBand(2, 2, 4)).toMatchObject({ x: -72, y: 426 }); // self, bottom-left
    expect(seatBand(3, 2, 4)).toMatchObject({ x: -72, y: -8 }); // front
    expect(seatBand(0, 2, 4)).toMatchObject({ x: 824, y: 426 }); // side
    expect(seatBand(1, 2, 4)).toMatchObject({ x: 824, y: -8 }); // diagonal
  });
});

describe("avatarPos", () => {
  it("sits below the viewer's own bottom-left band", () => {
    expect(avatarPos(0, 0, 4)).toEqual({ x: 328, y: 908 });
  });

  it("sits above the flipped front seat's band", () => {
    expect(avatarPos(1, 0, 4)).toEqual({ x: 328, y: -48 });
  });

  it("sits below the upright side seat and above the flipped diagonal", () => {
    expect(avatarPos(2, 0, 4)).toEqual({ x: 1224, y: 908 }); // side, avatar below
    expect(avatarPos(3, 0, 4)).toEqual({ x: 1224, y: -48 }); // diagonal, avatar above
  });
});

describe("manaTrayPos", () => {
  // Past the zone column (COL_X + COL_W + GAP), just outside the seat band on the outer edge.
  it("sits under the zone column below the viewer's upright band", () => {
    const band = seatBand(0, 0, 4);
    const tray = manaTrayPos(0, 0, 4);
    expect(tray).toEqual({ x: -8, y: 868 });
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
    expect(manaTrayPos(2, 0, 4)).toEqual({ x: 888, y: 868 });
    expect(manaTrayPos(3, 0, 4)).toEqual({ x: 888, y: -16 });
  });
});

describe("boardBounds", () => {
  // A 2-player table is a single (left) column; 3 and 4 players both span both columns, so their
  // bounds match (the 3p table just leaves the diagonal cell empty).
  it("fits a 2-player table (one column)", () => {
    expect(boardBounds(2)).toEqual({ minX: -72, minY: -88, maxX: 728, maxY: 948 });
  });

  it("fits a 3-player table (both columns, no diagonal)", () => {
    expect(boardBounds(3)).toEqual({ minX: -72, minY: -88, maxX: 1624, maxY: 948 });
  });

  it("fits a 4-player table (full 2×2)", () => {
    expect(boardBounds(4)).toEqual({ minX: -72, minY: -88, maxX: 1624, maxY: 948 });
  });
});

describe("layout", () => {
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

    // Viewer (o.y=434): Noncreature / Creatures / Lands at 434 / 576 / 718. Lone card centers
    // on the row: (SEAT_COLS - 1)/2 * CARD_HSTEP = 3 * 104 = 312.
    expect(byId.get(1)).toMatchObject({ x: 312, y: 576, w: 96, h: 134, zone: ZONE.Battlefield });
    expect(byId.get(2)).toMatchObject({ x: 312, y: 718, w: 96, h: 134 });
    // Zone column top -> bottom for the viewer: commander, deck (no exile), graveyard.
    // COL_STRIDE = 106.5 → commander@434, deck@647, graveyard@753.5.
    expect(byId.get(3)).toMatchObject({ x: -64, y: 434, w: 48, h: 67, pile: 0 });
    expect(byId.get(4)).toMatchObject({ x: -64, y: 753.5, w: 48, h: 67, pile: 1, zone: ZONE.Graveyard });

    // Opponent (o.y=0, flipped): Creatures at o.y+ROW_H = 142.
    expect(byId.get(5)).toMatchObject({ x: 312, y: 142, w: 96, h: 134 });

    // Opponent's library placeholder is the only zone-column card (synthetic id -1 - owner = -2),
    // and it lands at the flipped column's second slot (deck is index 1 once reversed).
    const opponentDeck = cards.find((c) => c.id === -2);
    expect(opponentDeck).toMatchObject({ x: -64, y: 106.5, w: 48, h: 67, pile: 25, faceDown: true });

    // Viewer's own library placeholder is the third slot (index 2) in the unreversed column.
    const viewerDeck = cards.find((c) => c.id === -1);
    expect(viewerDeck).toMatchObject({ x: -64, y: 647, w: 48, h: 67, pile: 30, faceDown: true });

    expect(cards).toHaveLength(7);
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
    // Viewer o.y = BAND_STRIDE (1p still uses bottom-left cell).
    expect(byId.get(1)?.y).toBe(434); // Noncreature
    expect(byId.get(2)?.y).toBe(434); // Noncreature
    expect(byId.get(3)?.y).toBe(576); // Creatures
    expect(byId.get(4)?.y).toBe(718); // Lands
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
    // Left block: artifacts then enchantments from x=0,1,2 * 104.
    expect(byId.get(1)?.x).toBe(0);
    expect(byId.get(2)?.x).toBe(104);
    expect(byId.get(3)?.x).toBe(208);
    // Single PW right-aligned: slot SEAT_COLS-1 → 6 * 104 = 624.
    expect(byId.get(4)?.x).toBe(624);
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
    // 11 slots → packed into [0, SEAT_RIGHT - CARD_W] = [0, 632]; no spill past the seat.
    expect(Math.min(...xs)).toBe(0);
    expect(Math.max(...xs)).toBe(632);
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
    expect(xs[xs.length - 1]).toBe(632);
    // Even center-out packing: equal steps across the band.
    const step = (632 - 0) / 11;
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
    expect(xs[xs.length - 1]).toBe(624); // (SEAT_COLS-1) * CARD_HSTEP
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
    expect(Math.max(...bf.map((c) => c.x))).toBe(632);
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

  it("stacks attached equipment under the host, not in the Noncreature row", () => {
    const state = mkState({
      players: [mkPlayer({ player: 0 })],
      objects: [
        mkObject({
          id: 1,
          name: "Bear",
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({ id: 2, name: "Bonesplitter", kind: { kind: "artifact" }, attached_to: 1 }),
        mkObject({ id: 3, name: "Sol Ring", kind: { kind: "artifact" } }),
      ],
    });
    const cards = layout(state, 0);
    const byId = new Map(cards.map((c) => [c.id, c]));
    const host = byId.get(1);
    const equip = byId.get(2);
    const ring = byId.get(3);
    expect(host).toBeDefined();
    expect(equip).toBeDefined();
    expect(ring).toBeDefined();
    if (!host || !equip || !ring) return;

    expect(host.y).toBe(576); // Creatures row
    expect(ring.y).toBe(434); // Noncreature row
    expect(ring.x).toBe(0); // left-aligned alone
    // Attachment centerward of host (smaller Y when upright), same X; under host in array order.
    expect(equip.x).toBe(host.x);
    expect(equip.y).toBe(host.y - 26.8);
    expect(cards.findIndex((c) => c.id === 2)).toBeLessThan(cards.findIndex((c) => c.id === 1));
  });

  it("stacks a cross-controller Aura on the opponent's host", () => {
    // Viewer enchants the opponent's bear — Aura stays under P0's control but renders on P1's seat.
    const state = mkState({
      players: [mkPlayer({ player: 0 }), mkPlayer({ player: 1 })],
      objects: [
        mkObject({
          id: 1,
          name: "Opposing Bear",
          controller: 1,
          owner: 1,
          kind: { kind: "creature", power: 2, toughness: 2 },
          power: 2,
          toughness: 2,
        }),
        mkObject({
          id: 2,
          name: "Pacifism",
          controller: 0,
          owner: 0,
          kind: { kind: "enchantment" },
          attached_to: 1,
        }),
      ],
    });
    const cards = layout(state, 0);
    const byId = new Map(cards.map((c) => [c.id, c]));
    const host = byId.get(1);
    const aura = byId.get(2);
    expect(host).toBeDefined();
    expect(aura).toBeDefined();
    if (!host || !aura) return;
    // Flipped opponent creature at y=142; Aura centerward (+ATTACH_OFFSET when flipped).
    expect(host).toMatchObject({ x: 312, y: 142 });
    expect(aura).toMatchObject({ x: host.x, y: host.y + 26.8 });
    expect(cards.findIndex((c) => c.id === 2)).toBeLessThan(cards.findIndex((c) => c.id === 1));
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
    // P1's flipped creature row sits at y=142 (see the opponent-bear case above), NOT P0's row.
    expect(bear).toMatchObject({ y: 142, owner: 0, controller: 1 });
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
    expect(byId.get(2)).toMatchObject({ x: 0, y: 434 }); // left block, Noncreature
    expect(byId.get(3)).toMatchObject({ x: 104, y: 434 });
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
    expect(byId.get(2)).toMatchObject({ x: 0, y: 434 });
    expect(byId.get(1)).toMatchObject({ x: 104, y: 434 });
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
    // Flipped o.y=0: Noncreature at 284, Lands at 0.
    expect(byId.get(1)?.y).toBe(284);
    expect(byId.get(2)?.y).toBe(0);
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
