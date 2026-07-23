import { describe, expect, it } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/wire/types";
import { ZONE } from "../geometry/layout";
import type { StagedAction } from "./execution";
import { emptyCostPicks } from "./execution";
import {
  pendingDamageAssignBlockers,
  pendingDamageAssignOverlay,
  pendingDivideSpellObjectIndexes,
  pendingDivideSpellOverlay,
  pendingPlayerAimOneClick,
  pendingPlayerAimOverlay,
  pendingTargetingOverlay,
  pendingTargetOneClick,
  stackAimOrigin,
  stagedPickTargets,
  stagedTargetTitle,
  stagingOverlay,
  targetMode,
} from "./targeting";

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
    players: [
      {
        player: 2,
        username: "Carol",
        life: 40,
        hand_count: 7,
        library_count: 80,
        lost: false,
        commander_tax: 0,
        mana_pool: { any: 0, colored: [0, 0, 0, 0, 0], colorless: 0 },
      },
    ],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

function staged(over: Partial<StagedAction> = {}): StagedAction {
  const card = object({ id: 5, name: "Reanimate" });
  return {
    card,
    action: action({ label: "Reanimate", object: 5, targets: [{ kind: "object", id: 9 }] }),
    picks: emptyCostPicks(),
    preferPick: false,
    playOrigin: { x: 0, y: 0 },
    playOriginScreen: { x: 0, y: 0 },
    ...over,
  };
}

describe("targetMode", () => {
  it("an action that takes no target needs no asking", () => {
    expect(targetMode(action({ needs_target: false }), state([]))).toEqual({ kind: "none" });
  });

  it("an ability that wants a target with none legal is impossible", () => {
    expect(targetMode(action({ kind: "activate", targets: [] }), state([]))).toEqual({ kind: "impossible" });
  });

  it("battlefield permanents and players are pointed at with the arrow", () => {
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

  it("a graveyard target falls back to the picker", () => {
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Grizzly Bear" });
    const mode = targetMode(action({ label: "Reanimate", targets: [{ kind: "object", id: 9 }] }), state([corpse]));
    expect(mode).toEqual({ kind: "pick", targets: [{ kind: "object", id: 9 }] });
  });

  it("a spell on the stack uses arrow aiming (stack faces are clickable)", () => {
    const spell = object({ id: 4, zone: ZONE.Stack, name: "Shock", kind: { kind: "instant" } });
    const mode = targetMode(action({ label: "Counterspell", targets: [{ kind: "object", id: 4 }] }), state([spell]));
    expect(mode.kind).toBe("arrow");
    if (mode.kind !== "arrow") throw new Error("unreachable");
    expect([...mode.objects]).toEqual([4]);
  });

  it("mixed stack and graveyard targets still use the picker", () => {
    const spell = object({ id: 4, zone: ZONE.Stack, name: "Shock", kind: { kind: "instant" } });
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Bear" });
    const mode = targetMode(
      action({
        label: "Weird",
        targets: [
          { kind: "object", id: 4 },
          { kind: "object", id: 9 },
        ],
      }),
      state([spell, corpse]),
    );
    expect(mode.kind).toBe("pick");
  });
});

describe("stagedPickTargets", () => {
  it("returns off-board targets when pick mode is required", () => {
    const corpse = object({ id: 9, zone: ZONE.Graveyard, name: "Bird" });
    const s = staged({ action: action({ label: "Reanimate", targets: [{ kind: "object", id: 9 }] }) });
    expect(stagedPickTargets(s, state([corpse]))).toEqual([{ kind: "object", id: 9 }]);
  });

  it("returns null when arrow mode and preferPick is false", () => {
    const bear = object({ id: 1 });
    const s = staged({
      action: action({ label: "Shock", targets: [{ kind: "object", id: 1 }] }),
      card: bear,
      preferPick: false,
    });
    expect(stagedPickTargets(s, state([bear]))).toBeNull();
  });

  it("returns all legal targets when preferPick is true after a cost dialog", () => {
    const bear = object({ id: 1 });
    const s = staged({
      action: action({
        label: "Shock",
        targets: [
          { kind: "object", id: 1 },
          { kind: "player", player: 2 },
        ],
      }),
      card: bear,
      preferPick: true,
    });
    expect(stagedPickTargets(s, state([bear]))).toEqual([
      { kind: "object", id: 1 },
      { kind: "player", player: 2 },
    ]);
  });
});

describe("stackAimOrigin", () => {
  it("anchors the staged spell ghost at the right-edge stack pile center", () => {
    const origin = stackAimOrigin(1440, 900, 2);
    expect(origin.x).toBe(1440 - 16 - 180 / 2);
    expect(origin.y).toBeCloseTo(900 / 2 - 34 / 2);
  });
});

describe("stagingOverlay", () => {
  it("highlights legal battlefield targets while arrow-aiming", () => {
    const bear = object({ id: 1 });
    const s = staged({
      action: action({
        label: "Shock",
        targets: [
          { kind: "object", id: 1 },
          { kind: "player", player: 2 },
        ],
      }),
      card: bear,
      preferPick: false,
    });
    const overlay = stagingOverlay(s, state([bear]), { width: 1440, height: 900 }, 0);
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects]).toEqual([1]);
    expect([...overlay.targetPlayers]).toEqual([2]);
    expect(overlay.aimFrom).not.toBeNull();
  });

  it("does not aim when preferPick forces the target picker", () => {
    const bear = object({ id: 1 });
    const s = staged({
      action: action({ label: "Shock", targets: [{ kind: "object", id: 1 }] }),
      card: bear,
      preferPick: true,
    });
    const overlay = stagingOverlay(s, state([bear]), { width: 1440, height: 900 }, 0);
    expect(overlay.aiming).toBe(false);
    expect(overlay.targetObjects.size).toBe(0);
    expect(overlay.aimFrom).toBeNull();
  });

  it("returns idle overlay when nothing is staged", () => {
    const overlay = stagingOverlay(null, state([]), { width: 800, height: 600 }, 2);
    expect(overlay).toEqual({
      aiming: false,
      targetObjects: new Set(),
      targetPlayers: new Set(),
      aimFrom: null,
    });
  });
});

