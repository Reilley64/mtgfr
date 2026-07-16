import { describe, expect, it } from "vitest";
import type { ActionView, ObjectView, VisibleState } from "~/api/generated";
import {
  mayYesNoTitle,
  payCostTitle,
  payEchoTitle,
  payOrCounterTitle,
  pendingTargetTitle,
  sourceHint,
  spellTargetsTitle,
  stagedTargetHint,
  stagedTargetTitle,
} from "~/lib/targetPrompt";

const state = (objects: ObjectView[]): VisibleState => ({
  objects,
  players: [],
  active_player: 0,
  priority: 0,
  step: 0,
  viewer: 0,
  can_act: true,
  combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
  stack: [],
});

const card = (id: number, name: string): ObjectView =>
  ({
    id,
    name,
    controller: 0,
    owner: 0,
    zone: 0,
    kind: { kind: "creature", power: 1, toughness: 1 },
    mana_cost: { generic: 0, colored: [0, 0, 0, 0, 0] },
    needs_target: false,
    tapped: false,
    summoning_sick: false,
    has_haste: false,
    power: 1,
    toughness: 1,
    plus_counters: 0,
    marked_damage: 0,
    is_commander: false,
  }) as ObjectView;

const action = (overrides: Partial<ActionView> & Pick<ActionView, "label">): ActionView =>
  ({
    id: 1,
    kind: "cast",
    section: "hand",
    needs_target: true,
    ...overrides,
  }) as ActionView;

describe("targetPrompt", () => {
  it("names the source permanent when it differs from the effect label", () => {
    const s = state([card(7, "Blood Artist")]);
    expect(pendingTargetTitle(s, "Deal 1 damage to any target", 7)).toBe("Deal 1 damage to any target — Blood Artist");
    expect(sourceHint(s, 7, "Deal 1 damage to any target")).toBe("Blood Artist");
  });

  it("omits the source subtitle when the label already is the card name", () => {
    const s = state([card(7, "Lightning Bolt")]);
    expect(pendingTargetTitle(s, "Lightning Bolt", 7)).toBe("Lightning Bolt");
    expect(sourceHint(s, 7, "Lightning Bolt")).toBeUndefined();
  });

  it("formats multi-target spell titles from min/max", () => {
    expect(spellTargetsTitle("Lightning Bolt", 1, 1)).toBe("Lightning Bolt: Choose 1 target");
    expect(spellTargetsTitle("Twinflame", 1, 99)).toBe("Twinflame: Choose 1 or more targets");
    expect(spellTargetsTitle("Aether Gale", 6, 6)).toBe("Aether Gale: Choose 6 targets");
  });

  it("uses the ability label for staged activations", () => {
    const staged = {
      card: card(3, "Mind Stone"),
      action: action({ kind: "activate", label: "Draw a card" }),
    };
    expect(stagedTargetTitle(staged)).toBe("Draw a card — Mind Stone");
    expect(stagedTargetHint(staged)).toBe("Targeting for Draw a card — Mind Stone");
  });

  it("uses the spell name for staged casts", () => {
    const staged = {
      card: card(9, "Counterspell"),
      action: action({ kind: "cast", label: "Counterspell" }),
    };
    expect(stagedTargetTitle(staged)).toBe("Counterspell");
  });

  it("titles an optional paid trigger with source, cost, and payoff", () => {
    expect(payCostTitle("Trudge Garden", "{2}", "Create 1 Fungus Beast token(s)")).toBe(
      "Trudge Garden: pay {2} to Create 1 Fungus Beast token(s)?",
    );
  });

  it("titles an optional may trigger with source and payoff", () => {
    expect(mayYesNoTitle("Blood Artist", "Each opponent loses 1 life")).toBe(
      "Blood Artist: Each opponent loses 1 life?",
    );
  });

  it("titles pay-or-counter and echo prompts with the named permanent/spell", () => {
    expect(payOrCounterTitle("Lightning Bolt", "{2}")).toBe("Pay {2} or Lightning Bolt is countered?");
    expect(payEchoTitle("Avalanche Riders", "{3} R")).toBe("Avalanche Riders: pay echo {3} R or sacrifice it?");
  });
});
