//! Pure-mint Effect dispatcher behind [`Game::run`] (card-dsl-and-card-pool spec deepen).
//!
//! One exhaustive `match` that calls family `mint_*` helpers. Apply stays in
//! [`crate::apply`]; pausing effects never reach here — [`Game::run`] intercepts them.

use crate::*;

impl Game {
    /// Private mint: the events one non-pausing effect would produce for `controller`
    /// against `target`. Pure — [`Game::run`] applies (and applies before minting more ids).
    /// Pausing / composite effects never reach this: [`Game::run`] intercepts them.
    pub(crate) fn execute_effect(
        &self,
        effect: Effect,
        controller: PlayerId,
        source: ObjectId,
        target: Option<Target>,
        x: u32,
    ) -> Vec<Event> {
        match effect {
            Effect::Control(control) => match control {
                c @ (ControlEffect::AttachSelfToEntering { .. }
                | ControlEffect::Equip
                | ControlEffect::GainControl { .. }
                | ControlEffect::GainControlUntilEndOfTurn { .. }
                | ControlEffect::ExchangeAllCreaturesUntilEndOfTurn { .. }
                | ControlEffect::GainControlAllUntilEndOfTurn { .. }
                | ControlEffect::GainControlWhile { .. }
                | ControlEffect::GoadTarget { .. }
                | ControlEffect::GrantSourceAbilitiesUntilEndOfTurn
                | ControlEffect::RegenerateShield { .. }
                | ControlEffect::RemoveFromCombat { .. }
                | ControlEffect::RevertAllCreaturesToOwners
                | ControlEffect::TapTarget { .. }
                | ControlEffect::UntapAll { .. }
                | ControlEffect::UntapTarget { .. }) => {
                    self.mint_control(c, controller, source, target, x)
                }
                ControlEffect::TargetOpponentGainsControl { .. }
                | ControlEffect::ExchangeControl { .. } => {
                    unreachable!("a pausing/composite effect resolves via Game::run")
                }
            },
            Effect::Counters(counters) => match counters {
                c @ (CountersEffect::AttackerDrawsControllerCounters { .. }
                | CountersEffect::DoubleCounters { .. }
                | CountersEffect::LevelUp { .. }
                | CountersEffect::PlaceVowCounters { .. }
                | CountersEffect::PutCounters { .. }
                | CountersEffect::PutCountersEach { .. }
                | CountersEffect::RemoveAllCountersThenDraw { .. }
                | CountersEffect::RemoveCounterFromSelf) => {
                    self.mint_counters(c, controller, source, target, x)
                }
                CountersEffect::CommanderEntersWithBonusCounters { .. }
                | CountersEffect::DoubleCountersOnTargetCreatures { .. }
                | CountersEffect::MoveCounters { .. }
                | CountersEffect::DoubleCountersOnAttachedCreature => {
                    unreachable!("a pausing/composite effect resolves via Game::run")
                }
            },
            Effect::Damage(damage) => self.mint_damage(damage, controller, source, target, x),
            Effect::Destroy(destroy) => self.mint_destroy(destroy, controller, source, target, x),
            Effect::Exile(exile) => self.mint_exile(exile, controller, source, target, x),
            Effect::Sacrifice(sacrifice) => {
                self.mint_sacrifice(sacrifice, controller, source, target, x)
            }
            Effect::Draw(draw) => self.mint_draw(draw, controller, source, target, x),
            Effect::Life(life) => self.mint_life(life, controller, source, target, x),
            Effect::Mana(mana) => self.mint_mana(mana, controller, source, target, x),
            Effect::Mill(mill) => self.mint_mill(mill, controller, source, target, x),
            Effect::Misc(misc) => match misc {
                m @ (MiscEffect::ArmCombatDamageWatch
                | MiscEffect::BecomePrepared
                | MiscEffect::FlipSource
                | MiscEffect::CounterTargetActivatedAbility
                | MiscEffect::CounterTargetSpell { .. }
                | MiscEffect::GrantChannelColorlessManaThisTurn
                | MiscEffect::GrantFlashThisTurn
                | MiscEffect::ScheduleAtNextUpkeep { .. }
                | MiscEffect::ScheduleColorlessManaForCounteredSpellNextMainPhase
                | MiscEffect::SkipNextUntapOpponentCreatures
                | MiscEffect::ScheduleNextCastTrigger { .. }
                | MiscEffect::ScheduleThisTurnCombatDamageCopy) => {
                    self.mint_misc(m, controller, source, target, x)
                }
                MiscEffect::Fight { .. }
                | MiscEffect::MustAttackRandomOpponent
                | MiscEffect::PreventCombatDamageToYouCreatingTokens { .. }
                | MiscEffect::PreventAllCombatDamageThisTurn => {
                    unreachable!("a pausing/composite effect resolves via Game::run")
                }
            },
            Effect::Pump(pump) => self.mint_pump(pump, controller, source, target, x),
            Effect::Reveal(reveal) => self.mint_reveal(reveal, controller, source, target, x),
            Effect::Token(token) => self.mint_tokens(token, controller, source, target, x),
            Effect::Zone(zone) => match zone {
                z @ (ZoneEffect::ExileDeadCreatureCreateCopyWithSubtype { .. }
                | ZoneEffect::FlickerTarget { .. }
                | ZoneEffect::Manifest
                | ZoneEffect::MassReturnFromGraveyard { .. }
                | ZoneEffect::ReanimateDyingEnchantedCreature { .. }
                | ZoneEffect::ReanimateToBattlefield { .. }
                | ZoneEffect::ReturnAllToHand { .. }
                | ZoneEffect::ReturnExiledCardToOwnersGraveyard { .. }
                | ZoneEffect::ReturnFlickeredCard { .. }
                | ZoneEffect::ReturnFromGraveyardToHand { .. }
                | ZoneEffect::ReturnThisAuraAttachedTo { .. }
                | ZoneEffect::ReturnThisFromGraveyardToBattlefield { .. }
                | ZoneEffect::ReturnThisToHand
                | ZoneEffect::ReturnToHand { .. }
                | ZoneEffect::ReturnObjectToHand { .. }
                | ZoneEffect::ExileGraveyardObjectGainLife { .. }
                | ZoneEffect::TuckFromGraveyard { .. }
                | ZoneEffect::TuckPermanentIntoLibrary { .. }
                | ZoneEffect::TuckSelfAndBlockedCreatures
                | ZoneEffect::ShuffleTargetPermanentIntoLibrary { .. }) => {
                    self.mint_zones(z, controller, source, target, x)
                }
                ZoneEffect::UntapSearchedLand
                | ZoneEffect::AttachTriggeringAuraToMintedToken { .. }
                | ZoneEffect::ReflexiveTrigger { .. }
                | ZoneEffect::ReturnFromGraveyardAttachedToToken { .. }
                | ZoneEffect::AttachSelfToReanimated
                | ZoneEffect::AttachSelfToMintedToken
                | ZoneEffect::AttachMintedAuraToTarget { .. }
                | ZoneEffect::ScheduleReturnThisAuraAttachedToReanimated
                | ZoneEffect::ScheduleReturnReanimatedToHand
                | ZoneEffect::ReturnThisAuraFromGraveyardAttachedToChosenHost
                | ZoneEffect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost
                | ZoneEffect::ShuffleTargetPermanentIntoLibraryThenReveal { .. }
                | ZoneEffect::ExileSelfWithTimeCounters { .. }
                | ZoneEffect::TuckSelfToLibraryBottom
                | ZoneEffect::ExileSelfOnResolve
                | ZoneEffect::ExileTargetGraveyardCardThenIfCreature { .. } => {
                    unreachable!("a pausing/composite effect resolves via Game::run")
                }
            },
            Effect::Static(_) => Vec::new(),
            Effect::Dig(_)
            | Effect::Choice(_)
            | Effect::Copy(_)
            | Effect::Sequence { .. }
            | Effect::Conditional { .. }
            | Effect::ChooseOne { .. } => {
                unreachable!("a pausing/composite effect resolves via Game::run")
            }
        }
    }
}
