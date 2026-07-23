// Pure VisibleEvent walks for canvas provenance and the game log.

import * as Match from "effect/Match";
import { playerLabel } from "./players";
import type { VisibleEvent, VisibleState, WireTarget } from "./wire/types";

export type ZonePileEntrance = { zone: "library" | "graveyard" | "exile"; seat: number };

/** One pass over a delta's events building glide-provenance structures. */
export function extractProvenance(
  events: VisibleEvent[],
  priorStack: Set<number>,
  _viewer: number,
): {
  moves: Map<number, number>;
  fromStack: Set<number>;
  fromStackExit: Set<number>;
  tokenCreators: Map<number, number>;
  landPlays: Map<number, number>;
  zonePileEntrances: Map<number, ZonePileEntrance>;
  stackEntrances: Map<number, { controller: number; from: number }>;
} {
  const moves = new Map<number, number>();
  const fromStack = new Set<number>();
  const fromStackExit = new Set<number>();
  const tokenCreators = new Map<number, number>();
  const landPlays = new Map<number, number>();
  const zonePileEntrances = new Map<number, ZonePileEntrance>();
  const stackEntrances = new Map<number, { controller: number; from: number }>();
  for (const e of events) {
    Match.value(e).pipe(
      Match.discriminator("kind")("moved_to_graveyard", "moved_to_exile", "milled", "moved_to_command_zone", (e) => {
        moves.set(e.card, e.from);
        if (priorStack.has(e.from)) fromStackExit.add(e.card);
      }),
      Match.discriminator("kind")("exiled_on_adventure", (e) => moves.set(e.card, e.from)),
      Match.discriminator("kind")("permanent_entered", (e) => {
        moves.set(e.permanent, e.from);
        fromStack.add(e.permanent);
      }),
      Match.discriminator("kind")("reanimated_to_battlefield", (e) => {
        moves.set(e.permanent, e.from);
        zonePileEntrances.set(e.permanent, { zone: "graveyard", seat: e.controller });
      }),
      Match.discriminator("kind")("flickered_to_battlefield", (e) => {
        moves.set(e.permanent, e.from);
        zonePileEntrances.set(e.permanent, { zone: "exile", seat: e.controller });
      }),
      Match.discriminator("kind")("searched_to_battlefield", (e) => {
        moves.set(e.permanent, e.from);
        zonePileEntrances.set(e.permanent, { zone: "library", seat: e.controller });
      }),
      Match.discriminator("kind")("put_onto_battlefield_from_hand", (e) => {
        moves.set(e.permanent, e.from);
      }),
      Match.discriminator("kind")("land_played", (e) => {
        landPlays.set(e.permanent, e.from);
      }),
      Match.discriminator("kind")("spell_cast", (e) => {
        stackEntrances.set(e.spell, { controller: e.controller, from: e.from });
      }),
      Match.discriminator("kind")("token_created", (e) => {
        if (e.creator != null) tokenCreators.set(e.token, e.creator);
      }),
      // Every other kind is deliberately not a glide-provenance move. Listed (not orElse'd) so a
      // new engine event kind is a compile error here until someone decides whether it glides.
      Match.discriminator("kind")(
        "abilities_granted",
        "ability_activated_this_turn",
        "ability_countered",
        "ability_resolved",
        "added_subtypes",
        "attached_to",
        "attacker_declared",
        "base_pt_set_until_end_of_turn",
        "became_copy",
        "cast_from_exile_free_bottoms_library_on_leave",
        "channel_colorless_mana_granted",
        "combat_damage_copy_armed",
        "combat_damage_prevented",
        "granted_abilities_ended",
        "leveled_up",
        "manifested",
        "reanimated_creature_became",
        "returned_exiled_card_to_graveyard",
        "spell_counters_divided",
        "time_counters_placed",
        "time_counters_removed",
        "token_granted_return_exiled_on_leave",
        "turned_face_up",
        "types_added_until_end_of_turn",
        "blocker_declared",
        "card_drawn",
        "card_exiled_with_source_left_exile",
        "cast_from_exile_free_ended",
        "cast_from_exile_free_permission_granted",
        "citys_blessing_gained",
        "combat_cleared",
        "combat_damage_dealt_to_creature",
        "combat_damage_dealt_to_player",
        "combat_damage_divided",
        "combat_damage_watch_armed",
        "combat_damage_watch_consumed",
        "commander_cast_from_command_zone",
        "commander_damage_dealt",
        "conditioned_control_ended",
        "conditioned_control_gained",
        "control_ended_until_end_of_turn",
        "control_gained",
        "control_gained_until_end_of_turn",
        "counters_placed",
        "creature_type_chosen",
        "damage_cleared",
        "damage_dealt_to_player",
        "damage_marked",
        "deathtouch_marked",
        "delayed_trigger_scheduled",
        "delayed_triggers_fired",
        "discarded",
        "drew_from_empty_library",
        "exiled_from_graveyard_may_play",
        "exiled_from_library_may_play",
        "exiled_from_library_to_choose_cast_free",
        "exiled_until_source_leaves",
        "exiled_until_source_leaves_minting_illusion",
        "exiled_with_source",
        "flash_permission_granted",
        "flipped",
        "goad_cleared",
        "goaded",
        "keywords_stripped",
        "kind_counters_placed",
        "leaves_illusion_minted",
        "library_shuffled",
        "life_changed",
        "lost_summoning_sickness",
        "loyalty_activated",
        "loyalty_changed",
        "mana_added",
        "mana_emptied",
        "mana_spent",
        "must_attack_declared",
        "next_cast_trigger_armed",
        "next_cast_trigger_consumed",
        "next_untap_skip_consumed",
        "next_untap_skip_marked",
        "phased_in",
        "phased_out",
        "play_from_exile_ended",
        "play_from_exile_permission_armed",
        "player_lost",
        "priority_passed",
        "put_from_hand_on_top",
        "put_on_bottom_of_library",
        "regenerated",
        "removed_from_combat",
        "regeneration_shield_created",
        "regeneration_shields_expired",
        "returned_from_linked_exile",
        "returned_to_hand",
        "revealed_top_of_library",
        "sacrificed",
        "searched_to_hand",
        "spell_ceased_to_exist",
        "spell_copied",
        "spell_damage_divided",
        "spell_targets_chosen",
        "prepared_changed",
        "prepared_spell_cast",
        "adventure_spell_cast",
        "color_chosen",
        "color_set_until_end_of_turn",
        "step_began",
        "tapped",
        "temp_boost",
        "temp_boosts_ended",
        "token_ceased_to_exist",
        "token_entered_attacking",
        "triggered_ability_on_stack",
        "triggered_ability_this_turn",
        "tucked_to_library",
        "untapped",
        "vow_counters_placed",
        () => {},
      ),
      Match.exhaustive,
    );
  }
  return { moves, fromStack, fromStackExit, tokenCreators, landPlays, zonePileEntrances, stackEntrances };
}

