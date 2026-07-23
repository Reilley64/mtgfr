# Nested Effect families

**Status:** Draft (design approved; implementation not started)  
**Date:** 2026-07-23  
**Module:** `crates/engine` (`types/effect`, `resolution/`, `effects.rs`, `label.rs`, `de.rs`), `crates/cards/data/`, `.agents/skills/card-dsl/DSL_REFERENCE.md`  
**Related:** [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md), [choices-actions-and-resolution](2026-07-20-choices-actions-and-resolution.md), [engine-core-and-event-model](2026-07-20-engine-core-and-event-model.md)

---

## Goal

Regroup the flat `Effect` enum into **family wrappers with inner mode enums**, and mirror that in the card DSL as adjacent tags:

```toml
[[abilities.effects]]
type = "damage"
mode = "each_creature"
amount = 1
```

End state: the top-level `Effect` surface is only family wrappers plus structural composers (`Sequence`, `Conditional`, `ChooseOne`). Every existing leaf becomes a mode under exactly one family. Hard cut — no serde aliases for old flat `type` strings. One bang: all families in a single change set.

This is a **behavior-identical** refactor of vocabulary shape. No new card behavior, no rules changes.

---

## Shape

```rust
pub enum Effect {
    Damage(DamageEffect),
    Draw(DrawEffect),
    Life(LifeEffect),
    Destroy(DestroyEffect),   // destroy + exile + sacrifice leaves (today's destroy mint family)
    Control(ControlEffect),
    Counters(CountersEffect),
    Mana(ManaEffect),
    Mill(MillEffect),
    Pump(PumpEffect),
    Reveal(RevealEffect),
    Token(TokenEffect),
    Zone(ZoneEffect),
    Copy(CopyEffect),
    Dig(DigEffect),           // scry/surveil/look/search/cascade/dance/…
    Choice(ChoiceEffect),     // may/pay/edict fan-outs/proliferate/choose color/…
    Static(StaticEffect),     // anthems, replacements, grants — never resolve via mint
    Misc(MiscEffect),         // schedule/flip/counter-spell/fight/… residual
    Sequence { steps: &'static [Effect] },
    Conditional { condition: Condition, then: &'static [Effect], negate: bool },
    ChooseOne { options: &'static [Effect] },
}
```

Leaf Rust names drop redundant family prefixes where clear (`DamageEffect::EachCreature`, not `DamageEachCreature`). Serde `mode` is `snake_case` of that leaf.

**TOML:** outer `#[serde(tag = "type", rename_all = "snake_case")]` on `Effect`; each family enum uses `#[serde(tag = "mode", rename_all = "snake_case")]`. Structural effects keep `type = "sequence"` / `"conditional"` / `"choose_one"` with **no** `mode`.

**Targeted burn example** (today’s `deal_damage`):

```toml
type = "damage"
mode = "target"
amount = 3
target = "any"
```

---

## Dispatch & helpers

Two-level matches, family-local:

```rust
match effect {
    Effect::Damage(d) => self.run_damage(d, ctx, events),
    Effect::Dig(d) => self.run_dig(d, ctx, events),
    Effect::Static(_) => { /* no-op at resolve */ }
    Effect::Sequence { steps } => self.run_sequence(steps, ctx, events),
    // …
}
```

Today’s `mint_*_family(effect: Effect)` becomes typed family entry points (`mint_damage(d: DamageEffect)`, …).

**Forwarding helpers** on `Effect` remain the query surface (`target()`, `label()`, pause/target-count predicates, contextualize fillers) so most call sites do not nest-match. Each family enum implements the same small inherent API (or a private trait) for those queries.

**Types layout:**

```
types/effect/
  mod.rs          // Effect + re-exports
  damage.rs       // DamageEffect
  draw.rs
  …
```

Resolution modules already roughly match families; Dig/Choice absorb today’s pause peels’ Effect routing. `pending` raise/answer stays choice-centric — only `if let Effect::…` pattern sites update to the nested form.

---

## Family membership (normative mapping)

Old flat TOML `type` (snake_case of today’s variant) → `(family, mode)`. Modes listed below are the serde `mode` strings.

### `damage` → `DamageEffect`

