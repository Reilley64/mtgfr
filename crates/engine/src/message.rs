//! Stable message references for engine-authored player-facing text.
//!
//! No CR chapter ownership — presentation contract only. The engine emits keys and typed params;
//! prose lives in the client catalog.

use crate::*;

macro_rules! message_keys {
    ($($name:ident => $value:literal,)+) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct MessageKey(&'static str);

        impl MessageKey {
            $(pub const $name: MessageKey = MessageKey($value);)+

            pub fn as_str(self) -> &'static str {
                self.0
            }

            pub fn all() -> &'static [MessageKey] {
                &[$(Self::$name,)+]
            }
        }
    };
}

message_keys! {
    EFFECT_CHOOSE_ONE => "effect.choose_one",
    EFFECT_CONDITIONAL => "effect.conditional",
    EFFECT_CONTROL_ATTACH_SELF_TO_ENTERING => "effect.control_attach_self_to_entering",
    EFFECT_CONTROL_EQUIP => "effect.control_equip",
    EFFECT_CONTROL_EXCHANGE_ALL_CREATURES_UNTIL_END_OF_TURN => "effect.control_exchange_all_creatures_until_end_of_turn",
    EFFECT_CONTROL_EXCHANGE_CONTROL => "effect.control_exchange_control",
    EFFECT_CONTROL_GAIN_CONTROL => "effect.control_gain_control",
    EFFECT_CONTROL_GAIN_CONTROL_ALL_UNTIL_END_OF_TURN => "effect.control_gain_control_all_until_end_of_turn",
    EFFECT_CONTROL_GAIN_CONTROL_UNTIL_END_OF_TURN => "effect.control_gain_control_until_end_of_turn",
    EFFECT_CONTROL_GAIN_CONTROL_WHILE => "effect.control_gain_control_while",
    EFFECT_CONTROL_GOAD_TARGET => "effect.control_goad_target",
    EFFECT_CONTROL_GRANT_SOURCE_ABILITIES_UNTIL_END_OF_TURN => "effect.control_grant_source_abilities_until_end_of_turn",
    EFFECT_CONTROL_REGENERATE_SHIELD => "effect.control_regenerate_shield",
    EFFECT_CONTROL_REMOVE_FROM_COMBAT => "effect.control_remove_from_combat",
    EFFECT_CONTROL_REVERT_ALL_CREATURES_TO_OWNERS => "effect.control_revert_all_creatures_to_owners",
    EFFECT_CONTROL_TAP_TARGET => "effect.control_tap_target",
    EFFECT_CONTROL_TARGET_OPPONENT_GAINS_CONTROL => "effect.control_target_opponent_gains_control",
    EFFECT_CONTROL_UNTAP_ALL => "effect.control_untap_all",
    EFFECT_CONTROL_UNTAP_TARGET => "effect.control_untap_target",
    EFFECT_COPY_CHANGE_TARGET_OF_TARGET_SPELL_OR_ABILITY => "effect.copy_change_target_of_target_spell_or_ability",
    EFFECT_COPY_COPY_TRIGGERING_ABILITY => "effect.copy_copy_triggering_ability",
    EFFECT_COPY_COPY_TRIGGERING_SPELL => "effect.copy_copy_triggering_spell",
    EFFECT_COPY_COPY_TRIGGERING_SPELL_FOR_EACH_OTHER_CREATURE_YOU_CONTROL => "effect.copy_copy_triggering_spell_for_each_other_creature_you_control",
    EFFECT_COPY_DEMONSTRATE => "effect.copy_demonstrate",
    EFFECT_COPY_MAY_PAY_TO_COPY_THIS => "effect.copy_may_pay_to_copy_this",
    EFFECT_COPY_MINT_FREE_COPY_OF_EXILED_CARD => "effect.copy_mint_free_copy_of_exiled_card",
    EFFECT_COPY_RETARGET_SPELL_COPY => "effect.copy_retarget_spell_copy",
    EFFECT_COPY_TARGET_SPELL => "effect.copy_target_spell",
    EFFECT_COPY_THIS_SPELL => "effect.copy_this_spell",
    EFFECT_COUNTERS_ATTACKER_DRAWS_CONTROLLER_COUNTERS => "effect.counters_attacker_draws_controller_counters",
    EFFECT_COUNTERS_COMMANDER_ENTERS_WITH_BONUS_COUNTERS => "effect.counters_commander_enters_with_bonus_counters",
    EFFECT_COUNTERS_DOUBLE_COUNTERS => "effect.counters_double_counters",
    EFFECT_COUNTERS_DOUBLE_COUNTERS_ON_ATTACHED_CREATURE => "effect.counters_double_counters_on_attached_creature",
    EFFECT_COUNTERS_DOUBLE_COUNTERS_ON_TARGET_CREATURES => "effect.counters_double_counters_on_target_creatures",
    EFFECT_COUNTERS_LEVEL_UP => "effect.counters_level_up",
    EFFECT_COUNTERS_MOVE_COUNTERS => "effect.counters_move_counters",
    EFFECT_COUNTERS_PLACE_VOW_COUNTERS => "effect.counters_place_vow_counters",
    EFFECT_COUNTERS_PUT_COUNTERS => "effect.counters_put_counters",
    EFFECT_COUNTERS_PUT_COUNTERS_EACH => "effect.counters_put_counters_each",
    EFFECT_COUNTERS_REMOVE_ALL_COUNTERS_THEN_DRAW => "effect.counters_remove_all_counters_then_draw",
    EFFECT_COUNTERS_REMOVE_COUNTER_FROM_SELF => "effect.counters_remove_counter_from_self",
    EFFECT_CHOICE_CAST_CREATURE_FACE_DOWN => "effect.choice_cast_creature_face_down",
    EFFECT_CHOICE_CASTER_KEEPS_ONE_OF_EACH_TYPE_PER_PLAYER => "effect.choice_caster_keeps_one_of_each_type_per_player",
    EFFECT_CHOICE_CHOOSE_COLOR => "effect.choice_choose_color",
    EFFECT_CHOICE_CHOOSE_CREATURE_TYPE => "effect.choice_choose_creature_type",
    EFFECT_CHOICE_COUNCILS_DILEMMA_VOTE => "effect.choice_councils_dilemma_vote",
    EFFECT_CHOICE_DAMAGING_CREATURE_CONTROLLER_MAY_DRAW => "effect.choice_damaging_creature_controller_may_draw",
    EFFECT_CHOICE_DEFENDING_PLAYER_SACRIFICES => "effect.choice_defending_player_sacrifices",
    EFFECT_CHOICE_DISCARD => "effect.choice_discard",
    EFFECT_CHOICE_EACH_OTHER_TOKEN_BECOMES_COPY_OF_CHOSEN => "effect.choice_each_other_token_becomes_copy_of_chosen",
    EFFECT_CHOICE_EACH_PLAYER_CONTROLLER_CHOOSES_COUNTER_TARGET => "effect.choice_each_player_controller_chooses_counter_target",
    EFFECT_CHOICE_EACH_PLAYER_CREATES_FRACTAL_FROM_EXILED_POWER => "effect.choice_each_player_creates_fractal_from_exiled_power",
    EFFECT_CHOICE_EACH_PLAYER_DISCARDS_HAND_THEN_DRAWS => "effect.choice_each_player_discards_hand_then_draws",
    EFFECT_CHOICE_EACH_PLAYER_EXILES_FROM_GRAVEYARD => "effect.choice_each_player_exiles_from_graveyard",
    EFFECT_CHOICE_EACH_PLAYER_NAMES_CARD_THEN_REVEALS_TOP => "effect.choice_each_player_names_card_then_reveals_top",
    EFFECT_CHOICE_EACH_PLAYER_SACRIFICES => "effect.choice_each_player_sacrifices",
    EFFECT_CHOICE_JOIN_FORCES_PAY_MANA => "effect.choice_join_forces_pay_mana",
    EFFECT_CHOICE_MAY_DISCARD => "effect.choice_may_discard",
    EFFECT_CHOICE_MAY_DRAW_UNLESS_PAYS => "effect.choice_may_draw_unless_pays",
    EFFECT_CHOICE_MAY_DRAW_UP_TO => "effect.choice_may_draw_up_to",
    EFFECT_CHOICE_MAY_DRAW_UP_TO_THEN_OPPONENT_MAY_REPEAT => "effect.choice_may_draw_up_to_then_opponent_may_repeat",
    EFFECT_CHOICE_MAY_RETURN_FROM_GRAVEYARD => "effect.choice_may_return_from_graveyard",
    EFFECT_CHOICE_MAY_SACRIFICE => "effect.choice_may_sacrifice",
    EFFECT_CHOICE_PHASE_OUT => "effect.choice_phase_out",
    EFFECT_CHOICE_PROLIFERATE => "effect.choice_proliferate",
    EFFECT_CHOICE_PUT_COUNTER_THEN_MAY_BECOME_COPY_OF_CARD_FROM_LIST => "effect.choice_put_counter_then_may_become_copy_of_card_from_list",
    EFFECT_CHOICE_PUT_CREATURE_FROM_HAND => "effect.choice_put_creature_from_hand",
    EFFECT_CHOICE_PUT_FROM_HAND_ON_TOP => "effect.choice_put_from_hand_on_top",
    EFFECT_CHOICE_PUT_LAND_FROM_HAND => "effect.choice_put_land_from_hand",
    EFFECT_CHOICE_SACRIFICE_OWN => "effect.choice_sacrifice_own",
    EFFECT_CHOICE_SACRIFICE_SELF_UNLESS_PAY => "effect.choice_sacrifice_self_unless_pay",
    EFFECT_CHOICE_SACRIFICE_SELF_UNLESS_RETURN_LAND => "effect.choice_sacrifice_self_unless_return_land",
    EFFECT_CHOICE_SET_OWN_COLOR_UNTIL_END_OF_TURN => "effect.choice_set_own_color_until_end_of_turn",
    EFFECT_CHOICE_TARGET_PLAYER_EXILES_FROM_GRAVEYARD => "effect.choice_target_player_exiles_from_graveyard",
    EFFECT_CHOICE_TARGET_PLAYER_MAY_DRAW => "effect.choice_target_player_may_draw",
    EFFECT_DAMAGE_EACH_CREATURE => "effect.damage_each_creature",
    EFFECT_DAMAGE_EACH_OTHER_OPPONENT => "effect.damage_each_other_opponent",
    EFFECT_DAMAGE_EACH_PLAYER => "effect.damage_each_player",
    EFFECT_DAMAGE_TARGET => "effect.damage_target",
    EFFECT_DAMAGE_TO_ENTERING_PERMANENT => "effect.damage_to_entering_permanent",
    EFFECT_DAMAGE_TO_SELF => "effect.damage_to_self",
    EFFECT_DAMAGE_TO_TARGET_CONTROLLER => "effect.damage_to_target_controller",
    EFFECT_DESTROY_ALL => "effect.destroy_all",
    EFFECT_DESTROY_TARGET => "effect.destroy_target",
    EFFECT_DESTROY_TRIGGERING_DAMAGED_CREATURE => "effect.destroy_triggering_damaged_creature",
    EFFECT_DIG_CASCADE => "effect.dig_cascade",
    EFFECT_DIG_CASH_OUT_EXILED_WITH_THIS => "effect.dig_cash_out_exiled_with_this",
    EFFECT_DIG_CAST_EXILED_WITH_THIS_FREE => "effect.dig_cast_exiled_with_this_free",
    EFFECT_DIG_CLASH => "effect.dig_clash",
    EFFECT_DIG_DISTRIBUTE_TOP => "effect.dig_distribute_top",
    EFFECT_DIG_EACH_PLAYER_EXILES_UNTIL_NONLAND_OPPONENT_PICKS => "effect.dig_each_player_exiles_until_nonland_opponent_picks",
    EFFECT_DIG_EXILE_RANDOM_FROM_GRAVEYARD_MAY_PLAY => "effect.dig_exile_random_from_graveyard_may_play",
    EFFECT_DIG_EXILE_TARGET_GRAVEYARD_CARD_RECORD_MANA_VALUE => "effect.dig_exile_target_graveyard_card_record_mana_value",
    EFFECT_DIG_EXILE_TARGET_GRAVEYARD_SPELL_CAST_FREE => "effect.dig_exile_target_graveyard_spell_cast_free",
    EFFECT_DIG_EXILE_TOP_CAST_MATCHING_FREE => "effect.dig_exile_top_cast_matching_free",
    EFFECT_DIG_EXILE_TOP_UNTIL_STOP_CAST_FREE_UNDER_BUDGET => "effect.dig_exile_top_until_stop_cast_free_under_budget",
    EFFECT_DIG_LOOK_AT_TOP => "effect.dig_look_at_top",
    EFFECT_DIG_OPPONENT_SPLITS_EXILE_PILES => "effect.dig_opponent_splits_exile_piles",
    EFFECT_DIG_REVEAL_TOP_OPPONENT_PICKS_ONE_TO_GRAVEYARD => "effect.dig_reveal_top_opponent_picks_one_to_graveyard",
    EFFECT_DIG_REVEAL_TOP_SPLIT_PILES => "effect.dig_reveal_top_split_piles",
    EFFECT_DIG_REVEAL_UNTIL_EXILE_CAST_FREE => "effect.dig_reveal_until_exile_cast_free",
    EFFECT_DIG_REVEAL_UNTIL_MAY_DEPLOY => "effect.dig_reveal_until_may_deploy",
    EFFECT_DIG_SEARCH_LIBRARY => "effect.dig_search_library",
    EFFECT_DIG_SHUFFLE_LIBRARY => "effect.dig_shuffle_library",
    EFFECT_DIG_SHUFFLE_TARGET_CARDS_FROM_GRAVEYARD_INTO_LIBRARY => "effect.dig_shuffle_target_cards_from_graveyard_into_library",
    EFFECT_DIG_SURVEIL => "effect.dig_surveil",
    EFFECT_DRAW_ATTACKING_PLAYER => "effect.draw_attacking_player",
    EFFECT_DRAW_CARDS => "effect.draw_cards",
    EFFECT_DRAW_EACH_DRAW_STEP_PLAYER => "effect.draw_each_draw_step_player",
    EFFECT_DRAW_EACH_PLAYER => "effect.draw_each_player",
    EFFECT_DRAW_TARGET_OWNER => "effect.draw_target_owner",
    EFFECT_DRAW_TARGET_PLAYER => "effect.draw_target_player",
    EFFECT_EXILE_ALL => "effect.exile_all",
    EFFECT_EXILE_ALL_GRAVEYARDS => "effect.exile_all_graveyards",
    EFFECT_EXILE_GRAVEYARD => "effect.exile_graveyard",
    EFFECT_EXILE_OBJECT => "effect.exile_object",
    EFFECT_EXILE_TARGET => "effect.exile_target",
    EFFECT_EXILE_TARGET_MINTING_ILLUSION_ON_LEAVE => "effect.exile_target_minting_illusion_on_leave",
    EFFECT_EXILE_UNTIL_SOURCE_LEAVES => "effect.exile_until_source_leaves",
    EFFECT_LIFE_ATTACKER_LOSES_YOU_DRAW => "effect.life_attacker_loses_you_draw",
    EFFECT_LIFE_ATTACKER_LOSES_YOU_GAIN => "effect.life_attacker_loses_you_gain",
    EFFECT_LIFE_DRAIN_TARGET => "effect.life_drain_target",
    EFFECT_LIFE_EACH_OPPONENT_DRAIN => "effect.life_each_opponent_drain",
    EFFECT_LIFE_EACH_OPPONENT_LOSES => "effect.life_each_opponent_loses",
    EFFECT_LIFE_EACH_PLAYER_BECOMES_HIGHEST => "effect.life_each_player_becomes_highest",
    EFFECT_LIFE_GAIN => "effect.life_gain",
    EFFECT_LIFE_GAIN_TARGET_CONTROLLER => "effect.life_gain_target_controller",
    EFFECT_LIFE_LOSE => "effect.life_lose",
    EFFECT_LIFE_OPPONENT_GAINS => "effect.life_opponent_gains",
    EFFECT_LIFE_TARGET_PLAYER_GAINS => "effect.life_target_player_gains",
    EFFECT_LIFE_TARGET_PLAYER_LOSES => "effect.life_target_player_loses",
    EFFECT_MANA_ADD => "effect.mana_add",
    EFFECT_MILL_EXILE_DISCARDED_WITH_THIS => "effect.mill_exile_discarded_with_this",
    EFFECT_MILL_EXILE_FROM_GRAVEYARD_MAY_PLAY => "effect.mill_exile_from_graveyard_may_play",
    EFFECT_MILL_EXILE_TARGET_FROM_GRAVEYARD_CREATE_TOKEN_COPY => "effect.mill_exile_target_from_graveyard_create_token_copy",
    EFFECT_MILL_EXILE_TARGET_FROM_GRAVEYARD_WITH_THIS => "effect.mill_exile_target_from_graveyard_with_this",
    EFFECT_MILL_EXILE_TOP_MAY_PLAY => "effect.mill_exile_top_may_play",
    EFFECT_MILL_MILL => "effect.mill_mill",
    EFFECT_MILL_MILL_SELF => "effect.mill_mill_self",
    EFFECT_MISC_ARM_COMBAT_DAMAGE_WATCH => "effect.misc_arm_combat_damage_watch",
    EFFECT_MISC_BECOME_PREPARED => "effect.misc_become_prepared",
    EFFECT_MISC_COUNTER_TARGET_ACTIVATED_ABILITY => "effect.misc_counter_target_activated_ability",
    EFFECT_MISC_COUNTER_TARGET_SPELL => "effect.misc_counter_target_spell",
    EFFECT_MISC_FIGHT => "effect.misc_fight",
    EFFECT_MISC_FLIP_SOURCE => "effect.misc_flip_source",
    EFFECT_MISC_GRANT_CHANNEL_COLORLESS_MANA_THIS_TURN => "effect.misc_grant_channel_colorless_mana_this_turn",
    EFFECT_MISC_GRANT_FLASH_THIS_TURN => "effect.misc_grant_flash_this_turn",
    EFFECT_MISC_MUST_ATTACK_RANDOM_OPPONENT => "effect.misc_must_attack_random_opponent",
    EFFECT_MISC_PREVENT_ALL_COMBAT_DAMAGE_THIS_TURN => "effect.misc_prevent_all_combat_damage_this_turn",
    EFFECT_MISC_PREVENT_COMBAT_DAMAGE_TO_YOU_CREATING_TOKENS => "effect.misc_prevent_combat_damage_to_you_creating_tokens",
    EFFECT_MISC_SCHEDULE_AT_NEXT_UPKEEP => "effect.misc_schedule_at_next_upkeep",
    EFFECT_MISC_SCHEDULE_COLORLESS_MANA_FOR_COUNTERED_SPELL_NEXT_MAIN_PHASE => "effect.misc_schedule_colorless_mana_for_countered_spell_next_main_phase",
    EFFECT_MISC_SCHEDULE_NEXT_CAST_TRIGGER => "effect.misc_schedule_next_cast_trigger",
    EFFECT_MISC_SCHEDULE_THIS_TURN_COMBAT_DAMAGE_COPY => "effect.misc_schedule_this_turn_combat_damage_copy",
    EFFECT_MISC_SKIP_NEXT_UNTAP_OPPONENT_CREATURES => "effect.misc_skip_next_untap_opponent_creatures",
    EFFECT_PUMP_ANIMATE_SELF_UNTIL_END_OF_TURN => "effect.pump_animate_self_until_end_of_turn",
    EFFECT_PUMP_ENCHANTED_ATTACKER_PUMP_ATTACKING_OPPONENT_ELSE_CONTROLLER_LOSES_LIFE => "effect.pump_enchanted_attacker_pump_attacking_opponent_else_controller_loses_life",
    EFFECT_PUMP_GRANT_KEYWORDS_TO_PERMANENTS_YOU_CONTROL_UNTIL_END_OF_TURN => "effect.pump_grant_keywords_to_permanents_you_control_until_end_of_turn",
    EFFECT_PUMP_PUMP_CREATURES_YOU_CONTROL_UNTIL_END_OF_TURN => "effect.pump_pump_creatures_you_control_until_end_of_turn",
    EFFECT_PUMP_PUMP_OTHER_ATTACKERS_ATTACKING_YOUR_OPPONENTS => "effect.pump_pump_other_attackers_attacking_your_opponents",
    EFFECT_PUMP_PUMP_SELF_UNTIL_END_OF_TURN => "effect.pump_pump_self_until_end_of_turn",
    EFFECT_PUMP_PUMP_UNTIL_END_OF_TURN => "effect.pump_pump_until_end_of_turn",
    EFFECT_PUMP_SET_BASE_PT_CREATURES_YOU_CONTROL_UNTIL_END_OF_TURN => "effect.pump_set_base_pt_creatures_you_control_until_end_of_turn",
    EFFECT_PUMP_SET_BASE_PT_TARGET_UNTIL_END_OF_TURN => "effect.pump_set_base_pt_target_until_end_of_turn",
    EFFECT_PUMP_SET_OWN_BASE_PT_FROM_AMOUNT => "effect.pump_set_own_base_pt_from_amount",
    EFFECT_PUMP_STRIP_KEYWORDS_FROM_OPPONENTS_CREATURES => "effect.pump_strip_keywords_from_opponents_creatures",
    EFFECT_PUMP_WEAKEN_EACH_CREATURE => "effect.pump_weaken_each_creature",
    EFFECT_REVEAL_TOP_AND_DRAIN_MUTUAL => "effect.reveal_top_and_drain_mutual",
    EFFECT_REVEAL_TOP_CARDS => "effect.reveal_top_cards",
    EFFECT_REVEAL_TOP_TO_HAND => "effect.reveal_top_to_hand",
    EFFECT_REVEAL_UNTIL => "effect.reveal_until",
    EFFECT_SACRIFICE_ENCHANTED_CREATURE => "effect.sacrifice_enchanted_creature",
    EFFECT_SACRIFICE_OBJECT => "effect.sacrifice_object",
    EFFECT_SACRIFICE_SOURCE => "effect.sacrifice_source",
    EFFECT_SCRY => "effect.scry",
    EFFECT_SEQUENCE => "effect.sequence",
    EFFECT_STATIC_ANTHEM => "effect.static_anthem",
    EFFECT_STATIC_ATTACK_TAX => "effect.static_attack_tax",
    EFFECT_STATIC_CANT_BE_ATTACKED_BY => "effect.static_cant_be_attacked_by",
    EFFECT_STATIC_CAST_X_REPLACEMENT => "effect.static_cast_x_replacement",
    EFFECT_STATIC_CONTROL_ATTACHED => "effect.static_control_attached",
    EFFECT_STATIC_COUNTER_REPLACEMENT => "effect.static_counter_replacement",
    EFFECT_STATIC_COUNTER_SCALED_ATTACK_TAX => "effect.static_counter_scaled_attack_tax",
    EFFECT_STATIC_CREATURES_YOU_CONTROL_ENTER_WITH_COUNTERS => "effect.static_creatures_you_control_enter_with_counters",
    EFFECT_STATIC_ENTERS_WITH_COUNTERS => "effect.static_enters_with_counters",
    EFFECT_STATIC_GRANT_MANA_ABILITY => "effect.static_grant_mana_ability",
    EFFECT_STATIC_GRANT_TO_ATTACHED => "effect.static_grant_to_attached",
    EFFECT_STATIC_KEYWORD_ANTHEM => "effect.static_keyword_anthem",
    EFFECT_STATIC_LIFE_GAIN_REPLACEMENT => "effect.static_life_gain_replacement",
    EFFECT_STATIC_NO_MAXIMUM_HAND_SIZE => "effect.static_no_maximum_hand_size",
    EFFECT_STATIC_PLAY_FROM_GRAVEYARD_ONCE_PER_TURN => "effect.static_play_from_graveyard_once_per_turn",
    EFFECT_STATIC_PREVENT_COMBAT_DAMAGE => "effect.static_prevent_combat_damage",
    EFFECT_STATIC_PREVENT_DAMAGE_TO_SELF_REMOVING_COUNTER => "effect.static_prevent_damage_to_self_removing_counter",
    EFFECT_STATIC_PREVENT_NONCOMBAT_DAMAGE_TO_OTHER_CREATURES_YOU_CONTROL => "effect.static_prevent_noncombat_damage_to_other_creatures_you_control",
    EFFECT_STATIC_REDUCE_SPELL_COST => "effect.static_reduce_spell_cost",
    EFFECT_STATIC_SET_ATTACHED_BASE_PT => "effect.static_set_attached_base_pt",
    EFFECT_STATIC_SET_ATTACHED_TYPES => "effect.static_set_attached_types",
    EFFECT_STATIC_TAPPED_FOR_MANA_BONUS => "effect.static_tapped_for_mana_bonus",
    EFFECT_STATIC_TOKEN_REPLACEMENT => "effect.static_token_replacement",
    EFFECT_STATIC_TRIGGER_DOUBLING => "effect.static_trigger_doubling",
    EFFECT_TOKEN_BECOME_COPY_OF_TARGET_CREATURE_GAINING_MYRIAD => "effect.token_become_copy_of_target_creature_gaining_myriad",
    EFFECT_TOKEN_COPY_EACH_ENTERED_THIS_TURN_TOKEN_TAPPED_ATTACKING => "effect.token_copy_each_entered_this_turn_token_tapped_attacking",
    EFFECT_TOKEN_CREATE => "effect.token_create",
    EFFECT_TOKEN_CREATE_COPY => "effect.token_create_copy",
    EFFECT_TOKEN_CREATE_TREASURE => "effect.token_create_treasure",
    EFFECT_TOKEN_MYRIAD_TOKEN_COPIES => "effect.token_myriad_token_copies",
    EFFECT_ZONE_ATTACH_MINTED_AURA_TO_TARGET => "effect.zone_attach_minted_aura_to_target",
    EFFECT_ZONE_ATTACH_SELF_TO_MINTED_TOKEN => "effect.zone_attach_self_to_minted_token",
    EFFECT_ZONE_ATTACH_SELF_TO_REANIMATED => "effect.zone_attach_self_to_reanimated",
    EFFECT_ZONE_ATTACH_TRIGGERING_AURA_TO_MINTED_TOKEN => "effect.zone_attach_triggering_aura_to_minted_token",
    EFFECT_ZONE_EXILE_DEAD_CREATURE_CREATE_COPY_WITH_SUBTYPE => "effect.zone_exile_dead_creature_create_copy_with_subtype",
    EFFECT_ZONE_EXILE_GRAVEYARD_OBJECT_GAIN_LIFE => "effect.zone_exile_graveyard_object_gain_life",
    EFFECT_ZONE_EXILE_SELF_ON_RESOLVE => "effect.zone_exile_self_on_resolve",
    EFFECT_ZONE_EXILE_SELF_WITH_TIME_COUNTERS => "effect.zone_exile_self_with_time_counters",
    EFFECT_ZONE_EXILE_TARGET_GRAVEYARD_CARD_THEN_IF_CREATURE => "effect.zone_exile_target_graveyard_card_then_if_creature",
    EFFECT_ZONE_FLICKER_TARGET => "effect.zone_flicker_target",
    EFFECT_ZONE_MANIFEST => "effect.zone_manifest",
    EFFECT_ZONE_MASS_RETURN_FROM_GRAVEYARD => "effect.zone_mass_return_from_graveyard",
    EFFECT_ZONE_REANIMATE_DYING_ENCHANTED_CREATURE => "effect.zone_reanimate_dying_enchanted_creature",
    EFFECT_ZONE_REANIMATE_TO_BATTLEFIELD => "effect.zone_reanimate_to_battlefield",
    EFFECT_ZONE_REFLEXIVE_TRIGGER => "effect.zone_reflexive_trigger",
    EFFECT_ZONE_RETURN_ALL_TO_HAND => "effect.zone_return_all_to_hand",
    EFFECT_ZONE_RETURN_EXILED_CARD_TO_OWNERS_GRAVEYARD => "effect.zone_return_exiled_card_to_owners_graveyard",
    EFFECT_ZONE_RETURN_FLICKERED_CARD => "effect.zone_return_flickered_card",
    EFFECT_ZONE_RETURN_FROM_GRAVEYARD_ATTACHED_TO_TOKEN => "effect.zone_return_from_graveyard_attached_to_token",
    EFFECT_ZONE_RETURN_FROM_GRAVEYARD_TO_HAND => "effect.zone_return_from_graveyard_to_hand",
    EFFECT_ZONE_RETURN_OBJECT_TO_HAND => "effect.zone_return_object_to_hand",
    EFFECT_ZONE_RETURN_THIS_AURA_ATTACHED_TO => "effect.zone_return_this_aura_attached_to",
    EFFECT_ZONE_RETURN_THIS_AURA_FROM_GRAVEYARD_ATTACHED_TO_CHOSEN_HOST => "effect.zone_return_this_aura_from_graveyard_attached_to_chosen_host",
    EFFECT_ZONE_RETURN_THIS_FROM_GRAVEYARD_TO_BATTLEFIELD => "effect.zone_return_this_from_graveyard_to_battlefield",
    EFFECT_ZONE_RETURN_THIS_TO_HAND => "effect.zone_return_this_to_hand",
    EFFECT_ZONE_RETURN_TO_HAND => "effect.zone_return_to_hand",
    EFFECT_ZONE_SCHEDULE_RETURN_REANIMATED_TO_HAND => "effect.zone_schedule_return_reanimated_to_hand",
    EFFECT_ZONE_SCHEDULE_RETURN_THIS_AURA_ATTACHED_TO_REANIMATED => "effect.zone_schedule_return_this_aura_attached_to_reanimated",
    EFFECT_ZONE_SCHEDULE_RETURN_THIS_AURA_FROM_GRAVEYARD_ATTACHED_TO_CHOSEN_HOST => "effect.zone_schedule_return_this_aura_from_graveyard_attached_to_chosen_host",
    EFFECT_ZONE_SHUFFLE_TARGET_PERMANENT_INTO_LIBRARY => "effect.zone_shuffle_target_permanent_into_library",
    EFFECT_ZONE_SHUFFLE_TARGET_PERMANENT_INTO_LIBRARY_THEN_REVEAL => "effect.zone_shuffle_target_permanent_into_library_then_reveal",
    EFFECT_ZONE_TUCK_FROM_GRAVEYARD => "effect.zone_tuck_from_graveyard",
    EFFECT_ZONE_TUCK_PERMANENT_INTO_LIBRARY => "effect.zone_tuck_permanent_into_library",
    EFFECT_ZONE_TUCK_SELF_AND_BLOCKED_CREATURES => "effect.zone_tuck_self_and_blocked_creatures",
    EFFECT_ZONE_TUCK_SELF_TO_LIBRARY_BOTTOM => "effect.zone_tuck_self_to_library_bottom",
    EFFECT_ZONE_UNTAP_SEARCHED_LAND => "effect.zone_untap_searched_land",
    REJECT_CANNOT_ACTIVATE => "reject.cannot_activate",
    REJECT_CANNOT_PAY_COST => "reject.cannot_pay_cost",
    REJECT_CANNOT_PRODUCE_MANA => "reject.cannot_produce_mana",
    REJECT_CHOICE_PENDING => "reject.choice_pending",
    REJECT_ILLEGAL_CHOICE => "reject.illegal_choice",
    REJECT_ILLEGAL_DECLARATION => "reject.illegal_declaration",
    REJECT_ILLEGAL_MODE => "reject.illegal_mode",
    REJECT_ILLEGAL_TARGET => "reject.illegal_target",
    REJECT_MULLIGANING => "reject.mulliganing",
    REJECT_NOT_CASTABLE => "reject.not_castable",
    REJECT_NOT_YOUR_PRIORITY => "reject.not_your_priority",
    REJECT_UNKNOWN_ACTION => "reject.unknown_action",
    REJECT_UNKNOWN_OBJECT => "reject.unknown_object",
    REJECT_WRONG_TIMING => "reject.wrong_timing",
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageParam {
    pub name: &'static str,
    pub value: MessageParamValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageParamValue {
    Str(&'static str),
    OwnedStr(String),
    Int(i64),
    Bool(bool),
    AmountToken(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRef {
    pub key: MessageKey,
    pub params: Vec<MessageParam>,
    pub children: Vec<MessageRef>,
}

impl MessageRef {
    fn new(key: MessageKey) -> Self {
        Self {
            key,
            params: Vec::new(),
            children: Vec::new(),
        }
    }

    fn with_params(mut self, params: Vec<MessageParam>) -> Self {
        self.params = params;
        self
    }

    fn with_children(mut self, children: Vec<MessageRef>) -> Self {
        self.children = children;
        self
    }
}

pub fn reject_message(reject: Reject) -> MessageRef {
    MessageRef::new(match reject {
        Reject::NotCastable => MessageKey::REJECT_NOT_CASTABLE,
        Reject::NotYourPriority => MessageKey::REJECT_NOT_YOUR_PRIORITY,
        Reject::CannotPayCost => MessageKey::REJECT_CANNOT_PAY_COST,
        Reject::CannotProduceMana => MessageKey::REJECT_CANNOT_PRODUCE_MANA,
        Reject::CannotActivate => MessageKey::REJECT_CANNOT_ACTIVATE,
        Reject::IllegalDeclaration => MessageKey::REJECT_ILLEGAL_DECLARATION,
        Reject::IllegalTarget => MessageKey::REJECT_ILLEGAL_TARGET,
        Reject::IllegalMode => MessageKey::REJECT_ILLEGAL_MODE,
        Reject::WrongTiming => MessageKey::REJECT_WRONG_TIMING,
        Reject::Mulliganing => MessageKey::REJECT_MULLIGANING,
        Reject::ChoicePending => MessageKey::REJECT_CHOICE_PENDING,
        Reject::IllegalChoice => MessageKey::REJECT_ILLEGAL_CHOICE,
        Reject::UnknownObject => MessageKey::REJECT_UNKNOWN_OBJECT,
        Reject::UnknownAction => MessageKey::REJECT_UNKNOWN_ACTION,
    })
}

pub fn amount_param(name: &'static str, amount: Amount) -> MessageParam {
    let value = match amount {
        Amount::Fixed(n) => MessageParamValue::Int(i64::from(n)),
        _ => MessageParamValue::AmountToken(amount_token(amount)),
    };
    MessageParam { name, value }
}

fn int_param(name: &'static str, value: impl Into<i64>) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::Int(value.into()),
    }
}

fn bool_param(name: &'static str, value: bool) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::Bool(value),
    }
}

fn str_param(name: &'static str, value: &'static str) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::Str(value),
    }
}

fn debug_param(name: &'static str, value: impl std::fmt::Debug) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::OwnedStr(debug_token(value)),
    }
}

