import * as Data from "effect/Data"
import * as Effect from "effect/Effect"
import * as Stream from "effect/Stream"
import * as HttpClient from "effect/unstable/http/HttpClient"
import * as HttpClientError from "effect/unstable/http/HttpClientError"
import * as HttpClientRequest from "effect/unstable/http/HttpClientRequest"
import * as HttpClientResponse from "effect/unstable/http/HttpClientResponse"
// non-recursive definitions
export type Ack = { "accepted": boolean, "reason"?: string | null }
export type ChoiceItem = { "id": number, "label": string, "player"?: never }
export type CommanderDamageView = { "amount": number, "from": number }
export type Credentials = { "email": string, "password": string }
export type DeckCardEntry = { "count": number, "id": string, "print": string }
export type DeckError = { "problems": Array<string> }
export type DeckSummary = { "commander": string, "commander_print"?: string, "id": number, "name": string }
export type Me = { "email": string, "id": number, "username": string }
export type ModifierSourceView = { "contributions": Array<string>, "source_card_id"?: string, "source_name": string }
export type SeatView = { "claimed": boolean, "deck_name"?: string | null, "is_host": boolean, "is_you": boolean, "player": number, "ready": boolean, "username"?: string | null }
export type SeedResponse = { "pod_dns": string, "table_id": string, "version": string }
export type SeedSeat = { "deck_id": number, "user_id": number, "username": string }
export type SignupCredentials = { "email": string, "password": string, "username": string }
export type StackDwellRequest = { "dwelling": boolean }
export type WireCost = { "colored": Array<number>, "generic": number, "has_x"?: boolean }
export type WireEitherMana = { "a": number, "amount": number, "b": number }
export type WireKind = { "kind": "creature", "power": number, "toughness": number } | { "kind": "instant" } | { "kind": "sorcery" } | { "kind": "enchantment" } | { "kind": "artifact" } | { "kind": "planeswalker", "loyalty": number } | { "colors": Array<number>, "kind": "land" }
export type WireOfColorsMana = { "amount": number, "mask": number }
export type YieldRequest = { "enabled": boolean }
export type U32 = number
export type DeckDetail = { "cards": Array<DeckCardEntry>, "commander": string, "commander_print": string, "id": number, "name": string }
export type SaveDeckRequest = { "cards": Array<DeckCardEntry>, "commander": string, "commander_print": string, "name": string }
export type SeedRequest = { "host_user_id": number, "seats": Array<SeedSeat>, "table_id": string }
export type CatalogCard = { "approximates"?: string | null, "back"?: null | { "approximates"?: string | null, "name": string, "oracle"?: string | null }, "color_identity": Array<number>, "cost": WireCost, "default_print": string, "id": string, "keywords": Array<string>, "kind": WireKind, "legendary": boolean, "name": string, "oracle"?: string | null, "otags": Array<string>, "set": string, "subtypes": Array<string>, "summary": string }
export type PlayerView = { "commander_damage"?: Array<CommanderDamageView>, "commander_tax": number, "hand_count": number, "library_count": number, "life": number, "lost": boolean, "mana_pool": { "any": number, "colored": Array<number>, "colorless": number, "either"?: Array<WireEitherMana>, "of_colors"?: Array<WireOfColorsMana> }, "player": number, "username"?: string }
export type ObjectView = { "attached_to"?: null | number, "card_id"?: string, "controller": number, "face_down"?: boolean, "goaded"?: boolean, "has_haste": boolean, "id": U32, "is_commander": boolean, "keywords"?: Array<string>, "kind": WireKind, "loyalty"?: number, "mana_cost": WireCost, "marked_damage": number, "modifiers"?: Array<ModifierSourceView>, "name": string, "needs_target": boolean, "owner": number, "phased_out"?: boolean, "plus_counters": number, "power": number, "prepared"?: boolean, "print"?: string, "summoning_sick": boolean, "tapped": boolean, "taps_for_mana"?: boolean, "toughness": number, "zone": number }
export type StackObjectView = { "controller": number, "kind": string, "label": string, "source": number, "target"?: null | { "id": U32, "kind": "object" } | { "kind": "player", "player": number } }
export type WireAttack = { "attacker": U32, "defender": number }
export type WireBlock = { "attacker": U32, "blocker": U32 }
export type WireDamage = { "amount": number, "blocker": U32 }
export type WireTarget = { "id": U32, "kind": "object" } | { "kind": "player", "player": number }
export type ModeView = { "label": string, "needs_target": boolean, "targets": Array<WireTarget> }
export type VisibleEvent = { "controller": number, "escape": boolean, "flashback": boolean, "from": U32, "kind": "spell_cast", "spell": U32, "target"?: null | WireTarget } | { "kind": "spell_targets_chosen", "spell": U32, "targets": Array<WireTarget> } | { "kind": "prepared_changed", "object": U32, "prepared": boolean } | { "kind": "leveled_up", "level": number, "object": U32 } | { "kind": "flipped", "object": U32 } | { "kind": "phased_out", "object": U32 } | { "kind": "phased_in", "object": U32 } | { "kind": "creature_type_chosen", "object": U32, "subtype": string } | { "color": number, "kind": "color_chosen", "object": U32 } | { "color": number, "kind": "color_set_until_end_of_turn", "object": U32 } | { "controller": number, "kind": "prepared_spell_cast", "source": U32, "spell": U32, "target"?: null | WireTarget, "x": number } | { "controller": number, "kind": "adventure_spell_cast", "source": U32, "spell": U32, "target"?: null | WireTarget, "x": number } | { "active_player": number, "kind": "step_began", "step": number } | { "controller": number, "kind": "triggered_ability_on_stack", "source": U32, "target"?: null | WireTarget } | { "kind": "ability_resolved", "source": U32 } | { "kind": "ability_countered", "source": U32 } | { "from": U32, "kind": "land_played", "permanent": U32, "player": number } | { "kind": "tapped", "object": U32 } | { "kind": "untapped", "object": U32 } | { "kind": "regeneration_shield_created", "object": U32 } | { "kind": "regenerated", "object": U32 } | { "kind": "regeneration_shields_expired", "object": U32 } | { "kind": "lost_summoning_sickness", "object": U32 } | { "count": number, "kind": "counters_placed", "object": U32 } | { "count": number, "counter_kind": number, "kind": "kind_counters_placed", "object": U32 } | { "amount": number, "kind": "loyalty_changed", "object": U32 } | { "active": boolean, "kind": "loyalty_activated", "object": U32 } | { "ability_index": number, "kind": "ability_activated_this_turn", "object": U32 } | { "kind": "triggered_ability_this_turn", "source": U32 } | { "host"?: null | U32, "kind": "attached_to", "object": U32 } | { "kind": "temp_boost", "object": U32, "power": number, "toughness": number } | { "kind": "temp_boosts_ended", "object": U32 } | { "kind": "base_pt_set_until_end_of_turn", "object": U32, "power": number, "toughness": number } | { "kind": "types_added_until_end_of_turn", "object": U32 } | { "kind": "reanimated_creature_became", "object": U32 } | { "kind": "added_subtypes", "object": U32 } | { "kind": "became_copy", "object": U32 } | { "kind": "keywords_stripped", "object": U32 } | { "controller": number, "kind": "control_gained_until_end_of_turn", "object": U32 } | { "kind": "control_ended_until_end_of_turn", "object": U32 } | { "kind": "abilities_granted", "source": U32, "target": U32 } | { "kind": "granted_abilities_ended" } | { "controller": number, "kind": "control_gained", "object": U32 } | { "controller": number, "kind": "conditioned_control_gained", "object": U32 } | { "kind": "conditioned_control_ended", "object": U32 } | { "defender": number, "kind": "attacker_declared", "object": U32 } | { "defender": number, "kind": "token_entered_attacking", "token": U32 } | { "by": number, "kind": "goaded", "object": U32 } | { "by": number, "kind": "goad_cleared" } | { "kind": "vow_counters_placed", "object": U32, "protected": number } | { "card": U32, "count": number, "kind": "time_counters_placed" } | { "card": U32, "kind": "time_counters_removed" } | { "defender": number, "kind": "must_attack_declared", "object": U32 } | { "controller": number, "kind": "delayed_trigger_scheduled", "source": U32 } | { "kind": "delayed_triggers_fired" } | { "controller": number, "kind": "next_cast_trigger_armed", "source": U32 } | { "controller": number, "kind": "next_cast_trigger_consumed", "source": U32 } | { "controller": number, "kind": "combat_damage_watch_armed", "source": U32, "watched": U32 } | { "controller": number, "kind": "combat_damage_watch_consumed", "source": U32 } | { "card": U32, "controller": number, "kind": "combat_damage_copy_armed", "source": U32 } | { "card": U32, "from": U32, "kind": "exiled_from_library_may_play", "player": number, "until_next_turn": boolean } | { "card": U32, "from": U32, "kind": "exiled_from_library_to_choose_cast_free", "player": number } | { "card": U32, "kind": "play_from_exile_permission_armed" } | { "kind": "play_from_exile_ended" } | { "attacker": U32, "blocker": U32, "kind": "blocker_declared" } | { "assignment": Array<[number?, number?, ...Array<never>]>, "attacker": U32, "kind": "combat_damage_divided" } | { "assignment": Array<[number?, number?, ...Array<never>]>, "kind": "spell_damage_divided", "players": Array<[number?, number?, ...Array<never>]>, "spell": U32 } | { "assignment": Array<[number?, number?, ...Array<never>]>, "kind": "spell_counters_divided", "spell": U32 } | { "kind": "deathtouch_marked", "object": U32 } | { "kind": "combat_cleared" } | { "kind": "commander_cast_from_command_zone", "player": number } | { "kind": "flash_permission_granted", "player": number } | { "kind": "channel_colorless_mana_granted", "player": number } | { "amount": number, "kind": "commander_damage_dealt", "player": number, "source": U32 } | { "amount": number, "kind": "combat_damage_dealt_to_player", "player": number, "source": U32 } | { "amount": number, "kind": "combat_damage_dealt_to_creature", "source": U32, "target": U32 } | { "amount": number, "kind": "damage_dealt_to_player", "player": number, "source": U32 } | { "amount": number, "kind": "combat_damage_prevented", "player": number } | { "card": U32, "from": U32, "kind": "moved_to_command_zone" } | { "kind": "mana_emptied", "player": number } | { "kind": "damage_cleared", "object": U32 } | { "amount": number, "kind": "mana_added", "mana": number, "player": number } | { "kind": "mana_spent", "mana": Array<number>, "player": number } | { "kind": "priority_passed", "player": number } | { "from": U32, "kind": "permanent_entered", "permanent": U32 } | { "controller": number, "finality": boolean, "from": U32, "kind": "reanimated_to_battlefield", "permanent": U32, "tapped": boolean } | { "controller": number, "kind": "token_created", "token": U32 } | { "kind": "token_ceased_to_exist", "token": U32 } | { "controller": number, "copy": U32, "kind": "spell_copied", "original": U32 } | { "kind": "spell_ceased_to_exist", "spell": U32 } | { "amount": number, "kind": "damage_marked", "object": U32, "source"?: null | number } | { "card": U32, "from": U32, "kind": "moved_to_graveyard" } | { "card": U32, "from": U32, "kind": "moved_to_exile" } | { "card": U32, "from": U32, "kind": "exiled_on_adventure", "owner": number } | { "kind": "exiled_until_source_leaves", "object": U32, "source": U32 } | { "kind": "exiled_until_source_leaves_minting_illusion", "object": U32, "source": U32 } | { "kind": "leaves_illusion_minted", "object": U32, "source": U32 } | { "exiled": U32, "kind": "token_granted_return_exiled_on_leave", "token": U32 } | { "card": U32, "from": U32, "kind": "returned_exiled_card_to_graveyard" } | { "kind": "exiled_with_source", "object": U32, "source": U32 } | { "kind": "card_exiled_with_source_left_exile", "object": U32, "source": U32 } | { "card": U32, "kind": "cast_from_exile_free_permission_granted", "player": number } | { "card": U32, "kind": "cast_from_exile_free_bottoms_library_on_leave" } | { "kind": "cast_from_exile_free_ended" } | { "controller": number, "from": U32, "kind": "returned_from_linked_exile", "permanent": U32, "source": U32 } | { "controller": number, "from": U32, "kind": "flickered_to_battlefield", "permanent": U32 } | { "card": U32, "from": U32, "kind": "returned_to_hand" } | { "card": U32, "from": U32, "kind": "tucked_to_library", "to_top": boolean } | { "kind": "library_shuffled", "player": number } | { "card": U32, "def": string, "kind": "revealed_top_of_library", "player": number } | { "card": U32, "kind": "put_on_bottom_of_library", "player": number } | { "card"?: string | null, "from"?: null | U32, "kind": "searched_to_hand", "object": U32, "player": number } | { "controller": number, "from": U32, "kind": "searched_to_battlefield", "permanent": U32, "tapped": boolean } | { "controller": number, "kind": "manifested", "permanent": U32 } | { "kind": "turned_face_up", "permanent": U32 } | { "controller": number, "from": U32, "kind": "put_onto_battlefield_from_hand", "permanent": U32, "tapped": boolean } | { "card": U32, "from": U32, "kind": "milled", "player": number } | { "amount": number, "kind": "life_changed", "player": number, "source"?: null | number } | { "kind": "drew_from_empty_library", "player": number } | { "kind": "player_lost", "player": number } | { "kind": "citys_blessing_gained", "player": number } | { "card"?: string | null, "from"?: null | U32, "kind": "card_drawn", "object": U32, "player": number } | { "by": number, "kind": "sacrificed", "object": U32 } | { "card": U32, "from": U32, "kind": "discarded", "player": number } | { "card": U32, "from": U32, "kind": "exiled_from_graveyard_may_play", "player": number }
export type WireModeChoice = { "index": number, "target"?: null | WireTarget }
export type WireSpellDamage = { "amount": number, "target": WireTarget }
export type ActionView = { "ability_index"?: never, "auto_tap"?: Array<U32>, "discard_choices"?: Array<number>, "discard_count"?: number, "graveyard_exile_choices"?: Array<number>, "graveyard_exile_max"?: number, "graveyard_exile_min"?: number, "has_x"?: boolean, "id": number, "kind": string, "label": string, "modal"?: null | { "choose": number, "choose_max": number, "modes": Array<ModeView> }, "needs_target": boolean, "object"?: null | number, "required_attacks"?: Array<WireAttack>, "sacrifice_choices"?: Array<number>, "section": string, "targets"?: Array<WireTarget> }
export type PendingChoiceView = { "count": number, "kind": "order_triggers", "labels": Array<string>, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_target", "label": string, "optional": boolean, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_spell_targets", "label": string, "max": number, "min": number, "player": number, "spell": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_target_players", "label": string, "max": number, "min": number, "player": number, "source": U32 } | { "kind": "may_yes_no", "label": string, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "decline_untap", "player": number } | { "items": Array<ChoiceItem>, "kind": "choose_dredge", "player": number } | { "cost": WireCost, "kind": "pay_cost", "label": string, "player": number, "source": U32 } | { "cost": WireCost, "kind": "pay_or_counter", "player": number, "spell": U32 } | { "controller": number, "cost": WireCost, "kind": "pay_or_controller_draws", "player": number } | { "kind": "choose_countered_spell_destination", "player": number, "spell": U32 } | { "cost": WireCost, "kind": "pay_echo_or_sacrifice", "player": number, "source": U32 } | { "cost": WireCost, "kind": "pay_recover_or_exile", "player": number, "source": U32 } | { "cost": WireCost, "kind": "sacrifice_unless_pay", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "sacrifice_unless_return_land", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "assign_combat_damage", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "divide_spell_damage", "player": number, "spell": U32, "total": number } | { "items": Array<ChoiceItem>, "kind": "divide_counters", "player": number, "spell": U32, "total": number } | { "items": Array<ChoiceItem>, "kind": "scry", "player": number } | { "items": Array<ChoiceItem>, "kind": "surveil", "player": number } | { "items": Array<ChoiceItem>, "kind": "search_library", "player": number } | { "items": Array<ChoiceItem>, "kind": "select_from_top", "player": number, "up_to": number } | { "items": Array<ChoiceItem>, "kind": "distribute_top", "player": number, "to_bottom": number, "to_exile_may_play": number, "to_hand": number } | { "items": Array<ChoiceItem>, "kind": "shuffle_from_graveyard", "max": number, "owner": number, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "keep_one"?: boolean, "kind": "sacrifice_edict", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "proliferate", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "phase_out", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_ability_targets", "label": string, "max": number, "min": number, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "may_sacrifice", "player": number, "source": U32 } | { "count": number, "items": Array<ChoiceItem>, "kind": "choose_own_sacrifices", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "devour", "multiplier": number, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "exile_from_graveyard", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "caster_keep_permanents", "player": number, "source": U32, "target_player": number } | { "items": Array<ChoiceItem>, "kind": "choose_counter_target_for_player", "player": number, "source": U32, "target_player": number } | { "items": Array<ChoiceItem>, "kind": "may_return_from_graveyard", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "may_discard", "player": number, "source": U32 } | { "count": number, "items": Array<ChoiceItem>, "kind": "discard", "player": number } | { "items": Array<ChoiceItem>, "kind": "put_land_from_hand", "player": number } | { "items": Array<ChoiceItem>, "kind": "put_creature_from_hand", "player": number } | { "items": Array<ChoiceItem>, "kind": "cast_creature_face_down", "player": number } | { "items": Array<ChoiceItem>, "kind": "choose_exiled_with_card", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_exiled_with_card_to_cast", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_exiled_dig_to_cast_free", "player": number, "source": U32 } | { "budget": number, "items": Array<ChoiceItem>, "kind": "dance_exile_more", "player": number, "source": U32, "total_mv": number } | { "kind": "opponent_chooses_pile", "pile_a": Array<ChoiceItem>, "pile_b": Array<ChoiceItem>, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "opponent_chooses_exiled_nonland", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_splitting_opponent", "label": string, "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "partition_revealed", "player": number, "source": U32 } | { "kind": "choose_pile_for_hand", "pile_a": Array<ChoiceItem>, "pile_b": Array<ChoiceItem>, "player": number, "source": U32 } | { "count": number, "items": Array<ChoiceItem>, "kind": "choose_exiled_to_cast_free", "player": number, "source": U32 } | { "item": ChoiceItem, "kind": "revealed_card_to_battlefield_or_hand", "player": number } | { "kind": "choose_mode", "labels": Array<string>, "player": number, "source": U32 } | { "choose": number, "kind": "choose_trigger_modes", "modes": Array<ModeView>, "optional": boolean, "player": number, "source": U32 } | { "amount": number, "kind": "choose_mana_color", "player": number, "source": U32 } | { "kind": "choose_creature_type", "options": Array<string>, "player": number, "source": U32 } | { "kind": "choose_color", "player": number, "source": U32 } | { "items": Array<ChoiceItem>, "kind": "choose_copy_target", "player": number, "source": U32 } | { "attachment": U32, "items": Array<ChoiceItem>, "kind": "choose_attach_host", "optional": boolean, "player": number }
export type WireIntent = { "bought_back"?: boolean, "discard_cost"?: Array<U32>, "evoked"?: boolean, "graveyard_exile"?: Array<U32>, "kicked"?: boolean, "kind": "cast", "modes"?: Array<WireModeChoice>, "object": U32, "player": number, "replicate_count"?: number, "sacrifice_cost"?: Array<U32>, "strive_count"?: number, "target"?: null | WireTarget, "x"?: number } | { "kind": "play_land", "object": U32, "player": number } | { "kind": "tap_for_mana", "object": U32, "player": number } | { "ability_index": number, "discard_cost"?: Array<U32>, "kind": "activate_ability", "object": U32, "player": number, "sacrifice"?: Array<U32>, "target"?: null | WireTarget } | { "attackers": Array<WireAttack>, "kind": "declare_attackers", "player": number } | { "blocks": Array<WireBlock>, "kind": "declare_blockers", "player": number } | { "kind": "choose_order", "order": Array<number>, "player": number } | { "kind": "choose_targets", "player": number, "targets": Array<WireTarget> } | { "kind": "choose_target_players", "player": number, "players": Array<number> } | { "kind": "answer_may", "player": number, "yes": boolean } | { "kind": "pay_optional_cost", "pay": boolean, "player": number } | { "assignment": Array<WireDamage>, "kind": "assign_damage", "player": number } | { "assignment": Array<WireSpellDamage>, "kind": "divide_spell_damage", "player": number } | { "bottom": Array<U32>, "kind": "arrange_top", "player": number, "top": Array<U32> } | { "cards"?: Array<U32>, "kind": "select_from_top", "player": number } | { "kind": "distribute_top", "player": number, "to_bottom"?: Array<U32>, "to_exile_may_play"?: Array<U32>, "to_hand"?: Array<U32> } | { "cards"?: Array<U32>, "kind": "shuffle_from_graveyard", "player": number } | { "choice"?: null | U32, "kind": "search_library", "player": number } | { "kind": "choose_sacrifices", "player": number, "sacrifices": Array<U32> } | { "cards": Array<U32>, "kind": "discard", "player": number } | { "keep_tapped": Array<U32>, "kind": "decline_untap", "player": number } | { "dredger"?: null | U32, "kind": "choose_dredge", "player": number } | { "choice"?: null | U32, "kind": "put_land_from_hand", "player": number } | { "choice"?: null | U32, "kind": "put_creature_from_hand", "player": number } | { "choice"?: null | U32, "kind": "cast_creature_face_down", "player": number } | { "kind": "return_land_or_sacrifice", "land"?: null | U32, "player": number } | { "choice"?: null | U32, "kind": "choose_exiled_with_card", "player": number } | { "choice"?: null | U32, "kind": "choose_exiled_with_card_to_cast", "player": number } | { "choice"?: null | U32, "kind": "choose_exiled_dig_to_cast_free", "player": number } | { "kind": "choose_opponent_pile", "pile": number, "player": number } | { "choice"?: null | U32, "kind": "revealed_card_to_battlefield_or_hand", "player": number } | { "kind": "choose_mode", "mode": number, "player": number } | { "kind": "choose_trigger_modes", "modes": Array<WireModeChoice>, "player": number } | { "color": number, "kind": "choose_mana_color", "player": number } | { "kind": "choose_creature_type", "player": number, "subtype": string } | { "color": number, "kind": "choose_color", "player": number } | { "host"?: null | U32, "kind": "choose_attach_host", "player": number } | { "copy"?: null | U32, "kind": "choose_copy_target", "player": number } | { "kind": "choose_top_or_bottom", "player": number, "top": boolean } | { "card": U32, "kind": "cycle", "player": number, "sacrifice"?: null | U32 } | { "card": U32, "kind": "activate_hand_ability", "player": number } | { "card": U32, "kind": "suspend", "player": number } | { "card": U32, "kind": "encore", "player": number } | { "kind": "turn_face_up", "permanent": U32, "player": number } | { "card": U32, "kind": "cast_face_down", "player": number } | { "kind": "cast_prepared", "player": number, "source": U32, "target"?: null | WireTarget, "x"?: number } | { "kind": "cast_adventure", "player": number, "source": U32, "target"?: null | WireTarget, "x"?: number } | { "kind": "cast_bestow", "object": U32, "player": number, "target"?: null | WireTarget } | { "kind": "pass_priority", "player": number } | { "kind": "concede", "player": number } | { "attackers"?: Array<WireAttack>, "blocks"?: Array<WireBlock>, "discard_cost"?: Array<U32>, "graveyard_exile"?: Array<U32>, "id": number, "kind": "take_action", "modes"?: Array<WireModeChoice>, "player": number, "sacrifice": Array<U32>, "target"?: null | WireTarget, "x"?: number }
export type VisibleState = { "actions"?: Array<ActionView>, "active_player": number, "can_act": boolean, "combat": { "attackers": Array<WireAttack>, "attackers_declared": boolean, "blockers_declared": Array<number>, "blocks": Array<WireBlock> }, "objects": Array<ObjectView>, "pending_choice"?: null | PendingChoiceView, "players": Array<PlayerView>, "priority": number, "stack": Array<StackObjectView>, "stack_hold_remaining_ms"?: number, "step": number, "turn_yielded"?: boolean, "viewer": number, "yielded"?: boolean }
export type IntentEnvelope = { "client_seq": number, "intent": WireIntent, "table_id": string }
export type StreamFrame = { "frame": "snapshot", "seq": number, "state": VisibleState } | { "auto_actions"?: Array<string>, "events": Array<VisibleEvent>, "seq": number, "state": VisibleState, "frame": "delta" } | { "frame": "heartbeat" }
// schemas
export type LoginRequestJson = Credentials
export type Login200 = Me
export type Me200 = Me
export type SignupRequestJson = SignupCredentials
export type Signup200 = Me
export type LookupCardsParams = { "ids": Array<string> }
export type LookupCards200 = Array<CatalogCard>
export type SearchCardsParams = { "q"?: string, "limit"?: number, "offset"?: number }
export type SearchCards200 = Array<CatalogCard>
export type Catalog200 = Array<CatalogCard>
export type ListDecks200 = Array<DeckSummary>
export type CreateDeckRequestJson = SaveDeckRequest
export type CreateDeck200 = DeckDetail
export type CreateDeck422 = DeckError
export type GetDeck200 = DeckDetail
export type UpdateDeckRequestJson = SaveDeckRequest
export type UpdateDeck200 = DeckDetail
export type UpdateDeck422 = DeckError
export type SeedTableRequestJson = SeedRequest
export type SeedTable200 = SeedResponse
export type SubmitIntentRequestJson = IntentEnvelope
export type SubmitIntent200 = Ack
export type SetStackDwellRequestJson = StackDwellRequest
export type SetStackDwell200 = Ack
export type Stream200Sse = StreamFrame
export type SetTurnYieldRequestJson = YieldRequest
export type SetTurnYield200 = Ack
export type SetYieldRequestJson = YieldRequest
export type SetYield200 = Ack