| Old `type` | `mode` |
|---|---|
| `deal_damage` | `target` |
| `deal_damage_to_self` | `to_self` |
| `deal_damage_to_target_controller` | `to_target_controller` |
| `deal_damage_to_entering_permanent` | `to_entering_permanent` |
| `damage_each_creature` | `each_creature` |
| `damage_each_player` | `each_player` |
| `damage_each_other_opponent` | `each_other_opponent` |

### `draw` → `DrawEffect`

| Old `type` | `mode` |
|---|---|
| `draw_cards` | `cards` |
| `target_player_draws` | `target_player` |
| `each_player_draws` | `each_player` |
| `attacking_player_draws` | `attacking_player` |
| `each_draw_step_player_draws` | `each_draw_step_player` |
| `target_owner_draws` | `target_owner` |

### `life` → `LifeEffect`

| Old `type` | `mode` |
|---|---|
| `gain_life` | `gain` |
| `lose_life` | `lose` |
| `opponent_gains_life` | `opponent_gains` |
| `gain_life_target_controller` | `gain_target_controller` |
| `target_player_gains_life` | `target_player_gains` |
| `target_player_loses_life` | `target_player_loses` |
| `drain_target` | `drain_target` |
| `each_opponent_drain` | `each_opponent_drain` |
| `each_opponent_loses_life` | `each_opponent_loses` |
| `each_player_life_becomes_highest` | `each_player_becomes_highest` |
| `attacker_loses_life_you_gain` | `attacker_loses_you_gain` |
| `attacker_loses_life_you_draw` | `attacker_loses_you_draw` |

### `destroy` → `DestroyEffect`

Destroy, exile, and sacrifice leaves that today mint through `mint_destroy_family` (and closely related sacrifice/exile object arms):

| Old `type` | `mode` |
|---|---|
| `destroy_target` | `destroy_target` |
| `destroy_all` | `destroy_all` |
| `destroy_triggering_damaged_creature` | `destroy_triggering_damaged_creature` |
| `exile_all` | `exile_all` |
| `exile_all_graveyards` | `exile_all_graveyards` |
| `exile_graveyard` | `exile_graveyard` |
| `exile_object` | `exile_object` |
| `exile_target` | `exile_target` |
| `exile_target_minting_illusion_on_leave` | `exile_target_minting_illusion_on_leave` |
| `exile_until_source_leaves` | `exile_until_source_leaves` |
| `sacrifice_enchanted_creature` | `sacrifice_enchanted_creature` |
| `sacrifice_object` | `sacrifice_object` |
| `sacrifice_source` | `sacrifice_source` |

### `control` → `ControlEffect`

| Old `type` | `mode` |
|---|---|
| `attach_self_to_entering` | `attach_self_to_entering` |
| `equip` | `equip` |
| `gain_control` | `gain_control` |
| `gain_control_until_end_of_turn` | `gain_control_until_end_of_turn` |
| `exchange_all_creatures_until_end_of_turn` | `exchange_all_creatures_until_end_of_turn` |
| `gain_control_all_until_end_of_turn` | `gain_control_all_until_end_of_turn` |
| `gain_control_while` | `gain_control_while` |
| `goad_target` | `goad_target` |
| `grant_source_abilities_until_end_of_turn` | `grant_source_abilities_until_end_of_turn` |
| `regenerate_shield` | `regenerate_shield` |
| `remove_from_combat` | `remove_from_combat` |
| `revert_all_creatures_to_owners` | `revert_all_creatures_to_owners` |
| `tap_target` | `tap_target` |
| `untap_all` | `untap_all` |
| `untap_target` | `untap_target` |
| `target_opponent_gains_control` | `target_opponent_gains_control` |
| `exchange_control` | `exchange_control` |

### `counters` → `CountersEffect`

| Old `type` | `mode` |
|---|---|
| `attacker_draws_controller_counters` | `attacker_draws_controller_counters` |
| `double_counters` | `double_counters` |
| `double_counters_on_attached_creature` | `double_counters_on_attached_creature` |
| `double_counters_on_target_creatures` | `double_counters_on_target_creatures` |
| `level_up` | `level_up` |
| `place_vow_counters` | `place_vow_counters` |
| `put_counters` | `put_counters` |
| `put_counters_each` | `put_counters_each` |
| `remove_all_counters_then_draw` | `remove_all_counters_then_draw` |
| `remove_counter_from_self` | `remove_counter_from_self` |
| `move_counters` | `move_counters` |
| `commander_enters_with_bonus_counters` | `commander_enters_with_bonus_counters` |