// A human-readable log line for an event, joining object ids → names against the delta's
// (post-apply) state. Returns null for events with no narrative value (priority, mana).
// Wire event kinds are projected in crates/schema/src/projection/event.rs — new kinds land
// there first; the exhaustive lists below are the client-side follow-up for log/provenance.
const colorName = (color: number): string => ["white", "blue", "black", "red", "green"][color] ?? `color ${color}`;

export function describe(e: VisibleEvent, state: VisibleState): string | null {
  const name = (id: number) => state.objects.find((o) => o.id === id)?.name ?? `#${id}`;
  const p = (n: number) => playerLabel(state.players, n);
  const t = (target: WireTarget) => (target.kind === "object" ? name(target.id) : p(target.player));
  return Match.value(e).pipe(
    Match.withReturnType<string | null>(),
    Match.discriminators("kind")({
      spell_cast: (e) => `${p(e.controller)} casts ${name(e.spell)}${e.target != null ? ` → ${t(e.target)}` : ""}`,
      land_played: (e) => `${p(e.player)} plays ${name(e.permanent)}`,
      permanent_entered: (e) => `${name(e.permanent)} enters`,
      triggered_ability_on_stack: (e) =>
        `${name(e.source)}'s ability triggers${e.target != null ? ` → ${t(e.target)}` : ""}`,
      damage_marked: (e) => `${name(e.object)} takes ${e.amount}${e.source != null ? ` from ${name(e.source)}` : ""}`,
      damage_dealt_to_player: (e) => `${name(e.source)} deals ${e.amount} damage to ${p(e.player)}`,
      life_changed: (e) => `${p(e.player)} ${e.amount < 0 ? "loses" : "gains"} ${Math.abs(e.amount)} life`,
      moved_to_graveyard: (e) => `${name(e.card)} dies`,
      moved_to_command_zone: (e) => `${name(e.card)} returns to the command zone`,
      counters_placed: (e) => `${name(e.object)} gets ${e.count} +1/+1 counter${e.count === 1 ? "" : "s"}`,
      attacker_declared: (e) => `${name(e.object)} attacks`,
      blocker_declared: (e) => `${name(e.blocker)} blocks ${name(e.attacker)}`,
      card_drawn: (e) => `${p(e.player)} draws${e.card ? ` ${e.card}` : " a card"}`,
      // Decking out is the one loss with no visible cause on the board — no lethal damage, no
      // commander damage, just a library that ran out. Say it, or the `player_lost` line below
      // reads as an unexplained death.
      drew_from_empty_library: (e) => `${p(e.player)} tries to draw from an empty library`,
      player_lost: (e) => `${p(e.player)} loses the game`,
      creature_type_chosen: (e) => `${name(e.object)} is chosen as ${e.subtype}`,
      color_chosen: (e) => `${name(e.object)} is chosen as ${colorName(e.color)}`,
      color_set_until_end_of_turn: (e) => `${name(e.object)} becomes ${colorName(e.color)} until end of turn`,
      // A flip card flipped (CR 709.4) — post-apply state already names the flipped half.
      flipped: (e) => `${name(e.object)} flips`,
      // counter_kind is a numeric engine index with no client name table, so the kind stays unnamed.
      kind_counters_placed: (e) => `${name(e.object)} gets ${e.count} counter${e.count === 1 ? "" : "s"}`,
      control_gained: (e) => `${p(e.controller)} gains control of ${name(e.object)}`,
      conditioned_control_gained: (e) => `${p(e.controller)} gains control of ${name(e.object)}`,
      conditioned_control_ended: (e) => `control of ${name(e.object)} reverts`,
      ability_countered: (e) => `${name(e.source)}'s ability is countered`,
      flickered_to_battlefield: (e) => `${name(e.permanent)} is exiled and returns`,
      token_entered_attacking: (e) => `${name(e.token)} enters attacking`,
      citys_blessing_gained: (e) => `${p(e.player)} gains the city's blessing`,
      // def is the card name string — a hidden-zone card not in state.objects, so use it directly.
      revealed_top_of_library: (e) => `${p(e.player)} reveals ${e.def}`,
      library_shuffled: (e) => `${p(e.player)} shuffles their library`,
      phased_out: (e) => `${name(e.object)} phases out`,
      phased_in: (e) => `${name(e.object)} phases in`,
      adventure_spell_cast: (e) =>
        `${p(e.controller)} casts ${name(e.spell)}${e.target != null ? ` → ${t(e.target)}` : ""}`,
      regenerated: (e) => `${name(e.object)} regenerates`,
      // Face-down transformations — don't leak a manifested card's identity (CR 708.2).
      manifested: (e) => `${p(e.controller)} manifests a card`,
      turned_face_up: (e) => `${name(e.permanent)} is turned face up`,
      became_copy: (e) => `${name(e.object)} becomes a copy`,
      leveled_up: (e) => `${name(e.object)} levels up to level ${e.level}`,
      // def is redacted to the owner's view like card_drawn — name the card only when visible.
      put_from_hand_on_top: (e) => `${p(e.player)} puts${e.def ? ` ${e.def}` : " a card"} on top of their library`,
      removed_from_combat: (e) => `${name(e.object)} is removed from combat`,
    }),
    // Events with no narrative value (priority, mana, bookkeeping) stay out of the log. Listed
    // (not orElse'd) so a new engine event kind is a compile error here until someone decides
    // whether it narrates.
    Match.discriminator("kind")(
      "abilities_granted",
      "ability_activated_this_turn",
      "ability_resolved",
      "added_subtypes",
      "attached_to",
      "base_pt_set_until_end_of_turn",
      "cast_from_exile_free_bottoms_library_on_leave",
      "channel_colorless_mana_granted",
      "combat_damage_copy_armed",
      "combat_damage_prevented",
      "granted_abilities_ended",
      "reanimated_creature_became",
      "returned_exiled_card_to_graveyard",
      "spell_counters_divided",
      "time_counters_placed",
      "time_counters_removed",
      "token_granted_return_exiled_on_leave",
      "types_added_until_end_of_turn",
      "card_exiled_with_source_left_exile",
      "cast_from_exile_free_ended",
      "cast_from_exile_free_permission_granted",
      "combat_cleared",
      // Always paired with a damage_marked that already narrates the hit — logging both would
      // double up ("X takes N from Y" twice).
      "combat_damage_dealt_to_creature",
      "combat_damage_dealt_to_player",
      "combat_damage_divided",
      "combat_damage_watch_armed",
      "combat_damage_watch_consumed",
      "commander_cast_from_command_zone",
      "commander_damage_dealt",
      "control_ended_until_end_of_turn",
      "control_gained_until_end_of_turn",
      "damage_cleared",
      "deathtouch_marked",
      "delayed_trigger_scheduled",
      "delayed_triggers_fired",
      "discarded",
      "exiled_from_graveyard_may_play",
      "exiled_from_library_may_play",
      "exiled_from_library_to_choose_cast_free",
      "exiled_on_adventure",
      "exiled_until_source_leaves",
      "exiled_until_source_leaves_minting_illusion",
      "exiled_with_source",
      "flash_permission_granted",
      "goad_cleared",
      "goaded",
      "keywords_stripped",
      "leaves_illusion_minted",
      "lost_summoning_sickness",
      "loyalty_activated",
      "loyalty_changed",
      "mana_added",
      "mana_emptied",
      "mana_spent",
      "milled",
      "moved_to_exile",
      "must_attack_declared",
      "next_cast_trigger_armed",
      "next_cast_trigger_consumed",
      "next_untap_skip_consumed",
      "next_untap_skip_marked",
      "play_from_exile_ended",
      "play_from_exile_permission_armed",
      "priority_passed",
      "put_on_bottom_of_library",
      "put_onto_battlefield_from_hand",
      "reanimated_to_battlefield",
      "regeneration_shield_created",
      "regeneration_shields_expired",
      "returned_from_linked_exile",
      "returned_to_hand",
      "sacrificed",
      "searched_to_battlefield",
      "searched_to_hand",
      "spell_ceased_to_exist",
      "spell_copied",
      "spell_damage_divided",
      "spell_targets_chosen",
      "prepared_changed",
      "prepared_spell_cast",
      "step_began",
      "tapped",
      "temp_boost",
      "temp_boosts_ended",
      "token_ceased_to_exist",
      "token_created",
      "triggered_ability_this_turn",
      "tucked_to_library",
      "untapped",
      "vow_counters_placed",
      () => null,
    ),
    Match.exhaustive,
  );
}
