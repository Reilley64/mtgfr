import { describe, expect, it } from "vitest";
import { formatMessage } from "./message";

describe("formatMessage", () => {
  it("formats effect.draw_cards", () => {
    expect(
      formatMessage({
        key: "effect.draw_cards",
        params: [{ name: "count", int_value: 2 }],
        children: [],
      }),
    ).toBe("Draw 2");
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
    ).toBe("Draw 2, then Discard 2");
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

  it("formats effect.life_each_player_loses", () => {
    expect(
      formatMessage({
        key: "effect.life_each_player_loses",
        params: [{ name: "amount", int_value: 3 }],
        children: [],
      }),
    ).toBe("Each player loses 3");
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