### `mana` → `ManaEffect`

| Old `type` | `mode` |
|---|---|
| `add_mana` | `add` |

### `mill` → `MillEffect`

| Old `type` | `mode` |
|---|---|
| `mill` | `mill` |
| `mill_self` | `mill_self` |
| `exile_discarded_with_this` | `exile_discarded_with_this` |
| `exile_from_graveyard_may_play` | `exile_from_graveyard_may_play` |
| `exile_target_from_graveyard_create_token_copy` | `exile_target_from_graveyard_create_token_copy` |
| `exile_target_from_graveyard_with_this` | `exile_target_from_graveyard_with_this` |
| `exile_top_may_play` | `exile_top_may_play` |

### `pump` → `PumpEffect`

| Old `type` | `mode` |
|---|---|
| `animate_self_until_end_of_turn` | `animate_self_until_end_of_turn` |
| `enchanted_attacker_pump_attacking_opponent_else_controller_loses_life` | `enchanted_attacker_pump_attacking_opponent_else_controller_loses_life` |
| `grant_keywords_to_permanents_you_control_until_end_of_turn` | `grant_keywords_to_permanents_you_control_until_end_of_turn` |
| `pump_creatures_you_control_until_end_of_turn` | `pump_creatures_you_control_until_end_of_turn` |
| `pump_other_attackers_attacking_your_opponents` | `pump_other_attackers_attacking_your_opponents` |
| `pump_self_until_end_of_turn` | `pump_self_until_end_of_turn` |
| `pump_until_end_of_turn` | `pump_until_end_of_turn` |
| `set_base_pt_creatures_you_control_until_end_of_turn` | `set_base_pt_creatures_you_control_until_end_of_turn` |
| `set_base_pt_target_until_end_of_turn` | `set_base_pt_target_until_end_of_turn` |
| `set_own_base_pt_from_amount` | `set_own_base_pt_from_amount` |
| `strip_keywords_from_opponents_creatures` | `strip_keywords_from_opponents_creatures` |
| `weaken_each_creature` | `weaken_each_creature` |

### `reveal` → `RevealEffect`

Non-pausing reveal/mint arms (pause-heavy digs live under `dig`):

| Old `type` | `mode` |
|---|---|
| `reveal_top_and_drain_mutual` | `top_and_drain_mutual` |
| `reveal_top_cards` | `top_cards` |
| `reveal_top_to_hand` | `top_to_hand` |
| `reveal_until` | `until` |

### `token` → `TokenEffect`

| Old `type` | `mode` |
|---|---|
| `become_copy_of_target_creature_gaining_myriad` | `become_copy_of_target_creature_gaining_myriad` |
| `copy_each_entered_this_turn_token_tapped_attacking` | `copy_each_entered_this_turn_token_tapped_attacking` |
| `create_token` | `create` |
| `create_token_copy` | `create_copy` |
| `create_treasure` | `create_treasure` |
| `myriad_token_copies` | `myriad_token_copies` |

### `zone` → `ZoneEffect`

