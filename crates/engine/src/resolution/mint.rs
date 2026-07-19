//! Pure-mint Effect dispatcher behind [`Game::run`] (ADR 0002 deepen).
//!
//! One exhaustive `match` that calls family `mint_*_family` helpers. Apply stays in
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
            // Control family
            Effect::AttachSelfToEntering { .. }
            | Effect::Equip
            | Effect::GainControl { .. }
            | Effect::GainControlUntilEndOfTurn { .. }
            | Effect::ExchangeAllCreaturesUntilEndOfTurn { .. }
            | Effect::GainControlAllUntilEndOfTurn { .. }
            | Effect::GainControlWhile { .. }
            | Effect::GoadTarget { .. }
            | Effect::GrantSourceAbilitiesUntilEndOfTurn
            | Effect::RegenerateShield { .. }
            | Effect::RemoveFromCombat { .. }
            | Effect::TapTarget { .. }
            | Effect::UntapAll { .. }
            | Effect::UntapTarget { .. } => {
                self.mint_control_family(effect, controller, source, target, x)
            }
            // Counters family
            Effect::AttackerDrawsControllerCounters { .. }
            | Effect::DoubleCounters { .. }
            | Effect::LevelUp { .. }
            | Effect::PlaceVowCounters { .. }
            | Effect::PutCounters { .. }
            | Effect::PutCountersEach { .. }
            | Effect::RemoveAllCountersThenDraw { .. }
            | Effect::RemoveCounterFromSelf => {
                self.mint_counters_family(effect, controller, source, target, x)
            }
            // Damage family
            Effect::DamageEachCreature { .. }
            | Effect::DamageEachPlayer { .. }
            | Effect::DealDamage { .. }
            | Effect::DealDamageToSelf { .. }
            | Effect::DealDamageToTargetController { .. }
            | Effect::DealDamageToEnteringPermanent { .. } => {
                self.mint_damage_family(effect, controller, source, target, x)
            }
            // Destroy family
            Effect::DestroyAll { .. }
            | Effect::DestroyTarget { .. }
            | Effect::DestroyTriggeringDamagedCreature { .. }
            | Effect::ExileAll { .. }
            | Effect::ExileAllGraveyards
            | Effect::ExileGraveyard
            | Effect::ExileObject { .. }
            | Effect::ExileTarget { .. }
            | Effect::ExileTargetMintingIllusionOnLeave { .. }
            | Effect::ExileUntilSourceLeaves { .. }
            | Effect::SacrificeEnchantedCreature { .. }
            | Effect::SacrificeObject { .. }
            | Effect::SacrificeSource => {
                self.mint_destroy_family(effect, controller, source, target, x)
            }
            // Draw family
            Effect::DrawCards { .. }
            | Effect::TargetPlayerDraws { .. }
            | Effect::EachPlayerDraws { .. }
            | Effect::AttackingPlayerDraws { .. }
            | Effect::EachDrawStepPlayerDraws { .. }
            | Effect::TargetOwnerDraws { .. } => {
                self.mint_draw_family(effect, controller, source, target, x)
            }
            // Life family
            Effect::AttackerLosesLifeYouDraw { .. }
            | Effect::AttackerLosesLifeYouGain { .. }
            | Effect::DrainTarget { .. }
            | Effect::EachOpponentDrain { .. }
            | Effect::EachOpponentLosesLife { .. }
            | Effect::EachPlayerLifeBecomesHighest
            | Effect::GainLife { .. }
            | Effect::GainLifeTargetController { .. }
            | Effect::LoseLife { .. }
            | Effect::TargetPlayerGainsLife { .. }
            | Effect::TargetPlayerLosesLife { .. } => {
                self.mint_life_family(effect, controller, source, target, x)
            }
            // Mana family
            Effect::AddMana { .. } => {
                self.mint_mana_family(effect, controller, source, target, x)
            }
            // Mill family
            Effect::ExileDiscardedWithThis { .. }
            | Effect::ExileFromGraveyardMayPlay { .. }
            | Effect::ExileTargetFromGraveyardCreateTokenCopy { .. }
            | Effect::ExileTargetFromGraveyardWithThis
            | Effect::ExileTopMayPlay { .. }
            | Effect::Mill { .. }
            | Effect::MillSelf { .. } => {
                self.mint_mill_family(effect, controller, source, target, x)
            }
            // Misc family
            Effect::ArmCombatDamageWatch
            | Effect::BecomePrepared
            | Effect::FlipSource
            | Effect::CounterTargetActivatedAbility
            | Effect::CounterTargetSpell { .. }
            | Effect::GrantChannelColorlessManaThisTurn
            | Effect::GrantFlashThisTurn
            | Effect::ScheduleAtNextUpkeep { .. }
            | Effect::ScheduleColorlessManaForCounteredSpellNextMainPhase
            | Effect::SkipNextUntapOpponentCreatures
            | Effect::ScheduleNextCastTrigger { .. }
            | Effect::ScheduleThisTurnCombatDamageCopy => {
                self.mint_misc_family(effect, controller, source, target, x)
            }
            // Pump family
            Effect::AnimateSelfUntilEndOfTurn { .. }
            | Effect::EnchantedAttackerPumpAttackingOpponentElseControllerLosesLife { .. }
            | Effect::GrantKeywordsToPermanentsYouControlUntilEndOfTurn { .. }
            | Effect::PumpCreaturesYouControlUntilEndOfTurn { .. }
            | Effect::PumpOtherAttackersAttackingYourOpponents { .. }
            | Effect::PumpSelfUntilEndOfTurn { .. }
            | Effect::PumpUntilEndOfTurn { .. }
            | Effect::SetBasePtCreaturesYouControlUntilEndOfTurn { .. }
            | Effect::SetBasePtTargetUntilEndOfTurn { .. }
            | Effect::StripKeywordsFromOpponentsCreatures { .. }
            | Effect::WeakenEachCreature { .. } => {
                self.mint_pump_family(effect, controller, source, target, x)
            }
            // Reveal family
            Effect::RevealTopAndDrainMutual
            | Effect::RevealTopCards { .. }
            | Effect::RevealTopToHand { .. }
            | Effect::RevealUntil { .. } => {
                self.mint_reveal_family(effect, controller, source, target, x)
            }
            // Tokens family
            Effect::BecomeCopyOfTargetCreatureGainingMyriad { .. }
            | Effect::CopyEachEnteredThisTurnTokenTappedAttacking { .. }
            | Effect::CreateToken { .. }
            | Effect::CreateTokenCopy { .. }
            | Effect::CreateTreasure { .. }
            | Effect::MyriadTokenCopies { .. } => {
                self.mint_tokens_family(effect, controller, source, target, x)
            }
            // Zones family
            Effect::ExileDeadCreatureCreateCopyWithSubtype { .. }
            | Effect::FlickerTarget { .. }
            | Effect::Manifest
            | Effect::MassReturnFromGraveyard { .. }
            | Effect::ReanimateDyingEnchantedCreature { .. }
            | Effect::ReanimateToBattlefield { .. }
            | Effect::ReturnAllToHand { .. }
            | Effect::ReturnExiledCardToOwnersGraveyard { .. }
            | Effect::ReturnFlickeredCard { .. }
            | Effect::ReturnFromGraveyardToHand { .. }
            | Effect::ReturnThisAuraAttachedTo { .. }
            | Effect::ReturnThisFromGraveyardToBattlefield { .. }
            | Effect::ReturnThisToHand
            | Effect::ReturnToHand { .. }
            | Effect::ReturnObjectToHand { .. }
            | Effect::ExileGraveyardObjectGainLife { .. }
            | Effect::TuckFromGraveyard { .. }
            | Effect::TuckPermanentIntoLibrary { .. }
            | Effect::TuckSelfAndBlockedCreatures
            | Effect::ShuffleTargetPermanentIntoLibrary { .. } => {
                self.mint_zones_family(effect, controller, source, target, x)
            }
            // Static abilities are read during recompute, never resolved from the stack.
            Effect::AnthemStatic { .. }
            | Effect::KeywordAnthemStatic { .. }
            | Effect::TappedForManaBonus { .. }
            | Effect::PreventNoncombatDamageToOtherCreaturesYouControl
            | Effect::PreventDamageToSelfRemovingCounter
            | Effect::PreventCombatDamageStatic { .. }
            | Effect::TriggerDoublingStatic { .. }
            | Effect::GrantManaAbility { .. }
            | Effect::GrantToAttached { .. }
            | Effect::SetAttachedBasePT { .. }
            | Effect::SetAttachedTypes { .. }
            | Effect::ControlAttached
            | Effect::ReduceSpellCost { .. }
            | Effect::AttackTax { .. }
            | Effect::CounterScaledAttackTax
            | Effect::CantBeAttackedBy { .. }
            | Effect::CounterReplacement { .. }
            | Effect::TokenReplacement { .. }
            | Effect::LifeGainReplacement { .. }
            | Effect::CastXReplacement { .. }
            | Effect::EntersWithCounters { .. }
            | Effect::CreaturesYouControlEnterWithCounters { .. }
            | Effect::NoMaximumHandSize
            | Effect::PlayFromGraveyardOncePerTurn => Vec::new(),
            // Pausing / composite — only via Game::run
            Effect::Scry { .. }
            // Clash pauses on the opponent chooser and per-player keep/bottom scries — only via
            // Game::run.
            | Effect::Clash
            // Needs `&mut self` to arm the prevention shield on `Game::combat_extras` — only
            // resolves via `Game::run`.
            | Effect::PreventCombatDamageToYouCreatingTokens { .. }
            | Effect::PreventAllCombatDamageThisTurn
            | Effect::Surveil { .. }
            | Effect::LookAtTop { .. }
            | Effect::DistributeTop { .. }
            | Effect::ExileTopCastMatchingFree { .. }
            | Effect::RevealUntilMayDeploy { .. }
            | Effect::RevealUntilExileCastFree { .. }
            | Effect::Cascade { .. }
            | Effect::SearchLibrary { .. }
            | Effect::EachPlayerSacrifices { .. }
            | Effect::EachPlayerExilesFromGraveyard
            | Effect::TargetPlayerExilesFromGraveyard { .. }
            | Effect::CasterKeepsOneOfEachTypePerPlayer
            | Effect::EachPlayerControllerChoosesCounterTarget
            | Effect::CouncilsDilemmaVote { .. }
            | Effect::OpponentSplitsExilePiles
            | Effect::RevealTopSplitPiles
            | Effect::RevealTopOpponentPicksOneToGraveyard { .. }
            | Effect::EachPlayerExilesUntilNonlandOpponentPicks
            | Effect::EachPlayerCreatesFractalFromExiledPower { .. }
            | Effect::EachOtherTokenBecomesCopyOfChosen
            | Effect::PutCounterThenMayBecomeCopyOfCardFromList { .. }
            | Effect::EachPlayerDiscardsHandThenDraws { .. }
            | Effect::MaySacrifice { .. }
            | Effect::SacrificeOwn { .. }
            | Effect::DefendingPlayerSacrifices { .. }
            | Effect::MayReturnFromGraveyard { .. }
            | Effect::MayDiscard { .. }
            // Needs `&mut self` to pause on the MayYesNo/PayOrControllerDraws chain — only
            // resolves via `Game::run`, never this pure path.
            | Effect::MayDrawUnlessPays { .. }
            // Needs `&mut self` to pause the targeted player on a MayYesNo — only resolves via
            // `Game::run`, never this pure path.
            | Effect::TargetPlayerMayDraw { .. }
            // Needs `&mut self` to pause the controller on a MayDrawUpTo count choice — only
            // resolves via `Game::run`, never this pure path.
            | Effect::MayDrawUpTo { .. }
            // Needs `&mut self` to pause the caster then chain to the opponent's repeat-or-stop
            // pause (Trade Secrets) — only resolves via `Game::run`, never this pure path.
            | Effect::MayDrawUpToThenOpponentMayRepeat { .. }
            | Effect::ShuffleTargetCardsFromGraveyardIntoLibrary { .. }
            | Effect::Discard { .. }
            | Effect::PutFromHandOnTop { .. }
            | Effect::PutLandFromHand { .. }
            | Effect::PutCreatureFromHand
            | Effect::CastCreatureFaceDown
            | Effect::CashOutExiledWithThis
            | Effect::CastExiledWithThisFree
            | Effect::Fight { .. }
            | Effect::ChooseOne { .. }
            | Effect::ChooseCreatureType
            | Effect::ChooseColor
            | Effect::SetOwnColorUntilEndOfTurn
            | Effect::CopyTargetSpell
            | Effect::CopyThisSpell { .. }
            | Effect::RetargetSpellCopy { .. }
            // Needs `&mut self` to pause the payer on a `PayCost` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::MayPayToCopyThis { .. }
            // Pauses on `ChooseSpellTargets` to bend the chosen spell — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ChangeTargetOfTargetSpellOrAbility { .. }
            | Effect::CopyTriggeringSpell { .. }
            | Effect::CopyTriggeringSpellForEachOtherCreatureYouControl { .. }
            // Needs `&mut self` to mint the ability copy (`push_ability_group_with_x`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::CopyTriggeringAbility { .. }
            | Effect::Demonstrate { .. }
            // Records onto `Game::pending_enter_bonus_counters` — needs `&mut self`, only resolves
            // via `Game::run`, never this pure path.
            | Effect::CommanderEntersWithBonusCounters { .. }
            | Effect::Sequence { .. }
            | Effect::Conditional { .. }
            | Effect::Proliferate { .. }
            | Effect::PhaseOut
            | Effect::DoubleCountersOnTargetCreatures { .. }
            // Reads its second target clause (`targets_second`, the recipient opponent) — only
            // resolves via `Game::run`, never this pure `target`-only path.
            | Effect::TargetOpponentGainsControl { .. }
            // Reads its second target clause (`targets_second`, the "an opponent controls"
            // permanent) — only resolves via `Game::run`, never this pure `target`-only path.
            | Effect::ExchangeControl { .. }
            | Effect::MoveCounters { .. }
            | Effect::UntapSearchedLand
            | Effect::AttachTriggeringAuraToMintedToken { .. }
            | Effect::ReflexiveTrigger { .. }
            | Effect::ReturnFromGraveyardAttachedToToken { .. }
            | Effect::AttachSelfToReanimated
            | Effect::AttachSelfToMintedToken
            | Effect::AttachMintedAuraToTarget { .. }
            | Effect::DoubleCountersOnAttachedCreature
            | Effect::ScheduleReturnThisAuraAttachedToReanimated
            // Needs `&mut self` to read back the resolution's own events — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ScheduleReturnReanimatedToHand
            // Needs `&mut self` to pause on `ChooseAttachHost` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ReturnThisAuraFromGraveyardAttachedToChosenHost
            | Effect::ScheduleReturnThisAuraFromGraveyardAttachedToChosenHost
            // Needs `&mut self` to draw from the injected RNG — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileRandomFromGraveyardMayPlay
            | Effect::MustAttackRandomOpponent
            | Effect::ShuffleLibrary
            // Player-driven exile loop + a running tally across pauses — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTopUntilStopCastFreeUnderBudget { .. }
            // Needs `&mut self` to read the actual post-shuffle library order — only resolves
            // via `Game::run`, never this pure path.
            | Effect::ShuffleTargetPermanentIntoLibraryThenReveal { .. }
            // Needs `&mut self` to mint the exiled object id (`Game::next_object_id`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardSpellCastFree { .. }
            // Needs `&mut self` to write `ResolutionFrame::surge_exiled_card` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardCardRecordManaValue { .. }
            // Needs `&mut self` to mark `Game::self_exile_time_counters` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileSelfWithTimeCounters { .. }
            // Needs `&mut self` to mark `Game::self_tuck_to_library_bottom` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::TuckSelfToLibraryBottom
            // Needs `&mut self` to mint the free copy (`Game::mint_spell_copies`) — only
            // resolves via `Game::run`, never this pure path.
            | Effect::MintFreeCopyOfExiledCard { .. }
            // Needs `&mut self` to conditionally `run_sequence` its `then` — only resolves via
            // `Game::run`, never this pure path.
            | Effect::ExileTargetGraveyardCardThenIfCreature { .. }
            // Needs `&mut self` to pause on `SacrificeUnlessPay` — only resolves via `Game::run`,
            // never this pure path.
            | Effect::SacrificeSelfUnlessPay { .. }
            // Needs `&mut self` to scan the battlefield and pause on `SacrificeUnlessReturnLand`
            // (or sacrifice outright with no candidates) — only resolves via `Game::run`, never
            // this pure path.
            | Effect::SacrificeSelfUnlessReturnLand { .. } => {
                unreachable!("a pausing/composite effect resolves via Game::run")
            }
        }
    }
}
