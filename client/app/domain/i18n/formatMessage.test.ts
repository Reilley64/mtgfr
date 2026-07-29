import { describe, expect, it } from "vitest";
import { formatMessage } from "./message";

describe("formatMessage", () => {
  it("names the drawer of a draw from its player set", () => {
    expect(
      formatMessage({
        key: "effect.draw_cards",
        params: [{ name: "count", int_value: 2 }],
        children: [],
      }),
    ).toBe("You draw 2");
    expect(
      formatMessage({
        key: "effect.draw_cards",
        params: [
          { name: "who", string_value: "targets_owner" },
          { name: "count", int_value: 2 },
        ],
        children: [],
      }),
    ).toBe("Target's owner draws 2");
  });

  it("joins sequence children with then", () => {
    expect(
      formatMessage({
        key: "effect.sequence",
        params: [],
        children: [
          {
            key: "effect.draw_cards",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
          {
            key: "effect.discard",
            params: [{ name: "count", int_value: 2 }],
            children: [],
          },
        ],
      }),
    ).toBe("You draw 2, then Discard 2");
  });

  it("returns raw key when missing", () => {
    expect(formatMessage({ key: "effect.unknown_zz", params: [], children: [] })).toBe("effect.unknown_zz");
  });

  it("does not pass bare strings through", () => {
    // @ts-expect-error strings are not MessageRef values.
    expect(() => formatMessage("Draw 2")).toThrow(TypeError);
  });

  it("formats effect.control_tap_target from an explicit historical catalog entry", () => {
    expect(formatMessage({ key: "effect.control_tap_target", params: [], children: [] })).toBe("Tap target");
  });

  it("formats reject.illegal_target", () => {
    expect(formatMessage({ key: "reject.illegal_target", params: [], children: [] })).toBe("Pick a legal target.");
  });

  it("names the recipient of a life change from its player set", () => {
    expect(
      formatMessage({
        key: "effect.life_lose",
        params: [
          { name: "who", string_value: "each_player" },
          { name: "amount", int_value: 3 },
        ],
        children: [],
      }),
    ).toBe("Each player loses 3 life");
    expect(
      formatMessage({
        key: "effect.life_gain",
        params: [
          { name: "who", string_value: "you" },
          { name: "amount", int_value: 3 },
        ],
        children: [],
      }),
    ).toBe("You gain 3 life");
  });

  it("reads a base-P/T-setting Aura back as fixed numbers or as one shared count", () => {
    expect(
      formatMessage({
        key: "effect.static_set_attached_base_pt",
        params: [
          { name: "power", int_value: 0 },
          { name: "toughness", int_value: 1 },
        ],
        children: [],
      }),
    ).toBe("Attached permanent has base power and toughness 0/1");
    expect(
      formatMessage({
        key: "effect.static_set_attached_base_pt",
        params: [
          { name: "power", amount_token: "source_mana_value" },
          { name: "toughness", amount_token: "source_mana_value" },
        ],
        children: [],
      }),
    ).toBe("Attached permanent has base power and toughness each equal to its mana value");
  });

  // One Rust effect covers every type-changing Aura in the pool, and which clause a card wants is
  // carried entirely in its flags — a reader that ignores them says the same thing about Evil
  // Presence turning a land into a Swamp and Darksteel Mutation shutting a creature off.
  it("reads each type-changing Aura back as the clause its flags actually describe", () => {
    const read = (params: Array<{ name: string; bool_value?: boolean; string_value?: string }>): string =>
      formatMessage({ key: "effect.static_set_attached_types", params, children: [] });

    // Angelic Destiny — additive subtype, nothing replaced. Rust sends every empty list as "none",
    // which has to read as no clause rather than as a type called "none".
    expect(
      read([
        { name: "types", string_value: "" },
        { name: "set_subtypes", string_value: "none" },
        { name: "add_subtypes", string_value: "angel" },
      ]),
    ).toBe("Attached permanent is an angel in addition to its other types");
    // Evil Presence — the land's type is replaced, so there is no "in addition".
    expect(read([{ name: "set_subtypes", string_value: "swamp" }])).toBe("Attached permanent is a swamp");
    // Animate Artifact — card types print conjunctively ("artifact creature"), not "artifact or creature".
    expect(read([{ name: "types", string_value: "artifact_creature" }])).toBe(
      "Attached permanent is an artifact creature in addition to its other types",
    );
    // Darksteel Mutation — replaces both halves and takes the abilities with it.
    expect(
      read([
        { name: "types", string_value: "artifact_creature" },
        { name: "set_types", bool_value: true },
        { name: "set_subtypes", string_value: "insect" },
        { name: "lose_all_abilities", bool_value: true },
      ]),
    ).toBe("Attached permanent is an insect artifact creature and has no abilities");
    // Phantasmal Terrain — the type isn't known until its controller names one.
    expect(read([{ name: "set_chosen_land_type", bool_value: true }])).toBe("Attached permanent is the chosen type");
  });

  // "Can't be blocked by Walls" and "can't be blocked except by Walls" share one Rust filter,
  // so the reader has to say which side of it a card landed on — the inverted authoring is
  // invisible in the key alone.
  it("reads the two wall clauses back as the opposite restrictions they are", () => {
    expect(
      formatMessage({
        key: "effect.static_cant_be_blocked_by",
        params: [{ name: "filter", string_value: "wall_creatures" }],
        children: [],
      }),
    ).toBe("This creature can't be blocked by wall creatures");
    expect(
      formatMessage({
        key: "effect.static_cant_be_blocked_by",
        params: [{ name: "filter", string_value: "non_wall_creatures" }],
        children: [],
      }),
    ).toBe("This creature can't be blocked by non wall creatures");
  });

  it("reads Ironclaw Orcs' and Two-Headed Giant's block clauses back", () => {
    expect(
      formatMessage({
        key: "effect.static_cant_block_attackers",
        params: [{ name: "filter", string_value: "creatures_with_power_2_or_greater" }],
        children: [],
      }),
    ).toBe("This creature can't block creatures with power 2 or greater");
    expect(
      formatMessage({
        key: "effect.static_can_block_additional",
        params: [{ name: "count", int_value: 1 }],
        children: [],
      }),
    ).toBe("This creature can block 1 additional creature(s) each combat");
  });

  // Juggernaut compels only itself; Avatar of Slaughter compels the board. One key, one flag.
  it("splits must-attack between the self-only and board-wide readings", () => {
    expect(
      formatMessage({
        key: "effect.static_must_attack_each_combat",
        params: [{ name: "self_only", bool_value: true }],
        children: [],
      }),
    ).toBe("This creature attacks each combat if able");
    expect(
      formatMessage({
        key: "effect.static_must_attack_each_combat",
        params: [{ name: "self_only", bool_value: false }],
        children: [],
      }),
    ).toBe("All creatures attack each combat if able");
  });

  // Both taxes render their filter through the same generic token humanizer every other
  // catalog key uses — "color_white" reads back as "color white", the way `reduce_spell_cost`
  // has always rendered Balefire Liege.
  it("reads Gloom's two taxes back as cast and activation costs", () => {
    expect(
      formatMessage({
        key: "effect.static_tax_spell_cost",
        params: [
          { name: "amount", int_value: 3 },
          { name: "filter", string_value: "color_white" },
        ],
        children: [],
      }),
    ).toBe("color white cost {3} more to cast");
    expect(
      formatMessage({
        key: "effect.static_tax_activated_ability",
        params: [
          { name: "amount", int_value: 3 },
          { name: "filter", string_value: "permanent_enchantment_white" },
        ],
        children: [],
      }),
    ).toBe("Activated abilities of permanent enchantment white cost {3} more to activate");
  });

  it("reads Island Sanctuary's skipped draw back with the attackers it turns away", () => {
    expect(
      formatMessage({
        key: "effect.static_may_skip_draw_for_cant_be_attacked_by",
        params: [{ name: "filter", string_value: "permanent_creature_without_flying_without_islandwalk" }],
        children: [],
      }),
    ).toBe(
      "You may skip your draw-step draw; if you do, permanent creature without flying without islandwalk can't attack you until your next turn",
    );
  });

  it("reads Kudzu's re-attachment offer back with the hosts on offer", () => {
    expect(
      formatMessage({
        key: "effect.choice_triggering_player_may_attach_this_aura_to_chosen",
        params: [{ name: "filter", string_value: "permanent_land" }],
        children: [],
      }),
    ).toBe("That permanent's controller may attach this Aura to a permanent land of their choice");
  });

  it("reads Power Leak's payment pause back with the damage it shields against", () => {
    expect(
      formatMessage({
        key: "effect.choice_triggering_player_may_pay_any_amount_to_prevent",
        params: [{ name: "amount", int_value: 2 }],
        children: [],
      }),
    ).toBe("That player may pay any amount of mana; prevent that much of the next 2 damage dealt to them");
  });

  it("formats catalog keyword summaries", () => {
    expect(formatMessage({ key: "keyword.flying", params: [], children: [] })).toBe("Flying");
    expect(
      formatMessage({
        key: "keyword.ward",
        params: [{ name: "amount", int_value: 2 }],
        children: [],
      }),
    ).toBe("Ward {2}");
    expect(
      formatMessage({
        key: "keyword.protection_from",
        params: [{ name: "scope", string_value: "red" }],
        children: [],
      }),
    ).toBe("Protection from red");
  });
});