| Old `type` | `mode` |
|---|---|
| `exile_dead_creature_create_copy_with_subtype` | `exile_dead_creature_create_copy_with_subtype` |
| `flicker_target` | `flicker_target` |
| `manifest` | `manifest` |
| `mass_return_from_graveyard` | `mass_return_from_graveyard` |
| `reanimate_dying_enchanted_creature` | `reanimate_dying_enchanted_creature` |
| `reanimate_to_battlefield` | `reanimate_to_battlefield` |
| `return_all_to_hand` | `return_all_to_hand` |
| `return_exiled_card_to_owners_graveyard` | `return_exiled_card_to_owners_graveyard` |
| `return_flickered_card` | `return_flickered_card` |
| `return_from_graveyard_to_hand` | `return_from_graveyard_to_hand` |
| `return_this_aura_attached_to` | `return_this_aura_attached_to` |
| `return_this_from_graveyard_to_battlefield` | `return_this_from_graveyard_to_battlefield` |
| `return_this_to_hand` | `return_this_to_hand` |
| `return_to_hand` | `return_to_hand` |
| `return_object_to_hand` | `return_object_to_hand` |
| `exile_graveyard_object_gain_life` | `exile_graveyard_object_gain_life` |
| `tuck_from_graveyard` | `tuck_from_graveyard` |
| `tuck_permanent_into_library` | `tuck_permanent_into_library` |
| `tuck_self_and_blocked_creatures` | `tuck_self_and_blocked_creatures` |
| `shuffle_target_permanent_into_library` | `shuffle_target_permanent_into_library` |
| `shuffle_target_permanent_into_library_then_reveal` | `shuffle_target_permanent_into_library_then_reveal` |
| `attach_triggering_aura_to_minted_token` | `attach_triggering_aura_to_minted_token` |
| `attach_self_to_reanimated` | `attach_self_to_reanimated` |
| `attach_self_to_minted_token` | `attach_self_to_minted_token` |
| `attach_minted_aura_to_target` | `attach_minted_aura_to_target` |
| `return_from_graveyard_attached_to_token` | `return_from_graveyard_attached_to_token` |
| `return_this_aura_from_graveyard_attached_to_chosen_host` | `return_this_aura_from_graveyard_attached_to_chosen_host` |
| `schedule_return_this_aura_attached_to_reanimated` | `schedule_return_this_aura_attached_to_reanimated` |
| `schedule_return_reanimated_to_hand` | `schedule_return_reanimated_to_hand` |
| `schedule_return_this_aura_from_graveyard_attached_to_chosen_host` | `schedule_return_this_aura_from_graveyard_attached_to_chosen_host` |
| `exile_target_graveyard_card_then_if_creature` | `exile_target_graveyard_card_then_if_creature` |
| `untap_searched_land` | `untap_searched_land` |
| `reflexive_trigger` | `reflexive_trigger` |
| `exile_self_with_time_counters` | `exile_self_with_time_counters` |
| `tuck_self_to_library_bottom` | `tuck_self_to_library_bottom` |
| `exile_self_on_resolve` | `exile_self_on_resolve` |

### `copy` → `CopyEffect`

| Old `type` | `mode` |
|---|---|
| `copy_target_spell` | `target_spell` |
| `copy_this_spell` | `this_spell` |
| `retarget_spell_copy` | `retarget_spell_copy` |
| `may_pay_to_copy_this` | `may_pay_to_copy_this` |
| `change_target_of_target_spell_or_ability` | `change_target_of_target_spell_or_ability` |
| `copy_triggering_spell` | `copy_triggering_spell` |
| `copy_triggering_spell_for_each_other_creature_you_control` | `copy_triggering_spell_for_each_other_creature_you_control` |
| `copy_triggering_ability` | `copy_triggering_ability` |
| `demonstrate` | `demonstrate` |
| `mint_free_copy_of_exiled_card` | `mint_free_copy_of_exiled_card` |

### `dig` → `DigEffect`

Library digs, cascade, dance, look/search, clash, partition piles:

| Old `type` | `mode` |
|---|---|
| `scry` | `scry` |
| `surveil` | `surveil` |
| `look_at_top` | `look_at_top` |
| `distribute_top` | `distribute_top` |
| `search_library` | `search_library` |
| `clash` | `clash` |
| `cascade` | `cascade` |
| `exile_top_cast_matching_free` | `exile_top_cast_matching_free` |
| `reveal_until_may_deploy` | `reveal_until_may_deploy` |
| `reveal_until_exile_cast_free` | `reveal_until_exile_cast_free` |
| `exile_top_until_stop_cast_free_under_budget` | `exile_top_until_stop_cast_free_under_budget` |
| `opponent_splits_exile_piles` | `opponent_splits_exile_piles` |
| `reveal_top_split_piles` | `reveal_top_split_piles` |
| `reveal_top_opponent_picks_one_to_graveyard` | `reveal_top_opponent_picks_one_to_graveyard` |
| `each_player_exiles_until_nonland_opponent_picks` | `each_player_exiles_until_nonland_opponent_picks` |
| `shuffle_library` | `shuffle_library` |
| `shuffle_target_cards_from_graveyard_into_library` | `shuffle_target_cards_from_graveyard_into_library` |
| `cash_out_exiled_with_this` | `cash_out_exiled_with_this` |
| `cast_exiled_with_this_free` | `cast_exiled_with_this_free` |
| `exile_target_graveyard_spell_cast_free` | `exile_target_graveyard_spell_cast_free` |
| `exile_target_graveyard_card_record_mana_value` | `exile_target_graveyard_card_record_mana_value` |
| `exile_random_from_graveyard_may_play` | `exile_random_from_graveyard_may_play` |

