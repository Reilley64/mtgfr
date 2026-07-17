// Browser-facing wire shapes (snake_case, schema-compatible). Proto adaptation lives in `protoMap`.

import * as Schema from "effect/Schema";

// ── Auth ──

export const Me = Schema.Struct({
  id: Schema.Number,
  email: Schema.String,
  username: Schema.String,
});
export type Me = typeof Me.Type;

export const Credentials = Schema.Struct({
  email: Schema.String,
  password: Schema.String,
});
export type Credentials = typeof Credentials.Type;

export const SignupCredentials = Schema.Struct({
  email: Schema.String,
  password: Schema.String,
  username: Schema.String,
});
export type SignupCredentials = typeof SignupCredentials.Type;

export const Ack = Schema.Struct({
  accepted: Schema.Boolean,
  reason: Schema.optional(Schema.NullOr(Schema.String)),
});
export type Ack = typeof Ack.Type;

export const decodeMe = Schema.decodeUnknownSync(Me);
export const decodeAck = Schema.decodeUnknownSync(Ack);

// ── Decks / cards / lobby ──

export type ChoiceItem = { id: number; label: string; print?: string; player?: never };
export type CommanderDamageView = { amount: number; from: number };
export type DeckCardEntry = { count: number; id: string; print: string };
export type DeckError = { problems: Array<string> };
export type DeckSummary = { commander: string; commander_print?: string; id: number; name: string };
export type ModifierSourceView = { contributions: Array<string>; source_card_id?: string; source_name: string };
export type SeatView = {
  claimed: boolean;
  deck_name?: string | null;
  is_host: boolean;
  is_you: boolean;
  player: number;
  ready: boolean;
  username?: string | null;
};
export type SeedResponse = { pod_dns: string; table_id: string; version: string };
export type SeedSeat = { deck_id: number; user_id: number; username: string };
export type StackDwellRequest = { dwelling: boolean };
export type WireCost = { colored: Array<number>; generic: number; has_x?: boolean };
export type WireEitherMana = { a: number; amount: number; b: number };
export type WireKind =
  | { kind: "creature"; power: number; toughness: number }
  | { kind: "instant" }
  | { kind: "sorcery" }
  | { kind: "enchantment" }
  | { kind: "artifact" }
  | { kind: "planeswalker"; loyalty: number }
  | { colors: Array<number>; kind: "land" };