describe("pendingTargetingOverlay", () => {
  it("aims when choose_target max=1 and all items are on the battlefield", () => {
    const bear = object({ id: 7 });
    const overlay = pendingTargetingOverlay(
      {
        kind: "choose_target",
        label: "Target creature",
        max: 1,
        optional: false,
        player: 0,
        source: 1,
        items: [{ id: 7, label: "Bear" }],
      },
      state([bear]),
      { width: 1440, height: 900 },
      0,
    );
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects]).toEqual([7]);
  });

  it("aims for multi-target choose_target when all items are on the battlefield", () => {
    const a = object({ id: 1 });
    const b = object({ id: 2 });
    const overlay = pendingTargetingOverlay(
      {
        kind: "choose_target",
        label: "Target creatures",
        max: 2,
        optional: false,
        player: 0,
        source: 1,
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
      },
      state([a, b]),
      { width: 1440, height: 900 },
      0,
    );
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects].sort()).toEqual([1, 2]);
  });

  it("stays idle when a legal item is off the battlefield", () => {
    const gy = object({ id: 9, zone: 3 });
    const overlay = pendingTargetingOverlay(
      {
        kind: "choose_target",
        label: "Target card",
        max: 1,
        optional: false,
        player: 0,
        source: 1,
        items: [{ id: 9, label: "Dead" }],
      },
      state([gy]),
      { width: 1440, height: 900 },
      0,
    );
    expect(overlay.aiming).toBe(false);
  });
});

describe("pendingTargetOneClick", () => {
  it("is true only for max=1 choose_target", () => {
    expect(
      pendingTargetOneClick({
        kind: "choose_target",
        label: "T",
        max: 1,
        optional: false,
        player: 0,
        source: 1,
        items: [{ id: 1, label: "A" }],
      }),
    ).toBe(true);
    expect(
      pendingTargetOneClick({
        kind: "choose_target",
        label: "T",
        max: 2,
        optional: false,
        player: 0,
        source: 1,
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
      }),
    ).toBe(false);
  });
});

