import { describe, expect, test } from "vitest";
import {
  answerFromDraft,
  assertAllKindsRegistered,
  buildAnswerFromDraft,
  cardPickReady,
  choiceDraftKey,
  choiceIntent,
  clickDamageAssign,
  damageAssignReady,
  declineAnswer,
  FORMULATOR_FOR_KIND,
  nextDistributeBucket,
  type PromptDraft,
} from "~/choice";
import type { ObjectView, PendingChoiceView, VisibleState, WireIntent, WireModeChoice } from "~/wire/types";

const ALL_PENDING_CHOICE_KINDS = [
  "order_triggers",
  "choose_target",
  "choose_spell_targets",
  "choose_target_players",
  "may_yes_no",
  "decline_untap",
  "pay_cost",
  "pay_or_counter",
  "pay_or_controller_draws",
  "choose_countered_spell_destination",
  "pay_echo_or_sacrifice",
  "pay_recover_or_exile",
  "sacrifice_unless_pay",
  "sacrifice_unless_return_land",
  "assign_combat_damage",
  "divide_spell_damage",
  "divide_counters",
  "scry",
  "surveil",
  "search_library",
  "select_from_top",
  "distribute_top",
  "shuffle_from_graveyard",
  "sacrifice_edict",
  "proliferate",
  "phase_out",
  "choose_ability_targets",
  "choose_activation_cost_targets",
  "may_sacrifice",
  "choose_own_sacrifices",
  "devour",
  "exile_from_graveyard",
  "caster_keep_permanents",
  "choose_counter_target_for_player",
  "may_return_from_graveyard",
  "may_discard",
  "discard",
  "put_land_from_hand",
  "put_creature_from_hand",
  "choose_dredge",
  "cast_creature_face_down",
  "choose_exiled_with_card",
  "choose_exiled_with_card_to_cast",
  "choose_exiled_dig_to_cast_free",
  "dance_exile_more",
  "opponent_chooses_pile",
  "opponent_chooses_exiled_nonland",
  "choose_splitting_opponent",
  "partition_revealed",
  "choose_pile_for_hand",
  "choose_exiled_to_cast_free",
  "revealed_card_to_battlefield_or_hand",
  "choose_mode",
  "choose_trigger_modes",
  "choose_mana_color",
  "choose_creature_type",
  "choose_color",
  "choose_copy_target",
  "choose_attach_host",
  "put_from_hand_on_top",
  "opponent_chooses_revealed_to_graveyard",
  "pay_cumulative_upkeep_or_sacrifice",
  "may_draw_up_to",
  "trade_secrets_caster_draw",
  "pay_any_amount_of_mana",
  "choose_card_name",
  "trade_secrets_repeat",
] as const satisfies readonly PendingChoiceView["kind"][];

const emptyCost = { colored: [], generic: 0 };
const triggerModes: WireModeChoice[] = [
  { index: 0, target: { kind: "object", id: 9 } },
  { index: 2, target: { kind: "player", player: 1 } },
];

function expectDraftIntent(pc: PendingChoiceView, draft: PromptDraft, expected: WireIntent): void {
  const answer = answerFromDraft(pc, draft);
  expect(answer).not.toBeNull();
  if (answer == null) {
    throw new Error(`Expected answer for ${pc.kind}`);
  }
  expect(choiceIntent(pc, answer)).toEqual(expected);
}

test("pending choice kinds list is exhaustive", () => {
  const exhaustive: Exclude<PendingChoiceView["kind"], (typeof ALL_PENDING_CHOICE_KINDS)[number]> extends never
    ? true
    : never = true;
  expect(exhaustive).toBe(true);
});

test("FORMULATOR_FOR_KIND registers every pending choice kind", () => {
  assertAllKindsRegistered(ALL_PENDING_CHOICE_KINDS);
  expect(Object.keys(FORMULATOR_FOR_KIND).sort()).toEqual([...ALL_PENDING_CHOICE_KINDS].sort());
});

