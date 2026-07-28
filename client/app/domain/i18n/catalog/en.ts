import type { MessageFormatter, MessageParams, MessageValue } from "../message";

function humanize(value: MessageValue): string {
  if (typeof value !== "string") return String(value);
  return value
    .replace(/_/g, " ")
    .replace(/\bmv\b/g, "mana value")
    .replace(/\blte\b/g, "at most")
    .replace(/\bgte\b/g, "at least")
    .replace(/\bpt\b/g, "power/toughness")
    .replace(/\bgy\b/g, "graveyard")
    .replace(/\s+/g, " ")
    .trim();
}

function param(params: MessageParams, name: string, fallback: MessageValue = ""): MessageValue {
  return params[name] ?? fallback;
}

function bool(params: MessageParams, name: string): boolean {
  return params[name] === true;
}

function literal(text: string): MessageFormatter {
  return () => text;
}

function nameOnly(params: MessageParams): string {
  return String(param(params, "name"));
}

function edictWho(scope: MessageValue): string {
  if (scope === "each_opponent") return "Each opponent";
  if (scope === "targeted_players") return "Any number of target players";
  if (scope === "targeted_opponent") return "Target opponent";
  return "Each player";
}

function definingPtLead(when: MessageValue): string {
  if (when === "attacking") return "As long as this creature is attacking, its";
  if (when === "not_attacking") return "As long as this creature isn't attacking, its";
  return "This creature's";
}

function searchDest(dest: MessageValue): string {
  if (dest === "battlefield") return "onto the battlefield";
  if (dest === "library_top") return "on top of your library";
  if (dest === "graveyard") return "into your graveyard";
  if (dest === "exile") return "into exile";
  return "into your hand";
}

function topDest(params: MessageParams): string {
  return param(params, "dest") === "battlefield" ? "onto the battlefield" : "into your hand";
}

function shuffleCount(params: MessageParams): string {
  return param(params, "max") === 0 ? "any number of" : `up to ${param(params, "max")}`;
}

function millPlayDuration(params: MessageParams): string {
  if (bool(params, "free_while_source")) return "for as long as this permanent remains on the battlefield";
  return bool(params, "until_next_turn") ? "until the end of your next turn" : "until end of turn";
}

function damageEachCreatureSubject(params: MessageParams): string {
  const noun = bool(params, "include_planeswalkers") ? "creature and planeswalker" : "creature";
  let subject = bool(params, "opponents_only") ? `each ${noun} your opponents control` : `each ${noun}`;
  const filter = String(param(params, "filter"));
  if (filter.includes("without_flying")) subject += " without flying";
  if (filter.includes("with_flying")) subject += " with flying";
  return subject;
}

function pumpLabel(params: MessageParams, prefix: string): string {
  const bonus = `+${param(params, "power")}/+${param(params, "toughness")}`;
  const keywords = String(param(params, "keywords"));
  const base = prefix === "" ? bonus : `${prefix} ${bonus}`;
  if (keywords === "") return `${base} until end of turn`;
  return `${base} and gains ${humanize(keywords)} until end of turn`;
}

function preventCombatDamageLabel(params: MessageParams): string {
  if (bool(params, "to_self") && bool(params, "by_self"))
    return "Prevent all combat damage that would be dealt to and dealt by this creature";
  if (bool(params, "to_self")) return "Prevent all combat damage that would be dealt to this creature";
  if (bool(params, "by_self")) return "Prevent all combat damage that would be dealt by this creature";
  return "Prevent no combat damage";
}

function staticAllLandsOfTypeBecome(params: MessageParams): string {
  const all = `All ${humanize(param(params, "land_types"))}s are`;
  if (!bool(params, "creature")) return `${all} ${humanize(param(params, "set_subtypes"))}`;
  const colors = humanize(param(params, "add_colors"));
  const color = colors === "" ? "" : `${colors} `;
  const pt = `${param(params, "base_power")}/${param(params, "base_toughness")}`;
  return `${all} ${pt} ${color}creatures that are still lands`;
}

function staticAnthem(params: MessageParams): string {
  const scope = bool(params, "self_only")
    ? "This creature"
    : bool(params, "untapped_only")
      ? "Untapped creatures you control"
      : "Creatures you control";
  const keywords = String(param(params, "keywords"));
  if (keywords !== "") return `${scope} have ${humanize(keywords)}`;
  return `${scope} get${bool(params, "self_only") ? "s" : ""} +${param(params, "power")}/+${param(params, "toughness")}`;
}