export type WireOfColorsMana = { amount: number; mask: number };
export type YieldRequest = { enabled: boolean };
export type U32 = number;
export type DeckDetail = {
  cards: Array<DeckCardEntry>;
  commander: string;
  commander_print: string;
  id: number;
  name: string;
};
export type SaveDeckRequest = { cards: Array<DeckCardEntry>; commander: string; commander_print: string; name: string };
export type SeedRequest = { host_user_id: number; seats: Array<SeedSeat>; table_id: string };
export type CatalogCard = {
  approximates?: string | null;
  back?: null | { approximates?: string | null; name: string; oracle?: string | null };
  color_identity: Array<number>;
  cost: WireCost;
  default_print: string;
  id: string;
  keywords: Array<string>;
  kind: WireKind;
  legendary: boolean;
  name: string;
  oracle?: string | null;
  otags: Array<string>;
  set: string;
  subtypes: Array<string>;
  summary: string;
};
export type PlayerView = {
  commander_damage?: Array<CommanderDamageView>;
  commander_tax: number;
  hand_count: number;
  library_count: number;
  life: number;
  lost: boolean;
  mana_pool: {
    any: number;
    colored: Array<number>;
    colorless: number;
    either?: Array<WireEitherMana>;
    of_colors?: Array<WireOfColorsMana>;
  };
  player: number;
  username?: string;
};
export type ObjectView = {
  attached_to?: null | number;
  card_id?: string;
  controller: number;
  face_down?: boolean;
  goaded?: boolean;
  has_haste: boolean;
  id: U32;
  is_commander: boolean;
  keywords?: Array<string>;
  kind: WireKind;
  loyalty?: number;
  mana_cost: WireCost;
  marked_damage: number;
  modifiers?: Array<ModifierSourceView>;
  name: string;
  needs_target: boolean;
  owner: number;
  phased_out?: boolean;
  plus_counters: number;
  power: number;
  prepared?: boolean;
  print?: string;
  summoning_sick: boolean;
  tapped: boolean;
  taps_for_mana?: boolean;
  toughness: number;
  zone: number;
};
export type StackObjectView = {
  controller: number;
  kind: string;
  label: string;
  source: number;
  target?: null | { id: U32; kind: "object" } | { kind: "player"; player: number };
};
export type WireAttack = { attacker: U32; defender: number };
export type WireBlock = { attacker: U32; blocker: U32 };
export type WireDamage = { amount: number; blocker: U32 };
export type WireTarget = { id: U32; kind: "object" } | { kind: "player"; player: number };
export type ModeView = { label: string; needs_target: boolean; targets: Array<WireTarget> };
export type VisibleEvent =
  | {
      controller: number;
      escape: boolean;
      flashback: boolean;
      from: U32;
      kind: "spell_cast";
      spell: U32;
      target?: null | WireTarget;
    }
  | { kind: "spell_targets_chosen"; spell: U32; targets: Array<WireTarget> }
  | { kind: "prepared_changed"; object: U32; prepared: boolean }
  | { kind: "leveled_up"; level: number; object: U32 }
  | { kind: "phased_out"; object: U32 }
  | { kind: "phased_in"; object: U32 }
  | { kind: "creature_type_chosen"; object: U32; subtype: string }
  | { color: number; kind: "color_chosen"; object: U32 }
  | { controller: number; kind: "prepared_spell_cast"; source: U32; spell: U32; target?: null | WireTarget; x: number }
  | { controller: number; kind: "adventure_spell_cast"; source: U32; spell: U32; target?: null | WireTarget; x: number }
  | { active_player: number; kind: "step_began"; step: number }
  | { controller: number; kind: "triggered_ability_on_stack"; source: U32; target?: null | WireTarget }
  | { kind: "ability_resolved"; source: U32 }
  | { kind: "ability_countered"; source: U32 }
  | { from: U32; kind: "land_played"; permanent: U32; player: number }
  | { kind: "tapped"; object: U32 }
  | { kind: "untapped"; object: U32 }
  | { kind: "regeneration_shield_created"; object: U32 }
  | { kind: "regenerated"; object: U32 }
  | { kind: "regeneration_shields_expired"; object: U32 }
  | { kind: "lost_summoning_sickness"; object: U32 }
  | { count: number; kind: "counters_placed"; object: U32 }
  | { count: number; counter_kind: number; kind: "kind_counters_placed"; object: U32 }
  | { amount: number; kind: "loyalty_changed"; object: U32 }
  | { active: boolean; kind: "loyalty_activated"; object: U32 }
  | { ability_index: number; kind: "ability_activated_this_turn"; object: U32 }
  | { kind: "triggered_ability_this_turn"; source: U32 }
  | { host?: null | U32; kind: "attached_to"; object: U32 }
  | { kind: "temp_boost"; object: U32; power: number; toughness: number }
  | { kind: "temp_boosts_ended"; object: U32 }
  | { kind: "base_pt_set_until_end_of_turn"; object: U32; power: number; toughness: number }
  | { kind: "types_added_until_end_of_turn"; object: U32 }
  | { kind: "reanimated_creature_became"; object: U32 }
  | { kind: "added_subtypes"; object: U32 }
  | { kind: "became_copy"; object: U32 }
  | { kind: "keywords_stripped"; object: U32 }
  | { controller: number; kind: "control_gained_until_end_of_turn"; object: U32 }
  | { kind: "control_ended_until_end_of_turn"; object: U32 }
  | { kind: "abilities_granted"; source: U32; target: U32 }
  | { kind: "granted_abilities_ended" }
  | { controller: number; kind: "control_gained"; object: U32 }
  | { controller: number; kind: "conditioned_control_gained"; object: U32 }
  | { kind: "conditioned_control_ended"; object: U32 }
  | { defender: number; kind: "attacker_declared"; object: U32 }
  | { defender: number; kind: "token_entered_attacking"; token: U32 }
  | { by: number; kind: "goaded"; object: U32 }
  | { by: number; kind: "goad_cleared" }
  | { kind: "vow_counters_placed"; object: U32; protected: number }
  | { card: U32; count: number; kind: "time_counters_placed" }
  | { card: U32; kind: "time_counters_removed" }
  | { defender: number; kind: "must_attack_declared"; object: U32 }
  | { controller: number; kind: "delayed_trigger_scheduled"; source: U32 }
  | { kind: "delayed_triggers_fired" }
  | { controller: number; kind: "next_cast_trigger_armed"; source: U32 }
  | { controller: number; kind: "next_cast_trigger_consumed"; source: U32 }
  | { controller: number; kind: "combat_damage_watch_armed"; source: U32; watched: U32 }
  | { controller: number; kind: "combat_damage_watch_consumed"; source: U32 }
  | { card: U32; controller: number; kind: "combat_damage_copy_armed"; source: U32 }
  | { card: U32; from: U32; kind: "exiled_from_library_may_play"; player: number; until_next_turn: boolean }
  | { card: U32; from: U32; kind: "exiled_from_library_to_choose_cast_free"; player: number }
  | { card: U32; kind: "play_from_exile_permission_armed" }
  | { kind: "play_from_exile_ended" }
  | { attacker: U32; blocker: U32; kind: "blocker_declared" }
  | { assignment: Array<[number?, number?, ...Array<never>]>; attacker: U32; kind: "combat_damage_divided" }
  | {
      assignment: Array<[number?, number?, ...Array<never>]>;
      kind: "spell_damage_divided";
      players: Array<[number?, number?, ...Array<never>]>;
      spell: U32;
    }
  | { assignment: Array<[number?, number?, ...Array<never>]>; kind: "spell_counters_divided"; spell: U32 }
  | { kind: "deathtouch_marked"; object: U32 }
  | { kind: "combat_cleared" }
  | { kind: "commander_cast_from_command_zone"; player: number }
  | { kind: "flash_permission_granted"; player: number }
  | { kind: "channel_colorless_mana_granted"; player: number }
  | { amount: number; kind: "commander_damage_dealt"; player: number; source: U32 }
  | { amount: number; kind: "combat_damage_dealt_to_player"; player: number; source: U32 }
  | { amount: number; kind: "damage_dealt_to_player"; player: number; source: U32 }
  | { amount: number; kind: "combat_damage_prevented"; player: number }
  | { card: U32; from: U32; kind: "moved_to_command_zone" }
  | { kind: "mana_emptied"; player: number }
  | { kind: "damage_cleared"; object: U32 }
  | { amount: number; kind: "mana_added"; mana: number; player: number }
  | { kind: "mana_spent"; mana: Array<number>; player: number }
  | { kind: "priority_passed"; player: number }
  | { from: U32; kind: "permanent_entered"; permanent: U32 }
  | {
      controller: number;
      finality: boolean;
      from: U32;
      kind: "reanimated_to_battlefield";
      permanent: U32;
      tapped: boolean;
    }
  | { controller: number; creator?: null | number; kind: "token_created"; token: U32 }
  | { kind: "token_ceased_to_exist"; token: U32 }
  | { controller: number; copy: U32; kind: "spell_copied"; original: U32 }
  | { kind: "spell_ceased_to_exist"; spell: U32 }
  | { amount: number; kind: "damage_marked"; object: U32; source?: null | number }
  | { card: U32; from: U32; kind: "moved_to_graveyard" }
  | { card: U32; from: U32; kind: "moved_to_exile" }
  | { card: U32; from: U32; kind: "exiled_on_adventure"; owner: number }
  | { kind: "exiled_until_source_leaves"; object: U32; source: U32 }
  | { kind: "exiled_until_source_leaves_minting_illusion"; object: U32; source: U32 }
  | { kind: "leaves_illusion_minted"; object: U32; source: U32 }
  | { exiled: U32; kind: "token_granted_return_exiled_on_leave"; token: U32 }
  | { card: U32; from: U32; kind: "returned_exiled_card_to_graveyard" }
  | { kind: "exiled_with_source"; object: U32; source: U32 }
  | { kind: "card_exiled_with_source_left_exile"; object: U32; source: U32 }
  | { card: U32; kind: "cast_from_exile_free_permission_granted"; player: number }
  | { card: U32; kind: "cast_from_exile_free_bottoms_library_on_leave" }
  | { kind: "cast_from_exile_free_ended" }
  | { controller: number; from: U32; kind: "returned_from_linked_exile"; permanent: U32; source: U32 }
  | { controller: number; from: U32; kind: "flickered_to_battlefield"; permanent: U32 }
  | { card: U32; from: U32; kind: "returned_to_hand" }
  | { card: U32; from: U32; kind: "tucked_to_library"; to_top: boolean }
  | { kind: "library_shuffled"; player: number }
  | { card: U32; def: string; kind: "revealed_top_of_library"; player: number }
  | { card: U32; kind: "put_on_bottom_of_library"; player: number }
  | { card?: string | null; from?: null | U32; kind: "searched_to_hand"; object: U32; player: number }
  | { controller: number; from: U32; kind: "searched_to_battlefield"; permanent: U32; tapped: boolean }
  | { controller: number; kind: "manifested"; permanent: U32 }
  | { kind: "turned_face_up"; permanent: U32 }
  | { controller: number; from: U32; kind: "put_onto_battlefield_from_hand"; permanent: U32; tapped: boolean }
  | { card: U32; from: U32; kind: "milled"; player: number }
  | { amount: number; kind: "life_changed"; player: number; source?: null | number }
  | { kind: "drew_from_empty_library"; player: number }
  | { kind: "player_lost"; player: number }
  | { kind: "citys_blessing_gained"; player: number }
  | { card?: string | null; from?: null | U32; kind: "card_drawn"; object: U32; player: number }
  | { by: number; kind: "sacrificed"; object: U32 }
  | { card: U32; from: U32; kind: "discarded"; player: number }
  | { card: U32; from: U32; kind: "exiled_from_graveyard_may_play"; player: number };