describe("pendingDamageAssignOverlay", () => {
  it("highlights battlefield blockers for assign_combat_damage", () => {
    const attacker = object({ id: 9, name: "Atk", power: 4 });
    const bear = object({ id: 4, name: "Bear", controller: 1 });
    const elf = object({ id: 5, name: "Elf", controller: 1 });
    const overlay = pendingDamageAssignOverlay(
      {
        kind: "assign_combat_damage",
        items: [
          { id: 4, label: "Bear" },
          { id: 5, label: "Elf" },
        ],
        player: 0,
        source: 9,
      },
      state([attacker, bear, elf]),
    );
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects].sort()).toEqual([4, 5]);
    expect(overlay.aimFrom).toBeNull();
  });

  it("is idle when a blocker is off the battlefield", () => {
    const blockers = pendingDamageAssignBlockers(
      {
        kind: "assign_combat_damage",
        items: [{ id: 4, label: "Bear" }],
        player: 0,
        source: 9,
      },
      state([object({ id: 4, zone: ZONE.Graveyard })]),
    );
    expect(blockers).toBeNull();
  });

  it("highlights battlefield permanents for divide_counters", () => {
    const wolf = object({ id: 12, name: "Wolf" });
    const cat = object({ id: 13, name: "Cat" });
    const overlay = pendingDamageAssignOverlay(
      {
        kind: "divide_counters",
        items: [
          { id: 12, label: "Wolf" },
          { id: 13, label: "Cat" },
        ],
        player: 0,
        spell: 77,
        total: 2,
      },
      state([wolf, cat]),
    );
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects].sort()).toEqual([12, 13]);
  });
});

describe("pendingDivideSpellOverlay", () => {
  it("highlights battlefield spell-damage targets with item indexes", () => {
    const bear = object({ id: 21, name: "Bear", controller: 1 });
    const elf = object({ id: 22, name: "Elf", controller: 1 });
    const pc = {
      kind: "divide_spell_damage" as const,
      items: [
        { id: 21, label: "Bear" },
        { id: 22, label: "Elf" },
      ],
      player: 0,
      spell: 99,
      total: 3,
    };
    const indexes = pendingDivideSpellObjectIndexes(pc, state([bear, elf]));
    expect(indexes).not.toBeNull();
    if (indexes == null) throw new Error("expected divide indexes");
    expect([...indexes.entries()].sort(([a], [b]) => a - b)).toEqual([
      [21, 0],
      [22, 1],
    ]);
    const overlay = pendingDivideSpellOverlay(pc, state([bear, elf]));
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetObjects].sort()).toEqual([21, 22]);
    expect(overlay.aimFrom).toBeNull();
  });

  it("is idle when any target is a player or off the battlefield", () => {
    expect(
      pendingDivideSpellObjectIndexes(
        {
          kind: "divide_spell_damage",
          items: [
            { id: 21, label: "Bear" },
            { id: 0, label: "Bob", player: 1 },
          ],
          player: 0,
          spell: 99,
          total: 3,
        },
        state([object({ id: 21, name: "Bear" })]),
      ),
    ).toBeNull();
    expect(
      pendingDivideSpellObjectIndexes(
        {
          kind: "divide_spell_damage",
          items: [{ id: 21, label: "Bear" }],
          player: 0,
          spell: 99,
          total: 3,
        },
        state([object({ id: 21, zone: ZONE.Graveyard })]),
      ),
    ).toBeNull();
  });
});

describe("pendingPlayerAimOverlay", () => {
  it("highlights player seats for choose_target_players", () => {
    const overlay = pendingPlayerAimOverlay(
      {
        kind: "choose_target_players",
        label: "Choose opponents",
        min: 1,
        max: 2,
        player: 0,
        source: 1,
        items: [
          { id: 0, label: "Alice", player: 1 },
          { id: 1, label: "Bob", player: 2 },
        ],
      },
      state([]),
    );
    expect(overlay.aiming).toBe(true);
    expect([...overlay.targetPlayers].sort()).toEqual([1, 2]);
  });

  it("is one-click when max is 1", () => {
    expect(
      pendingPlayerAimOneClick({
        kind: "choose_target_players",
        label: "Choose a player",
        min: 1,
        max: 1,
        player: 0,
        source: 1,
        items: [{ id: 0, label: "Bob", player: 1 }],
      }),
    ).toBe(true);
  });
});

describe("stagedTargetTitle", () => {
  it("names activate abilities separately from the source card", () => {
    const card = object({ id: 3, name: "Spirebluff Canal" });
    const s = staged({
      card,
      action: action({ kind: "activate", label: "Loot", object: 3, targets: [] }),
    });
    expect(stagedTargetTitle(s)).toBe("Loot — Spirebluff Canal");
  });

  it("uses the action label for casts", () => {
    expect(stagedTargetTitle(staged())).toBe("Reanimate");
  });
});
