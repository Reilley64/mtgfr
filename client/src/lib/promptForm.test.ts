import { describe, expect, it } from "vitest";
import {
  cardPickIsSearchable,
  dedupeChoiceItems,
  filterChoiceItems,
  isFullscreenPrompt,
  PICK_CARD_SCROLL_MIN_CLASS,
  promptChrome,
  searchableChoiceItems,
} from "~/lib/promptForm";
import type { ChoiceItem, PendingChoiceView } from "~/wire/types";

const item = (id: number, label: string): ChoiceItem => ({ id, label });

/** Every PendingChoiceView kind — keep in sync with generated.ts (compile fails if FORMS drifts). */
const ALL_KINDS: PendingChoiceView["kind"][] = [
  "order_triggers",
  "choose_target",
  "choose_spell_targets",
  "choose_target_players",
  "may_yes_no",
  "pay_cost",
  "pay_or_counter",
  "pay_echo_or_sacrifice",
  "assign_combat_damage",
  "divide_spell_damage",
  "scry",
  "surveil",
  "search_library",
  "select_from_top",
  "distribute_top",
  "shuffle_from_graveyard",
  "sacrifice_edict",
  "proliferate",
  "phase_out",
  "may_sacrifice",
  "choose_own_sacrifices",
  "devour",
  "exile_from_graveyard",
  "caster_keep_permanents",
  "may_return_from_graveyard",
  "may_discard",
  "discard",
  "put_land_from_hand",
  "choose_exiled_with_card",
  "choose_exiled_with_card_to_cast",
  "choose_exiled_dig_to_cast_free",
  "dance_exile_more",
  "opponent_chooses_pile",
  "opponent_chooses_exiled_nonland",
  "choose_exiled_to_cast_free",
  "revealed_card_to_battlefield_or_hand",
  "choose_mode",
  "choose_trigger_modes",
  "choose_mana_color",
  "choose_creature_type",
  "choose_color",
  "choose_attach_host",
  "choose_copy_target",
  "choose_counter_target_for_player",
  "choose_ability_targets",
  "divide_counters",
  "choose_dredge",
  "put_from_hand_on_top",
  "opponent_chooses_revealed_to_graveyard",
  "pay_cumulative_upkeep_or_sacrifice",
  "may_draw_up_to",
  "trade_secrets_caster_draw",
  "trade_secrets_repeat",
];

describe("promptChrome", () => {
  it("classifies every wire kind without throwing", () => {
    for (const kind of ALL_KINDS) {
      expect(["panel", "fullscreen"]).toContain(promptChrome(kind));
    }
  });

  it("puts card picks fullscreen and binary choices in the panel", () => {
    expect(promptChrome("discard")).toBe("fullscreen");
    expect(promptChrome("search_library")).toBe("fullscreen");
    expect(isFullscreenPrompt("may_yes_no")).toBe(false);
    expect(promptChrome("choose_mode")).toBe("panel");
    expect(promptChrome("dance_exile_more")).toBe("panel");
    expect(promptChrome("choose_dredge")).toBe("fullscreen");
  });
});

describe("dedupeChoiceItems", () => {
  it("keeps the first object per card name", () => {
    expect(dedupeChoiceItems([item(1, "Forest"), item(2, "Island"), item(3, "Forest"), item(4, "Island")])).toEqual([
      item(1, "Forest"),
      item(2, "Island"),
    ]);
  });

  it("returns an empty list unchanged", () => {
    expect(dedupeChoiceItems([])).toEqual([]);
  });
});

describe("filterChoiceItems", () => {
  const pool = [item(1, "Sol Ring"), item(2, "Ring of Evos Isle"), item(3, "Forest")];

  it("keeps all items when the query is blank", () => {
    expect(filterChoiceItems(pool, "  ")).toEqual(pool);
  });

  it("matches card names case-insensitively by substring", () => {
    expect(filterChoiceItems(pool, "ring")).toEqual([item(1, "Sol Ring"), item(2, "Ring of Evos Isle")]);
  });
});

describe("searchableChoiceItems", () => {
  it("dedupes then filters — a library full of basics stays one face per name", () => {
    expect(
      searchableChoiceItems([item(1, "Forest"), item(2, "Forest"), item(3, "Sol Ring"), item(4, "Island")], "for"),
    ).toEqual([item(1, "Forest")]);
  });
});

describe("cardPickIsSearchable", () => {
  it("enables the library-search filter surface and leaves other card picks alone", () => {
    expect(cardPickIsSearchable("search_library")).toBe(true);
    expect(cardPickIsSearchable("discard")).toBe(false);
    expect(cardPickIsSearchable("scry")).toBe(false);
  });
});

describe("PICK_CARD_SCROLL_MIN_CLASS", () => {
  it("keeps a non-zero min-height so short viewports cannot hide every library card", () => {
    // Regression: containScroll flex shrink collapsed pick-card-scroll to 0px; Fail-to-find stayed.
    expect(PICK_CARD_SCROLL_MIN_CLASS).toMatch(/min-h-/);
    expect(PICK_CARD_SCROLL_MIN_CLASS).toMatch(/280px|40vh/);
  });
});