fn mana_param(name: &'static str, cost: Cost) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::OwnedStr(cost.mana_label()),
    }
}

fn name_param(name: &'static str, value: &'static str) -> MessageParam {
    MessageParam {
        name,
        value: MessageParamValue::OwnedStr(value.to_string()),
    }
}

fn debug_token(value: impl std::fmt::Debug) -> String {
    let raw = format!("{value:?}");
    let mut out = String::new();
    let mut prev_sep = true;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
            continue;
        }
        if !prev_sep {
            out.push('_');
            prev_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

fn amount_token(amount: Amount) -> &'static str {
    match amount {
        Amount::Fixed(_) => "fixed",
        Amount::X => "x",
        Amount::HalfX => "half_x",
        Amount::HalfXRoundedDown => "half_x_rounded_down",
        Amount::TwiceX => "twice_x",
        Amount::PerCreatureYouControl => "per_creature_you_control",
        Amount::PerCreatureOnBattlefield => "per_creature_on_battlefield",
        Amount::PerPermanentMatching { .. } => "per_permanent_matching",
        Amount::SourcePower => "source_power",
        Amount::SourceToughness => "source_toughness",
        Amount::TargetPower => "target_power",
        Amount::TargetToughness => "target_toughness",
        Amount::TargetManaValue => "target_mana_value",
        Amount::PerCounterOnSource => "per_counter_on_source",
        Amount::PerCounterOfKindOnSource { .. } => "per_counter_of_kind_on_source",
        Amount::LifeGainedThisTurn => "life_gained_this_turn",
        Amount::SpellsCastThisTurn => "spells_cast_this_turn",
        Amount::CardsInTargetPlayerHand => "cards_in_target_player_hand",
        Amount::CardsInYourHand => "cards_in_your_hand",
        Amount::CommanderCastsFromCommandZone => "commander_casts_from_command_zone",
        Amount::CreaturesDiedThisTurn => "creatures_died_this_turn",
        Amount::NontokenCreaturesEnteredThisTurn => "nontoken_creatures_entered_this_turn",
        Amount::SacrificedCreaturePower => "sacrificed_creature_power",
        Amount::SacrificedCreatureToughness => "sacrificed_creature_toughness",
        Amount::CommanderColorCount => "commander_color_count",
        Amount::TotalPowerYouControl => "total_power_you_control",
        Amount::PermanentsYouOwnOpponentsControl => "permanents_you_own_opponents_control",
        Amount::IfCondition { .. } => "if_condition",
        Amount::TriggeringSpellManaValue => "triggering_spell_mana_value",
        Amount::TriggeringSpellManaSpent => "triggering_spell_mana_spent",
        Amount::SpellSacrificeCount => "spell_sacrifice_count",
        Amount::RevealedCreatureManaValue => "revealed_creature_mana_value",
        Amount::PermanentsDiedThisTurn => "permanents_died_this_turn",
        Amount::PermanentsDestroyedThisWay { .. } => "permanents_destroyed_this_way",
        Amount::NonlandCardsExiledThisWay => "nonland_cards_exiled_this_way",
        Amount::CardsExiledBySearchThisWay => "cards_exiled_by_search_this_way",
        Amount::ManaPaidThisWay => "mana_paid_this_way",
        Amount::PastVotes => "past_votes",
        Amount::PresentVotes => "present_votes",
        Amount::TotalManaValueMilledThisWay => "total_mana_value_milled_this_way",
        Amount::ExiledCardManaValueThisWay => "exiled_card_mana_value_this_way",
        Amount::ReturnedNonlandCardManaValue => "returned_nonland_card_mana_value",
        Amount::AurasYouControlledAttachedToDyingCreature => {
            "auras_you_controlled_attached_to_dying_creature"
        }
        Amount::IfSpellKicked { .. } => "if_spell_kicked",
        Amount::GreatestInstantOrSorceryManaValueCastThisTurn => {
            "greatest_instant_or_sorcery_mana_value_cast_this_turn"
        }
        Amount::OnePlusInstantsAndSorceriesCastThisTurn => {
            "one_plus_instants_and_sorceries_cast_this_turn"
        }
        Amount::AurasAttachedToSource => "auras_attached_to_source",
        Amount::InstantOrSorceryCardsInYourGraveyard => {
            "instant_or_sorcery_cards_in_your_graveyard"
        }
        Amount::CombatDamageDealt => "combat_damage_dealt",
        Amount::TriggeringDamageDealt => "triggering_damage_dealt",
        Amount::SpellsCastBeforeThisThisTurn => "spells_cast_before_this_this_turn",
    }
}