export interface OperationConfig {
  /**
   * Whether or not the response should be included in the value returned from
   * an operation.
   *
   * If set to `true`, a tuple of `[A, HttpClientResponse]` will be returned,
   * where `A` is the success type of the operation.
   *
   * If set to `false`, only the success type of the operation will be returned.
   */
  includeResponse?: boolean | undefined
}

/**
 * A utility type which optionally includes the response in the return result
 * of an operation based upon the value of the `includeResponse` configuration
 * option.
 */
export type WithOptionalResponse<A, Config extends OperationConfig> = Config extends {
  includeResponse: true
} ? [A, HttpClientResponse.HttpClientResponse] : A

export const make = (
  httpClient: HttpClient.HttpClient,
  options: {
    transformClient?: ((client: HttpClient.HttpClient) => Effect.Effect<HttpClient.HttpClient>) | undefined
  } = {}
): Mtgfr => {
  const unexpectedStatus = (response: HttpClientResponse.HttpClientResponse) =>
    Effect.flatMap(
      Effect.orElseSucceed(response.json, () => "Unexpected status code"),
      (description) =>
        Effect.fail(
          new HttpClientError.HttpClientError({
            reason: new HttpClientError.StatusCodeError({
              request: response.request,
              response,
              description: typeof description === "string" ? description : JSON.stringify(description),
            }),
          }),
        ),
    )
  const withResponse = <Config extends OperationConfig>(config: Config | undefined) => (
    f: (response: HttpClientResponse.HttpClientResponse) => Effect.Effect<any, any>,
  ): (request: HttpClientRequest.HttpClientRequest) => Effect.Effect<any, any> => {
    const withOptionalResponse = (
      config?.includeResponse
        ? (response: HttpClientResponse.HttpClientResponse) => Effect.map(f(response), (a) => [a, response])
        : (response: HttpClientResponse.HttpClientResponse) => f(response)
    ) as any
    return options?.transformClient
      ? (request) =>
          Effect.flatMap(
            Effect.flatMap(options.transformClient!(httpClient), (client) => client.execute(request)),
            withOptionalResponse
          )
      : (request) => Effect.flatMap(httpClient.execute(request), withOptionalResponse)
  }
  const sseRequest = (request: HttpClientRequest.HttpClientRequest): Stream.Stream<any, HttpClientError.HttpClientError> =>
    HttpClient.filterStatusOk(httpClient).execute(request).pipe(
      Effect.map((response) => response.stream),
      Stream.unwrap,
      Stream.decodeText(),
      Stream.splitLines,
      Stream.filter((line) => line.startsWith("data: ")),
      Stream.map((line) => JSON.parse(line.slice(6)))
    )
  const decodeSuccess = <A>(response: HttpClientResponse.HttpClientResponse) =>
    response.json as Effect.Effect<A, HttpClientError.HttpClientError>
  const decodeVoid = (_response: HttpClientResponse.HttpClientResponse) =>
    Effect.void
  const decodeError =
    <Tag extends string, E>(tag: Tag) =>
    (
      response: HttpClientResponse.HttpClientResponse,
    ): Effect.Effect<
      never,
      MtgfrError<Tag, E> | HttpClientError.HttpClientError
    > =>
      Effect.flatMap(
        response.json as Effect.Effect<E, HttpClientError.HttpClientError>,
        (cause) => Effect.fail(MtgfrError(tag, cause, response)),
      )
  const onRequest = <Config extends OperationConfig>(config: Config | undefined) => (
    successCodes: Array<string>,
    errorCodes?: Record<string, string>,
  ) => {
    const cases: any = { orElse: unexpectedStatus }
    for (const code of successCodes) {
      cases[code] = decodeSuccess
    }
    if (errorCodes) {
      for (const [code, tag] of Object.entries(errorCodes)) {
        cases[code] = decodeError(tag)
      }
    }
    if (successCodes.length === 0) {
      cases["2xx"] = decodeVoid
    }
    return withResponse(config)(HttpClientResponse.matchStatus(cases) as any)
  }
  return {
    httpClient,
    "login": (options) => HttpClientRequest.post(`/auth/login/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "logout": (options) => HttpClientRequest.post(`/auth/logout/v1`).pipe(
    onRequest(options?.config)([])
  ),
    "me": (options) => HttpClientRequest.get(`/auth/me/v1`).pipe(
    onRequest(options?.config)(["2xx"])
  ),
    "signup": (options) => HttpClientRequest.post(`/auth/signup/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "lookupCards": (options) => HttpClientRequest.get(`/cards/lookup/v1`).pipe(
    HttpClientRequest.setUrlParams({ "ids": options.params["ids"] as any }),
    onRequest(options.config)(["2xx"])
  ),
    "searchCards": (options) => HttpClientRequest.get(`/cards/search/v1`).pipe(
    HttpClientRequest.setUrlParams({ "q": options?.params?.["q"] as any, "limit": options?.params?.["limit"] as any, "offset": options?.params?.["offset"] as any }),
    onRequest(options?.config)(["2xx"])
  ),
    "catalog": (options) => HttpClientRequest.get(`/cards/v1`).pipe(
    onRequest(options?.config)(["2xx"])
  ),
    "listDecks": (options) => HttpClientRequest.get(`/decks/v1`).pipe(
    onRequest(options?.config)(["2xx"])
  ),
    "createDeck": (options) => HttpClientRequest.post(`/decks/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"], {"422":"CreateDeck422"})
  ),
    "getDeck": (id, options) => HttpClientRequest.get(`/decks/${id}/v1`).pipe(
    onRequest(options?.config)(["2xx"])
  ),
    "updateDeck": (id, options) => HttpClientRequest.put(`/decks/${id}/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"], {"422":"UpdateDeck422"})
  ),
    "deleteDeck": (id, options) => HttpClientRequest.delete(`/decks/${id}/v1`).pipe(
    onRequest(options?.config)([])
  ),
    "seedTable": (options) => HttpClientRequest.post(`/tables/seed/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "submitIntent": (table, options) => HttpClientRequest.post(`/tables/${table}/intent/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "setStackDwell": (table, options) => HttpClientRequest.post(`/tables/${table}/stack-dwell/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "stream": (table, options) => HttpClientRequest.get(`/tables/${table}/stream/v1`).pipe(
    onRequest(options?.config)([])
  ),
    "streamSse": (table) => HttpClientRequest.get(`/tables/${table}/stream/v1`).pipe(
      sseRequest
    ),
    "setTurnYield": (table, options) => HttpClientRequest.post(`/tables/${table}/turn-yield/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  ),
    "setYield": (table, options) => HttpClientRequest.post(`/tables/${table}/yield/v1`).pipe(
    HttpClientRequest.bodyJsonUnsafe(options.payload),
    onRequest(options.config)(["2xx"])
  )
  }
}

export interface Mtgfr {
  httpClient: HttpClient.HttpClient
  /**
* Sign in to an existing account.
* response (see `signup`), so the generated client surfaces it as a catchable `HttpClientError`
* rather than swallowing it to void.
*/
"login": <Config extends OperationConfig>(options: { payload: LoginRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<Login200, Config>, HttpClientError.HttpClientError>
  /**
* Sign out: delete the session row and clear the cookie.
*/
"logout": <Config extends OperationConfig>(options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError>
  /**
* The currently signed-in user (401 if not signed in).
*/
"me": <Config extends OperationConfig>(options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<Me200, Config>, HttpClientError.HttpClientError>
  /**
* Register a new account and sign in. A duplicate email is a 409 — deliberately *not* declared
* as a response, so the generated client surfaces it as a catchable `HttpClientError` (a
* documented bodiless status is instead swallowed to void). The client reads the 409 off the error.
*/
"signup": <Config extends OperationConfig>(options: { payload: SignupRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<Signup200, Config>, HttpClientError.HttpClientError>
  /**
* Fetch specific pool cards by Card id — lets the deck builder hydrate a saved decklist and
* commander without pulling the whole pool. Public and best-effort like `/cards/search/v1`.
*/
"lookupCards": <Config extends OperationConfig>(options: { params: LookupCardsParams; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<LookupCards200, Config>, HttpClientError.HttpClientError>
  /**
* Search the pool from the deck builder's single input: cards matching every token of `q` against
* name, card type, subtype, set, color, and keywords. Public (no auth) — the pool isn't private.
* A DB error yields an empty page rather than a 500 (the projection is best-effort, ADR 0010).
*/
"searchCards": <Config extends OperationConfig>(options: { params?: SearchCardsParams | undefined; config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<SearchCards200, Config>, HttpClientError.HttpClientError>
  /**
* The HTTP application, wired to the shared table.
* The whole card pool, for the deck builder to browse. Public (no auth) and stateless —
* the pool is a load-once static registry.
*/
"catalog": <Config extends OperationConfig>(options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<Catalog200, Config>, HttpClientError.HttpClientError>
  /**
* List the signed-in user's decks.
*/
"listDecks": <Config extends OperationConfig>(options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<ListDecks200, Config>, HttpClientError.HttpClientError>
  /**
* Create a deck for the signed-in user (422 with all legality problems if illegal).
*/
"createDeck": <Config extends OperationConfig>(options: { payload: CreateDeckRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<CreateDeck200, Config>, HttpClientError.HttpClientError | MtgfrError<"CreateDeck422", CreateDeck422>>
  /**
* Get a deck's full contents.
*/
"getDeck": <Config extends OperationConfig>(id: string, options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<GetDeck200, Config>, HttpClientError.HttpClientError>
  /**
* Update a deck (re-validated; 422 if the new list is illegal).
*/
"updateDeck": <Config extends OperationConfig>(id: string, options: { payload: UpdateDeckRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<UpdateDeck200, Config>, HttpClientError.HttpClientError | MtgfrError<"UpdateDeck422", UpdateDeck422>>
  /**
* Delete a deck.
*/
"deleteDeck": <Config extends OperationConfig>(id: string, options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError>
  /**
* Seed a running game from BFF-resolved seats. 503 while draining.
*/
"seedTable": <Config extends OperationConfig>(options: { payload: SeedTableRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<SeedTable200, Config>, HttpClientError.HttpClientError>
  /**
* Submit a player's intent: validate against the engine, and on success bump the delta
* sequence and broadcast the resulting events to every viewer's stream.
*/
"submitIntent": <Config extends OperationConfig>(table: string, options: { payload: SubmitIntentRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<SubmitIntent200, Config>, HttpClientError.HttpClientError>
  /**
* Helpless-reader hover on the stack during a hold (see [`set_stack_dwell`]).
*/
"setStackDwell": <Config extends OperationConfig>(table: string, options: { payload: SetStackDwellRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<SetStackDwell200, Config>, HttpClientError.HttpClientError>
  /**
* The per-viewer delta stream, as Server-Sent Events (`text/event-stream`). The first event is a
* full redacted snapshot at the current seq; every later event is a redacted delta. On (re)connect
* the client just gets a fresh snapshot, so there's no history buffer — the snapshot's seq is the
* resume point. SSE (over fetch, not `EventSource`) so the generated client can consume it as a
* typed `Stream<StreamFrame>` (ADR 0005).
*/
"stream": <Config extends OperationConfig>(table: string, options: { config?: Config | undefined } | undefined) => Effect.Effect<WithOptionalResponse<void, Config>, HttpClientError.HttpClientError>
  /**
* The per-viewer delta stream, as Server-Sent Events (`text/event-stream`). The first event is a
* full redacted snapshot at the current seq; every later event is a redacted delta. On (re)connect
* the client just gets a fresh snapshot, so there's no history buffer — the snapshot's seq is the
* resume point. SSE (over fetch, not `EventSource`) so the generated client can consume it as a
* typed `Stream<StreamFrame>` (ADR 0005).
*/
"streamSse": (table: string) => Stream.Stream<Stream200Sse, HttpClientError.HttpClientError>
  /**
* Mark (or clear) a seat's turn yield: auto-pass until that seat's next turn, or until they
* take an intentional action (ADR 0029). Independent of stack yield.
*/
"setTurnYield": <Config extends OperationConfig>(table: string, options: { payload: SetTurnYieldRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<SetTurnYield200, Config>, HttpClientError.HttpClientError>
  /**
* Mark (or clear) a seat's "don't care" yield: while the stack is non-empty, that seat is
* auto-passed as if it had no meaningful action, so the stack resolves without waiting on
* them. Cleared automatically once the stack empties (it's a per-stack yield, not a standing
* one). Enabling may unstick the game immediately — the yielder might be the very player
* everyone is waiting on — so this drives auto-advance and broadcasts like any intent.
*/
"setYield": <Config extends OperationConfig>(table: string, options: { payload: SetYieldRequestJson; config?: Config | undefined }) => Effect.Effect<WithOptionalResponse<SetYield200, Config>, HttpClientError.HttpClientError>
}

export interface MtgfrError<Tag extends string, E> {
  _tag: Tag
  request: HttpClientRequest.HttpClientRequest
  response: HttpClientResponse.HttpClientResponse
  cause: E
}

class MtgfrErrorImpl extends Data.Error<{
  _tag: string
  cause: any
  request: HttpClientRequest.HttpClientRequest
  response: HttpClientResponse.HttpClientResponse
}> {}

export const MtgfrError = <Tag extends string, E>(
  tag: Tag,
  cause: E,
  response: HttpClientResponse.HttpClientResponse,
): MtgfrError<Tag, E> =>
  new MtgfrErrorImpl({
    _tag: tag,
    cause,
    response,
    request: response.request,
  }) as any