export const enCatalog: Readonly<Record<string, MessageFormatter>> = {
  "action.activate": literal("Activate"),
  "action.card_name": nameOnly,
  "action.cast_face_down": literal("Cast face down"),
  "action.cycle": (params) => `Cycle ${param(params, "name")}`,
  "action.declare_attackers": literal("Declare attackers"),
  "action.declare_blockers": literal("Declare blockers"),
  "action.discard_card": (params) => `Discard ${param(params, "name")}`,
  "action.discard_effect": (_params, children) => `Discard: ${children[0] ?? "Activate ability"}`,
  "action.encore": (params) => `Encore ${param(params, "name")}`,
  "action.keep_hand": literal("Keep hand"),
  "action.mulligan": literal("Mulligan"),
  "action.suspend": (params) => `Suspend ${param(params, "name")}`,
  "action.turn_face_up": literal("Turn face up"),
  "auto.automatic": literal("Automatic action."),
  "auto.discarded": literal("Discarded automatically."),
  "auto.discarded_to_hand_size": literal("Discarded to hand size."),
  "auto.only_one_legal_target": literal("Chose the only legal target."),
  "auto.sacrificed_forced": (params) => `Sacrificed ${param(params, "name", "a permanent")}.`,
  "auto.trigger_order_forced": literal("Ordered the only trigger sequence."),
  "card.name": nameOnly,
  "choice.option": nameOnly,
  "effect.choice_cast_creature_face_down": literal("Cast a creature card from hand face down as a 2/2"),
  "effect.choice_caster_keeps_one_of_each_type_per_player": literal(
    "For each player, you choose an artifact, a creature, an enchantment, and a planeswalker they control; each player sacrifices their other nonland permanents",
  ),
  "effect.choice_choose_basic_land_type": literal("Choose a basic land type"),
  "effect.choice_choose_color": literal("Choose a color"),
  "effect.choice_choose_creature_type": literal("Choose a creature type"),
  "effect.choice_choose_opponent": literal("Choose an opponent"),
  "effect.choice_councils_dilemma_vote": (params) =>
    `Starting with you, each player votes for ${humanize(param(params, "options"))}`,
  "effect.choice_damaging_creature_controller_may_draw": (params) =>
    `That creature's controller may draw ${param(params, "count")}`,
  "effect.choice_defending_player_sacrifices": (params) =>
    `Defending player sacrifices ${param(params, "count")} permanents of their choice`,
  "effect.choice_discard": (params) => {
    const suffix = bool(params, "random") ? " at random" : "";
    return bool(params, "target_player")
      ? `Target player discards ${param(params, "count")}${suffix}${bool(params, "or_one_matching") ? " unless they discard a land card" : ""}`
      : `Discard ${param(params, "count")}${suffix}${bool(params, "or_one_matching") ? " unless you discard a land card" : ""}`;
  },
  "effect.choice_each_other_token_becomes_copy_of_chosen": literal(
    "You may choose a token you control; if you do, each other token you control becomes a copy of that token",
  ),
  "effect.choice_each_player_controller_chooses_counter_target": literal(
    "For each player, put a +1/+1 counter on up to one creature that player controls",
  ),
  "effect.choice_each_player_creates_fractal_from_exiled_power": (params) =>
    `Each player creates a ${param(params, "token")} token with +1/+1 counters equal to the total power of creatures they controlled that were exiled this way`,
  "effect.choice_each_player_discards_hand_then_draws": (params) =>
    `Each player discards their hand, then draws ${param(params, "count")}`,
  "effect.choice_each_player_exiles_from_graveyard": literal("Each player exiles a card from their graveyard"),
  "effect.choice_each_player_names_card_then_reveals_top": literal(
    "Each player chooses a card name. Then each player reveals the top card of their library. If the card a player revealed has the name they chose, that player puts it into their hand. If it does not, that player puts it on the bottom of their library",
  ),
  "effect.choice_each_player_sacrifices": (params) =>
    `${edictWho(param(params, "scope"))} ${bool(params, "keep_one") ? "keeps one creature and sacrifices the rest" : "sacrifices a permanent"}`,
  "effect.choice_each_player_shuffles_hand_and_graveyard_then_draws": (params) =>
    `Each player shuffles their hand and graveyard into their library, then draws ${param(params, "count")}`,
  "effect.choice_join_forces_pay_mana": literal("Starting with you, each player may pay any amount of mana"),
  "effect.choice_triggering_player_may_attach_this_aura_to_chosen": (params) =>
    `That permanent's controller may attach this Aura to a ${humanize(param(params, "filter"))} of their choice`,
  "effect.choice_triggering_player_may_pay_any_amount_to_prevent": (params) =>
    `That player may pay any amount of mana; prevent that much of the next ${param(params, "amount")} damage dealt to them`,
  "effect.choice_may_discard": literal("You may discard a card"),
  "effect.choice_may_reveal_land_from_hand": literal("You may reveal a matching land card from your hand"),
  "effect.choice_may_draw_unless_pays": (params) =>
    `You may draw a card unless that player pays ${param(params, "cost")}`,
  "effect.choice_may_draw_up_to": (params) => `You may draw up to ${param(params, "count")}`,
  "effect.choice_may_draw_up_to_then_opponent_may_repeat": (params) =>
    `You may draw up to ${param(params, "count")}, then that opponent may repeat this process`,
  "effect.choice_may_exile_discarded_nonland_may_play": literal(
    "You may exile one of the discarded nonland cards; play it this turn",
  ),
  "effect.choice_may_put_counter_on_creature": literal("You may put a +1/+1 counter on a creature"),
  "effect.choice_may_return_from_graveyard": (params) =>
    `You may return ${humanize(param(params, "filter", "a card"))} from your graveyard to your hand`,
  "effect.choice_may_sacrifice": (params) => `You may sacrifice ${humanize(param(params, "filter", "a permanent"))}`,
  "effect.choice_phase_out": literal("Any number of other target creatures you control phase out"),
  "effect.choice_proliferate": (params) => `Proliferate ${param(params, "times")} times`,
  "effect.choice_put_counter_then_may_become_copy_of_card_from_list": literal(
    "Put a +1/+1 counter on this creature, then you may have this creature become a copy of an artifact or creature card from among those cards until end of turn",
  ),
  "effect.choice_discard_your_hand": literal("Discard your hand"),
  "effect.choice_each_opponent_discards": literal("Each other player discards a card"),
  "effect.choice_each_player_chooses_war_or_peace": literal("Each player chooses war or peace"),
  "effect.choice_put_creature_from_hand": literal(
    "You may put a creature card from your hand onto the battlefield. It gains haste. Sacrifice it at the beginning of the next end step",
  ),
  "effect.choice_put_creature_from_hand_attacking": literal(
    "You may put a creature card from your hand onto the battlefield tapped and attacking",
  ),
  "effect.choice_put_from_hand_on_top": (params) =>
    `Put ${param(params, "count")} cards from your hand on top of your library in any order`,
  "effect.choice_put_land_from_hand": (params) =>
    `Put a land from hand onto the battlefield${bool(params, "tapped") ? " tapped" : ""}`,
  "effect.choice_sacrifice_own": (params) =>
    `Sacrifice ${param(params, "count")} ${humanize(param(params, "filter", "permanents"))}`,
  "effect.choice_pay_or_else": (params, children) => `Pay ${param(params, "cost")} or: ${children.join(", then ")}`,
  "effect.choice_sacrifice_self_unless_return_land": literal(
    "Sacrifice this unless you return a non-Lair land you control",
  ),
  "effect.choice_set_own_color_until_end_of_turn": literal("Become the color of your choice until end of turn"),
  "effect.choice_target_player_exiles_from_graveyard": literal("Target player exiles a card from their graveyard"),
  "effect.choice_target_player_may_draw": (params) => `Target player may draw ${param(params, "count")}`,
  "effect.choose_one": (_params, children) => `Choose one -- ${children.join(" • ")}`,
  "effect.conditional": (_params, children) => children.join(", then "),
  "effect.control_attach_self_to_entering": literal("Attach this to that creature"),
  "effect.control_equip": literal("Equip"),
  "effect.control_exchange_all_creatures_until_end_of_turn": literal(
    "You and target opponent each gain control of all creatures the other controls until end of turn",
  ),
  "effect.control_exchange_control": literal(
    "Exchange control of target permanent you control and target permanent an opponent controls",
  ),
  "effect.control_gain_control": literal("Gain control of target creature"),
  "effect.control_gain_control_all_until_end_of_turn": literal(
    "Untap all creatures and gain control of them until end of turn",
  ),
  "effect.control_gain_control_until_end_of_turn": literal("Gain control of target creature until end of turn"),
  "effect.control_gain_control_while": literal(
    "Gain control of target creature for as long as you control this and it remains tapped",
  ),
  "effect.control_goad_target": literal("Goad target creature"),
  "effect.control_grant_source_abilities_until_end_of_turn": literal(
    "It gains this creature's other abilities until end of turn",
  ),
  "effect.control_regenerate_shield": literal("Regenerate target"),
  "effect.control_remove_from_combat": literal("Remove target from combat"),
  "effect.control_revert_all_creatures_to_owners": literal("Each player gains control of all creatures they own"),
  "effect.control_tap_source": literal("Tap this permanent"),
  "effect.control_tap_target": literal("Tap target"),
  "effect.control_target_opponent_gains_control": literal(
    "Target opponent gains control of target permanent you control",
  ),
  "effect.control_tap_all": (params) => `Tap all ${humanize(param(params, "filter", "permanents"))} you control`,
  "effect.control_tap_all_target_player_controls": (params) =>
    `Tap all ${humanize(param(params, "filter", "permanents"))} target player controls`,
  "effect.control_untap_all": (params) => `Untap all ${humanize(param(params, "filter", "permanents"))} you control`,
  "effect.control_untap_target": literal("Untap target"),
  "effect.copy_change_target_of_target_spell_or_ability": (params) =>
    bool(params, "optional")
      ? "You may choose new targets for target instant or sorcery spell"
      : "Change the target of target spell or ability with a single target",
  "effect.copy_copy_triggering_ability": literal("Copy that ability"),
  "effect.copy_copy_triggering_spell": (params) => `Copy it ${param(params, "count")} times`,
  "effect.copy_copy_triggering_spell_for_each_other_creature_you_control": literal(
    "Copy it for each other creature you control it could target",
  ),
  "effect.copy_demonstrate": literal("Demonstrate"),
  "effect.copy_may_pay_to_copy_this": (params) =>
    `That player or that permanent's controller may pay ${param(params, "cost")} to copy this`,
  "effect.copy_mint_free_copy_of_exiled_card": literal(
    "Copy the exiled card; you may cast the copy without paying its mana cost",
  ),
  "effect.copy_retarget_spell_copy": literal("Choose new targets for the copy"),
  "effect.copy_target_spell": literal("Copy target spell"),
  "effect.copy_this_spell": literal("Copy this spell"),
  "effect.counters_attacker_draws_controller_counters": (params) =>
    `Attacking player draws; put ${param(params, "counters")} +1/+1 counters on a creature`,
  "effect.counters_commander_enters_with_bonus_counters": (params) =>
    `It enters with ${param(params, "count")} additional +1/+1 counters on it`,
  "effect.counters_double_counters": literal("Double its +1/+1 counters"),
  "effect.counters_double_counters_on_attached_creature": literal("Double the +1/+1 counters on equipped creature"),
  "effect.counters_double_counters_on_target_creatures": literal(
    "Double the number of +1/+1 counters on any number of other target creatures",
  ),
  "effect.counters_level_up": (params) => `Level ${param(params, "level")}`,
  "effect.counters_monstrosity": (params) => `Monstrosity ${param(params, "count")}`,
  "effect.counters_put_counters_on_player": (params) =>
    `${edictWho(param(params, "scope"))} gets ${param(params, "count")} ${humanize(param(params, "kind"))} counters`,
  "effect.counters_put_loyalty_counter_each": literal("Put a loyalty counter on each"),
  "effect.counters_remove_all_but_one_plus_one_counter_then_gain_life": literal(
    "Remove all but one +1/+1 counter, gain 1 life for each removed",
  ),
  "effect.counters_remove_all_player_counters": (params) => `${edictWho(param(params, "scope"))} loses all counters`,
  "effect.counters_top_up_counters_on_player": (params) =>
    `Give target player ${humanize(param(params, "kind"))} counters up to ${param(params, "to")}`,
  "effect.counters_move_counters": (params) =>
    bool(params, "all_kinds")
      ? "Move all counters onto another permanent"
      : "Move +1/+1 counters onto another permanent",
  "effect.counters_place_vow_counters": literal("Put a vow counter on each surviving creature"),
  "effect.counters_put_counters": (params) =>
    params.kind == null
      ? `Put ${param(params, "count")} +1/+1 counters`
      : `Put ${param(params, "count")} ${humanize(param(params, "kind"))} counters`,
  "effect.counters_put_counters_each": (params) => `Put ${param(params, "count")} +1/+1 counters on each`,
  "effect.counters_remove_all_counters_then_draw": literal("Remove all counters, draw a card for each removed"),
  "effect.counters_remove_counter_from_self": (params) =>
    params.kind === "plus_one_plus_one"
      ? "Remove a +1/+1 counter from it"
      : `Remove a ${humanize(param(params, "kind"))} counter from it`,
  "effect.damage_each_creature": (params) =>
    `Deal ${param(params, "amount")} damage to ${damageEachCreatureSubject(params)}`,
  "effect.damage_each_opponent": (params) => `Deal ${param(params, "amount")} damage to each opponent`,
  "effect.damage_each_other_opponent": (params) => `Deal ${param(params, "amount")} damage to each other opponent`,
  "effect.damage_each_player": (params) => `Deal ${param(params, "amount")} damage to each player`,
  "effect.damage_radiance": (params) =>
    `Deal ${param(params, "amount")} damage to target creature and each other creature that shares a color with it`,
  "effect.damage_target": (params) => `Deal ${param(params, "amount")} damage`,
  "effect.damage_to_dying_enchanted_creatures_controller": (params) =>
    `Deals ${param(params, "amount")} damage to that creature's controller`,
  "effect.damage_to_entering_permanent": (params) =>
    `Deal ${param(params, "amount")} damage to the permanent that entered`,
  "effect.damage_to_entering_permanent_controller": (params) =>
    `Deals ${param(params, "amount")} damage to that permanent's controller`,
  "effect.damage_to_self": (params) => `Deals ${param(params, "amount")} damage to you`,
  "effect.damage_to_target_controller": (params) =>
    `Deals ${param(params, "amount")} damage to that creature's controller`,
  "effect.damage_to_triggering_player": (params) => `Deals ${param(params, "amount")} damage to that player`,
  "effect.destroy_all": (params) => `Destroy all ${humanize(param(params, "filter", "permanents"))}`,
  "effect.destroy_target": literal("Destroy target"),
  "effect.destroy_triggering_damaged_creature": literal("Destroy that creature"),
  "effect.dig_cascade": literal("Cascade"),
  "effect.dig_cash_out_exiled_with_this": literal("Put a card exiled with this into its owner's graveyard"),
  "effect.dig_cast_exiled_with_this_free": literal(
    "Choose target card exiled with this; you may cast it this turn without paying its mana cost",
  ),
  "effect.dig_clash": literal("Clash with an opponent"),
  "effect.dig_distribute_top": (params) =>
    `Look at the top ${param(params, "count")} cards, put ${param(params, "to_hand")} into your hand, ${param(params, "to_bottom")} on the bottom, and exile ${param(params, "to_exile_may_play")} (you may play it this turn)`,
  "effect.dig_each_player_exiles_until_nonland_opponent_picks": literal(
    "Each player exiles cards from the top of their library until they exile a nonland card. An opponent chooses a nonland card exiled this way. You may cast up to two of the other exiled cards without paying their mana costs",
  ),
  "effect.dig_exile_random_from_graveyard_may_play": literal(
    "Exile a card from your graveyard at random; you may play it this turn",
  ),
  "effect.dig_exile_target_graveyard_card_record_mana_value": (params) =>
    `Exile target ${humanize(param(params, "filter", "card"))} from your graveyard`,
  "effect.dig_exile_target_graveyard_spell_cast_free": (params) =>
    `Exile up to one target ${humanize(param(params, "filter", "card"))} from your graveyard; you may cast it without paying its mana cost`,
  "effect.dig_exile_top_cast_matching_free": (params) =>
    `Exile the top ${param(params, "count")} card(s); you may cast ${humanize(param(params, "filter", "a card"))} from among them without paying its mana cost. Put the rest on the bottom of your library`,
  "effect.dig_exile_top_until_stop_cast_free_under_budget": (params) =>
    `As many times as you choose, you may exile the top card of your library. If the total mana value of the cards exiled this way is ${param(params, "budget")} or less, you may cast any number of spells from among those cards without paying their mana costs`,
  "effect.dig_look_at_target_players_hand": literal("Look at target player's hand"),
  "effect.dig_look_at_top": (params) =>
    `Look at the top ${param(params, "count")} cards, put up to ${param(params, "up_to")} ${topDest(params)}, rest on the bottom`,
  "effect.dig_may_shuffle_target_players_library": literal("You may have that player shuffle"),
  "effect.dig_opponent_splits_exile_piles": literal(
    "Exile the top four cards in one pile, then the top four in a second pile. An opponent chooses one pile; put it into your graveyard. You may cast a card from the other pile without paying its mana cost; put the rest into your hand",
  ),
  "effect.dig_rearrange_target_players_top": (params) =>
    `Look at the top ${param(params, "count")} cards of target player's library, then put them back in any order`,
  "effect.dig_reveal_top_opponent_picks_one_to_graveyard": (params) =>
    `Reveal the top ${param(params, "count")} cards of your library. An opponent chooses one of them. Put that card into your graveyard and the rest into your hand`,
  "effect.dig_reveal_top_split_piles": literal(
    "Reveal the top five cards of your library. An opponent separates those cards into two piles. Put one pile into your hand and the other into your graveyard",
  ),
  "effect.dig_reveal_until_exile_cast_free": (params) =>
    `Reveal cards from the top of your library until you reveal ${humanize(param(params, "filter", "a card"))}. Exile that card and put the rest on the bottom of your library. You may cast the exiled card without paying its mana cost`,
  "effect.dig_reveal_until_may_deploy": (params) =>
    `Reveal cards from the top of your library until you reveal ${humanize(param(params, "filter", "a card"))}. You may put that card onto the battlefield. If you do not, put it into your hand. Put the rest on the bottom of your library`,
  "effect.dig_search_library": (params) =>
    `Search your library for ${humanize(param(params, "filter", "a card"))}, put it ${searchDest(param(params, "to_zone"))}`,
  "effect.dig_shuffle_library": literal("Shuffle your library"),
  "effect.dig_shuffle_target_cards_from_graveyard_into_library": (params) =>
    `${bool(params, "target_player") ? "Target player shuffles" : "Shuffle"} ${shuffleCount(params)} target cards from ${bool(params, "target_player") ? "their" : "your"} graveyard into ${bool(params, "target_player") ? "their" : "your"} library`,
  "effect.dig_surveil": (params) => `Surveil ${param(params, "count")}`,
  "effect.discard": (params) => `Discard ${param(params, "count")}`,
  "effect.draw_attacking_player": (params) => `The attacking player draws ${param(params, "count")}`,
  "effect.draw_cards": (params) => `Draw ${param(params, "count")}`,
  "effect.draw_each_draw_step_player": (params) => `That player draws ${param(params, "count")}`,
  "effect.draw_each_player": (params) => `Each player draws ${param(params, "count")}`,
  "effect.draw_target_owner": (params) =>
    `${bool(params, "controller") ? "That target's controller" : "That target's owner"} draws ${param(params, "count")}`,
  "effect.draw_target_player": (params) =>
    `${bool(params, "opponent") ? "Target opponent" : "Target player"} draws ${param(params, "count")}`,
  "effect.exile_all": (params) => `Exile all ${humanize(param(params, "filter", "permanents"))}`,
  "effect.exile_all_graveyards": literal("Exile all graveyards"),
  "effect.exile_graveyard": literal("Exile target player's graveyard"),
  "effect.exile_object": literal("Exile it"),
  "effect.exile_target": literal("Exile target"),
  "effect.exile_target_minting_illusion_on_leave": literal("Exile target"),
  "effect.exile_until_source_leaves": literal("Exile target until this leaves the battlefield"),
  "effect.life_attacker_loses_you_draw": (params) =>
    `That opponent loses ${param(params, "life_loss")} life; you draw a card`,
  "effect.life_attacker_loses_you_gain": (params) =>
    `Enchanted creature's controller loses ${param(params, "amount")} life; you gain ${param(params, "amount")}`,
  "effect.life_drain_target": (params) =>
    `Target player loses ${param(params, "amount")}, you gain ${param(params, "amount")}`,
  "effect.life_each_opponent_drain": (params) =>
    `Each opponent loses ${param(params, "amount")}, you gain ${bool(params, "sum_gain") ? "life equal to the life lost this way" : param(params, "amount")}`,
  "effect.life_each_opponent_loses": (params) => `Each opponent loses ${param(params, "amount")}`,
  "effect.life_each_player_loses": (params) => `Each player loses ${param(params, "amount")}`,
  "effect.life_each_player_becomes_highest": literal(
    "Each player's life total becomes the highest life total among all players",
  ),
  "effect.life_gain": (params) => `Gain ${param(params, "amount")} life`,
  "effect.life_gain_target_controller": (params) => `Target's controller gains ${param(params, "amount")} life`,
  "effect.life_lose": (params) => `Lose ${param(params, "amount")} life`,
  "effect.life_opponent_gains": (params) => `An opponent gains ${param(params, "amount")} life`,
  "effect.life_source_owner_loses_half_their_life": literal("Its owner loses half their life, rounded up"),
  "effect.life_target_player_gains": (params) => `Target player gains ${param(params, "amount")} life`,
  "effect.life_target_player_loses": (params) => `Target player loses ${param(params, "amount")} life`,
  "effect.mana_add": literal("Add mana"),
  "effect.mana_lose_all_unspent": (params) =>
    bool(params, "to_you")
      ? "That player loses all unspent mana and you add the mana lost this way"
      : "That player loses all unspent mana",
  "effect.mana_target_player_taps_lands_for_mana": literal(
    "Target player activates a mana ability of each land they control",
  ),
  "effect.mill_exile_discarded_with_this": literal("Exile that card from your graveyard with this"),
  "effect.mill_exile_from_graveyard_may_play": literal("Exile that card from your graveyard; play it this turn"),
  "effect.mill_exile_target_from_graveyard_create_token_copy": (params) =>
    `Exile target ${humanize(param(params, "filter", "card"))} from your graveyard. Create a token that's a copy of it`,
  "effect.mill_exile_target_from_graveyard_with_this": literal(
    "Exile target noncreature, nonland card from your graveyard",
  ),
  "effect.mill_exile_top_may_play": (params) =>
    `Exile the top ${param(params, "count")} card(s)${bool(params, "face_down") ? " face down" : ""}; play ${millPlayDuration(params)}${bool(params, "free_while_source") ? " without paying its mana cost" : ""}`,
  "effect.mill_mill": (params) => `Target player mills ${param(params, "count")}`,
  "effect.mill_mill_self": (params) => `Mill ${param(params, "count")}`,
  "effect.misc_arm_combat_damage_watch": literal(
    "Arm a delayed watch: this creature becomes prepared when target creature deals combat damage to a player this combat",
  ),
  "effect.misc_become_prepared": literal("Become prepared"),
  "effect.misc_counter_target_activated_ability": literal("Counter target activated ability"),
  "effect.misc_counter_target_spell": (params) =>
    `Counter target ${humanize(param(params, "filter", "spell"))}${params.unless_pays == null ? "" : ` unless its controller pays ${param(params, "unless_pays")}`}`,
  "effect.misc_fight": (params) =>
    bool(params, "ally_is_shared_target")
      ? "Then it fights up to one target creature you do not control"
      : "Target creature you control fights target creature you do not control",
  "effect.misc_flip_source": literal("Flip this permanent"),
  "effect.misc_get_emblem": literal("You get an emblem"),
  "effect.misc_grant_channel_colorless_mana_this_turn": literal(
    "Until end of turn, any time you could activate a mana ability, you may pay 1 life. If you do, add {C}",
  ),
  "effect.misc_grant_flash_this_turn": literal("You may cast spells this turn as though they had flash"),
  "effect.misc_must_attack_all": literal("Creatures the active player controls attack this turn if able"),
  "effect.misc_must_attack_target": literal("Target creature attacks this turn if able"),
  "effect.misc_you_choose_which_creatures_attack": literal("You choose which creatures attack this turn"),
  "effect.misc_you_choose_which_creatures_block": literal(
    "You choose which creatures block this turn and how those creatures block",
  ),
  "effect.misc_must_attack_random_opponent": literal(
    "Choose an opponent at random. This attacks that player this combat if able",
  ),
  "effect.misc_prevent_all_combat_damage_this_turn": literal("Prevent all combat damage that would be dealt this turn"),
  "effect.misc_prevent_next_damage": literal("Prevent the next N damage that would be dealt to any target this turn"),
  "effect.misc_prevent_combat_damage_to_you_creating_tokens": literal(
    "Prevent all combat damage that would be dealt to you this turn, creating a token per point prevented",
  ),
  "effect.misc_schedule_at_next_upkeep": (_params, children) => `Delayed: ${children[0] ?? "resolve effect"}`,
  "effect.misc_schedule_colorless_mana_for_countered_spell_next_main_phase": literal(
    "Add {C} for each mana in that spell's mana cost at your next main phase",
  ),
  "effect.misc_schedule_next_cast_trigger": (_params, children) =>
    `When you next cast a spell this turn: ${children.join(", then ")}`,
  "effect.misc_schedule_this_turn_combat_damage_copy": literal(
    "Whenever a creature you control deals combat damage to a player this turn, copy the exiled card; you may cast the copy without paying its mana cost",
  ),
  "effect.misc_skip_next_untap_opponent_creatures": literal(
    "Creatures your opponents control do not untap during their next untap steps",
  ),
  "effect.misc_take_extra_turn": literal("Take an extra turn after this one"),
  "effect.pump_animate_self_until_end_of_turn": (params) =>
    `Becomes a ${param(params, "base_power")}/${param(params, "base_toughness")} creature until end of turn`,
  "effect.pump_enchanted_attacker_pump_attacking_opponent_else_controller_loses_life": (params) =>
    `It gets +${param(params, "power")}/+${param(params, "toughness")} until end of turn if it's attacking one of your opponents. Otherwise, its controller loses ${param(params, "life")} life`,
  "effect.pump_enchanted_creature_loses_keywords": (params) =>
    `Enchanted creature loses ${humanize(param(params, "keywords"))}`,
  "effect.pump_grant_chosen_color_protection_until_end_of_turn": literal(
    "Target creature you control gains protection from the color of your choice until end of turn",
  ),
  "effect.pump_grant_keywords_to_permanents_you_control_until_end_of_turn": (params) =>
    `Permanents you control gain ${humanize(param(params, "keywords"))} until end of turn`,
  "effect.pump_pump_creatures_you_control_until_end_of_turn": (params) =>
    pumpLabel(params, "Creatures you control get"),
  "effect.pump_pump_each_creature_until_end_of_turn": (params) => pumpLabel(params, "Each creature gets"),
  "effect.pump_pump_other_attackers_attacking_your_opponents": (params) =>
    `Each other creature that's attacking one of your opponents gets +${param(params, "power")}/+${param(params, "toughness")} until end of turn`,
  "effect.pump_pump_self_until_end_of_turn": (params) => pumpLabel(params, ""),
  "effect.pump_pump_until_end_of_turn": (params) => pumpLabel(params, ""),
  "effect.pump_radiance_chosen_color_protection_until_end_of_turn": literal(
    "Target creature and each other creature that shares a color with it gain protection from the chosen color until end of turn",
  ),
  "effect.pump_set_base_pt_creatures_you_control_until_end_of_turn": (params) =>
    `${bool(params, "other") ? "Other creatures" : "Creatures"} you control have base power and toughness ${param(params, "power")}/${param(params, "toughness")} until end of turn`,
  "effect.pump_set_base_pt_target_until_end_of_turn": (params) =>
    `Target creature has base power and toughness ${param(params, "power")}/${param(params, "toughness")} until end of turn`,
  "effect.pump_set_own_base_pt_from_amount": (params) =>
    `This creature has base power and toughness each equal to ${param(params, "amount")}`,
  "effect.pump_strip_keywords_from_opponents_creatures": (params) =>
    `Creatures your opponents control lose ${humanize(param(params, "keywords"))} until end of turn and can't have ${humanize(param(params, "keywords"))} this turn`,
  "effect.pump_target_becomes_color": (params) => `Target spell or permanent becomes ${param(params, "color")}`,
  "effect.pump_target_becomes_subtypes_while_source_remains": (params) =>
    `Target land becomes a ${humanize(param(params, "set_subtypes"))} until this permanent leaves the battlefield`,
  "effect.pump_target_becomes_treasure": literal(
    'Target creature becomes a Treasure artifact with "{T}, Sacrifice this artifact: Add one mana of any color" and loses all other card types and abilities',
  ),
  "effect.pump_weaken_each_creature": (params) =>
    `${bool(params, "opponents_only") ? "Creatures your opponents control get" : "Each creature gets"} -${param(params, "power")}/-${param(params, "toughness")} until end of turn`,
  "effect.reveal_top_and_drain_mutual": literal(
    "You and target opponent each reveal the top card of your library, lose life equal to the mana value of the other's, and put it into your hand",
  ),
  "effect.reveal_top_cards": (params) =>
    `Reveal the top ${param(params, "count")} cards of your library, put all cards among them that are ${humanize(param(params, "filter", "cards"))} ${searchDest(param(params, "matched_dest"))}, and put the rest on the bottom of your library`,
  "effect.reveal_top_to_hand": (params) =>
    `Defending player reveals the top card of their library; if it's ${humanize(param(params, "filter", "a card"))}, put it into their hand`,
  "effect.reveal_until": (params) =>
    `Reveal cards from the top of your library until you reveal ${param(params, "count")} ${humanize(param(params, "filter", "card"))}, put them ${searchDest(param(params, "matched_dest"))}, and put the rest on the bottom of your library`,
  "effect.sacrifice_enchanted_creature": literal("That creature's controller sacrifices it"),
  "effect.sacrifice_object": literal("Sacrifice it"),
  "effect.sacrifice_source": literal("Sacrifice it"),
  "effect.scry": (params) => `Scry ${param(params, "count")}`,
  "effect.sequence": (_params, children) => children.join(", then "),
  "effect.static_all_lands_of_type_become": (params) => staticAllLandsOfTypeBecome(params),
  "effect.static_anthem": (params) => staticAnthem(params),
  "effect.static_attack_tax": (params) =>
    `Creatures can't attack you unless their controller pays {${param(params, "amount")}} for each creature they control that's attacking you`,
  "effect.static_base_power_toughness_from_amount": (params) =>
    `${definingPtLead(param(params, "when"))} power and toughness are each equal to ${param(params, "power")}`,
  "effect.static_cant_attack_if_cast_this_turn": literal(
    "Each opponent who cast a spell this turn can't attack with creatures",
  ),
  "effect.static_cant_attack_unless_defender_controls": (params) =>
    `This creature can't attack unless defending player controls ${param(params, "filter")}`,
  "effect.static_cant_be_attacked_by": (params) => `${humanize(param(params, "filter", "Creatures"))} can't attack you`,
  "effect.static_may_skip_draw_for_cant_be_attacked_by": (params) =>
    `You may skip your draw-step draw; if you do, ${humanize(param(params, "filter", "creatures"))} can't attack you until your next turn`,
  "effect.static_cant_block_filter": (params) => `${humanize(param(params, "filter", "Creatures"))} can't block`,
  "effect.static_cant_cast_during_combat": literal("Players can't cast spells during combat"),
  "effect.static_cant_cast_if_attacked_this_turn": literal(
    "Each opponent who attacked with a creature this turn can't cast spells",
  ),
  "effect.static_discard_to_library_top_instead": literal(
    "If an effect causes you to discard a card, discard it, but you may put it on top of your library instead of into your graveyard",
  ),
  "effect.static_doesnt_untap": (params) =>
    bool(params, "self_only")
      ? "This permanent doesn't untap during your untap step"
      : `${humanize(param(params, "filter", "Permanents"))} don't untap during their controllers' untap steps`,
  "effect.static_must_attack_each_combat": literal("All creatures attack each combat if able"),
  "effect.static_opponents_cant_search_libraries": literal("Your opponents can't search libraries"),
  "effect.static_protection_from_chosen_color": literal("This creature has protection from the chosen color"),
  "effect.static_cast_x_replacement": (params) => `value of X: X x ${param(params, "times")}`,
  "effect.static_control_attached": literal("You control enchanted creature"),
  "effect.static_counter_replacement": (params) =>
    `+1/+1 counters placed: (n + ${param(params, "add")}) x ${param(params, "times")}`,
  "effect.static_counter_scaled_attack_tax": literal(
    "Creatures with counters on them can't attack you unless their controller pays generic mana equal to their counters",
  ),
  "effect.static_creatures_you_control_enter_with_counters": (params) =>
    `${humanize(param(params, "filter", "Creatures"))} you control enter with ${param(params, "count")} additional +1/+1 counters`,
  "effect.static_enters_with_counters": (params) =>
    params.kind == null
      ? `Enters with ${param(params, "amount")} +1/+1 counters`
      : `Enters with ${param(params, "amount")} ${humanize(param(params, "kind"))} counters`,
  "effect.static_grant_activated_ability": (params) =>
    `${humanize(param(params, "filter", "Permanents"))} gain an activated ability`,
  "effect.static_grant_mana_ability": (params) =>
    `${humanize(param(params, "filter", "Artifacts"))} you control gain a mana ability`,
  "effect.static_grant_to_attached": (params) =>
    `Attached creature gets +${param(params, "power")}/+${param(params, "toughness")}`,
  "effect.static_keyword_anthem": (params) =>
    `${bool(params, "all_players") ? "All permanents" : "Permanents you control"} have ${humanize(param(params, "keywords"))}`,
  "effect.static_life_gain_replacement": (params) => `life gained: n + ${param(params, "plus")}`,
  "effect.static_no_maximum_hand_size": literal("You have no maximum hand size"),
  "effect.static_play_any_number_of_lands": literal("You may play any number of lands on each of your turns"),
  "effect.static_play_from_graveyard_once_per_turn": literal(
    "Once during each of your turns, you may play a land or cast a permanent spell with mana value 3 or less from your graveyard",
  ),
  "effect.static_prevent_combat_damage": (params) => preventCombatDamageLabel(params),
  "effect.static_prevent_damage_to_self_removing_counter": literal(
    "If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature",
  ),
  "effect.static_prevent_damage_to_self_removing_counter_per_point": literal(
    "For each 1 damage that would be dealt to this creature, if it has a +1/+1 counter on it, remove a +1/+1 counter from it and prevent that 1 damage",
  ),
  "effect.static_prevent_damage_to_self_removing_counters_giving_rad": literal(
    "If damage would be dealt to this creature while it has a +1/+1 counter on it, prevent that damage, remove that many +1/+1 counters from it, then give each player a rad counter for each +1/+1 counter removed this way",
  ),
  "effect.static_prevent_noncombat_damage_to_other_creatures_you_control": literal(
    "Prevent all noncombat damage that would be dealt to other creatures you control",
  ),
  "effect.static_redirect_unblocked_damage_to_self": literal(
    "As long as this creature is untapped, all damage that would be dealt to you by unblocked creatures is dealt to this creature instead",
  ),
  "effect.static_reduce_spell_cost": (params) =>
    `${bool(params, "first_x_spell_each_turn") ? "The first spell you cast with {X} in its mana cost each turn" : humanize(param(params, "filter", "Spells you cast"))} cost {${param(params, "amount")}} less`,
  "effect.static_tax_spell_cost": (params) =>
    `${humanize(param(params, "filter", "Spells"))} cost {${param(params, "amount")}} more to cast`,
  "effect.static_tax_activated_ability": (params) =>
    `Activated abilities of ${humanize(param(params, "filter", "permanents"))} cost {${param(params, "amount")}} more to activate`,
  "effect.static_set_attached_base_pt": (params) => {
    const power = param(params, "power");
    const toughness = param(params, "toughness");
    // Animate Artifact sets both halves from one count ("each equal to its mana value"); reading
    // that back as "its mana value/its mana value" is noise.
    return power === toughness && typeof power !== "number"
      ? `Attached permanent has base power and toughness each equal to ${power}`
      : `Attached permanent has base power and toughness ${power}/${toughness}`;
  },
  "effect.static_set_attached_types": (params) => `Attached creature is a ${humanize(param(params, "subtypes"))}`,
  "effect.static_spend_mana_as_though_another_color": (params) =>
    `You may spend ${param(params, "from")} mana as though it were ${param(params, "to")} mana`,
  "effect.static_tapped_for_mana_bonus": (params) => {
    const bonus = param(params, "bonus_color");
    const added =
      bonus === "any_color"
        ? "one mana of any color"
        : bonus === "produced"
          ? "one mana of any type that land produced"
          : `one ${bonus} mana`;
    const scope = param(params, "scope");
    if (scope === "enchanted_host") {
      return `Whenever enchanted land is tapped for mana, its controller adds an additional ${added}`;
    }
    if (scope === "any_land") {
      return `Whenever ${humanize(param(params, "filter", "lands"))} are tapped for mana, their controller adds an additional ${added}`;
    }
    return `Whenever you tap a land for mana, add an additional ${added}`;
  },
  "effect.static_token_replacement": (params) => `tokens created: n x ${param(params, "times")}`,
  "effect.static_trigger_doubling": literal("That triggered ability triggers an additional time"),
  "effect.token_become_copy_of_target_creature_gaining_myriad": literal(
    "This creature becomes a copy of up to one target nonlegendary creature you control until end of turn, except it has myriad",
  ),
  "effect.token_copy_each_entered_this_turn_token_tapped_attacking": literal(
    "For each creature token you control that entered this turn, create a tapped and attacking copy of it; sacrifice those tokens at the beginning of the next end step",
  ),
  "effect.token_create": (params) =>
    `Create ${param(params, "count", 1)} ${humanize(param(params, "token", "token"))} token(s)`,
  "effect.token_create_copy": (params) =>
    `Create ${param(params, "count", 1)} token copy/copies of ${bool(params, "entering") ? "that creature" : "target creature"}${bool(params, "sacrifice_at_next_end_step") ? "; sacrifice it at the beginning of the next end step" : ""}${bool(params, "exile_at_next_end_step") ? "; exile it at the beginning of the next end step" : ""}`,
  "effect.token_create_treasure": (params) =>
    `${bool(params, "target_player") ? "Target player creates" : "Create"} ${param(params, "count", 1)} Treasure token(s)`,
  "effect.token_myriad_token_copies": literal(
    "For each opponent other than the defending player, create a token copy that's tapped and attacking that opponent; exile the tokens at the end of combat",
  ),
  "effect.zone_attach_minted_aura_to_target": literal("Attach the token to target creature an opponent controls"),
  "effect.zone_attach_self_to_minted_token": literal("Attach this to the token"),
  "effect.zone_attach_self_to_reanimated": literal("Attach this to it"),
  "effect.zone_attach_triggering_aura_to_minted_token": literal("Attach it to the token"),
  "effect.zone_exile_dead_creature_create_copy_with_subtype": (params) =>
    params.add_subtypes == null
      ? "Exile it, then create a token that's a copy of it"
      : `Exile it, then create a token that's a copy of it that's a ${humanize(param(params, "add_subtypes"))}`,
  "effect.zone_exile_graveyard_object_gain_life": (params) => `Exile it and gain ${param(params, "amount")} life`,
  "effect.zone_exile_self_on_resolve": literal("Exile this"),
  "effect.zone_exile_self_with_time_counters": (params) =>
    `Exile this with ${param(params, "counters")} time counters on it`,
  "effect.zone_exile_target_graveyard_card_then_if_creature": (_params, children) =>
    `Exile target card from a graveyard. If a creature card is exiled this way, ${children.join(", then ")}`,
  "effect.zone_flicker_target": (params) =>
    bool(params, "delayed")
      ? "Exile target creature. Return that card to the battlefield under its owner's control at the beginning of the next end step"
      : "Exile target creature, then return it to the battlefield under its owner's control",
  "effect.zone_manifest": literal("Its controller manifests the top card of their library"),
  "effect.zone_mass_return_from_graveyard": (params) =>
    bool(params, "all_players")
      ? `Each player returns all ${humanize(param(params, "filter", "cards"))} from their graveyard to the battlefield`
      : `Return all ${humanize(param(params, "filter", "cards"))} from your graveyard to the battlefield`,
  "effect.zone_reanimate_dying_enchanted_creature": (params) =>
    bool(params, "under_owner")
      ? "Return that card to the battlefield under its owner's control"
      : "Return it to the battlefield under your control",
  "effect.zone_reanimate_random_from_target_opponent_graveyard": literal(
    "Reanimate a random creature from target opponent's graveyard",
  ),
  "effect.zone_reanimate_to_battlefield": literal("Reanimate to battlefield"),
  "effect.zone_reflexive_trigger": (_params, children) => children[0] ?? "",
  "effect.zone_return_all_to_hand": (params) =>
    `Return all ${humanize(param(params, "filter", "permanents"))} to their owners' hands`,
  "effect.zone_return_exiled_card_to_owners_graveyard": literal("Return the exiled card to its owner's graveyard"),
  "effect.zone_return_flickered_card": literal("Return that card to the battlefield under its owner's control"),
  "effect.zone_return_from_graveyard_attached_to_token": (params) =>
    `Return up to one ${humanize(param(params, "filter", "card"))} from your graveyard to the battlefield attached to that token`,
  "effect.zone_return_from_graveyard_to_hand": literal("Return from graveyard to hand"),
  "effect.zone_return_object_to_hand": literal("Return it to your hand"),
  "effect.zone_return_this_aura_attached_to": literal("Return this to the battlefield attached to that creature"),
  "effect.zone_return_this_aura_from_graveyard_attached_to_chosen_host": literal(
    "Return this from your graveyard to the battlefield",
  ),
  "effect.zone_return_this_from_graveyard_to_battlefield": (params) =>
    `Return this card from your graveyard to the battlefield${bool(params, "tapped") ? " tapped" : ""}`,
  "effect.zone_return_this_to_hand": literal("Return this card to its owner's hand"),
  "effect.zone_return_to_hand": literal("Return to owner's hand"),
  "effect.zone_schedule_return_reanimated_to_hand": literal(
    "That creature gains haste. Return it to your hand at the beginning of the next end step",
  ),
  "effect.zone_schedule_return_this_aura_attached_to_reanimated": literal(
    "Return this to the battlefield attached to that creature at the beginning of the next end step",
  ),
  "effect.zone_schedule_return_this_aura_from_graveyard_attached_to_chosen_host": literal(
    "Return this to the battlefield at the beginning of the next end step",
  ),
  "effect.zone_shuffle_target_permanent_into_library": literal(
    "The owner of target permanent shuffles it into their library",
  ),
  "effect.zone_shuffle_target_permanent_into_library_then_reveal": literal(
    "The owner of target permanent shuffles it into their library, then reveals the top card of their library. If it's a permanent card, they put it onto the battlefield",
  ),
  "effect.zone_tuck_from_graveyard": (params) =>
    bool(params, "to_top") ? "Put graveyard card on top of library" : "Put graveyard card on bottom of library",
  "effect.zone_tuck_permanent_into_library": (params) =>
    bool(params, "to_top")
      ? "Put target permanent on top of its owner's library"
      : "Put target permanent on the bottom of its owner's library",
  "effect.zone_tuck_self_and_blocked_creatures": literal(
    "Put this creature and each creature it's blocking on top of their owners' libraries, then those players shuffle",
  ),
  "effect.zone_tuck_self_to_library_bottom": literal("Put this on the bottom of its owner's library"),
  "effect.zone_untap_searched_land": literal("Untap the searched land"),
  "keyword.can_block_only_flyers": literal("Can block only creatures with flying"),
  "keyword.cant_block": literal("Can't block"),
  "keyword.deathtouch": literal("Deathtouch"),
  "keyword.decayed": literal("Decayed"),
  "keyword.defender": literal("Defender"),
  "keyword.double_strike": literal("Double strike"),
  "keyword.fear": literal("Fear"),
  "keyword.first_strike": literal("First strike"),
  "keyword.flash": literal("Flash"),
  "keyword.flying": literal("Flying"),
  "keyword.haste": literal("Haste"),
  "keyword.hexproof": literal("Hexproof"),
  "keyword.indestructible": literal("Indestructible"),
  "keyword.landwalk": (params) => `${param(params, "land")}walk`,
  "keyword.lesser_power_cant_block": literal("Lesser-power creatures can't block it"),
  "keyword.lifelink": literal("Lifelink"),
  "keyword.menace": literal("Menace"),
  "keyword.myriad": literal("Myriad"),
  "keyword.protection_from": (params) => `Protection from ${humanize(param(params, "scope"))}`,
  "keyword.prowess": literal("Prowess"),
  "keyword.reach": literal("Reach"),
  "keyword.shadow": literal("Shadow"),
  "keyword.shroud": literal("Shroud"),
  "keyword.skulk": literal("Skulk"),
  "keyword.trample": literal("Trample"),
  "keyword.unblockable": literal("Unblockable"),
  "keyword.vigilance": literal("Vigilance"),
  "keyword.ward": (params) => `Ward {${param(params, "amount")}}`,
  "reject.cannot_activate": literal("That ability isn't available."),
  "reject.cannot_pay_cost": literal("Not enough mana for that."),
  "reject.cannot_produce_mana": literal("That can't make mana right now."),
  "reject.choice_pending": literal("Resolve the current choice first."),
  "reject.engine_error": literal("Something went wrong resolving that."),
  "reject.game_not_started": literal("The game hasn't started yet."),
  "reject.illegal_choice": literal("That choice isn't valid."),
  "reject.illegal_declaration": literal("That attack or block isn't legal."),
  "reject.illegal_mode": literal("Choose a valid mode."),
  "reject.illegal_target": literal("Pick a legal target."),
  "reject.mulliganing": literal("Finish mulligans first."),
  "reject.not_castable": literal("You can't play that right now."),
  "reject.not_helpless": literal("You can only yield when no actions are available."),
  "reject.not_seated": literal("That's not your seat."),
  "reject.not_your_priority": literal("It's not your turn to act."),
  "reject.stack_yield_one_shot": literal("Stack yield stopped after one automatic pass."),
  "reject.unknown_action": literal("That action expired -- try again."),
  "reject.unknown_object": literal("That card is no longer there."),
  "reject.unknown_table": literal("That table no longer exists."),
  "reject.wrong_timing": literal("You can't do that at this time."),
  "reject.cannot_discard_cost": literal("You don't have cards to discard for that."),
  "reject.cannot_exile_cost": literal("You don't have cards to exile for that."),
};