test("choiceIntent maps discard answer", () => {
  const pc = { kind: "discard" as const, count: 2, items: [], player: 0 };
  expect(choiceIntent(pc, { kind: "discard", cards: [3, 7] })).toEqual({
    kind: "discard",
    player: 0,
    cards: [3, 7],
  });
});

test("choiceIntent maps search_library decline", () => {
  const pc = { kind: "search_library" as const, items: [], player: 1 };
  expect(choiceIntent(pc, { kind: "search", choice: null })).toEqual({
    kind: "search_library",
    player: 1,
    choice: null,
  });
});

test("choiceIntent maps scry arrange", () => {
  const pc = { kind: "scry" as const, items: [{ id: 1, label: "A" }], player: 0 };
  expect(choiceIntent(pc, { kind: "arrange", top: [1], bottom: [2] })).toEqual({
    kind: "arrange_top",
    player: 0,
    top: [1],
    bottom: [2],
  });
});

test("choiceIntent maps order_triggers", () => {
  const pc = { kind: "order_triggers" as const, count: 2, labels: ["A", "B"], player: 0, source: 5 };
  expect(choiceIntent(pc, { kind: "order", order: [1, 0] })).toEqual({
    kind: "choose_order",
    player: 0,
    order: [1, 0],
  });
});

test("choiceIntent maps assign combat damage", () => {
  const pc = { kind: "assign_combat_damage" as const, items: [], player: 0, source: 9 };
  expect(choiceIntent(pc, { kind: "assign", assignment: [{ blocker: 4, amount: 3 }] })).toEqual({
    kind: "assign_damage",
    player: 0,
    assignment: [{ blocker: 4, amount: 3 }],
  });
});

function attackerObject(overrides: Partial<ObjectView> = {}): ObjectView {
  return {
    controller: 0,
    has_haste: false,
    id: 9,
    is_commander: false,
    kind: { kind: "creature", power: 4, toughness: 4 },
    mana_cost: { colored: [0, 0, 0, 0, 0], generic: 0 },
    marked_damage: 0,
    name: "Attacker",
    needs_target: false,
    owner: 0,
    plus_counters: 0,
    power: 4,
    print: "",
    summoning_sick: false,
    tapped: false,
    toughness: 4,
    zone: 2,
    ...overrides,
  };
}

function damageState(attacker: ObjectView): VisibleState {
  return {
    active_player: 0,
    can_act: true,
    combat: { attackers: [], blocks: [], attackers_declared: false, blockers_declared: [] },
    objects: [attacker],
    pending_choice: null,
    players: [],
    priority: 0,
    stack: [],
    step: 3,
    viewer: 0,
  };
}

describe("damageAssignReady", () => {
  const pc = {
    kind: "assign_combat_damage" as const,
    items: [
      { id: 4, label: "Bear" },
      { id: 5, label: "Elf" },
    ],
    player: 0,
    source: 9,
  };

  test("non-trample requires exact power sum", () => {
    const state = damageState(attackerObject());
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 3, 5: 1 } }, state)).toBe(true);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 2, 5: 1 } }, state)).toBe(false);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 5, 5: 0 } }, state)).toBe(false);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 5, 5: -1 } }, state)).toBe(false);
  });

  test("trample allows under-assign; rejects over-assign and negatives", () => {
    const state = damageState(attackerObject({ keywords: ["trample"] }));
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 2, 5: 0 } }, state)).toBe(true);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 0, 5: 0 } }, state)).toBe(true);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 3, 5: 1 } }, state)).toBe(true);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: 5, 5: 0 } }, state)).toBe(false);
    expect(damageAssignReady(pc, { kind: "damage", amounts: { 4: -1, 5: 0 } }, state)).toBe(false);
  });
});

