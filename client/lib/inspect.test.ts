import { describe, expect, it } from "vitest";
import { commanderDamageBreakdown, type InspectPin, inspectPinChanged, pinFromPlayer } from "./inspect";
import type { ObjectView, PlayerView } from "./wire/types";

function player(seat: number, overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    commander_tax: 0,
    hand_count: 7,
    library_count: 80,
    life: 40,
    lost: false,
    mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
    player: seat,
    username: `P${seat}`,
    ...overrides,
  };
}

function object(id: number, overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 1 },
    marked_damage: 0,
    name: `Card ${id}`,
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 2,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 2,
    zone: 2,
    ...overrides,
  };
}

describe("commanderDamageBreakdown", () => {
  it("returns empty when commander_damage is absent or empty", () => {
    expect(commanderDamageBreakdown(player(0), [player(0), player(1)], [])).toEqual([]);
    expect(commanderDamageBreakdown(player(0, { commander_damage: [] }), [player(0), player(1)], [])).toEqual([]);
  });

  it("labels each source by owner username and amount /21", () => {
    const victim = player(0, {
      username: "Alice",
      commander_damage: [
        { from: 1, amount: 14 },
        { from: 2, amount: 7 },
      ],
    });
    const players = [victim, player(1, { username: "Bob" }), player(2, { username: "Carol" })];
    expect(commanderDamageBreakdown(victim, players, [])).toEqual([
      { fromSeat: 1, label: "Bob", amount: 14, text: "Bob: 14 / 21" },
      { fromSeat: 2, label: "Carol", amount: 7, text: "Carol: 7 / 21" },
    ]);
  });

  it("appends visible commander name when an is_commander object is owned by the source seat", () => {
    const victim = player(0, {
      commander_damage: [{ from: 1, amount: 14 }],
    });
    const players = [victim, player(1, { username: "Bob" })];
    const objects = [object(9, { owner: 1, controller: 1, is_commander: true, name: "Atraxa, Praetors' Voice" })];
    expect(commanderDamageBreakdown(victim, players, objects)).toEqual([
      {
        fromSeat: 1,
        label: "Bob — Atraxa, Praetors' Voice",
        amount: 14,
        text: "Bob — Atraxa, Praetors' Voice: 14 / 21",
      },
    ]);
  });
});

describe("pinFromPlayer", () => {
  it("returns null when Alt is not held or seat is missing", () => {
    expect(pinFromPlayer(false, 1, player(1, { username: "Bob" }))).toBeNull();
    expect(pinFromPlayer(true, null, player(1))).toBeNull();
    expect(pinFromPlayer(true, 1, null)).toBeNull();
  });

  it("builds a player inspect pin from the life-orb seat", () => {
    expect(pinFromPlayer(true, 1, player(1, { username: "Bob" }))).toEqual({
      name: "Bob",
      prepared: false,
      playerSeat: 1,
    });
  });
});

describe("inspectPinChanged", () => {
  it("treats a different playerSeat as a pin change", () => {
    const prev: InspectPin = { name: "Bob", prepared: false, playerSeat: 1 };
    const next: InspectPin = { name: "Carol", prepared: false, playerSeat: 2 };
    expect(inspectPinChanged(prev, next)).toBe(true);
    expect(inspectPinChanged(prev, { ...prev })).toBe(false);
  });
});
