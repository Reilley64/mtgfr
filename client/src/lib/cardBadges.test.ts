import { describe, expect, it } from "vitest";
import { abilityGlyph, hiddenKeywordCount, keywordBadges, showsSummoningSick } from "~/lib/cardBadges";

describe("keywordBadges", () => {
  it("orders combat keywords and drops unknowns", () => {
    const { shown, overflow } = keywordBadges(["prowess", "flying", "trample", "not_a_real_kw"]);
    expect(shown).toEqual(["flying", "trample", "prowess"]);
    expect(overflow).toBe(0);
  });

  it("keeps ward and protection after evergreens", () => {
    const { shown } = keywordBadges(["ward:2", "flying", "protection:red"]);
    expect(shown).toEqual(["flying", "ward:2", "protection:red"]);
  });

  it("caps at MAX and reports overflow", () => {
    const { shown, overflow } = keywordBadges([
      "flying",
      "first_strike",
      "vigilance",
      "haste",
      "trample",
      "deathtouch",
      "lifelink",
    ]);
    expect(shown).toHaveLength(4);
    expect(shown).toEqual(["flying", "first_strike", "vigilance", "haste"]);
    expect(overflow).toBe(3);
  });
});

describe("abilityGlyph", () => {
  it("maps wire ids to Mana private-use glyphs", () => {
    expect(abilityGlyph("flying")).toBe("\ue952");
    expect(abilityGlyph("first_strike")).toBe("\ue950");
    expect(abilityGlyph("ward:2")).toBe("\ue992");
    expect(abilityGlyph("protection:black")).toBe("\uea7f");
    expect(abilityGlyph("summoning_sick")).toBe("\ue96a");
    expect(abilityGlyph("goaded")).toBe("\ue9c9");
    expect(abilityGlyph("unblockable")).toBe("\uea5c");
  });

  it("returns null for unknown keywords", () => {
    expect(abilityGlyph("not_a_real_kw")).toBeNull();
  });
});

describe("hiddenKeywordCount", () => {
  it("adds rail-clipped icons to the pre-cap overflow", () => {
    expect(hiddenKeywordCount(4, 4, 2)).toBe(2);
    expect(hiddenKeywordCount(4, 2, 2)).toBe(4);
    expect(hiddenKeywordCount(4, 0, 0)).toBe(4);
  });
});

describe("showsSummoningSick", () => {
  it("hides the sick badge when the permanent has haste", () => {
    expect(showsSummoningSick(true, false)).toBe(true);
    expect(showsSummoningSick(true, true)).toBe(false);
    expect(showsSummoningSick(false, false)).toBe(false);
  });
});