describe("clickDamageAssign", () => {
  test("moves one damage from the largest other blocker onto the clicked blocker", () => {
    expect(clickDamageAssign({ 4: 4, 5: 0 }, 5, 4, false)).toEqual({ 4: 3, 5: 1 });
    expect(clickDamageAssign({ 4: 3, 5: 1 }, 5, 4, false)).toEqual({ 4: 2, 5: 2 });
  });

  test("trample under-assign adds without stealing until power is full", () => {
    expect(clickDamageAssign({ 4: 0, 5: 0 }, 4, 4, true)).toEqual({ 4: 1, 5: 0 });
    expect(clickDamageAssign({ 4: 4, 5: 0 }, 5, 4, true)).toEqual({ 4: 3, 5: 1 });
  });

  test("no-op when the clicked blocker already holds all assigned damage", () => {
    expect(clickDamageAssign({ 4: 4, 5: 0 }, 4, 4, false)).toEqual({ 4: 4, 5: 0 });
  });
});

describe("nextDistributeBucket", () => {
  const caps = { to_hand: 1, to_bottom: 1, to_exile_may_play: 1 };

  test("cycles unassigned into the first bucket with room", () => {
    expect(nextDistributeBucket(null, { to_hand: 0, to_bottom: 0, to_exile_may_play: 0 }, caps)).toBe("to_hand");
    expect(nextDistributeBucket(null, { to_hand: 1, to_bottom: 0, to_exile_may_play: 0 }, caps)).toBe("to_bottom");
  });

  test("cycles through buckets then clears", () => {
    expect(nextDistributeBucket("to_hand", { to_hand: 1, to_bottom: 0, to_exile_may_play: 0 }, caps)).toBe("to_bottom");
    expect(nextDistributeBucket("to_bottom", { to_hand: 1, to_bottom: 1, to_exile_may_play: 0 }, caps)).toBe(
      "to_exile_may_play",
    );
    expect(nextDistributeBucket("to_exile_may_play", { to_hand: 1, to_bottom: 1, to_exile_may_play: 1 }, caps)).toBe(
      null,
    );
  });

  test("skips full buckets when cycling", () => {
    expect(nextDistributeBucket("to_hand", { to_hand: 1, to_bottom: 1, to_exile_may_play: 0 }, caps)).toBe(
      "to_exile_may_play",
    );
  });
});

test("choiceDraftKey changes when scry items change", () => {
  const a = { kind: "scry" as const, items: [{ id: 1, label: "A" }], player: 0 };
  const b = { kind: "scry" as const, items: [{ id: 2, label: "B" }], player: 0 };
  expect(choiceDraftKey(a)).not.toBe(choiceDraftKey(b));
});

test("choiceDraftKey changes when pay_any_amount max shrinks", () => {
  const a = { kind: "pay_any_amount_of_mana" as const, max: 12, player: 0, source: 7 };
  const b = { kind: "pay_any_amount_of_mana" as const, max: 4, player: 0, source: 7 };
  expect(choiceDraftKey(a)).not.toBe(choiceDraftKey(b));
});

test("buildAnswerFromDraft builds discard from card-pick draft", () => {
  const pc = { kind: "discard" as const, count: 2, items: [], player: 0 };
  const draft: PromptDraft = { kind: "card-pick", picked: [1, 2] };
  expect(buildAnswerFromDraft(pc, draft)).toEqual({ kind: "discard", cards: [1, 2] });
});

test("buildAnswerFromDraft builds proliferate from empty card-pick", () => {
  const pc = { kind: "proliferate" as const, items: [], player: 0, source: 1 };
  const draft: PromptDraft = { kind: "card-pick", picked: [] };
  expect(buildAnswerFromDraft(pc, draft)).toEqual({ kind: "sacrifice", ids: [] });
});