export type WireModeChoice = { index: number; target?: null | WireTarget };
export type WireSpellDamage = { amount: number; target: WireTarget };
export type ActionView = {
  ability_index?: never;
  auto_tap?: Array<U32>;
  discard_choices?: Array<number>;
  discard_count?: number;
  graveyard_exile_choices?: Array<number>;
  graveyard_exile_max?: number;
  graveyard_exile_min?: number;
  has_x?: boolean;
  id: number;
  kind: string;
  label: string;
  modal?: null | { choose: number; choose_max: number; modes: Array<ModeView> };
  needs_target: boolean;
  object?: null | number;
  required_attacks?: Array<WireAttack>;
  sacrifice_choices?: Array<number>;
  section: string;
  targets?: Array<WireTarget>;
};
export type PendingChoiceView =
  | { count: number; kind: "order_triggers"; labels: Array<string>; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "choose_target"; label: string; optional: boolean; player: number; source: U32 }
  | {
      items: Array<ChoiceItem>;
      kind: "choose_spell_targets";
      label: string;
      max: number;
      min: number;
      player: number;
      spell: U32;
    }
  | {
      items: Array<ChoiceItem>;
      kind: "choose_target_players";
      label: string;
      max: number;
      min: number;
      player: number;
      source: U32;
    }
  | { kind: "may_yes_no"; label: string; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "decline_untap"; player: number }
  | { cost: WireCost; kind: "pay_cost"; label: string; player: number; source: U32 }
  | { cost: WireCost; kind: "pay_or_counter"; player: number; spell: U32 }
  | { controller: number; cost: WireCost; kind: "pay_or_controller_draws"; player: number }
  | { kind: "choose_countered_spell_destination"; player: number; spell: U32 }
  | { cost: WireCost; kind: "pay_echo_or_sacrifice"; player: number; source: U32 }
  | { cost: WireCost; kind: "sacrifice_unless_pay"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "sacrifice_unless_return_land"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "assign_combat_damage"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "divide_spell_damage"; player: number; spell: U32; total: number }
  | { items: Array<ChoiceItem>; kind: "divide_counters"; player: number; spell: U32; total: number }
  | { items: Array<ChoiceItem>; kind: "scry"; player: number }
  | { items: Array<ChoiceItem>; kind: "surveil"; player: number }
  | { items: Array<ChoiceItem>; kind: "search_library"; player: number }
  | { items: Array<ChoiceItem>; kind: "select_from_top"; player: number; up_to: number }
  | {
      items: Array<ChoiceItem>;
      kind: "distribute_top";
      player: number;
      to_bottom: number;
      to_exile_may_play: number;
      to_hand: number;
    }
  | {
      items: Array<ChoiceItem>;
      kind: "shuffle_from_graveyard";
      max: number;
      owner: number;
      player: number;
      source: U32;
    }
  | { items: Array<ChoiceItem>; keep_one?: boolean; kind: "sacrifice_edict"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "proliferate"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "phase_out"; player: number; source: U32 }
  | {
      items: Array<ChoiceItem>;
      kind: "choose_ability_targets";
      label: string;
      max: number;
      min: number;
      player: number;
      source: U32;
    }
  | { items: Array<ChoiceItem>; kind: "may_sacrifice"; player: number; source: U32 }
  | { count: number; items: Array<ChoiceItem>; kind: "choose_own_sacrifices"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "devour"; multiplier: number; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "exile_from_graveyard"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "caster_keep_permanents"; player: number; source: U32; target_player: number }
  | {
      items: Array<ChoiceItem>;
      kind: "choose_counter_target_for_player";
      player: number;
      source: U32;
      target_player: number;
    }
  | { items: Array<ChoiceItem>; kind: "may_return_from_graveyard"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "may_discard"; player: number; source: U32 }
  | { count: number; items: Array<ChoiceItem>; kind: "discard"; player: number }
  | { items: Array<ChoiceItem>; kind: "put_land_from_hand"; player: number }
  | { items: Array<ChoiceItem>; kind: "cast_creature_face_down"; player: number }
  | { items: Array<ChoiceItem>; kind: "choose_exiled_with_card"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "choose_exiled_with_card_to_cast"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "choose_exiled_dig_to_cast_free"; player: number; source: U32 }
  | {
      budget: number;
      items: Array<ChoiceItem>;
      kind: "dance_exile_more";
      player: number;
      source: U32;
      total_mv: number;
    }
  | { kind: "opponent_chooses_pile"; pile_a: Array<ChoiceItem>; pile_b: Array<ChoiceItem>; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "opponent_chooses_exiled_nonland"; player: number; source: U32 }
  | {
      items: Array<ChoiceItem>;
      kind: "choose_splitting_opponent";
      label: string;
      player: number;
      source: U32;
    }
  | { items: Array<ChoiceItem>; kind: "partition_revealed"; player: number; source: U32 }
  | { kind: "choose_pile_for_hand"; pile_a: Array<ChoiceItem>; pile_b: Array<ChoiceItem>; player: number; source: U32 }
  | { count: number; items: Array<ChoiceItem>; kind: "choose_exiled_to_cast_free"; player: number; source: U32 }
  | { item: ChoiceItem; kind: "revealed_card_to_battlefield_or_hand"; player: number }
  | { kind: "choose_mode"; labels: Array<string>; player: number; source: U32 }
  | {
      choose: number;
      kind: "choose_trigger_modes";
      modes: Array<ModeView>;
      optional: boolean;
      player: number;
      source: U32;
    }
  | { amount: number; kind: "choose_mana_color"; player: number; source: U32 }
  | { kind: "choose_creature_type"; options: Array<string>; player: number; source: U32 }
  | { kind: "choose_color"; player: number; source: U32 }
  | { items: Array<ChoiceItem>; kind: "choose_copy_target"; player: number; source: U32 }
  | { attachment: U32; items: Array<ChoiceItem>; kind: "choose_attach_host"; optional: boolean; player: number };

