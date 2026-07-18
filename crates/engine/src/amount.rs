//! Numeric [`Amount`] evaluation — the single resolver every derived quantity routes through.
//!
//! Primary consumers: effect resolution (`Game::run` / mint), cast cost reduction, and
//! characteristic anthems. Trigger enqueue stays in [`crate::triggers`]; intervening-if
//! [`Condition`] checks stay there too and call into these helpers where they need a count.

use crate::*;

impl Game {
    /// Resolve an [`Amount`] to a concrete number, in the context of an effect resolving for
    /// `controller`, sourced from `source`, aimed at `target`, with the casting spell's chosen `x`.
    /// The single amount evaluator — every numeric effect routes here, so a new derived value is
    /// one match arm.
    pub(crate) fn resolve_amount(
        &self,
        amount: Amount,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> i32 {
        match amount {
            Amount::Fixed(n) => n,
            Amount::X => x as i32,
            // "half X, rounded up" (CR: the round-up default).
            Amount::HalfX => x.div_ceil(2) as i32,
            // ponytail: a live read (`x = 0` for every non-spell resolution, see `Amount`'s own
            // doc) never actually reaches here for `Trigger::YouCastThis` — `fill_cast_x` rewrites
            // this to `Fixed` at trigger placement (CR 603.4). The arm exists only so this match
            // stays exhaustive, mirroring `TriggeringSpellManaValue` below.
            Amount::HalfXRoundedDown => (x / 2) as i32,
            Amount::TwiceX => 2 * x as i32,
            Amount::PerCreatureYouControl => self.creatures_controlled(controller) as i32,
            Amount::PerCreatureOnBattlefield => self.creatures_on_battlefield() as i32,
            Amount::PerPermanentMatching { filter, zone } => {
                self.count_matching(&filter, zone, controller, source) as i32
            }
            Amount::SourcePower => self.power(source),
            Amount::SourceToughness => self.toughness(source),
            Amount::TargetPower => {
                self.power(expect_object_target(target, "a power-derived amount"))
            }
            Amount::TargetToughness => {
                self.toughness(expect_object_target(target, "a toughness-derived amount"))
            }
            Amount::TargetManaValue => self
                .def_of(expect_object_target(target, "a mana-value amount"))
                .mana_value() as i32,
            Amount::PerCounterOnSource => self.plus_counters(source),
            Amount::PerCounterOfKindOnSource { kind } => self.counters_of_kind(source, kind) as i32,
            Amount::LifeGainedThisTurn => {
                self.players[controller.0 as usize].life_gained_this_turn as i32
            }
            Amount::SpellsCastThisTurn => {
                self.players[controller.0 as usize].spells_cast_this_turn as i32
            }
            // Reads the resolving spell's chosen player target's hand size (Rousing Refrain's
            // "for each card in target opponent's hand"), off the target like
            // `CommanderCastsFromCommandZone` above.
            Amount::CardsInTargetPlayerHand => match target {
                Some(Target::Player(player)) => self.hand_of(player).len() as i32,
                other => panic!(
                    "a target-player-hand amount resolves with a chosen player target, got {other:?}"
                ),
            },
            // A live read off the effect's controller (Empyrial Armor) — no target involved.
            Amount::CardsInYourHand => self.hand_of(controller).len() as i32,
            // ponytail: reads the single commander's counter (matches the shared command_casts
            // tax counter, apply.rs). A partner-commander pair would need to sum both commanders'
            // counts; no soc-pool player has more than one commander.
            Amount::CommanderCastsFromCommandZone => {
                // A chosen player target (Commander's Insight's "target player") reads off that
                // player; a no-target context (an anthem like Vanguard of the Restless, which
                // always reads its own controller's count) falls back to `controller`.
                let player = match target {
                    Some(Target::Player(player)) => player,
                    None => controller,
                    Some(other) => panic!(
                        "a command-zone-cast amount resolves with a chosen player target or no target, got {other:?}"
                    ),
                };
                self.players[player.0 as usize].command_casts as i32
            }
            Amount::CreaturesDiedThisTurn => {
                self.players[controller.0 as usize].creatures_died_this_turn as i32
            }
            Amount::NontokenCreaturesEnteredThisTurn => {
                self.players[controller.0 as usize].nontoken_creatures_entered_this_turn as i32
            }
            Amount::TotalPowerYouControl => self
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    self.owner_of(id) == controller
                        && matches!(self.def_of(id).kind, CardKind::Creature { .. })
                })
                .map(|id| self.power(id))
                .sum(),
            Amount::IfCondition { condition, then } => {
                if !self.condition_holds(condition, TriggerContext::of(controller)) {
                    return 0;
                }
                self.resolve_amount(*then, controller, source, target, x)
            }
            // A placeholder [`contextualize_sacrifice_effect`] must have already rewritten to
            // `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::SacrificedCreaturePower => panic!(
                "Amount::SacrificedCreaturePower must be contextualized to Fixed before resolving"
            ),
            // A placeholder [`contextualize_sacrifice_effect`] must have already rewritten to
            // `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::SacrificedCreatureToughness => panic!(
                "Amount::SacrificedCreatureToughness must be contextualized to Fixed before resolving"
            ),
            Amount::CommanderColorCount => self
                .commander_identity_of(controller)
                .iter()
                .filter(|&&has_color| has_color)
                .count() as i32,
            // ponytail: like `SacrificedCreaturePower` above, a placeholder — `fill_cast_mana_value`
            // must have already rewritten it to `Fixed` before the ability reaches the stack (every
            // `CastSpell`-triggered ability's effect is contextualized at placement), so a live read (CR 603, CR 113)
            // here never happens. The arm exists only so this match stays exhaustive.
            Amount::TriggeringSpellManaValue => 0,
            // Same placeholder shape as `TriggeringSpellManaValue` above, one arm down —
            // `fill_cast_mana_spent` rewrites it to `Fixed` before the ability reaches the stack.
            Amount::TriggeringSpellManaSpent => 0,
            Amount::SpellSacrificeCount => self.spell_sacrifice_count(source) as i32,
            Amount::PermanentsDiedThisTurn => self.permanents_died_this_turn as i32,
            // Reads the snapshot `Effect::DestroyAll`'s resolve path just recorded on
            // [`ResolutionFrame`], restricted to `filter` (empty/default matches every destroyed
            // permanent — Culling Ritual's unfiltered mana count). No `permanent_matches` reuse: the
            // permanents are already off the battlefield by the time a following `Sequence`
            // step reads this, so matching runs against the snapshot's `def`/`controller`/
            // `token` facts instead of live board state.
            Amount::PermanentsDestroyedThisWay { filter } => self
                .resolution_frame
                .destroyed_this_way
                .iter()
                .filter(|snap| destroyed_this_way_matches(&filter, controller, snap))
                .count() as i32,
            // Reads the snapshot `Effect::EachPlayerExilesFromGraveyard` recorded (Augusta's "put
            // that many +1/+1 counters"); resolution-scoped, like `PermanentsDestroyedThisWay`.
            Amount::NonlandCardsExiledThisWay => {
                self.resolution_frame.nonland_cards_exiled_this_way as i32
            }
            // Reads the tallies this resolution's own `Effect::CouncilsDilemmaVote` round
            // accumulated (Fateful Tempest); resolution-scoped, like `NonlandCardsExiledThisWay`.
            Amount::PastVotes => self.resolution_frame.council_past_votes as i32,
            Amount::PresentVotes => self.resolution_frame.council_present_votes as i32,
            // Reads the mana value the preceding `Effect::MillSelf` step snapshotted (Fateful
            // Tempest's "damage … equal to the total mana value of cards milled this way").
            Amount::TotalManaValueMilledThisWay => {
                self.resolution_frame.milled_mana_value_this_way as i32
            }
            // Reads the mana value the preceding `Effect::ExileTargetGraveyardCardRecordManaValue`
            // step snapshotted (Surge to Victory's team +X/+0 pump); `0` if unset — unreachable in
            // practice, since a fizzled target drops the whole ability before this reads.
            Amount::ExiledCardManaValueThisWay => self
                .resolution_frame
                .surge_exiled_card
                .map_or(0, |(_, mv)| mv as i32),
            // A placeholder [`fill_auras_attached_to_dying_creature`] must have already rewritten
            // to `Fixed` before the ability reaches the stack — see the variant's own doc comment.
            Amount::AurasYouControlledAttachedToDyingCreature => panic!(
                "Amount::AurasYouControlledAttachedToDyingCreature must be contextualized to \
                 Fixed before resolving"
            ),
            Amount::IfSpellKicked { then, else_ } => {
                let amount = if self.spell_was_kicked(source) {
                    *then
                } else {
                    *else_
                };
                self.resolve_amount(amount, controller, source, target, x)
            }
            Amount::GreatestInstantOrSorceryManaValueCastThisTurn => {
                self.players[controller.0 as usize]
                    .greatest_instant_or_sorcery_mana_value_cast_this_turn as i32
            }
            Amount::OnePlusInstantsAndSorceriesCastThisTurn => {
                self.players[controller.0 as usize].instants_and_sorceries_cast_this_turn as i32 + 1
            }
            // CR 303.4: any Aura attached, regardless of controller — unlike
            // `AurasYouControlledAttachedToDyingCreature`, no controller filter and no death
            // involved (Kor Spiritdancer reads its own live attachments).
            Amount::AurasAttachedToSource => self
                .attachments(source)
                .into_iter()
                .filter(|&a| matches!(self.def_of(a).kind, CardKind::Aura))
                .count() as i32,
            Amount::InstantOrSorceryCardsInYourGraveyard => self
                .graveyard_cards(controller)
                .into_iter()
                .filter(|&id| matches!(self.def_of(id).kind, CardKind::Spell { .. }))
                .count() as i32,
            // ponytail: like `TriggeringSpellManaValue` above, a placeholder — `fill_combat_damage`
            // must have already rewritten it to `Fixed` with the batch's summed damage before the
            // watch's ability reaches the stack (see `queue_zero_base_power_combat_damage_triggers`),
            // so a live read here never happens for the pool. The arm exists only so this match
            // stays exhaustive.
            Amount::CombatDamageDealt => 0,
            // ponytail: like `TriggeringSpellManaValue` above, a placeholder — `fill_spells_cast_before_this`
            // must have already rewritten it to `Fixed` with the snapshotted storm count before a
            // `Trigger::YouCastThis` ability's effect reaches the stack (see the `Event::SpellCast`
            // arm of `Game::enqueue_triggers`), so a live read here never happens for the pool.
            // The arm exists only so this match stays exhaustive.
            Amount::SpellsCastBeforeThisThisTurn => 0,
            // ponytail: same placeholder shape as `CombatDamageDealt` above — `fill_triggering_damage_dealt`
            // must have already rewritten it to `Fixed` with the dealt amount before an
            // `EnchantedCreatureDealsDamage` watch's ability reaches the stack (see
            // `queue_enchanted_creature_deals_damage_triggers`), so a live read here never happens
            // for the pool. The arm exists only so this match stays exhaustive.
            Amount::TriggeringDamageDealt => 0,
        }
    }

    /// [`resolve_amount`](Self::resolve_amount) clamped to a non-negative count (for draw / mill /
    /// token / counter effects, which can't take a negative quantity).
    pub(crate) fn resolve_count(
        &self,
        amount: Amount,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> u32 {
        self.resolve_amount(amount, controller, source, target, x)
            .max(0) as u32
    }

    /// How many permanents (battlefield) or cards (graveyard) match `filter`. On the battlefield
    /// this reuses [`Game::permanent_matches`]; in a graveyard only the type / controller / mana-
    /// value axes apply (a card in a graveyard isn't tapped, enchanted, or a token/nontoken).
    pub(crate) fn count_matching(
        &self,
        filter: &PermanentFilter,
        zone: AmountZone,
        controller: PlayerId,
        source: ObjectId,
    ) -> usize {
        match zone {
            AmountZone::Battlefield => self
                .battlefield()
                .into_iter()
                .filter(|&id| self.permanent_matches(filter, id, controller, Some(source)))
                .count(),
            AmountZone::Graveyard => self
                .objects
                .iter()
                .filter_map(|o| match o {
                    Object::Card(c) if c.zone == Zone::Graveyard => Some(c),
                    _ => None,
                })
                .filter(|c| self.graveyard_card_matches(filter, c, controller))
                .count(),
        }
    }

    /// Whether a graveyard card matches the type / controller / mana-value axes of `filter`.
    fn graveyard_card_matches(
        &self,
        filter: &PermanentFilter,
        card: &Card,
        controller: PlayerId,
    ) -> bool {
        if !filter.types.is_empty() && !filter.types.intersects(card.def.kind.types()) {
            return false;
        }
        let yours = card.owner == controller;
        match filter.controller {
            FilterController::You if !yours => return false,
            FilterController::Opponent if yours => return false,
            _ => {}
        }
        if let Some(max) = filter.mv_max
            && card.def.mana_value() > max as u32
        {
            return false;
        }
        true
    }

    /// How many creatures are on the battlefield in total (all controllers).
    pub(crate) fn creatures_on_battlefield(&self) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| self.is_creature_on_battlefield(id))
            .count()
    }

    /// How many creatures `player` controls on the battlefield.
    pub(crate) fn creatures_controlled(&self, player: PlayerId) -> usize {
        self.battlefield()
            .into_iter()
            .filter(|&id| {
                self.owner_of(id) == player
                    && matches!(self.def_of(id).kind, CardKind::Creature { .. })
            })
            .count()
    }
}

