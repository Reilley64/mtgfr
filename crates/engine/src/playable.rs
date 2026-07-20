//! Cast / play zone and timing gates shared by list, one-click, and full cast paths.
//!
//! Primary: CR 601 (timing and zone legality), CR 307 (sorcery speed), flash and
//! alternative cast windows. Shared by [`Game::cast`], [`Game::meaningful_actions`], and
//! one-click take-action. Deferred / gaps: see `docs/FIDELITY_BACKLOG.md`.

use crate::*;

/// Chosen parameters for a cast — absent in the list query ([`CastPlayKind::List`]).
pub(crate) struct CastInputs<'a> {
    pub target: Option<Target>,
    pub x: u32,
    pub modes: &'a [(usize, Option<Target>)],
    pub discard_cost: &'a [ObjectId],
    pub graveyard_exile: &'a [ObjectId],
    pub sacrifice_cost: &'a [ObjectId],
    /// Whether the caster is paying the spell's kicker cost (CR 702.33d — [`AdditionalCost::kicker`]).
    pub kicked: bool,
    /// Whether the caster is paying the spell's buyback cost (CR 702.27c —
    /// [`AdditionalCost::buyback`]), mirroring `kicked`'s own opt-in shape.
    pub bought_back: bool,
    /// Whether the caster is casting the spell for its evoke cost (CR 702.74a —
    /// [`CardDef::evoke`]), instead of the printed cost.
    pub evoked: bool,
    /// The caster's declared Strive target count (CR 702.42 — [`AdditionalCost::strive`]); 0 for
    /// a spell with no Strive, or "choose zero targets." See [`Intent::Cast`]'s own doc.
    pub strive_count: u8,
    /// How many times the caster paid the spell's Replicate cost (CR 702.108 —
    /// [`AdditionalCost::replicate`]); 0 for a spell with no Replicate, or "pay it zero times."
    /// See [`Intent::Cast`]'s own doc.
    pub replicate_count: u8,
}

/// Which cast legality surface is asking — list, one-click execute, or full execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CastPlayKind {
    /// [`Game::meaningful_actions`] / auto-pass (ADR 0007): timing, zone, affordability, and
    /// enough legal targets (including at least `modal_choose` playable modes); no priority; no
    /// cost picks.
    List,
    /// [`Intent::TakeAction`]: validates target/modes/`x` and chosen discard / graveyard-exile
    /// picks the same way as [`CastPlayKind::Full`].
    OneClick,
    /// [`Intent::Cast`]: full validation including discard and graveyard-exile payments.
    Full,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ValidatedCast {
    pub zone: Zone,
    pub def: CardDef,
    pub cost: Cost,
    pub from_command: bool,
    pub cast_via_flashback: bool,
    pub cast_via_escape: bool,
    pub chosen_modes: Modes,
    pub multi_target: Option<(TargetSpec, TargetCount)>,
    pub x: u32,
    pub target: Option<Target>,
}

impl Game {
    /// Whether `player` may cast `object` for auto-pass / meaningful-action enumeration.
    /// Requires a completable target set (or enough playable modes for a modal); prices with
    /// `x = 0` and no delve count.
    pub(crate) fn cast_listable(&self, player: PlayerId, object: ObjectId) -> Option<Zone> {
        self.validate_cast(
            player,
            object,
            &CastInputs {
                target: None,
                x: 0,
                modes: &[],
                discard_cost: &[],
                graveyard_exile: &[],
                sacrifice_cost: &[],
                kicked: false,
                bought_back: false,
                evoked: false,
                strive_count: 0,
                replicate_count: 0,
            },
            CastPlayKind::List,
        )
        .ok()
        .map(|v| v.zone)
    }