impl Effect {
    /// Stable key + params for effect prose rendered by the client catalog.
    ///
    /// The match is intentionally exhaustive: every new [`Effect`] variant must choose an i18n key.
    pub fn message(self) -> MessageRef {
        use ChoiceEffect::*;
        use ControlEffect::*;
        use CopyEffect::*;
        use CountersEffect::*;
        use DamageEffect::*;
        use DestroyEffect::*;
        use DigEffect::*;
        use DrawEffect::*;
        use ExileEffect::*;
        use LifeEffect::*;
        use MillEffect::*;
        use MiscEffect::*;
        use PumpEffect::*;
        use RevealEffect::*;
        use SacrificeEffect::*;
        use StaticEffect::*;
        use TokenEffect::*;
        use ZoneEffect::*;

        match self {
            Effect::Damage(DamageEffect::Target { amount, .. }) => MessageRef::new(MessageKey::EFFECT_DAMAGE_TARGET)
                .with_params(vec![amount_param("amount", amount)]),
            Effect::Damage(ToSelf { amount }) => MessageRef::new(MessageKey::EFFECT_DAMAGE_TO_SELF)
                .with_params(vec![amount_param("amount", amount)]),
            Effect::Damage(ToTargetController { amount }) => {
                MessageRef::new(MessageKey::EFFECT_DAMAGE_TO_TARGET_CONTROLLER)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Damage(EachCreature {
                amount,
                opponents_only,
                filter,
                include_planeswalkers,
            }) => MessageRef::new(MessageKey::EFFECT_DAMAGE_EACH_CREATURE).with_params(vec![
                amount_param("amount", amount),
                bool_param("opponents_only", opponents_only),
                bool_param("include_planeswalkers", include_planeswalkers),
                debug_param("filter", filter),
            ]),
            Effect::Damage(DamageEffect::EachPlayer { amount }) => {
                MessageRef::new(MessageKey::EFFECT_DAMAGE_EACH_PLAYER)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Damage(EachOtherOpponent { amount, .. }) => {
                MessageRef::new(MessageKey::EFFECT_DAMAGE_EACH_OTHER_OPPONENT)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Damage(ToEnteringPermanent { amount, .. }) => {
                MessageRef::new(MessageKey::EFFECT_DAMAGE_TO_ENTERING_PERMANENT)
                    .with_params(vec![int_param("amount", amount)])
            }
            Effect::Draw(Cards { count }) => MessageRef::new(MessageKey::EFFECT_DRAW_CARDS)
                .with_params(vec![amount_param("count", count)]),
            Effect::Draw(TargetPlayer { count, opponent }) => {
                MessageRef::new(MessageKey::EFFECT_DRAW_TARGET_PLAYER)
                    .with_params(vec![amount_param("count", count), bool_param("opponent", opponent)])
            }
            Effect::Draw(TargetOwner { count, controller }) => {
                MessageRef::new(MessageKey::EFFECT_DRAW_TARGET_OWNER).with_params(vec![
                    amount_param("count", count),
                    bool_param("controller", controller),
                ])
            }
            Effect::Draw(DrawEffect::EachPlayer { count }) => MessageRef::new(MessageKey::EFFECT_DRAW_EACH_PLAYER)
                .with_params(vec![amount_param("count", count)]),
            Effect::Draw(AttackingPlayer { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_DRAW_ATTACKING_PLAYER)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Draw(EachDrawStepPlayer { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_DRAW_EACH_DRAW_STEP_PLAYER)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Life(Gain { amount }) => MessageRef::new(MessageKey::EFFECT_LIFE_GAIN)
                .with_params(vec![amount_param("amount", amount)]),
            Effect::Life(OpponentGains { amount }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_OPPONENT_GAINS)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Life(Lose { amount }) => MessageRef::new(MessageKey::EFFECT_LIFE_LOSE)
                .with_params(vec![amount_param("amount", amount)]),
            Effect::Life(GainTargetController { amount }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_GAIN_TARGET_CONTROLLER)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Life(DrainTarget { amount, opponent }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_DRAIN_TARGET)
                    .with_params(vec![int_param("amount", amount), bool_param("opponent", opponent)])
            }
            Effect::Life(TargetPlayerGains { amount, opponent }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_TARGET_PLAYER_GAINS)
                    .with_params(vec![int_param("amount", amount), bool_param("opponent", opponent)])
            }
            Effect::Life(EachOpponentDrain { amount, sum_gain }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_EACH_OPPONENT_DRAIN)
                    .with_params(vec![amount_param("amount", amount), bool_param("sum_gain", sum_gain)])
            }
            Effect::Life(EachOpponentLoses { amount }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_EACH_OPPONENT_LOSES)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Life(EachPlayerBecomesHighest) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_EACH_PLAYER_BECOMES_HIGHEST)
            }
            Effect::Life(TargetPlayerLoses { amount }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_TARGET_PLAYER_LOSES)
                    .with_params(vec![int_param("amount", amount)])
            }
            Effect::Life(AttackerLosesYouGain { amount, .. }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_ATTACKER_LOSES_YOU_GAIN)
                    .with_params(vec![int_param("amount", amount)])
            }
            Effect::Life(AttackerLosesYouDraw { life_loss, .. }) => {
                MessageRef::new(MessageKey::EFFECT_LIFE_ATTACKER_LOSES_YOU_DRAW)
                    .with_params(vec![int_param("life_loss", life_loss)])
            }
            Effect::Destroy(DestroyEffect::Target { .. }) => MessageRef::new(MessageKey::EFFECT_DESTROY_TARGET),
            Effect::Destroy(DestroyEffect::All { filter }) => {
                MessageRef::new(MessageKey::EFFECT_DESTROY_ALL)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Destroy(TriggeringDamagedCreature { .. }) => {
                MessageRef::new(MessageKey::EFFECT_DESTROY_TRIGGERING_DAMAGED_CREATURE)
            }
            Effect::Exile(ExileEffect::Target { .. }) => MessageRef::new(MessageKey::EFFECT_EXILE_TARGET),
            Effect::Exile(ExileEffect::All { filter }) => MessageRef::new(MessageKey::EFFECT_EXILE_ALL)
                .with_params(vec![debug_param("filter", filter)]),
            Effect::Exile(UntilSourceLeaves { .. }) => {
                MessageRef::new(MessageKey::EFFECT_EXILE_UNTIL_SOURCE_LEAVES)
            }
            Effect::Exile(TargetMintingIllusionOnLeave { .. }) => {
                MessageRef::new(MessageKey::EFFECT_EXILE_TARGET_MINTING_ILLUSION_ON_LEAVE)
            }
            Effect::Exile(Graveyard) => MessageRef::new(MessageKey::EFFECT_EXILE_GRAVEYARD),
            Effect::Exile(AllGraveyards) => MessageRef::new(MessageKey::EFFECT_EXILE_ALL_GRAVEYARDS),
            Effect::Exile(ExileEffect::Object { .. }) => MessageRef::new(MessageKey::EFFECT_EXILE_OBJECT),
            Effect::Sacrifice(SacrificeEffect::Object { .. }) => {
                MessageRef::new(MessageKey::EFFECT_SACRIFICE_OBJECT)
            }
            Effect::Sacrifice(Source) => MessageRef::new(MessageKey::EFFECT_SACRIFICE_SOURCE),
            Effect::Sacrifice(EnchantedCreature { .. }) => {
                MessageRef::new(MessageKey::EFFECT_SACRIFICE_ENCHANTED_CREATURE)
            }
            Effect::Control(RegenerateShield { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_REGENERATE_SHIELD)
            }
            Effect::Control(Equip) => MessageRef::new(MessageKey::EFFECT_CONTROL_EQUIP),
            Effect::Control(AttachSelfToEntering { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_ATTACH_SELF_TO_ENTERING)
            }
            Effect::Control(GoadTarget { .. }) => MessageRef::new(MessageKey::EFFECT_CONTROL_GOAD_TARGET),
            Effect::Control(TapTarget { .. }) => MessageRef::new(MessageKey::EFFECT_CONTROL_TAP_TARGET),
            Effect::Control(UntapTarget { .. }) => MessageRef::new(MessageKey::EFFECT_CONTROL_UNTAP_TARGET),
            Effect::Control(RemoveFromCombat { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_REMOVE_FROM_COMBAT)
            }
            Effect::Control(GainControlUntilEndOfTurn { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_GAIN_CONTROL_UNTIL_END_OF_TURN)
            }
            Effect::Control(GainControl { .. }) => MessageRef::new(MessageKey::EFFECT_CONTROL_GAIN_CONTROL),
            Effect::Control(GainControlWhile { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_GAIN_CONTROL_WHILE)
            }
            Effect::Control(TargetOpponentGainsControl { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_TARGET_OPPONENT_GAINS_CONTROL)
            }
            Effect::Control(ExchangeControl { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_EXCHANGE_CONTROL)
            }
            Effect::Control(ExchangeAllCreaturesUntilEndOfTurn { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_EXCHANGE_ALL_CREATURES_UNTIL_END_OF_TURN)
            }
            Effect::Control(GainControlAllUntilEndOfTurn { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_GAIN_CONTROL_ALL_UNTIL_END_OF_TURN)
            }
            Effect::Control(RevertAllCreaturesToOwners) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_REVERT_ALL_CREATURES_TO_OWNERS)
            }
            Effect::Control(GrantSourceAbilitiesUntilEndOfTurn) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_GRANT_SOURCE_ABILITIES_UNTIL_END_OF_TURN)
            }
            Effect::Control(UntapAll { filter }) => {
                MessageRef::new(MessageKey::EFFECT_CONTROL_UNTAP_ALL)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Counters(PlaceVowCounters { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_PLACE_VOW_COUNTERS)
            }
            Effect::Counters(PutCounters { count, kind, .. }) => {
                let mut params = vec![amount_param("count", count)];
                if let Some(kind) = kind {
                    params.push(debug_param("kind", kind));
                }
                MessageRef::new(MessageKey::EFFECT_COUNTERS_PUT_COUNTERS).with_params(params)
            }
            Effect::Counters(DoubleCounters { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_DOUBLE_COUNTERS)
            }
            Effect::Counters(DoubleCountersOnAttachedCreature) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_DOUBLE_COUNTERS_ON_ATTACHED_CREATURE)
            }
            Effect::Counters(PutCountersEach { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_PUT_COUNTERS_EACH)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Counters(MoveCounters { all_kinds, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_MOVE_COUNTERS)
                    .with_params(vec![bool_param("all_kinds", all_kinds)])
            }
            Effect::Counters(RemoveAllCountersThenDraw { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_REMOVE_ALL_COUNTERS_THEN_DRAW)
            }
            Effect::Counters(DoubleCountersOnTargetCreatures { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_DOUBLE_COUNTERS_ON_TARGET_CREATURES)
            }
            Effect::Counters(CommanderEntersWithBonusCounters { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_COMMANDER_ENTERS_WITH_BONUS_COUNTERS)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Counters(AttackerDrawsControllerCounters { counters, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_ATTACKER_DRAWS_CONTROLLER_COUNTERS)
                    .with_params(vec![int_param("counters", counters)])
            }
            Effect::Counters(LevelUp { level }) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_LEVEL_UP)
                    .with_params(vec![int_param("level", level)])
            }
            Effect::Counters(RemoveCounterFromSelf) => {
                MessageRef::new(MessageKey::EFFECT_COUNTERS_REMOVE_COUNTER_FROM_SELF)
            }
            Effect::Mana(ManaEffect::Add { .. }) => MessageRef::new(MessageKey::EFFECT_MANA_ADD),
            Effect::Mill(Mill { count, .. }) => MessageRef::new(MessageKey::EFFECT_MILL_MILL)
                .with_params(vec![amount_param("count", count)]),
            Effect::Mill(MillSelf { count }) => MessageRef::new(MessageKey::EFFECT_MILL_MILL_SELF)
                .with_params(vec![amount_param("count", count)]),
            Effect::Mill(ExileTopMayPlay {
                count,
                until_next_turn,
                face_down,
                free_while_source,
            }) => MessageRef::new(MessageKey::EFFECT_MILL_EXILE_TOP_MAY_PLAY).with_params(vec![
                amount_param("count", count),
                bool_param("until_next_turn", until_next_turn),
                bool_param("face_down", face_down),
                bool_param("free_while_source", free_while_source),
            ]),
            Effect::Mill(ExileFromGraveyardMayPlay { .. }) => {
                MessageRef::new(MessageKey::EFFECT_MILL_EXILE_FROM_GRAVEYARD_MAY_PLAY)
            }
            Effect::Mill(ExileDiscardedWithThis { .. }) => {
                MessageRef::new(MessageKey::EFFECT_MILL_EXILE_DISCARDED_WITH_THIS)
            }
            Effect::Mill(ExileTargetFromGraveyardWithThis) => {
                MessageRef::new(MessageKey::EFFECT_MILL_EXILE_TARGET_FROM_GRAVEYARD_WITH_THIS)
            }
            Effect::Mill(ExileTargetFromGraveyardCreateTokenCopy { filter }) => {
                MessageRef::new(MessageKey::EFFECT_MILL_EXILE_TARGET_FROM_GRAVEYARD_CREATE_TOKEN_COPY)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Pump(PumpUntilEndOfTurn {
                power,
                toughness,
                keywords,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_PUMP_UNTIL_END_OF_TURN).with_params(vec![
                amount_param("power", power),
                amount_param("toughness", toughness),
                debug_param("keywords", keywords),
            ]),
            Effect::Pump(PumpSelfUntilEndOfTurn {
                power,
                toughness,
                keywords,
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_PUMP_SELF_UNTIL_END_OF_TURN).with_params(vec![
                amount_param("power", power),
                amount_param("toughness", toughness),
                debug_param("keywords", keywords),
            ]),
            Effect::Pump(PumpCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                keywords,
                filter,
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_PUMP_CREATURES_YOU_CONTROL_UNTIL_END_OF_TURN)
                .with_params(vec![
                    amount_param("power", power),
                    amount_param("toughness", toughness),
                    debug_param("keywords", keywords),
                    debug_param("filter", filter),
                ]),
            Effect::Pump(GrantKeywordsToPermanentsYouControlUntilEndOfTurn { keywords, filter }) => {
                MessageRef::new(MessageKey::EFFECT_PUMP_GRANT_KEYWORDS_TO_PERMANENTS_YOU_CONTROL_UNTIL_END_OF_TURN)
                    .with_params(vec![debug_param("keywords", keywords), debug_param("filter", filter)])
            }
            Effect::Pump(SetBasePtCreaturesYouControlUntilEndOfTurn {
                power,
                toughness,
                other,
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_SET_BASE_PT_CREATURES_YOU_CONTROL_UNTIL_END_OF_TURN)
                .with_params(vec![
                    amount_param("power", power),
                    amount_param("toughness", toughness),
                    bool_param("other", other),
                ]),
            Effect::Pump(SetBasePtTargetUntilEndOfTurn {
                power, toughness, ..
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_SET_BASE_PT_TARGET_UNTIL_END_OF_TURN)
                .with_params(vec![amount_param("power", power), amount_param("toughness", toughness)]),
            Effect::Pump(AnimateSelfUntilEndOfTurn {
                base_power,
                base_toughness,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_ANIMATE_SELF_UNTIL_END_OF_TURN)
                .with_params(vec![int_param("base_power", base_power), int_param("base_toughness", base_toughness)]),
            Effect::Pump(SetOwnBasePtFromAmount { amount }) => {
                MessageRef::new(MessageKey::EFFECT_PUMP_SET_OWN_BASE_PT_FROM_AMOUNT)
                    .with_params(vec![amount_param("amount", amount)])
            }
            Effect::Pump(PumpOtherAttackersAttackingYourOpponents { power, toughness }) => {
                MessageRef::new(MessageKey::EFFECT_PUMP_PUMP_OTHER_ATTACKERS_ATTACKING_YOUR_OPPONENTS)
                    .with_params(vec![int_param("power", power), int_param("toughness", toughness)])
            }
            Effect::Pump(EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife {
                power,
                toughness,
                life,
            }) => MessageRef::new(
                MessageKey::EFFECT_PUMP_ENCHANTED_ATTACKER_PUMP_ATTACKING_OPPONENT_ELSE_CONTROLLER_LOSES_LIFE,
            )
            .with_params(vec![
                int_param("power", power),
                int_param("toughness", toughness),
                int_param("life", life),
            ]),
            Effect::Pump(StripKeywordsFromOpponentsCreatures { keywords }) => {
                MessageRef::new(MessageKey::EFFECT_PUMP_STRIP_KEYWORDS_FROM_OPPONENTS_CREATURES)
                    .with_params(vec![debug_param("keywords", keywords)])
            }
            Effect::Pump(WeakenEachCreature {
                power,
                toughness,
                opponents_only,
            }) => MessageRef::new(MessageKey::EFFECT_PUMP_WEAKEN_EACH_CREATURE).with_params(vec![
                amount_param("power", power),
                amount_param("toughness", toughness),
                bool_param("opponents_only", opponents_only),
            ]),
            Effect::Reveal(TopToHand { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_REVEAL_TOP_TO_HAND)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Reveal(TopAndDrainMutual) => {
                MessageRef::new(MessageKey::EFFECT_REVEAL_TOP_AND_DRAIN_MUTUAL)
            }
            Effect::Reveal(Until {
                filter,
                count,
                matched_dest,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_REVEAL_UNTIL).with_params(vec![
                debug_param("filter", filter),
                amount_param("count", count),
                debug_param("matched_dest", matched_dest),
            ]),
            Effect::Reveal(TopCards {
                filter,
                count,
                matched_dest,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_REVEAL_TOP_CARDS).with_params(vec![
                debug_param("filter", filter),
                amount_param("count", count),
                debug_param("matched_dest", matched_dest),
            ]),
            Effect::Token(Create { token, count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_TOKEN_CREATE)
                    .with_params(vec![amount_param("count", count), name_param("token", token.name)])
            }
            Effect::Token(CreateTreasure {
                count,
                target_player,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_TOKEN_CREATE_TREASURE)
                .with_params(vec![amount_param("count", count), bool_param("target_player", target_player)]),
            Effect::Token(CreateCopy {
                count,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                entering,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_TOKEN_CREATE_COPY).with_params(vec![
                amount_param("count", count),
                bool_param("sacrifice_at_next_end_step", sacrifice_at_next_end_step),
                bool_param("exile_at_next_end_step", exile_at_next_end_step),
                bool_param("entering", entering.is_some()),
            ]),
            Effect::Token(CopyEachEnteredThisTurnTokenTappedAttacking { .. }) => {
                MessageRef::new(MessageKey::EFFECT_TOKEN_COPY_EACH_ENTERED_THIS_TURN_TOKEN_TAPPED_ATTACKING)
            }
            Effect::Token(BecomeCopyOfTargetCreatureGainingMyriad { .. }) => {
                MessageRef::new(MessageKey::EFFECT_TOKEN_BECOME_COPY_OF_TARGET_CREATURE_GAINING_MYRIAD)
            }
            Effect::Token(MyriadTokenCopies { .. }) => {
                MessageRef::new(MessageKey::EFFECT_TOKEN_MYRIAD_TOKEN_COPIES)
            }
            Effect::Zone(Manifest) => MessageRef::new(MessageKey::EFFECT_ZONE_MANIFEST),
            Effect::Zone(AttachTriggeringAuraToMintedToken { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_ATTACH_TRIGGERING_AURA_TO_MINTED_TOKEN)
            }
            Effect::Zone(ReflexiveTrigger { then }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_REFLEXIVE_TRIGGER)
                    .with_children(then.iter().map(|effect| effect.message()).collect())
            }
            Effect::Zone(ReturnFromGraveyardAttachedToToken { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_FROM_GRAVEYARD_ATTACHED_TO_TOKEN)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Zone(AttachSelfToReanimated) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_ATTACH_SELF_TO_REANIMATED)
            }
            Effect::Zone(AttachSelfToMintedToken) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_ATTACH_SELF_TO_MINTED_TOKEN)
            }
            Effect::Zone(AttachMintedAuraToTarget { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_ATTACH_MINTED_AURA_TO_TARGET)
            }
            Effect::Zone(ScheduleReturnThisAuraAttachedToReanimated) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_SCHEDULE_RETURN_THIS_AURA_ATTACHED_TO_REANIMATED)
            }
            Effect::Zone(ReturnThisAuraAttachedTo { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_THIS_AURA_ATTACHED_TO)
            }
            Effect::Zone(ScheduleReturnReanimatedToHand) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_SCHEDULE_RETURN_REANIMATED_TO_HAND)
            }
            Effect::Zone(ReturnObjectToHand { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_OBJECT_TO_HAND)
            }
            Effect::Zone(ReturnThisAuraFromGraveyardAttachedToChosenHost) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_THIS_AURA_FROM_GRAVEYARD_ATTACHED_TO_CHOSEN_HOST)
            }
            Effect::Zone(ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_SCHEDULE_RETURN_THIS_AURA_FROM_GRAVEYARD_ATTACHED_TO_CHOSEN_HOST)
            }
            Effect::Zone(FlickerTarget { return_at, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_FLICKER_TARGET)
                    .with_params(vec![bool_param("delayed", return_at.is_some())])
            }
            Effect::Zone(ReturnFlickeredCard { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_FLICKERED_CARD)
            }
            Effect::Zone(ExileTargetGraveyardCardThenIfCreature { then }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_EXILE_TARGET_GRAVEYARD_CARD_THEN_IF_CREATURE)
                    .with_children(then.iter().map(|effect| effect.message()).collect())
            }
            Effect::Zone(ReturnToHand { .. }) => MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_TO_HAND),
            Effect::Zone(ReturnThisToHand) => MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_THIS_TO_HAND),
            Effect::Zone(ReturnThisFromGraveyardToBattlefield { tapped }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_THIS_FROM_GRAVEYARD_TO_BATTLEFIELD)
                    .with_params(vec![bool_param("tapped", tapped)])
            }
            Effect::Zone(ReturnAllToHand { filter }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_ALL_TO_HAND)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Zone(ReturnFromGraveyardToHand { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_FROM_GRAVEYARD_TO_HAND)
            }
            Effect::Zone(ReanimateToBattlefield { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_REANIMATE_TO_BATTLEFIELD)
            }
            Effect::Zone(TuckFromGraveyard { to_top, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_TUCK_FROM_GRAVEYARD)
                    .with_params(vec![bool_param("to_top", to_top)])
            }
            Effect::Zone(MassReturnFromGraveyard {
                filter,
                all_players,
            }) => MessageRef::new(MessageKey::EFFECT_ZONE_MASS_RETURN_FROM_GRAVEYARD).with_params(vec![
                debug_param("filter", filter),
                bool_param("all_players", all_players),
            ]),
            Effect::Zone(ShuffleTargetPermanentIntoLibraryThenReveal { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_SHUFFLE_TARGET_PERMANENT_INTO_LIBRARY_THEN_REVEAL)
            }
            Effect::Zone(ShuffleTargetPermanentIntoLibrary { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_SHUFFLE_TARGET_PERMANENT_INTO_LIBRARY)
            }
            Effect::Zone(TuckPermanentIntoLibrary { to_top, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_TUCK_PERMANENT_INTO_LIBRARY)
                    .with_params(vec![bool_param("to_top", to_top)])
            }
            Effect::Zone(TuckSelfAndBlockedCreatures) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_TUCK_SELF_AND_BLOCKED_CREATURES)
            }
            Effect::Zone(ReanimateDyingEnchantedCreature { under_owner, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_REANIMATE_DYING_ENCHANTED_CREATURE)
                    .with_params(vec![bool_param("under_owner", under_owner)])
            }
            Effect::Zone(ExileDeadCreatureCreateCopyWithSubtype { add_subtypes, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_EXILE_DEAD_CREATURE_CREATE_COPY_WITH_SUBTYPE)
                    .with_params(vec![debug_param("add_subtypes", add_subtypes)])
            }
            Effect::Zone(ReturnExiledCardToOwnersGraveyard { .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_RETURN_EXILED_CARD_TO_OWNERS_GRAVEYARD)
            }
            Effect::Zone(ExileGraveyardObjectGainLife { amount, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_EXILE_GRAVEYARD_OBJECT_GAIN_LIFE)
                    .with_params(vec![int_param("amount", amount)])
            }
            Effect::Zone(ExileSelfWithTimeCounters { counters, .. }) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_EXILE_SELF_WITH_TIME_COUNTERS)
                    .with_params(vec![int_param("counters", counters)])
            }
            Effect::Zone(TuckSelfToLibraryBottom) => {
                MessageRef::new(MessageKey::EFFECT_ZONE_TUCK_SELF_TO_LIBRARY_BOTTOM)
            }
            Effect::Zone(ExileSelfOnResolve) => MessageRef::new(MessageKey::EFFECT_ZONE_EXILE_SELF_ON_RESOLVE),
            Effect::Zone(UntapSearchedLand) => MessageRef::new(MessageKey::EFFECT_ZONE_UNTAP_SEARCHED_LAND),
            Effect::Copy(TargetSpell) => MessageRef::new(MessageKey::EFFECT_COPY_TARGET_SPELL),
            Effect::Copy(ThisSpell { .. }) => MessageRef::new(MessageKey::EFFECT_COPY_THIS_SPELL),
            Effect::Copy(RetargetSpellCopy { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_RETARGET_SPELL_COPY)
            }
            Effect::Copy(MayPayToCopyThis { cost, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_MAY_PAY_TO_COPY_THIS)
                    .with_params(vec![mana_param("cost", cost)])
            }
            Effect::Copy(ChangeTargetOfTargetSpellOrAbility { optional, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_CHANGE_TARGET_OF_TARGET_SPELL_OR_ABILITY)
                    .with_params(vec![bool_param("optional", optional)])
            }
            Effect::Copy(CopyTriggeringSpell { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_COPY_TRIGGERING_SPELL)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Copy(CopyTriggeringSpellForEachOtherCreatureYouControl { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_COPY_TRIGGERING_SPELL_FOR_EACH_OTHER_CREATURE_YOU_CONTROL)
            }
            Effect::Copy(CopyTriggeringAbility { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_COPY_TRIGGERING_ABILITY)
            }
            Effect::Copy(Demonstrate { .. }) => MessageRef::new(MessageKey::EFFECT_COPY_DEMONSTRATE),
            Effect::Copy(MintFreeCopyOfExiledCard { .. }) => {
                MessageRef::new(MessageKey::EFFECT_COPY_MINT_FREE_COPY_OF_EXILED_CARD)
            }
            Effect::Dig(RevealUntilMayDeploy { filter }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_REVEAL_UNTIL_MAY_DEPLOY)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Dig(RevealUntilExileCastFree { filter }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_REVEAL_UNTIL_EXILE_CAST_FREE)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Dig(ShuffleLibrary) => MessageRef::new(MessageKey::EFFECT_DIG_SHUFFLE_LIBRARY),
            Effect::Dig(Clash) => MessageRef::new(MessageKey::EFFECT_DIG_CLASH),
            Effect::Dig(ExileTopUntilStopCastFreeUnderBudget { budget }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EXILE_TOP_UNTIL_STOP_CAST_FREE_UNDER_BUDGET)
                    .with_params(vec![int_param("budget", budget)])
            }
            Effect::Dig(ExileTopCastMatchingFree { count, filter }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EXILE_TOP_CAST_MATCHING_FREE)
                    .with_params(vec![int_param("count", count), debug_param("filter", filter)])
            }
            Effect::Dig(Cascade { mana_value }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_CASCADE)
                    .with_params(vec![int_param("mana_value", mana_value)])
            }
            Effect::Dig(OpponentSplitsExilePiles) => {
                MessageRef::new(MessageKey::EFFECT_DIG_OPPONENT_SPLITS_EXILE_PILES)
            }
            Effect::Dig(RevealTopSplitPiles) => MessageRef::new(MessageKey::EFFECT_DIG_REVEAL_TOP_SPLIT_PILES),
            Effect::Dig(RevealTopOpponentPicksOneToGraveyard { count }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_REVEAL_TOP_OPPONENT_PICKS_ONE_TO_GRAVEYARD)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Dig(EachPlayerExilesUntilNonlandOpponentPicks) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EACH_PLAYER_EXILES_UNTIL_NONLAND_OPPONENT_PICKS)
            }
            Effect::Dig(ExileRandomFromGraveyardMayPlay) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EXILE_RANDOM_FROM_GRAVEYARD_MAY_PLAY)
            }
            Effect::Dig(ExileTargetGraveyardSpellCastFree { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EXILE_TARGET_GRAVEYARD_SPELL_CAST_FREE)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Dig(ExileTargetGraveyardCardRecordManaValue { filter }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_EXILE_TARGET_GRAVEYARD_CARD_RECORD_MANA_VALUE)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Dig(CashOutExiledWithThis) => {
                MessageRef::new(MessageKey::EFFECT_DIG_CASH_OUT_EXILED_WITH_THIS)
            }
            Effect::Dig(CastExiledWithThisFree) => {
                MessageRef::new(MessageKey::EFFECT_DIG_CAST_EXILED_WITH_THIS_FREE)
            }
            Effect::Dig(Scry { count }) => MessageRef::new(MessageKey::EFFECT_SCRY)
                .with_params(vec![amount_param("count", count)]),
            Effect::Dig(Surveil { count }) => MessageRef::new(MessageKey::EFFECT_DIG_SURVEIL)
                .with_params(vec![int_param("count", count)]),
            Effect::Dig(LookAtTop {
                count,
                up_to,
                dest,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_DIG_LOOK_AT_TOP).with_params(vec![
                int_param("count", count),
                int_param("up_to", up_to),
                debug_param("dest", dest),
            ]),
            Effect::Dig(DistributeTop {
                count,
                to_hand,
                to_bottom,
                to_exile_may_play,
            }) => MessageRef::new(MessageKey::EFFECT_DIG_DISTRIBUTE_TOP).with_params(vec![
                int_param("count", count),
                int_param("to_hand", to_hand),
                int_param("to_bottom", to_bottom),
                int_param("to_exile_may_play", to_exile_may_play),
            ]),
            Effect::Dig(SearchLibrary {
                filter, to_zone, ..
            }) => MessageRef::new(MessageKey::EFFECT_DIG_SEARCH_LIBRARY)
                .with_params(vec![debug_param("filter", filter), debug_param("to_zone", to_zone)]),
            Effect::Dig(ShuffleTargetCardsFromGraveyardIntoLibrary { max, target_player }) => {
                MessageRef::new(MessageKey::EFFECT_DIG_SHUFFLE_TARGET_CARDS_FROM_GRAVEYARD_INTO_LIBRARY)
                    .with_params(vec![int_param("max", max), bool_param("target_player", target_player)])
            }
            Effect::Choice(Discard {
                count,
                target_player,
                or_one_matching,
            }) => MessageRef::new(MessageKey::EFFECT_CHOICE_DISCARD).with_params(vec![
                int_param("count", count),
                bool_param("target_player", target_player),
                bool_param("or_one_matching", or_one_matching.is_some()),
            ]),
            Effect::Choice(Proliferate { times }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_PROLIFERATE)
                    .with_params(vec![amount_param("times", times)])
            }
            Effect::Choice(TargetPlayerMayDraw { count, opponent }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_TARGET_PLAYER_MAY_DRAW)
                    .with_params(vec![amount_param("count", count), bool_param("opponent", opponent)])
            }
            Effect::Choice(MayDrawUpTo { count }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_DRAW_UP_TO)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Choice(MayDrawUpToThenOpponentMayRepeat { count }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_DRAW_UP_TO_THEN_OPPONENT_MAY_REPEAT)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Choice(PutFromHandOnTop { count }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_PUT_FROM_HAND_ON_TOP)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Choice(PutLandFromHand { tapped }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_PUT_LAND_FROM_HAND)
                    .with_params(vec![bool_param("tapped", tapped)])
            }
            Effect::Choice(PutCreatureFromHand) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_PUT_CREATURE_FROM_HAND)
            }
            Effect::Choice(CastCreatureFaceDown) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_CAST_CREATURE_FACE_DOWN)
            }
            Effect::Choice(SacrificeSelfUnlessPay { cost }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_SACRIFICE_SELF_UNLESS_PAY)
                    .with_params(vec![mana_param("cost", cost)])
            }
            Effect::Choice(SacrificeSelfUnlessReturnLand { filter }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_SACRIFICE_SELF_UNLESS_RETURN_LAND)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Choice(PhaseOut) => MessageRef::new(MessageKey::EFFECT_CHOICE_PHASE_OUT),
            Effect::Choice(DamagingCreatureControllerMayDraw { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_DAMAGING_CREATURE_CONTROLLER_MAY_DRAW)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Choice(EachPlayerSacrifices {
                scope, keep_one, ..
            }) => MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_SACRIFICES)
                .with_params(vec![debug_param("scope", scope), bool_param("keep_one", keep_one)]),
            Effect::Choice(EachPlayerExilesFromGraveyard) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_EXILES_FROM_GRAVEYARD)
            }
            Effect::Choice(TargetPlayerExilesFromGraveyard { target }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_TARGET_PLAYER_EXILES_FROM_GRAVEYARD)
                    .with_params(vec![debug_param("target", target)])
            }
            Effect::Choice(CasterKeepsOneOfEachTypePerPlayer) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_CASTER_KEEPS_ONE_OF_EACH_TYPE_PER_PLAYER)
            }
            Effect::Choice(EachPlayerControllerChoosesCounterTarget) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_CONTROLLER_CHOOSES_COUNTER_TARGET)
            }
            Effect::Choice(JoinForcesPayMana) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_JOIN_FORCES_PAY_MANA)
            }
            Effect::Choice(CouncilsDilemmaVote { options }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_COUNCILS_DILEMMA_VOTE)
                    .with_params(vec![debug_param("options", options)])
            }
            Effect::Choice(EachPlayerNamesCardThenRevealsTop) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_NAMES_CARD_THEN_REVEALS_TOP)
            }
            Effect::Choice(EachPlayerCreatesFractalFromExiledPower { token }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_CREATES_FRACTAL_FROM_EXILED_POWER)
                    .with_params(vec![name_param("token", token.name)])
            }
            Effect::Choice(EachPlayerDiscardsHandThenDraws { count }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_PLAYER_DISCARDS_HAND_THEN_DRAWS)
                    .with_params(vec![amount_param("count", count)])
            }
            Effect::Choice(EachOtherTokenBecomesCopyOfChosen) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_EACH_OTHER_TOKEN_BECOMES_COPY_OF_CHOSEN)
            }
            Effect::Choice(PutCounterThenMayBecomeCopyOfCardFromList { .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_PUT_COUNTER_THEN_MAY_BECOME_COPY_OF_CARD_FROM_LIST)
            }
            Effect::Choice(MaySacrifice { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_SACRIFICE)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Choice(MayReturnFromGraveyard { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_RETURN_FROM_GRAVEYARD)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Choice(MayDiscard { .. }) => MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_DISCARD),
            Effect::Choice(MayDrawUnlessPays { cost, .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_MAY_DRAW_UNLESS_PAYS)
                    .with_params(vec![amount_param("cost", cost)])
            }
            Effect::Choice(ChooseCreatureType) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_CHOOSE_CREATURE_TYPE)
            }
            Effect::Choice(ChooseColor) => MessageRef::new(MessageKey::EFFECT_CHOICE_CHOOSE_COLOR),
            Effect::Choice(SetOwnColorUntilEndOfTurn) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_SET_OWN_COLOR_UNTIL_END_OF_TURN)
            }
            Effect::Choice(SacrificeOwn { filter, count }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_SACRIFICE_OWN)
                    .with_params(vec![debug_param("filter", filter), int_param("count", count)])
            }
            Effect::Choice(DefendingPlayerSacrifices { count, .. }) => {
                MessageRef::new(MessageKey::EFFECT_CHOICE_DEFENDING_PLAYER_SACRIFICES)
                    .with_params(vec![int_param("count", count)])
            }
            Effect::Static(GrantManaAbility { filter, .. }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_GRANT_MANA_ABILITY)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Static(KeywordAnthem { keywords, filter }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_KEYWORD_ANTHEM)
                    .with_params(vec![debug_param("keywords", keywords), debug_param("filter", filter)])
            }
            Effect::Static(Anthem {
                power,
                toughness,
                self_only,
                exclude_source,
                tokens_only,
                keywords,
                subtypes,
                colors,
                chosen_subtype,
                attacking_only,
                blocking_only,
                commander_only,
                has_counters,
                all_players,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_STATIC_ANTHEM).with_params(vec![
                amount_param("power", power),
                amount_param("toughness", toughness),
                bool_param("self_only", self_only),
                bool_param("exclude_source", exclude_source),
                bool_param("tokens_only", tokens_only),
                debug_param("keywords", keywords),
                debug_param("subtypes", subtypes),
                debug_param("colors", colors),
                bool_param("chosen_subtype", chosen_subtype),
                bool_param("attacking_only", attacking_only),
                bool_param("blocking_only", blocking_only),
                bool_param("commander_only", commander_only),
                bool_param("has_counters", has_counters),
                bool_param("all_players", all_players),
            ]),
            Effect::Static(TappedForManaBonus { scope, bonus_color }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_TAPPED_FOR_MANA_BONUS)
                    .with_params(vec![debug_param("scope", scope), debug_param("bonus_color", bonus_color)])
            }
            Effect::Static(TriggerDoubling { .. }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_TRIGGER_DOUBLING)
            }
            Effect::Static(PreventNoncombatDamageToOtherCreaturesYouControl) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_PREVENT_NONCOMBAT_DAMAGE_TO_OTHER_CREATURES_YOU_CONTROL)
            }
            Effect::Static(PreventDamageToSelfRemovingCounter) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_PREVENT_DAMAGE_TO_SELF_REMOVING_COUNTER)
            }
            Effect::Static(NoMaximumHandSize) => MessageRef::new(MessageKey::EFFECT_STATIC_NO_MAXIMUM_HAND_SIZE),
            Effect::Static(PlayFromGraveyardOncePerTurn) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_PLAY_FROM_GRAVEYARD_ONCE_PER_TURN)
            }
            Effect::Static(ReduceSpellCost {
                amount,
                filter,
                first_x_spell_each_turn,
            }) => MessageRef::new(MessageKey::EFFECT_STATIC_REDUCE_SPELL_COST).with_params(vec![
                amount_param("amount", amount),
                debug_param("filter", filter),
                bool_param("first_x_spell_each_turn", first_x_spell_each_turn),
            ]),
            Effect::Static(AttackTax { amount }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_ATTACK_TAX)
                    .with_params(vec![int_param("amount", amount)])
            }
            Effect::Static(CounterScaledAttackTax) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_COUNTER_SCALED_ATTACK_TAX)
            }
            Effect::Static(CantBeAttackedBy { filter }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_CANT_BE_ATTACKED_BY)
                    .with_params(vec![debug_param("filter", filter)])
            }
            Effect::Static(PreventCombatDamage { to_self, by_self }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_PREVENT_COMBAT_DAMAGE)
                    .with_params(vec![bool_param("to_self", to_self), bool_param("by_self", by_self)])
            }
            Effect::Static(CounterReplacement { add, times, .. }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_COUNTER_REPLACEMENT)
                    .with_params(vec![int_param("add", add), int_param("times", times)])
            }
            Effect::Static(TokenReplacement { times }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_TOKEN_REPLACEMENT)
                    .with_params(vec![int_param("times", times)])
            }
            Effect::Static(LifeGainReplacement { plus }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_LIFE_GAIN_REPLACEMENT)
                    .with_params(vec![int_param("plus", plus)])
            }
            Effect::Static(CastXReplacement { times }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_CAST_X_REPLACEMENT)
                    .with_params(vec![int_param("times", times)])
            }
            Effect::Static(EntersWithCounters { amount, kind }) => {
                let mut params = vec![amount_param("amount", amount)];
                if let Some(kind) = kind {
                    params.push(debug_param("kind", kind));
                }
                MessageRef::new(MessageKey::EFFECT_STATIC_ENTERS_WITH_COUNTERS).with_params(params)
            }
            Effect::Static(CreaturesYouControlEnterWithCounters { filter, count }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_CREATURES_YOU_CONTROL_ENTER_WITH_COUNTERS)
                    .with_params(vec![debug_param("filter", filter), amount_param("count", count)])
            }
            Effect::Static(GrantToAttached {
                power, toughness, ..
            }) => MessageRef::new(MessageKey::EFFECT_STATIC_GRANT_TO_ATTACHED)
                .with_params(vec![amount_param("power", power), amount_param("toughness", toughness)]),
            Effect::Static(SetAttachedBasePt { power, toughness }) => {
                MessageRef::new(MessageKey::EFFECT_STATIC_SET_ATTACHED_BASE_PT)
                    .with_params(vec![int_param("power", power), int_param("toughness", toughness)])
            }
            Effect::Static(SetAttachedTypes {
                set_subtypes,
                add_subtypes,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_STATIC_SET_ATTACHED_TYPES).with_params(vec![
                debug_param("set_subtypes", set_subtypes),
                debug_param("add_subtypes", add_subtypes),
            ]),
            Effect::Static(ControlAttached) => MessageRef::new(MessageKey::EFFECT_STATIC_CONTROL_ATTACHED),
            Effect::Misc(ScheduleColorlessManaForCounteredSpellNextMainPhase) => {
                MessageRef::new(MessageKey::EFFECT_MISC_SCHEDULE_COLORLESS_MANA_FOR_COUNTERED_SPELL_NEXT_MAIN_PHASE)
            }
            Effect::Misc(SkipNextUntapOpponentCreatures) => {
                MessageRef::new(MessageKey::EFFECT_MISC_SKIP_NEXT_UNTAP_OPPONENT_CREATURES)
            }
            Effect::Misc(MustAttackRandomOpponent) => {
                MessageRef::new(MessageKey::EFFECT_MISC_MUST_ATTACK_RANDOM_OPPONENT)
            }
            Effect::Misc(PreventCombatDamageToYouCreatingTokens { .. }) => {
                MessageRef::new(MessageKey::EFFECT_MISC_PREVENT_COMBAT_DAMAGE_TO_YOU_CREATING_TOKENS)
            }
            Effect::Misc(PreventAllCombatDamageThisTurn) => {
                MessageRef::new(MessageKey::EFFECT_MISC_PREVENT_ALL_COMBAT_DAMAGE_THIS_TURN)
            }
            Effect::Misc(Fight {
                ally_is_shared_target,
                ..
            }) => MessageRef::new(MessageKey::EFFECT_MISC_FIGHT)
                .with_params(vec![bool_param("ally_is_shared_target", ally_is_shared_target)]),
            Effect::Misc(CounterTargetSpell {
                unless_pays,
                filter,
                countered_dest,
            }) => MessageRef::new(MessageKey::EFFECT_MISC_COUNTER_TARGET_SPELL).with_params(vec![
                debug_param("filter", filter),
                debug_param("countered_dest", countered_dest),
                bool_param("unless_pays", unless_pays.is_some()),
                unless_pays
                    .map(|amount| amount_param("amount", amount))
                    .unwrap_or_else(|| str_param("amount", "none")),
            ]),
            Effect::Misc(CounterTargetActivatedAbility) => {
                MessageRef::new(MessageKey::EFFECT_MISC_COUNTER_TARGET_ACTIVATED_ABILITY)
            }
            Effect::Misc(ScheduleAtNextUpkeep { then, fire_at, .. }) => {
                MessageRef::new(MessageKey::EFFECT_MISC_SCHEDULE_AT_NEXT_UPKEEP)
                    .with_params(vec![debug_param("fire_at", fire_at)])
                    .with_children(vec![then.message()])
            }
            Effect::Misc(ScheduleNextCastTrigger { filter, then }) => {
                MessageRef::new(MessageKey::EFFECT_MISC_SCHEDULE_NEXT_CAST_TRIGGER)
                    .with_params(vec![debug_param("filter", filter)])
                    .with_children(then.iter().map(|effect| effect.message()).collect())
            }
            Effect::Misc(ScheduleThisTurnCombatDamageCopy) => {
                MessageRef::new(MessageKey::EFFECT_MISC_SCHEDULE_THIS_TURN_COMBAT_DAMAGE_COPY)
            }
            Effect::Misc(BecomePrepared) => MessageRef::new(MessageKey::EFFECT_MISC_BECOME_PREPARED),
            Effect::Misc(FlipSource) => MessageRef::new(MessageKey::EFFECT_MISC_FLIP_SOURCE),
            Effect::Misc(ArmCombatDamageWatch) => {
                MessageRef::new(MessageKey::EFFECT_MISC_ARM_COMBAT_DAMAGE_WATCH)
            }
            Effect::Misc(GrantFlashThisTurn) => {
                MessageRef::new(MessageKey::EFFECT_MISC_GRANT_FLASH_THIS_TURN)
            }
            Effect::Misc(GrantChannelColorlessManaThisTurn) => {
                MessageRef::new(MessageKey::EFFECT_MISC_GRANT_CHANNEL_COLORLESS_MANA_THIS_TURN)
            }
            Effect::Sequence { steps } => MessageRef::new(MessageKey::EFFECT_SEQUENCE)
                .with_children(steps.iter().map(|effect| effect.message()).collect()),
            Effect::ChooseOne { options } => MessageRef::new(MessageKey::EFFECT_CHOOSE_ONE)
                .with_children(options.iter().map(|effect| effect.message()).collect()),
            Effect::Conditional { then, .. } => MessageRef::new(MessageKey::EFFECT_CONDITIONAL)
                .with_children(then.iter().map(|effect| effect.message()).collect()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn message_refs_are_stable() {
        let draw = Effect::Draw(DrawEffect::Cards {
            count: Amount::Fixed(2),
        })
        .message();
        assert_eq!(draw.key.as_str(), "effect.draw_cards");
        assert_eq!(draw.params[0].name, "count");
        assert!(matches!(draw.params[0].value, MessageParamValue::Int(2)));

        let life = Effect::Life(LifeEffect::Gain {
            amount: Amount::Fixed(1),
        })
        .message();
        assert_eq!(life.key.as_str(), "effect.life_gain");
        assert!(matches!(life.params[0].value, MessageParamValue::Int(1)));

        let scry = Effect::Dig(DigEffect::Scry {
            count: Amount::Fixed(3),
        })
        .message();
        assert_eq!(scry.key.as_str(), "effect.scry");

        let seq = Effect::Sequence {
            steps: &[
                Effect::Draw(DrawEffect::Cards {
                    count: Amount::Fixed(2),
                }),
                Effect::Choice(ChoiceEffect::Discard {
                    count: 2,
                    target_player: false,
                    or_one_matching: None,
                }),
            ],
        }
        .message();
        assert_eq!(seq.key.as_str(), "effect.sequence");
        assert_eq!(seq.children.len(), 2);
        assert_eq!(seq.children[0].key.as_str(), "effect.draw_cards");
    }

    #[test]
    fn reject_messages_use_reject_namespace() {
        assert_eq!(
            reject_message(Reject::IllegalTarget).key.as_str(),
            "reject.illegal_target"
        );
    }
}
