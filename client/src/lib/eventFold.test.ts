import { describe as suite, expect, it } from "vitest";
import { describe, extractProvenance } from "~/lib/eventFold";
import { ZONE } from "~/layout";
import type { ObjectView, VisibleEvent, VisibleState } from "~/wire/types";

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

function mkState(objects: ObjectView[] = []): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects,
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 0,
    viewer: 0,
  };
}

suite("extractProvenance", () => {
  it("maps a moved card's new id to the id it came from", () => {
    const { moves } = extractProvenance(
      [{ kind: "moved_to_graveyard", card: 50, from: 12 }],
      new Set(),
      0,
    );
    expect(moves.get(50)).toBe(12);
  });

  it("marks permanent_entered as from-stack and records the move", () => {
    const { moves, fromStack } = extractProvenance(
      [
        { kind: "permanent_entered", permanent: 60, from: 30 },
        { kind: "moved_to_graveyard", card: 50, from: 12 },
      ],
      new Set(),
      0,
    );
    expect(moves.get(60)).toBe(30);
    expect(fromStack.has(60)).toBe(true);
    expect(fromStack.has(50)).toBe(false);
  });

  it("records land_played permanent → hand card", () => {
    const { landPlays } = extractProvenance(
      [{ kind: "land_played", from: 9, permanent: 3, player: 0 }],
      new Set(),
      0,
    );
    expect(landPlays.get(3)).toBe(9);
  });

  it("marks stack exits when the predecessor was on the prior stack", () => {
    const { fromStackExit } = extractProvenance(
      [{ kind: "moved_to_graveyard", card: 50, from: 30 }],
      new Set([30]),
      0,
    );
    expect(fromStackExit.has(50)).toBe(true);
  });

  it("ignores bookkeeping-only events", () => {
    const out = extractProvenance([{ kind: "priority_passed", player: 0 }], new Set(), 0);
    expect(out.moves.size).toBe(0);
    expect(out.fromStack.size).toBe(0);
  });
});

suite("describe", () => {
  const bolt = mkObject({ id: 1, name: "Lightning Bolt" });
  const bear = mkObject({ id: 2, name: "Grizzly Bears" });
  const forest = mkObject({ id: 3, name: "Forest" });

  it("spell_cast, with and without a target", () => {
    expect(
      describe(
        { kind: "spell_cast", controller: 0, from: 9, spell: 1, escape: false, flashback: false },
        mkState([bolt]),
      ),
    ).toBe("P0 casts Lightning Bolt");
    expect(
      describe(
        {
          kind: "spell_cast",
          controller: 0,
          from: 9,
          spell: 1,
          escape: false,
          flashback: false,
          target: { kind: "object", id: 2 },
        },
        mkState([bolt, bear]),
      ),
    ).toBe("P0 casts Lightning Bolt → Grizzly Bears");
  });

  it("land_played", () => {
    expect(describe({ kind: "land_played", from: 9, permanent: 3, player: 1 }, mkState([forest]))).toBe(
      "P1 plays Forest",
    );
  });

  it("returns null for bookkeeping-only kinds", () => {
    expect(describe({ kind: "step_began", active_player: 0, step: 3 }, mkState())).toBeNull();
    expect(describe({ kind: "priority_passed", player: 0 }, mkState())).toBeNull();
  });

  it("falls back to a #id label for a missing object", () => {
    expect(describe({ kind: "damage_marked", amount: 1, object: 999 }, mkState())).toBe("#999 takes 1");
  });

  it("player_lost / drew_from_empty_library", () => {
    expect(describe({ kind: "drew_from_empty_library", player: 2 }, mkState())).toBe(
      "P2 tries to draw from an empty library",
    );
    expect(describe({ kind: "player_lost", player: 2 }, mkState())).toBe("P2 loses the game");
  });

  it("revealed_top_of_library uses def", () => {
    expect(describe({ kind: "revealed_top_of_library", card: 77, def: "Sol Ring", player: 0 }, mkState())).toBe(
      "P0 reveals Sol Ring",
    );
  });

  it("permanent_entered / damage_marked / life_changed", () => {
    expect(describe({ kind: "permanent_entered", from: 9, permanent: 2 }, mkState([bear]))).toBe(
      "Grizzly Bears enters",
    );
    expect(describe({ kind: "damage_marked", amount: 3, object: 2, source: 1 }, mkState([bolt, bear]))).toBe(
      "Grizzly Bears takes 3 from Lightning Bolt",
    );
    expect(describe({ kind: "life_changed", amount: -3, player: 0 }, mkState())).toBe("P0 loses 3 life");
  });

  it("card_drawn names the card when known", () => {
    expect(describe({ kind: "card_drawn", from: 9, object: 1, player: 1, card: "Shock" }, mkState())).toBe(
      "P1 draws Shock",
    );
    expect(describe({ kind: "card_drawn", from: 9, object: 1, player: 1, card: null }, mkState())).toBe(
      "P1 draws a card",
    );
  });
});
