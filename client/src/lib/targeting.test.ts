import { describe, expect, it } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/api/generated";
import { ZONE } from "~/layout";
import { targetMode } from "~/lib/targeting";

function object(over: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 1,
    is_commander: false,
    kind: { kind: "creature", power: 2, toughness: 2 },
    mana_cost: { generic: 1, colored: [0, 0, 0, 0, 0] },
    marked_damage: 0,
    name: "Grizzly Bear",
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

function action(over: Partial<ActionView> = {}): ActionView {
  return { id: 7, kind: "cast", label: "Shock", needs_target: true, section: "hand", ...over };
}

function state(objects: ObjectView[]): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects,
    players: [],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

describe("targetMode", () => {
  it("an action that takes no target needs no asking", () => {
    expect(targetMode(action({ needs_target: false }), state([]))).toEqual({ kind: "none" });
  });

  it("an ability that wants a target with none legal is impossible", () => {
    // The cast gate never offers such a spell, but `meaningful_actions` offers abilities without
    // checking — so this is reachable, and the board must say so rather than send a doomed intent.
    expect(targetMode(action({ kind: "activate", targets: [] }), state([]))).toEqual({ kind: "impossible" });
  });

  it("battlefield permanents and players are pointed at with the arrow", () => {
    // Shock is "any target": every battlefield creature and every living player.
    const bear = object({ id: 1 });
    const mode = targetMode(
      action({
        targets: [
          { kind: "object", id: 1 },
          { kind: "player", player: 0 },
          { kind: "player", player: 2 },
        ],
      }),
      state([bear]),
    );
    expect(mode.kind).toBe("arrow");
    if (mode.kind !== "arrow") throw new Error("unreachable");
    expect([...mode.objects]).toEqual([1]);
    expect([...mode.players]).toEqual([0, 2]);
  });

  it("a graveyard target falls back to the picker — a pile card can't be clicked", () => {
    // Reanimate: the legal targets are creature cards in graveyards, which the board collapses into
    // a single pile card, so there is nothing to point the arrow at.
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Grizzly Bear" });
    const mode = targetMode(action({ label: "Reanimate", targets: [{ kind: "object", id: 9 }] }), state([corpse]));
    expect(mode).toEqual({ kind: "pick", targets: [{ kind: "object", id: 9 }] });
  });

  it("a spell on the stack falls back to the picker — the stack is a DOM overlay", () => {
    // Counterspell: the stack renders outside the canvas, so its objects are unpointable.
    const spell = object({ id: 4, zone: ZONE.Stack, name: "Shock", kind: { kind: "instant" } });
    const mode = targetMode(action({ label: "Counterspell", targets: [{ kind: "object", id: 4 }] }), state([spell]));
    expect(mode).toEqual({ kind: "pick", targets: [{ kind: "object", id: 4 }] });
  });

  it("a picker keeps player targets — nothing is silently dropped from the choice", () => {
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Bird" });
    const mode = targetMode(
      action({
        targets: [
          { kind: "object", id: 9 },
          { kind: "player", player: 2 },
        ],
      }),
      state([corpse]),
    );
    expect(mode).toEqual({
      kind: "pick",
      targets: [
        { kind: "object", id: 9 },
        { kind: "player", player: 2 },
      ],
    });
  });

  it("one off-board target sends the whole choice to the picker", () => {
    // Mixed sets must not offer half the targets on the arrow and hide the rest.
    const bear = object({ id: 1 });
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Bird" });
    const mode = targetMode(
      action({
        targets: [
          { kind: "object", id: 1 },
          { kind: "object", id: 9 },
        ],
      }),
      state([bear, corpse]),
    );
    expect(mode).toEqual({
      kind: "pick",
      targets: [
        { kind: "object", id: 1 },
        { kind: "object", id: 9 },
      ],
    });
  });
});