### `choice` → `ChoiceEffect`

May/pay, hand picks, edicts, votes, proliferate, phase out, choose color/type:

| Old `type` | `mode` |
|---|---|
| `may_draw_unless_pays` | `may_draw_unless_pays` |
| `may_sacrifice` | `may_sacrifice` |
| `may_return_from_graveyard` | `may_return_from_graveyard` |
| `may_discard` | `may_discard` |
| `target_player_may_draw` | `target_player_may_draw` |
| `damaging_creature_controller_may_draw` | `damaging_creature_controller_may_draw` |
| `may_draw_up_to` | `may_draw_up_to` |
| `may_draw_up_to_then_opponent_may_repeat` | `may_draw_up_to_then_opponent_may_repeat` |
| `sacrifice_self_unless_pay` | `sacrifice_self_unless_pay` |
| `sacrifice_self_unless_return_land` | `sacrifice_self_unless_return_land` |
| `discard` | `discard` |
| `put_from_hand_on_top` | `put_from_hand_on_top` |
| `put_land_from_hand` | `put_land_from_hand` |
| `put_creature_from_hand` | `put_creature_from_hand` |
| `cast_creature_face_down` | `cast_creature_face_down` |
| `each_player_sacrifices` | `each_player_sacrifices` |
| `each_player_exiles_from_graveyard` | `each_player_exiles_from_graveyard` |
| `target_player_exiles_from_graveyard` | `target_player_exiles_from_graveyard` |
| `caster_keeps_one_of_each_type_per_player` | `caster_keeps_one_of_each_type_per_player` |
| `each_player_controller_chooses_counter_target` | `each_player_controller_chooses_counter_target` |
| `councils_dilemma_vote` | `councils_dilemma_vote` |
| `join_forces_pay_mana` | `join_forces_pay_mana` |
| `each_player_names_card_then_reveals_top` | `each_player_names_card_then_reveals_top` |
| `each_other_token_becomes_copy_of_chosen` | `each_other_token_becomes_copy_of_chosen` |
| `put_counter_then_may_become_copy_of_card_from_list` | `put_counter_then_may_become_copy_of_card_from_list` |
| `sacrifice_own` | `sacrifice_own` |
| `defending_player_sacrifices` | `defending_player_sacrifices` |
| `each_player_discards_hand_then_draws` | `each_player_discards_hand_then_draws` |
| `each_player_creates_fractal_from_exiled_power` | `each_player_creates_fractal_from_exiled_power` |
| `proliferate` | `proliferate` |
| `phase_out` | `phase_out` |
| `choose_creature_type` | `choose_creature_type` |
| `choose_color` | `choose_color` |
| `set_own_color_until_end_of_turn` | `set_own_color_until_end_of_turn` |

### `static` → `StaticEffect`

Never mint via `execute_effect` (today’s static arms return `Vec::new()`):

| Old `type` | `mode` |
|---|---|
| `anthem_static` | `anthem` |
| `keyword_anthem_static` | `keyword_anthem` |
| `tapped_for_mana_bonus` | `tapped_for_mana_bonus` |
| `prevent_noncombat_damage_to_other_creatures_you_control` | `prevent_noncombat_damage_to_other_creatures_you_control` |
| `prevent_damage_to_self_removing_counter` | `prevent_damage_to_self_removing_counter` |
| `prevent_combat_damage_static` | `prevent_combat_damage` |
| `trigger_doubling_static` | `trigger_doubling` |
| `grant_mana_ability` | `grant_mana_ability` |
| `grant_to_attached` | `grant_to_attached` |
| `set_attached_base_pt` | `set_attached_base_pt` |
| `set_attached_types` | `set_attached_types` |
| `control_attached` | `control_attached` |
| `reduce_spell_cost` | `reduce_spell_cost` |
| `attack_tax` | `attack_tax` |
| `counter_scaled_attack_tax` | `counter_scaled_attack_tax` |
| `cant_be_attacked_by` | `cant_be_attacked_by` |
| `counter_replacement` | `counter_replacement` |
| `token_replacement` | `token_replacement` |
| `life_gain_replacement` | `life_gain_replacement` |
| `cast_x_replacement` | `cast_x_replacement` |
| `enters_with_counters` | `enters_with_counters` |
| `creatures_you_control_enter_with_counters` | `creatures_you_control_enter_with_counters` |
| `no_maximum_hand_size` | `no_maximum_hand_size` |
| `play_from_graveyard_once_per_turn` | `play_from_graveyard_once_per_turn` |