/// Whether a [`state::DestroyedThisWay`] snapshot matches `filter`, relative to `you` (the
/// resolving effect's controller) — the snapshot-data sibling of [`Game::permanent_matches`],
/// for [`Amount::PermanentsDestroyedThisWay`] counting permanents already off the battlefield.
/// ponytail: only the types/subtypes/controller/token axes — the pool's two cards (Ceaseless
/// Conflict's "nontoken creature you controlled", Culling Ritual's unfiltered count) need no
/// more; a destroyed permanent's snapshot has no live tapped/mv/power context to widen into.
fn destroyed_this_way_matches(
    filter: &PermanentFilter,
    you: PlayerId,
    snap: &state::DestroyedThisWay,
) -> bool {
    if !filter.types.is_empty() && !filter.types.intersects(snap.def.kind.types()) {
        return false;
    }
    if !filter.subtypes.is_empty()
        && !filter
            .subtypes
            .iter()
            .any(|s| snap.def.subtypes.contains(s))
    {
        return false;
    }
    match filter.controller {
        FilterController::Any => {}
        FilterController::You if snap.controller != you => return false,
        FilterController::Opponent if snap.controller == you => return false,
        _ => {}
    }
    match filter.token {
        TokenFilter::Any => {}
        TokenFilter::Token if !snap.token => return false,
        TokenFilter::Nontoken if snap.token => return false,
        _ => {}
    }
    true
}