describe("answerFromDraft builds accepted intents", () => {
  test("builds an order answer for order_triggers", () => {
    expectDraftIntent(
      { kind: "order_triggers", count: 2, labels: ["A", "B"], player: 0, source: 5 },
      { kind: "order", order: [1, 0] },
      { kind: "choose_order", order: [1, 0], player: 0 },
    );
  });

  test("builds a combat damage assignment", () => {
    expectDraftIntent(
      {
        kind: "assign_combat_damage",
        items: [
          { id: 4, label: "Bear" },
          { id: 5, label: "Elf" },
        ],
        player: 0,
        source: 9,
      },
      { kind: "damage", amounts: { 4: 3, 5: 1 } },
      {
        kind: "assign_damage",
        assignment: [
          { blocker: 4, amount: 3 },
          { blocker: 5, amount: 1 },
        ],
        player: 0,
      },
    );
  });

  test("builds a divide spell damage assignment", () => {
    expectDraftIntent(
      {
        kind: "divide_spell_damage",
        items: [
          { id: 7, label: "Target A" },
          { id: 11, label: "Target B" },
        ],
        player: 0,
        spell: 99,
        total: 4,
      },
      { kind: "divide", amounts: { 0: 3, 1: 1 } },
      {
        kind: "divide_spell_damage",
        assignment: [
          { amount: 3, target: { kind: "object", id: 7 } },
          { amount: 1, target: { kind: "object", id: 11 } },
        ],
        player: 0,
      },
    );
  });

  test("allows zero damage on some divide spell targets", () => {
    expectDraftIntent(
      {
        kind: "divide_spell_damage",
        items: [
          { id: 7, label: "Target A" },
          { id: 11, label: "Target B" },
        ],
        player: 0,
        spell: 99,
        total: 4,
      },
      { kind: "divide", amounts: { 0: 4, 1: 0 } },
      {
        kind: "divide_spell_damage",
        assignment: [
          { amount: 4, target: { kind: "object", id: 7 } },
          { amount: 0, target: { kind: "object", id: 11 } },
        ],
        player: 0,
      },
    );
  });

  test("builds a divide counters assignment", () => {
    expectDraftIntent(
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
      { kind: "damage", amounts: { 12: 1, 13: 1 } },
      {
        kind: "assign_damage",
        assignment: [
          { blocker: 12, amount: 1 },
          { blocker: 13, amount: 1 },
        ],
        player: 0,
      },
    );
  });

  test("builds may answers for yes-no prompts", () => {
    expectDraftIntent(
      { kind: "trade_secrets_repeat", caster: 1, player: 0 },
      { kind: "may", yes: true },
      { kind: "answer_may", player: 0, yes: true },
    );
  });

  test("builds pay answers for optional cost prompts", () => {
    expectDraftIntent(
      { kind: "pay_cost", cost: emptyCost, label: "Pay", player: 0, source: 1 },
      { kind: "pay", pay: true },
      { kind: "pay_optional_cost", pay: true, player: 0 },
    );
  });

  test("builds mode answers for choose_mode", () => {
    expectDraftIntent(
      { kind: "choose_mode", labels: ["A", "B"], player: 0, source: 1 },
      { kind: "mode", mode: 1 },
      { kind: "choose_mode", mode: 1, player: 0 },
    );
  });

  test("builds trigger mode answers for choose_trigger_modes", () => {
    expectDraftIntent(
      {
        kind: "choose_trigger_modes",
        choose: 2,
        modes: [
          { label: "A", needs_target: false, targets: [] },
          { label: "B", needs_target: true, targets: [{ kind: "object", id: 9 }] },
          { label: "C", needs_target: true, targets: [{ kind: "player", player: 1 }] },
        ],
        optional: false,
        player: 0,
        source: 1,
      },
      { kind: "modes", modes: triggerModes },
      { kind: "choose_trigger_modes", modes: triggerModes, player: 0 },
    );
  });

  test("builds target player answers for choose_target_players", () => {
    expectDraftIntent(
      {
        kind: "choose_target_players",
        items: [
          { id: 0, label: "Player 2" },
          { id: 0, label: "Player 3" },
        ],
        label: "Choose players",
        max: 2,
        min: 1,
        player: 0,
        source: 1,
      },
      { kind: "player-pick", players: [1, 2] },
      { kind: "choose_target_players", player: 0, players: [1, 2] },
    );
  });

  test("builds a player target for choose_splitting_opponent", () => {
    expectDraftIntent(
      {
        kind: "choose_splitting_opponent",
        items: [
          { id: 0, label: "Player 2" },
          { id: 0, label: "Player 3" },
        ],
        label: "Fact or Fiction",
        player: 0,
        source: 1,
      },
      { kind: "target", id: 0, player: 2 },
      {
        kind: "choose_targets",
        player: 0,
        targets: [{ kind: "player", player: 2 }],
      },
    );
  });

  test("builds pile answers", () => {
    expectDraftIntent(
      {
        kind: "choose_pile_for_hand",
        pile_a: [{ id: 1, label: "A" }],
        pile_b: [{ id: 2, label: "B" }],
        player: 0,
        source: 8,
      },
      { kind: "pile", pile: 1 },
      { kind: "choose_opponent_pile", pile: 1, player: 0 },
    );
  });

  test("builds distribute answers from partition buckets", () => {
    expectDraftIntent(
      {
        kind: "distribute_top",
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
          { id: 3, label: "C" },
        ],
        player: 0,
        to_bottom: 1,
        to_exile_may_play: 1,
        to_hand: 1,
      },
      {
        kind: "partition",
        buckets: {
          to_hand: [1],
          to_bottom: [2],
          to_exile_may_play: [3],
        },
      },
      {
        kind: "distribute_top",
        player: 0,
        to_bottom: [2],
        to_exile_may_play: [3],
        to_hand: [1],
      },
    );
  });

  test("builds partition answers as sacrifices", () => {
    expectDraftIntent(
      {
        kind: "partition_revealed",
        items: [
          { id: 6, label: "A" },
          { id: 7, label: "B" },
        ],
        player: 1,
        source: 8,
      },
      { kind: "partition", buckets: { pile_a: [6] } },
      { kind: "choose_sacrifices", player: 1, sacrifices: [6] },
    );
  });

  test("builds color and mana color answers", () => {
    expectDraftIntent(
      { kind: "choose_color", player: 0, source: 5 },
      { kind: "color", color: 3 },
      { kind: "choose_color", color: 3, player: 0 },
    );
    expectDraftIntent(
      { kind: "choose_mana_color", amount: 1, player: 0, source: 5 },
      { kind: "color", color: 4 },
      { kind: "choose_mana_color", color: 4, player: 0 },
    );
  });

  test("builds creature type answers", () => {
    expectDraftIntent(
      { kind: "choose_creature_type", options: ["Wizard", "Cleric"], player: 0, source: 5 },
      { kind: "string", value: "Wizard" },
      { kind: "choose_creature_type", player: 0, subtype: "Wizard" },
    );
  });

  test("builds draw count answers", () => {
    expectDraftIntent(
      { kind: "may_draw_up_to", max: 3, player: 0 },
      { kind: "number", count: 2 },
      { kind: "choose_draw_count", count: 2, player: 0 },
    );
  });

  test("builds destination answers", () => {
    expectDraftIntent(
      { kind: "choose_countered_spell_destination", player: 0, spell: 9 },
      { kind: "destination", choice: true },
      { kind: "choose_top_or_bottom", player: 0, top: true },
    );
    expectDraftIntent(
      {
        kind: "revealed_card_to_battlefield_or_hand",
        item: { id: 17, label: "Aura" },
        player: 0,
      },
      { kind: "destination", choice: 17 },
      { kind: "revealed_card_to_battlefield_or_hand", choice: 17, player: 0 },
    );
  });

  test("builds arrange, search, select, and discard answers", () => {
    expectDraftIntent(
      {
        kind: "scry",
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
        player: 0,
      },
      { kind: "partition", buckets: { top: [2], bottom: [1] } },
      { kind: "arrange_top", bottom: [1], player: 0, top: [2] },
    );
    expectDraftIntent(
      {
        kind: "scry",
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
        player: 0,
      },
      { kind: "card-pick", picked: [2] },
      { kind: "arrange_top", bottom: [1], player: 0, top: [2] },
    );
    expectDraftIntent(
      { kind: "search_library", items: [{ id: 7, label: "Card" }], player: 0 },
      { kind: "card-pick", picked: [7] },
      { kind: "search_library", choice: 7, player: 0 },
    );
    expectDraftIntent(
      {
        kind: "select_from_top",
        items: [
          { id: 7, label: "Card A" },
          { id: 8, label: "Card B" },
        ],
        player: 0,
        up_to: 2,
      },
      { kind: "card-pick", picked: [7, 8] },
      { kind: "select_from_top", cards: [7, 8], player: 0 },
    );
    expectDraftIntent(
      {
        kind: "discard",
        count: 2,
        items: [
          { id: 3, label: "Card A" },
          { id: 4, label: "Card B" },
        ],
        player: 0,
      },
      { kind: "card-pick", picked: [3, 4] },
      { kind: "discard", cards: [3, 4], player: 0 },
    );
  });

  test("builds target answers", () => {
    expectDraftIntent(
      {
        kind: "choose_target",
        items: [{ id: 11, label: "Bear" }],
        label: "Choose target",
        max: 1,
        optional: false,
        player: 0,
        source: 1,
      },
      { kind: "card-pick", picked: [11] },
      {
        kind: "choose_targets",
        player: 0,
        targets: [{ kind: "object", id: 11 }],
      },
    );
  });

  test("submits every required target for a mandatory multi-target choose_target", () => {
    expectDraftIntent(
      {
        kind: "choose_target",
        items: [
          { id: 21, label: "Forest" },
          { id: 22, label: "Island" },
        ],
        label: "Untap two target lands",
        max: 2,
        optional: false,
        player: 0,
        source: 1,
      },
      { kind: "card-pick", picked: [21, 22] },
      {
        kind: "choose_targets",
        player: 0,
        targets: [
          { kind: "object", id: 21 },
          { kind: "object", id: 22 },
        ],
      },
    );
  });

  test("builds sacrifice answers for cumulative upkeep payment", () => {
    expectDraftIntent(
      {
        kind: "pay_cumulative_upkeep_or_sacrifice",
        count: 1,
        items: [{ id: 33, label: "Creature" }],
        player: 0,
        source: 1,
      },
      { kind: "card-pick", picked: [33] },
      { kind: "choose_sacrifices", player: 0, sacrifices: [33] },
    );
  });

  test("declines cumulative upkeep with empty sacrifices", () => {
    const pc = {
      kind: "pay_cumulative_upkeep_or_sacrifice" as const,
      count: 1,
      items: [{ id: 33, label: "Creature" }],
      player: 0,
      source: 1,
    };
    const answer = declineAnswer(pc);
    expect(answer).toEqual({ kind: "sacrifice", ids: [] });
    if (answer == null) return;
    expect(choiceIntent(pc, answer)).toEqual({
      kind: "choose_sacrifices",
      player: 0,
      sacrifices: [],
    });
  });

  test("declines dredge to draw normally", () => {
    const pc = {
      kind: "choose_dredge" as const,
      items: [{ id: 61, label: "Stinkweed Imp" }],
      player: 0,
    };
    expect(cardPickReady(pc, [])).toBe(false);
    expect(cardPickReady(pc, [61])).toBe(true);
    const answer = declineAnswer(pc);
    expect(answer).toEqual({ kind: "dredge", dredger: null });
    if (answer == null) return;
    expect(choiceIntent(pc, answer)).toEqual({
      kind: "choose_dredge",
      player: 0,
      dredger: null,
    });
  });

  test("builds keep tapped answers", () => {
    expectDraftIntent(
      {
        kind: "decline_untap",
        items: [
          { id: 1, label: "A" },
          { id: 2, label: "B" },
        ],
        player: 0,
      },
      { kind: "card-pick", picked: [1] },
      { kind: "decline_untap", keep_tapped: [1], player: 0 },
    );
  });

  test("builds return land answers", () => {
    expectDraftIntent(
      {
        kind: "sacrifice_unless_return_land",
        items: [{ id: 14, label: "Island" }],
        player: 0,
        source: 1,
      },
      { kind: "card-pick", picked: [14] },
      { kind: "return_land_or_sacrifice", land: 14, player: 0 },
    );
  });

  test("builds single-card movement answers", () => {
    expectDraftIntent(
      { kind: "put_land_from_hand", items: [{ id: 20, label: "Forest" }], player: 0 },
      { kind: "card-pick", picked: [20] },
      { kind: "put_land_from_hand", choice: 20, player: 0 },
    );
    expectDraftIntent(
      { kind: "put_creature_from_hand", items: [{ id: 21, label: "Elf" }], player: 0 },
      { kind: "card-pick", picked: [21] },
      { kind: "put_creature_from_hand", choice: 21, player: 0 },
    );
  });

  test("builds exiled-card answers", () => {
    expectDraftIntent(
      { kind: "opponent_chooses_exiled_nonland", items: [{ id: 31, label: "Spell" }], player: 0, source: 1 },
      { kind: "card-pick", picked: [31] },
      { kind: "choose_exiled_with_card", choice: 31, player: 0 },
    );
    expectDraftIntent(
      { kind: "choose_exiled_with_card_to_cast", items: [{ id: 32, label: "Spell" }], player: 0, source: 1 },
      { kind: "card-pick", picked: [32] },
      { kind: "choose_exiled_with_card_to_cast", choice: 32, player: 0 },
    );
    expectDraftIntent(
      { kind: "choose_exiled_dig_to_cast_free", items: [{ id: 33, label: "Spell" }], player: 0, source: 1 },
      { kind: "card-pick", picked: [33] },
      { kind: "choose_exiled_dig_to_cast_free", choice: 33, player: 0 },
    );
  });

  test("builds copy, attach, hand-on-top, dredge, and face-down answers", () => {
    expectDraftIntent(
      { kind: "choose_copy_target", items: [{ id: 41, label: "Copy me" }], player: 0, source: 1 },
      { kind: "card-pick", picked: [41] },
      { kind: "choose_copy_target", copy: 41, player: 0 },
    );
    expectDraftIntent(
      {
        kind: "choose_attach_host",
        attachment: 50,
        items: [{ id: 42, label: "Host" }],
        optional: false,
        player: 0,
      },
      { kind: "card-pick", picked: [42] },
      { kind: "choose_attach_host", host: 42, player: 0 },
    );
    expectDraftIntent(
      {
        kind: "put_from_hand_on_top",
        count: 2,
        items: [
          { id: 51, label: "A" },
          { id: 52, label: "B" },
        ],
        player: 0,
      },
      { kind: "card-pick", picked: [51, 52] },
      { kind: "put_from_hand_on_top", cards: [51, 52], player: 0 },
    );
    expectDraftIntent(
      { kind: "choose_dredge", items: [{ id: 61, label: "Stinkweed Imp" }], player: 0 },
      { kind: "card-pick", picked: [61] },
      { kind: "choose_dredge", dredger: 61, player: 0 },
    );
    expectDraftIntent(
      { kind: "cast_creature_face_down", items: [{ id: 71, label: "Morph" }], player: 0 },
      { kind: "card-pick", picked: [71] },
      { kind: "cast_creature_face_down", choice: 71, player: 0 },
    );
  });

  test("builds multi-target and shuffle answers", () => {
    expectDraftIntent(
      {
        kind: "choose_spell_targets",
        items: [
          { id: 81, label: "A" },
          { id: 82, label: "B" },
        ],
        label: "Choose targets",
        max: 2,
        min: 1,
        player: 0,
        spell: 90,
      },
      { kind: "targets", ids: [81, 82] },
      {
        kind: "choose_targets",
        player: 0,
        targets: [
          { kind: "object", id: 81 },
          { kind: "object", id: 82 },
        ],
      },
    );
    expectDraftIntent(
      {
        kind: "shuffle_from_graveyard",
        items: [
          { id: 91, label: "A" },
          { id: 92, label: "B" },
        ],
        max: 2,
        owner: 0,
        player: 0,
        source: 1,
      },
      { kind: "card-pick", picked: [91] },
      { kind: "shuffle_from_graveyard", cards: [91], player: 0 },
    );
  });
});