    /// Shared cast legality for list, one-click, and full execute paths.
    pub(crate) fn validate_cast(
        &self,
        player: PlayerId,
        object: ObjectId,
        inputs: &CastInputs<'_>,
        kind: CastPlayKind,
    ) -> Result<ValidatedCast, Reject> {
        let Object::Card(card) = &self.objects[object as usize] else {
            return Err(Reject::NotCastable);
        };
        if matches!(card.def.kind, CardKind::Land { .. }) {
            return Err(Reject::NotCastable);
        }
        let Some(zone) = self.playable_zone(object, player) else {
            return Err(Reject::NotCastable);
        };
        if kind != CastPlayKind::List && player != self.priority {
            return Err(Reject::NotYourPriority);
        }
        if !self.cast_timing_ok(player, object, card.def, kind) {
            return Err(Reject::WrongTiming);
        }
        let from_command = zone == Zone::Command;
        let from_graveyard = zone == Zone::Graveyard;
        let cast_via_flashback = from_graveyard && card.def.flashback.is_some();
        let cast_via_escape = from_graveyard && card.def.escape.is_some();

        let mut multi_target = (!card.def.modal)
            .then(|| self.spell_multi_target(card.def))
            .flatten();

        // CR 601.2b: {X} (and modes) are chosen before targets (CR 601.2c) — computed here,
        // ahead of every target-legality check below, so a `PermanentFilter::mv_eq_x` target
        // filter (Entrancing Melody) sees the caster's chosen X even though `object` is still
        // an `Object::Card` (not yet the `Object::Spell` a resolution-time re-check reads X off
        // of directly) at this point in the cast sequence.
        let x = inputs.x.min(u8::MAX as u32);

        let chosen = if card.def.modal {
            if inputs.target.is_some() {
                return Err(Reject::IllegalMode);
            }
            if kind == CastPlayKind::List {
                Modes::default()
            } else {
                self.validate_modes(object, card.def, inputs.modes, player, x)?;
                // The chosen mode may itself be multi-target (Prismari Charm's "one or two
                // targets") — same post-cast target-choice shape as a non-modal multi-target
                // spell, just scoped to the mode that was picked.
                multi_target = self.modal_multi_target(card.def, inputs.modes);
                if let Some((spec, count)) = multi_target {
                    let n = self
                        .legal_targets_for(spec, object, player, color_identity(card.def), x)
                        .len();
                    if count.min > 0 && n == 0 {
                        return Err(Reject::IllegalTarget);
                    }
                }
                Modes::from_choices(inputs.modes)
            }
        } else if let Some((spec, count)) = multi_target {
            if inputs.target.is_some() || !inputs.modes.is_empty() {
                return Err(Reject::IllegalTarget);
            }
            if kind != CastPlayKind::List {
                let n = self
                    .legal_targets_for(spec, object, player, color_identity(card.def), x)
                    .len();
                if count.min > 0 && n == 0 {
                    return Err(Reject::IllegalTarget);
                }
                // Strive (CR 601.2c/702.42): the caster commits to a target count before
                // choosing which targets — it can't exceed how many legal targets actually
                // exist (n) or the engine's fixed target-list width (MAX_TARGETS), or the
                // declared count and the eventual chosen-targets count would silently diverge
                // (overpaying `cast_cost`'s Strive pips for targets `choose_spell_targets` then
                // can't fill).
                if count.strive_scaled
                    && (inputs.strive_count as usize > n
                        || inputs.strive_count as usize > MAX_TARGETS)
                {
                    return Err(Reject::IllegalTarget);
                }
            }
            Modes::default()
        } else {
            if !inputs.modes.is_empty() {
                return Err(Reject::IllegalMode);
            }
            if kind != CastPlayKind::List
                && !self.targets_are_legal(object, card.def, inputs.target, player, None, x)
            {
                return Err(Reject::IllegalTarget);
            }
            Modes::default()
        };

        let delve_count = inputs.graveyard_exile.len().min(u8::MAX as usize) as u8;
        let cost = self.cast_cost(
            player,
            object,
            card.def,
            inputs.target,
            x,
            zone,
            delve_count,
            inputs.kicked,
            inputs.bought_back,
            inputs.evoked,
            inputs.strive_count,
            inputs.replicate_count,
        );

        if kind == CastPlayKind::List {
            if !self.cast_affordable_list(player, object, card.def, zone) {
                return Err(Reject::CannotPayCost);
            }
            return Ok(ValidatedCast {
                zone,
                def: card.def,
                cost,
                from_command,
                cast_via_flashback,
                cast_via_escape,
                chosen_modes: chosen,
                multi_target,
                x,
                target: inputs.target,
            });
        }

        self.cast_additional_cost_gate(player, object, cost, x)?;
        self.validate_cast_cost_picks(player, object, card.def, cost, cast_via_escape, inputs)?;

        Ok(ValidatedCast {
            zone,
            def: card.def,
            cost,
            from_command,
            cast_via_flashback,
            cast_via_escape,
            chosen_modes: chosen,
            multi_target,
            x,
            target: inputs.target,
        })
    }