### `misc` → `MiscEffect`

Residual schedule / counter / fight / prevention shields / grants that are not static continuous effects:

| Old `type` | `mode` |
|---|---|
| `arm_combat_damage_watch` | `arm_combat_damage_watch` |
| `become_prepared` | `become_prepared` |
| `flip_source` | `flip_source` |
| `counter_target_activated_ability` | `counter_target_activated_ability` |
| `counter_target_spell` | `counter_target_spell` |
| `grant_channel_colorless_mana_this_turn` | `grant_channel_colorless_mana_this_turn` |
| `grant_flash_this_turn` | `grant_flash_this_turn` |
| `schedule_at_next_upkeep` | `schedule_at_next_upkeep` |
| `schedule_colorless_mana_for_countered_spell_next_main_phase` | `schedule_colorless_mana_for_countered_spell_next_main_phase` |
| `skip_next_untap_opponent_creatures` | `skip_next_untap_opponent_creatures` |
| `schedule_next_cast_trigger` | `schedule_next_cast_trigger` |
| `schedule_this_turn_combat_damage_copy` | `schedule_this_turn_combat_damage_copy` |
| `fight` | `fight` |
| `must_attack_random_opponent` | `must_attack_random_opponent` |
| `prevent_combat_damage_to_you_creating_tokens` | `prevent_combat_damage_to_you_creating_tokens` |
| `prevent_all_combat_damage_this_turn` | `prevent_all_combat_damage_this_turn` |

### Structural (no `mode`)

| Old `type` | Remains |
|---|---|
| `sequence` | `type = "sequence"` + `steps` |
| `conditional` | `type = "conditional"` + fields |
| `choose_one` | `type = "choose_one"` + `options` |

**Completeness rule:** every current `Effect` leaf must appear in exactly one table above (or structural). Implementation must fail compile/tests if a leaf is left flat or double-mapped. Prefer a checked migrator driven by this table.

---

## Migration

1. Introduce `types/effect/*.rs` family enums + nested `Effect` + serde tags.
2. Mechanically rewrite every `crates/cards/data/**/*.toml` (and token profiles) effect table using the mapping; no hand-maintained dual spelling.
3. Update Rust: `Game::run`, `execute_effect`, family mints/peels, `Effect` helpers, `label.rs`, contextualize/fill helpers, engine tests constructing `Effect::…`, DSL_REFERENCE.
4. Update [card-dsl-and-card-pool](2026-07-20-card-dsl-and-card-pool.md) examples to nested form; brief note in choices/resolution spec if it cites flat type strings.
5. **No** serde aliases for old flat `type`s.

Branch commits may be family-by-family for reviewability; merge is still one bang with the whole pool converted.

---

## Verification

- Every pool TOML deserializes under the new shape.
- `cargo nextest` / `just server-check` green — behavior-identical.
- Existing `tests/game.rs` scenarios pass without intentional rules edits.
- DSL_REFERENCE documents `type` + `mode` and the family list.

---

## Risks

- Large PR (types + ~all card TOMLs + exhaustive matches). Mitigate with migrator + mapping table as review focus.
- Family bucket debates (exile under `destroy` vs `zone`). Mapping above is authoritative for this bang; re-bucketing is a follow-up.
- Nested `Sequence`/`Conditional` steps must deserialize recursively with the same rules; keep `deny_unknown_fields`.

---

## Non-goals

- New card behavior or fidelity gains.
- Strategy/trait-object Effect dispatch.
- Flat `type` compatibility aliases.
- Nesting `Amount` / `Condition` / `TargetSpec`.
- Moving dig kickoffs out of `pending/` (optional follow-up; not required for the nest).

---

## Success criteria

- Top-level `Effect` is only the family wrappers listed above plus `Sequence` / `Conditional` / `ChooseOne`.
- Every deckable/token TOML effect uses `type` + `mode` (except structural).
- Same engine tests green; no intentional rules changes.