export type WireIntent =
  | {
      bought_back?: boolean;
      discard_cost?: Array<U32>;
      evoked?: boolean;
      graveyard_exile?: Array<U32>;
      kicked?: boolean;
      kind: "cast";
      modes?: Array<WireModeChoice>;
      object: U32;
      player: number;
      replicate_count?: number;
      sacrifice_cost?: Array<U32>;
      strive_count?: number;
      target?: null | WireTarget;
      x?: number;
    }
  | { kind: "play_land"; object: U32; player: number }
  | { kind: "tap_for_mana"; object: U32; player: number }
  | {
      ability_index: number;
      kind: "activate_ability";
      object: U32;
      player: number;
      sacrifice?: Array<U32>;
      target?: null | WireTarget;
    }
  | { attackers: Array<WireAttack>; kind: "declare_attackers"; player: number }
  | { blocks: Array<WireBlock>; kind: "declare_blockers"; player: number }
  | { kind: "choose_order"; order: Array<number>; player: number }
  | { kind: "choose_targets"; player: number; targets: Array<WireTarget> }
  | { kind: "choose_target_players"; player: number; players: Array<number> }
  | { kind: "answer_may"; player: number; yes: boolean }
  | { kind: "pay_optional_cost"; pay: boolean; player: number }
  | { keep_tapped: Array<U32>; kind: "decline_untap"; player: number }
  | { assignment: Array<WireDamage>; kind: "assign_damage"; player: number }
  | { assignment: Array<WireSpellDamage>; kind: "divide_spell_damage"; player: number }
  | { bottom: Array<U32>; kind: "arrange_top"; player: number; top: Array<U32> }
  | { cards?: Array<U32>; kind: "select_from_top"; player: number }
  | {
      kind: "distribute_top";
      player: number;
      to_bottom?: Array<U32>;
      to_exile_may_play?: Array<U32>;
      to_hand?: Array<U32>;
    }
  | { cards?: Array<U32>; kind: "shuffle_from_graveyard"; player: number }
  | { choice?: null | U32; kind: "search_library"; player: number }
  | { kind: "choose_sacrifices"; player: number; sacrifices: Array<U32> }
  | { cards: Array<U32>; kind: "discard"; player: number }
  | { choice?: null | U32; kind: "put_land_from_hand"; player: number }
  | { choice?: null | U32; kind: "cast_creature_face_down"; player: number }
  | { kind: "return_land_or_sacrifice"; land?: null | U32; player: number }
  | { choice?: null | U32; kind: "choose_exiled_with_card"; player: number }
  | { choice?: null | U32; kind: "choose_exiled_with_card_to_cast"; player: number }
  | { choice?: null | U32; kind: "choose_exiled_dig_to_cast_free"; player: number }
  | { kind: "choose_opponent_pile"; pile: number; player: number }
  | { choice?: null | U32; kind: "revealed_card_to_battlefield_or_hand"; player: number }
  | { kind: "choose_mode"; mode: number; player: number }
  | { kind: "choose_trigger_modes"; modes: Array<WireModeChoice>; player: number }
  | { color: number; kind: "choose_mana_color"; player: number }
  | { kind: "choose_creature_type"; player: number; subtype: string }
  | { color: number; kind: "choose_color"; player: number }
  | { host?: null | U32; kind: "choose_attach_host"; player: number }
  | { copy?: null | U32; kind: "choose_copy_target"; player: number }
  | { kind: "choose_top_or_bottom"; player: number; top: boolean }
  | { card: U32; kind: "cycle"; player: number }
  | { card: U32; kind: "activate_hand_ability"; player: number }
  | { card: U32; kind: "suspend"; player: number }
  | { card: U32; kind: "encore"; player: number }
  | { kind: "turn_face_up"; permanent: U32; player: number }
  | { card: U32; kind: "cast_face_down"; player: number }
  | { kind: "cast_prepared"; player: number; source: U32; target?: null | WireTarget; x?: number }
  | { kind: "cast_adventure"; player: number; source: U32; target?: null | WireTarget; x?: number }
  | { kind: "cast_bestow"; object: U32; player: number; target?: null | WireTarget }
  | { kind: "pass_priority"; player: number }
  | { kind: "concede"; player: number }
  | {
      attackers?: Array<WireAttack>;
      blocks?: Array<WireBlock>;
      discard_cost?: Array<U32>;
      graveyard_exile?: Array<U32>;
      id: number;
      kind: "take_action";
      modes?: Array<WireModeChoice>;
      player: number;
      sacrifice: Array<U32>;
      target?: null | WireTarget;
      x?: number;
    };
export type VisibleState = {
  actions?: Array<ActionView>;
  active_player: number;
  can_act: boolean;
  combat: {
    attackers: Array<WireAttack>;
    attackers_declared: boolean;
    blockers_declared: Array<number>;
    blocks: Array<WireBlock>;
  };
  objects: Array<ObjectView>;
  pending_choice?: null | PendingChoiceView;
  players: Array<PlayerView>;
  priority: number;
  stack: Array<StackObjectView>;
  stack_hold_remaining_ms?: number;
  step: number;
  turn_yielded?: boolean;
  viewer: number;
  yielded?: boolean;
};
export type IntentEnvelope = { client_seq: number; intent: WireIntent; table_id: string };
export type StreamFrame =
  | { frame: "snapshot"; seq: number; state: VisibleState }
  | { auto_actions?: Array<string>; events: Array<VisibleEvent>; seq: number; state: VisibleState; frame: "delta" }
  | { frame: "heartbeat" };