    /// List ([`CastPlayKind::List`]) follows ADR 0007: instants count only in a reaction
    /// window or at sorcery speed. Execute ([`CastPlayKind::OneClick`]/[`CastPlayKind::Full`])
    /// follows [`Game::cast`]: any instant-speed spell may be cast whenever its caster holds
    /// priority (CR 117.1a). The post-attack declare-attackers window is a reaction window for
    /// each defending player so empty-stack removal can stop auto-pass before blockers (ADR 0007).
    pub(crate) fn cast_timing_ok(
        &self,
        player: PlayerId,
        object: ObjectId,
        def: CardDef,
        kind: CastPlayKind,
    ) -> bool {
        // "Cast this spell only during combat" (CR 601.3e — Cauldron Dance) is a restriction
        // layered on top of, not a substitute for, the ordinary instant/sorcery-speed gate below
        // — an instant with this flag is still open only during combat's steps, not any time it
        // would otherwise hold priority.
        if def.cast_only_during_combat && !self.step.is_combat() {
            return false;
        }
        let sorcery_ok = self.can_take_sorcery_speed_action(player);
        // Alchemist's Refuge's "you may cast spells this turn as though they had flash" (CR 601.3a)
        // is an unfiltered per-player permission — every spell the granted player casts
        // is treated as instant-speed for the rest of the turn. A card cast for free from exile
        // via a resolving spell/ability (cascade's hit, Herald of Amity's dig) likewise ignores
        // timing (CR 601.3e — the permission comes mid-resolution, e.g. with the cascading spell
        // still on the stack).
        let as_instant = def.is_instant_speed()
            || self.players[player.0 as usize].flash_permission_this_turn
            || self.may_cast_from_exile_free(object, player);
        if as_instant {
            if kind == CastPlayKind::List {
                return sorcery_ok
                    || !self.stack.is_empty()
                    || self.in_attack_response_window(player);
            }
            return true;
        }
        sorcery_ok
    }

    /// After attackers are declared, each defending seat's declare-attackers priority is a
    /// reaction window for empty-stack instants (before blockers).
    pub(crate) fn in_attack_response_window(&self, player: PlayerId) -> bool {
        self.step == Step::DeclareAttackers
            && self.combat.attackers_declared
            && self.is_attacked_player(player)
    }

    fn cast_affordable_list(
        &self,
        player: PlayerId,
        object: ObjectId,
        def: CardDef,
        zone: Zone,
    ) -> bool {
        let available = self.available_mana(player);
        // Delve (CR 702.66) can reduce generic — list as affordable when some exile count
        // in 0..=graveyard size makes the cost payable (otherwise Treasure Cruise never appears
        // until the caster already has {7}{U} floating).
        let max_delve = if def.delve {
            self.graveyard_of(player).len().min(u8::MAX as usize) as u8
        } else {
            0
        };
        let spell = Some(def.spell_characteristics());
        let affordable = |target: Option<Target>, delve: u8| {
            let cost = self.cast_cost(
                player, object, def, target, 0, zone, delve, false, false, false, 0, 0,
            );
            Self::affordable_from(available, cost, spell)
                && self
                    .cast_additional_cost_gate(player, object, cost, 0)
                    .is_ok()
        };
        let any_delve = |target: Option<Target>| (0..=max_delve).any(|d| affordable(target, d));

        // Modal: mana first, then enough playable modes for `modal_choose` (CR 700.2) — an Abrade
        // with nothing to hit must not brighten the hand or stop auto-pass.
        if def.modal {
            return any_delve(None) && self.modal_modes_listable(object, player, def);
        }
        // Multi-target: need at least `count.min` legal targets (an "up to N" with min 0 stays
        // listable on an empty board; Ashes to Ashes with one creature does not).
        if let Some((spec, count)) = self.spell_multi_target(def) {
            let n = self
                .legal_targets_for(spec, object, player, color_identity(def), 0)
                .len();
            return n >= count.min as usize && any_delve(None);
        }
        let spec = self.required_target(def, None);
        if spec == TargetSpec::None {
            return any_delve(None);
        }
        // Single-target: try each legal target so a per-target reducer (Killian) can make the
        // cast affordable against one creature but not another.
        self.legal_targets_for(spec, object, player, color_identity(def), 0)
            .into_iter()
            .any(|t| any_delve(Some(t)))
    }

    /// Whether `def`'s modal spell has at least [`CardDef::modal_choose`] modes the caster can
    /// actually pick right now (each mode either needs no target, or has enough legal ones).
    fn modal_modes_listable(&self, object: ObjectId, player: PlayerId, def: CardDef) -> bool {
        let colors = color_identity(def);
        let available = (0..MAX_MODES)
            .map_while(|m| nth_mode(def, m))
            .filter(|a| self.effect_targets_listable(a.effect, object, player, colors, 0))
            .count();
        available >= def.modal_choose as usize
    }

    fn validate_cast_cost_picks(
        &self,
        player: PlayerId,
        object: ObjectId,
        def: CardDef,
        cost: Cost,
        cast_via_escape: bool,
        inputs: &CastInputs<'_>,
    ) -> Result<(), Reject> {
        // Retrace's discard-a-land rider (CR 702.83a) shares the same discard-cost slot as the
        // unfiltered `discard` count — no pool card carries both, so the one slot suffices.
        let discard_n =
            cost.additional.discard as usize + usize::from(cost.additional.discard_land);
        let hand = self.hand_of(player);
        let distinct_discards = inputs
            .discard_cost
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        if inputs.discard_cost.len() != discard_n
            || distinct_discards != inputs.discard_cost.len()
            || inputs.discard_cost.contains(&object)
            || !inputs.discard_cost.iter().all(|id| hand.contains(id))
        {
            // Wrong or missing discard picks — not a mana shortfall.
            return Err(Reject::IllegalChoice);
        }
        if cost.additional.discard_land
            && !inputs
                .discard_cost
                .iter()
                .any(|&id| matches!(self.def_of(id).kind, CardKind::Land { .. }))
        {
            return Err(Reject::CannotPayCost);
        }

        let graveyard = self.graveyard_of(player);
        let distinct_exiled = inputs
            .graveyard_exile
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        let all_owned_graveyard_cards = distinct_exiled == inputs.graveyard_exile.len()
            && inputs
                .graveyard_exile
                .iter()
                .all(|id| graveyard.contains(id));
        let graveyard_exile_valid = if cast_via_escape {
            let escape = def.escape.expect("cast_via_escape implies def.escape");
            all_owned_graveyard_cards
                && inputs.graveyard_exile.len() == escape.exile as usize
                && !inputs.graveyard_exile.contains(&object)
        } else if def.delve {
            all_owned_graveyard_cards
        } else {
            inputs.graveyard_exile.is_empty()
        };
        if !graveyard_exile_valid {
            // Wrong or missing delve/escape exile picks — not a mana shortfall. (CR 702.19, CR 702.66, CR 406.5)
            return Err(Reject::IllegalChoice);
        }

        // An additional sacrifice cost (CR 601.2f), either optional (Plumb the Forbidden: 0 up to
        // however many of the caster's own permanents match the filter, all legal — "you may") or
        // mandatory and fixed (Dread Return's Flashback—Sacrifice three creatures: exactly N
        // distinct matches or the cast is rejected, CR 602.2b). No such cost on this spell rejects
        // any nonempty pick outright. Control is enforced here directly (CR 701.16d — you can only
        // sacrifice what you control), not via the filter's own `controller` axis, mirroring
        // `Game::activate_ability`'s sacrifice cost.
        let distinct_sacrifices = inputs
            .sacrifice_cost
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        let sacrifice_valid = match cost.additional.sacrifice {
            Some(SacrificeAdditionalCost { filter, count }) => {
                let count_valid = match count {
                    SacrificeAdditionalCostCount::OneOrMore => true,
                    SacrificeAdditionalCostCount::Exactly(n) => {
                        inputs.sacrifice_cost.len() == n as usize
                    }
                };
                count_valid
                    && distinct_sacrifices == inputs.sacrifice_cost.len()
                    && inputs.sacrifice_cost.iter().all(|&id| {
                        self.controller_of(id) == player
                            && self.permanent_matches(&filter, id, player, None)
                    })
            }
            None => inputs.sacrifice_cost.is_empty(),
        };
        if !sacrifice_valid {
            return Err(Reject::CannotPayCost);
        }

        // Kicker (CR 702.33d): only payable if the spell actually has one — a client can't opt
        // into a nonexistent rider. Its mana is already folded into `cost` by `Game::cast_cost`;
        // affordability of that total is checked by `Game::settle_payment` downstream.
        if inputs.kicked && cost.additional.kicker.is_none() {
            return Err(Reject::CannotPayCost);
        }
        // Buyback (CR 702.27c): only payable if the spell actually has one, mirroring kicker's
        // own gate above. Its mana is already folded into `cost` by `Game::cast_cost`;
        // affordability of that total is checked by `Game::settle_payment` downstream.
        if inputs.bought_back && cost.additional.buyback.is_none() {
            return Err(Reject::CannotPayCost);
        }
        // Evoke (CR 702.74a): only declarable if the card actually has an evoke cost — a client
        // can't opt into a nonexistent alternative cost, mirroring kicker's own gate above.
        if inputs.evoked && def.evoke.is_none() {
            return Err(Reject::CannotPayCost);
        }
        // Strive (CR 702.42): only declarable if the spell actually has one, mirroring kicker's
        // own gate above. Its mana is already folded into `cost` by `Game::cast_cost`.
        if inputs.strive_count > 0 && cost.additional.strive.is_none() {
            return Err(Reject::CannotPayCost);
        }
        // Replicate (CR 702.108): only declarable if the spell actually has one, mirroring
        // strive's own gate above. Its mana is already folded into `cost` by `Game::cast_cost`.
        if inputs.replicate_count > 0 && cost.additional.replicate.is_none() {
            return Err(Reject::CannotPayCost);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P0: PlayerId = PlayerId(0);
    const NO_ADD: AdditionalCost = AdditionalCost {
        discard: 0,
        discard_land: false,
        pay_life_x: false,
        pay_life: 0,
        sacrifice: None,
        kicker: None,
        buyback: None,
        strive: None,
        replicate: None,
    };

    fn flash_cost(generic: u8) -> Cost {
        Cost {
            generic,
            colored: [0; Color::COUNT],
            colorless: 0,
            x: 0,
            hybrid: &[],
            additional: NO_ADD,
            reduce_own_generic: None,
        }
    }

    fn spell_def(name: &'static str, cost: Cost, modal: bool) -> CardDef {
        CardDef {
            name,
            id: "",
            default_print: "",
            cost,
            kind: CardKind::Spell {
                speed: SpellSpeed::Sorcery,
            },
            legendary: false,
            uncounterable: false,
            modal,
            modal_choose: 1,
            modal_choose_max: None,
            modal_choose_max_if_commander: false,
            keywords: &[],
            conditional_keywords: &[],
            abilities: if modal {
                Box::leak(Box::new([
                    Ability {
                        timing: Timing::Spell,
                        effect: Effect::DealDamage {
                            amount: Amount::Fixed(2),
                            target: TargetSpec::AnyTarget,
                            count: TargetCount {
                                min: 1,
                                max: 1,
                                x_scaled: false,
                                sacrifice_scaled: false,
                                strive_scaled: false,
                            },
                            divided: false,
                        },
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    },
                    Ability {
                        timing: Timing::Spell,
                        effect: Effect::DestroyAll {
                            filter: PermanentFilter::of(TypeSet::ARTIFACT),
                        },
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    },
                ]))
            } else {
                Box::leak(Box::new([spell_ability(Effect::DrawCards {
                    count: Amount::Fixed(1),
                })]))
            },
            identity_pips: &[],
            colors: &[],
            devoid: false,
            enters_tapped: false,
            enters_tapped_unless: None,
            free_cast_if: None,
            cast_only_during_combat: false,
            approximates: None,
            oracle: None,
            set: "",
            subtypes: &[],
            otags: &[],
            cycling: None,
            cycling_sacrifice: SacrificeCost::None,
            flashback: None,
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            delve: false,
            escape: None,
            retrace: false,
            graveyard_cast_cost: None,
            cascade: false,
            functions_in_graveyard: false,
            enchant: None,
            enchant_graveyard: false,
            back: None,
            adventure: None,
            suspend: None,
            devour: None,
            demonstrate: false,
            enter_as_copy: None,
            encore: None,
            hand_ability: None,
            forecast: None,
            may_choose_not_to_untap: false,
            dredge: None,
        }
    }

    fn spell_ability(effect: Effect) -> Ability {
        Ability {
            timing: Timing::Spell,
            effect,
            optional: false,
            min_level: 0,
            cost: Cost::FREE,
            condition: None,
            once_each_turn: false,
        }
    }

    #[test]
    fn flashback_in_graveyard_is_listable_when_affordable() {
        let mut game = Game::new();
        let def = CardDef {
            flashback: Some(flash_cost(2)),
            echo: None,
            cumulative_upkeep: None,
            recover: None,
            bestow: None,
            morph: None,
            evoke: None,
            ..spell_def("Flashback Draw", Cost::FREE, false)
        };
        let object = game.spawn_in_graveyard(P0, def);
        game.fund_mana(P0);
        assert_eq!(
            game.cast_listable(P0, object),
            Some(Zone::Graveyard),
            "flashback casts share the list query with hand casts",
        );
    }

    #[test]
    fn list_and_one_click_agree_on_a_simple_hand_cast() {
        let mut game = Game::new();
        let object = game.spawn_in_hand(P0, spell_def("Simple", flash_cost(1), false));
        game.fund_mana(P0);
        assert_eq!(game.cast_listable(P0, object), Some(Zone::Hand));

        let inputs = CastInputs {
            target: None,
            x: 0,
            modes: &[],
            discard_cost: &[],
            graveyard_exile: &[],
            sacrifice_cost: &[],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        };
        assert!(
            game.validate_cast(P0, object, &inputs, CastPlayKind::OneClick)
                .is_ok(),
            "the same card passes the one-click gate when listed",
        );
    }

    #[test]
    fn discard_rider_lists_but_one_click_rejects() {
        let mut cost = flash_cost(0);
        cost.additional.discard = 1;
        let mut game = Game::new();
        let spell = game.spawn_in_hand(P0, spell_def("Discard rider", cost, false));
        let _other = game.spawn_in_hand(P0, spell_def("Pitch", Cost::FREE, false));
        game.fund_mana(P0);

        assert_eq!(
            game.cast_listable(P0, spell),
            Some(Zone::Hand),
            "auto-pass may stop — enough other hand cards exist",
        );

        let inputs = CastInputs {
            target: None,
            x: 0,
            modes: &[],
            discard_cost: &[],
            graveyard_exile: &[],
            sacrifice_cost: &[],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        };
        assert!(matches!(
            game.validate_cast(P0, spell, &inputs, CastPlayKind::OneClick),
            Err(Reject::IllegalChoice),
        ));
    }

    #[test]
    fn discard_rider_full_cast_accepts_named_cards() {
        let mut cost = flash_cost(0);
        cost.additional.discard = 1;
        let mut game = Game::new();
        let spell = game.spawn_in_hand(P0, spell_def("Discard rider", cost, false));
        let pitch = game.spawn_in_hand(P0, spell_def("Pitch", Cost::FREE, false));
        game.fund_mana(P0);

        let inputs = CastInputs {
            target: None,
            x: 0,
            modes: &[],
            discard_cost: &[pitch],
            graveyard_exile: &[],
            sacrifice_cost: &[],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        };
        assert!(
            game.validate_cast(P0, spell, &inputs, CastPlayKind::Full)
                .is_ok(),
            "Intent::Cast can name the discard payment",
        );
    }

    #[test]
    fn modal_spell_is_listable_when_a_mode_has_targets() {
        let mut game = Game::new();
        let object = game.spawn_in_hand(P0, spell_def("Modal", flash_cost(2), true));
        game.fund_mana(P0);
        // Mode 0 is AnyTarget — living players keep it choosable even on an empty board.
        assert_eq!(
            game.cast_listable(P0, object),
            Some(Zone::Hand),
            "a modal spell with a playable mode lists when affordable",
        );
    }

    #[test]
    fn modal_spell_is_not_listable_when_no_mode_has_targets() {
        let mut game = Game::new();
        // Both modes want a creature; empty board → nothing to choose.
        let object = game.spawn_in_hand(
            P0,
            CardDef {
                abilities: Box::leak(Box::new([
                    Ability {
                        timing: Timing::Spell,
                        effect: Effect::DealDamage {
                            amount: Amount::Fixed(3),
                            target: TargetSpec::Creature,
                            count: TargetCount::default(),
                            divided: false,
                        },
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    },
                    Ability {
                        timing: Timing::Spell,
                        effect: Effect::DestroyTarget {
                            target: TargetSpec::Permanent(PermanentFilter::of(TypeSet::ARTIFACT)),
                            count: TargetCount::default(),
                            cant_be_regenerated: false,
                        },
                        optional: false,
                        min_level: 0,
                        cost: Cost::FREE,
                        condition: None,
                        once_each_turn: false,
                    },
                ])),
                ..spell_def("ModalNoTargets", flash_cost(2), true)
            },
        );
        game.fund_mana(P0);
        assert_eq!(
            game.cast_listable(P0, object),
            None,
            "a modal spell with no playable mode is not listed",
        );
    }

    #[test]
    fn modal_spell_one_click_requires_chosen_modes() {
        let mut game = Game::new();
        let object = game.spawn_in_hand(P0, spell_def("Modal", flash_cost(2), true));
        game.fund_mana(P0);

        let empty = CastInputs {
            target: None,
            x: 0,
            modes: &[],
            discard_cost: &[],
            graveyard_exile: &[],
            sacrifice_cost: &[],
            kicked: false,
            bought_back: false,
            evoked: false,
            strive_count: 0,
            replicate_count: 0,
        };
        assert!(matches!(
            game.validate_cast(P0, object, &empty, CastPlayKind::OneClick),
            Err(Reject::IllegalMode),
        ));
    }
}
