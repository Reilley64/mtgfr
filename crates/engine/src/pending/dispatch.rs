//! Choice-discriminant answer / forced table (ADR 0004 deepen).
//!
//! [`answer`] matches the pending [`PendingChoice`] first, then accepts only the Intent
//! shape that variant expects — wrong Intent → [`Reject::IllegalChoice`]. [`forced`]
//! lives on the same table: most arms are `None`; only singleton / no-real-choice cases
//! return an Intent. Handlers under [`super::handlers`] still own apply logic.

use crate::{Event, Game, Intent, PendingChoice, Reject};

/// Apply `intent` as the answer to the current [`PendingChoice`].
///
/// Caller guarantees: a choice is pending, [`super::is_answer`], and
/// `intent.actor() == choice.player()` (`submit`'s existing gate).
///
/// Does **not** run `resume_deferred_sequence` / `after_events` — `submit` owns the
/// post-intent pipeline.
pub(crate) fn answer(game: &mut Game, intent: Intent) -> Result<Vec<Event>, Reject> {
    let Some(choice) = game.pending_choice.as_ref() else {
        return Err(Reject::IllegalChoice);
    };
    match choice {
        PendingChoice::OrderTriggers { .. } => match intent {
            Intent::ChooseOrder { player, order } => game.choose_order(player, order),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseTarget { .. }
        | PendingChoice::ChooseSpellTargets { .. }
        | PendingChoice::ChooseAbilityTargets { .. }
        | PendingChoice::ChooseActivationCostTargets { .. }
        | PendingChoice::ChooseSplittingOpponent { .. } => match intent {
            Intent::ChooseTargets { player, targets } => game.choose_targets(player, targets),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::MayYesNo { .. } => match intent {
            Intent::AnswerMay { player, yes } => game.answer_may(player, yes),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::MayDrawUpTo { .. } => match intent {
            Intent::ChooseDrawCount { player, count } => game.answer_may_draw_up_to(player, count),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::TradeSecretsCasterDraw { .. } => match intent {
            Intent::ChooseDrawCount { player, count } => {
                game.answer_trade_secrets_caster_draw(player, count)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::TradeSecretsRepeat { .. } => match intent {
            Intent::AnswerMay { player, yes } => game.answer_trade_secrets_repeat(player, yes),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DeclineUntap { .. } => match intent {
            Intent::DeclineUntap {
                player,
                keep_tapped,
            } => game.answer_decline_untap(player, keep_tapped),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseDredge { .. } => match intent {
            Intent::ChooseDredge { player, dredger } => game.answer_choose_dredge(player, dredger),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayCost { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_optional_cost(player, pay),
            Intent::PayOptionalCostX { player, pay, x } => {
                game.pay_optional_cost_with_x(player, pay, x)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayOrCounter { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_or_counter(player, pay),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayOrControllerDraws { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_or_controller_draws(player, pay),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseCounteredSpellDestination { .. } => match intent {
            Intent::ChooseTopOrBottom { player, top } => {
                game.choose_countered_spell_destination(player, top)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayEchoOrSacrifice { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_echo(player, pay),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayCumulativeUpkeepOrSacrifice { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.pay_cumulative_upkeep(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PayRecoverOrExile { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_recover(player, pay),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::SacrificeUnlessPay { .. } => match intent {
            Intent::PayOptionalCost { player, pay } => game.pay_sacrifice_unless(player, pay),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::SacrificeUnlessReturnLand { .. } => match intent {
            Intent::ReturnLandOrSacrifice { player, land } => {
                game.return_land_or_sacrifice(player, land)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::AssignCombatDamage { .. } => match intent {
            Intent::AssignDamage { player, assignment } => game.assign_damage(player, assignment),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DivideSpellDamage { .. } => match intent {
            Intent::DivideSpellDamage { player, assignment } => {
                game.divide_spell_damage(player, assignment)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DivideCounters { .. } => match intent {
            Intent::AssignDamage { player, assignment } => game.divide_counters(player, assignment),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DivideMovedCounters { .. } => match intent {
            Intent::AssignDamage { player, assignment } => {
                game.divide_moved_counters(player, assignment)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ArrangeTop { .. } => match intent {
            Intent::ArrangeTop {
                player,
                top,
                bottom,
            } => game.arrange_top(player, top, bottom),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::SelectFromTop { .. } => match intent {
            Intent::SelectFromTop { player, cards } => game.select_from_top(player, cards),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DanceExileMore { .. } => match intent {
            Intent::AnswerMay { player, yes } => game.dance_exile_more(player, yes),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DistributeTop { .. } => match intent {
            Intent::DistributeTop {
                player,
                to_hand,
                to_bottom,
                to_exile_may_play,
            } => game.distribute_top(player, to_hand, to_bottom, to_exile_may_play),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::Proliferate { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_proliferate(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PhaseOut { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_phase_out(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ShuffleFromGraveyard { .. } => match intent {
            Intent::ShuffleFromGraveyard { player, cards } => {
                game.shuffle_from_graveyard(player, cards)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::SearchLibrary { .. } => match intent {
            Intent::SearchLibrary { player, choice } => game.search_library(player, choice),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseMode { .. } => match intent {
            Intent::ChooseMode { player, mode } => game.answer_choose_mode(player, mode),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseTriggerModes { .. } => match intent {
            Intent::ChooseTriggerModes { player, modes } => {
                game.answer_choose_trigger_modes(player, modes)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::SacrificeEdict { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.choose_sacrifices(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseTargetPlayers { .. } => match intent {
            Intent::ChooseTargetPlayers { player, players } => {
                game.choose_target_players(player, players)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ExileFromGraveyard { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.choose_graveyard_exile(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::CasterKeepPermanents { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_caster_keep(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseCounterTargetForPlayer { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_choose_counter_target(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::CastVote { .. } => match intent {
            Intent::ChooseMode { player, mode } => game.answer_vote(player, mode),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::MaySacrifice { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_may_sacrifice(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::MayReturnFromGraveyard { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_may_return_from_graveyard(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::MayDiscard { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_may_discard(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::DiscardToHandSize { .. } | PendingChoice::DiscardCards { .. } => {
            match intent {
                Intent::Discard { player, cards } => game.answer_discard(player, cards),
                _ => Err(Reject::IllegalChoice),
            }
        }
        PendingChoice::PutFromHandOnTop { .. } => match intent {
            Intent::PutFromHandOnTop { player, cards } => {
                game.answer_put_from_hand_on_top(player, cards)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PutLandFromHand { .. } => match intent {
            Intent::PutLandFromHand { player, choice } => game.put_land_from_hand(player, choice),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PutCreatureFromHand { .. } => match intent {
            Intent::PutCreatureFromHand { player, choice } => {
                game.put_creature_from_hand(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::CastCreatureFaceDown { .. } => match intent {
            Intent::CastCreatureFaceDown { player, choice } => {
                game.cast_creature_face_down(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseExiledWithCard { .. } => match intent {
            Intent::ChooseExiledWithCard { player, choice } => {
                game.choose_exiled_with_card(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseExiledWithCardToCast { .. } => match intent {
            Intent::ChooseExiledWithCardToCast { player, choice } => {
                game.choose_exiled_with_card_to_cast(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseExiledDigToCastFree { .. } => match intent {
            Intent::ChooseExiledDigToCastFree { player, choice } => {
                game.choose_exiled_dig_to_cast_free(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::OpponentChoosesPile { .. } => match intent {
            Intent::ChooseOpponentPile { player, pile } => game.choose_opponent_pile(player, pile),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::OpponentChoosesExiledNonland { .. } => match intent {
            Intent::ChooseExiledWithCard { player, choice } => {
                game.choose_opponent_exiled_nonland(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::PartitionRevealed { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.partition_revealed(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::OpponentChoosesRevealedToGraveyard { .. } => match intent {
            Intent::ChooseExiledWithCard { player, choice } => {
                game.choose_opponent_revealed_to_graveyard(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChoosePileForHand { .. } => match intent {
            Intent::ChooseOpponentPile { player, pile } => game.choose_pile_for_hand(player, pile),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseExiledToCastFree { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.choose_exiled_to_cast_free(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::RevealedCardToBattlefieldOrHand { .. } => match intent {
            Intent::RevealedCardToBattlefieldOrHand { player, choice } => {
                game.revealed_card_to_battlefield_or_hand(player, choice)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseOwnSacrifices { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.choose_own_sacrifices(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::Devour { .. } => match intent {
            Intent::ChooseSacrifices { player, sacrifices } => {
                game.answer_devour(player, sacrifices)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseManaColor { .. } => match intent {
            Intent::ChooseManaColor { player, color } => game.choose_mana_color(player, color),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseCreatureType { .. } => match intent {
            Intent::ChooseCreatureType { player, subtype } => {
                game.choose_creature_type(player, subtype)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseColor { .. } => match intent {
            Intent::ChooseColor { player, color } => game.choose_color(player, color),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseCopyTarget { .. } => match intent {
            Intent::ChooseCopyTarget { player, copy } => game.answer_enter_as_copy(player, copy),
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseTokenToCopy { .. } => match intent {
            Intent::ChooseCopyTarget { player, copy } => {
                game.answer_each_other_token_becomes_copy(player, copy)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseCopyCardFromList { .. } => match intent {
            Intent::ChooseCopyTarget { player, copy } => {
                game.answer_choose_copy_card_from_list(player, copy)
            }
            _ => Err(Reject::IllegalChoice),
        },
        PendingChoice::ChooseAttachHost { .. } => match intent {
            Intent::ChooseAttachHost { player, host } => game.choose_attach_host(player, host),
            _ => Err(Reject::IllegalChoice),
        },
    }
}

/// The single legal answer when the pending choice is *forced*; else `None`.
///
/// Same discriminant table as [`answer`]: most variants default to `None`. Conservative —
/// never force May / Pay / Scry / fail-to-find / keep-one edicts.
pub(crate) fn forced(game: &Game) -> Option<Intent> {
    let choice = game.pending_choice.as_ref()?;
    match choice {
        PendingChoice::DiscardToHandSize {
            player,
            hand,
            count,
        } => (*count == hand.len()).then(|| Intent::Discard {
            player: *player,
            cards: hand.clone(),
        }),
        PendingChoice::DiscardCards {
            player,
            hand,
            count,
            or_one_matching,
        } => {
            // A land-escape-valve filter with a matching card in hand is a genuine choice (discard
            // the whole hand vs. the single land) even when `count` happens to equal the hand
            // size — only force the whole-hand answer when that alternative isn't on the table.
            let land_escape_available = or_one_matching
                .is_some_and(|filter| hand.iter().any(|&id| filter.matches(game.def_of(id))));
            (!land_escape_available && *count == hand.len()).then(|| Intent::Discard {
                player: *player,
                cards: hand.clone(),
            })
        }
        // Forced only when there's no real choice left: an exact (non-"up to") count that
        // already equals the whole legal set — same "take all `n`" shape
        // `Game::place_ability_second_clause` auto-fills without even pausing.
        PendingChoice::ChooseTarget {
            player,
            legal,
            count,
            ..
        } => (count.min == count.max && count.max as usize == legal.len()).then(|| {
            Intent::ChooseTargets {
                player: *player,
                targets: legal.clone(),
            }
        }),
        PendingChoice::OrderTriggers {
            player, effects, ..
        } => (effects.len() == 1).then(|| Intent::ChooseOrder {
            player: *player,
            order: vec![0],
        }),
        PendingChoice::SacrificeEdict {
            player,
            options,
            keep_one,
            ..
        } => (!keep_one && options.len() == 1).then(|| Intent::ChooseSacrifices {
            player: *player,
            sacrifices: options.clone(),
        }),
        PendingChoice::ExileFromGraveyard {
            player, options, ..
        } => (options.len() == 1).then(|| Intent::ChooseSacrifices {
            player: *player,
            sacrifices: options.clone(),
        }),
        // Default for every other discriminant — same table, no forced Intent.
        PendingChoice::ChooseSpellTargets { .. }
        | PendingChoice::MayYesNo { .. }
        | PendingChoice::MayDrawUpTo { .. }
        | PendingChoice::TradeSecretsCasterDraw { .. }
        | PendingChoice::TradeSecretsRepeat { .. }
        | PendingChoice::DeclineUntap { .. }
        | PendingChoice::ChooseDredge { .. }
        | PendingChoice::PayCost { .. }
        | PendingChoice::PayOrCounter { .. }
        | PendingChoice::PayOrControllerDraws { .. }
        | PendingChoice::ChooseCounteredSpellDestination { .. }
        | PendingChoice::PayEchoOrSacrifice { .. }
        | PendingChoice::PayCumulativeUpkeepOrSacrifice { .. }
        | PendingChoice::PayRecoverOrExile { .. }
        | PendingChoice::SacrificeUnlessPay { .. }
        | PendingChoice::SacrificeUnlessReturnLand { .. }
        | PendingChoice::AssignCombatDamage { .. }
        | PendingChoice::DivideSpellDamage { .. }
        | PendingChoice::DivideCounters { .. }
        | PendingChoice::DivideMovedCounters { .. }
        | PendingChoice::ArrangeTop { .. }
        | PendingChoice::SelectFromTop { .. }
        | PendingChoice::DanceExileMore { .. }
        | PendingChoice::DistributeTop { .. }
        | PendingChoice::Proliferate { .. }
        | PendingChoice::PhaseOut { .. }
        | PendingChoice::ChooseAbilityTargets { .. }
        | PendingChoice::ChooseActivationCostTargets { .. }
        | PendingChoice::ShuffleFromGraveyard { .. }
        | PendingChoice::SearchLibrary { .. }
        | PendingChoice::ChooseMode { .. }
        | PendingChoice::ChooseTriggerModes { .. }
        | PendingChoice::ChooseTargetPlayers { .. }
        | PendingChoice::CasterKeepPermanents { .. }
        | PendingChoice::ChooseCounterTargetForPlayer { .. }
        | PendingChoice::CastVote { .. }
        | PendingChoice::MaySacrifice { .. }
        | PendingChoice::MayReturnFromGraveyard { .. }
        | PendingChoice::MayDiscard { .. }
        | PendingChoice::PutFromHandOnTop { .. }
        | PendingChoice::PutLandFromHand { .. }
        | PendingChoice::PutCreatureFromHand { .. }
        | PendingChoice::CastCreatureFaceDown { .. }
        | PendingChoice::ChooseExiledWithCard { .. }
        | PendingChoice::ChooseExiledWithCardToCast { .. }
        | PendingChoice::ChooseExiledDigToCastFree { .. }
        | PendingChoice::OpponentChoosesPile { .. }
        | PendingChoice::OpponentChoosesExiledNonland { .. }
        | PendingChoice::ChooseSplittingOpponent { .. }
        | PendingChoice::PartitionRevealed { .. }
        | PendingChoice::OpponentChoosesRevealedToGraveyard { .. }
        | PendingChoice::ChoosePileForHand { .. }
        | PendingChoice::ChooseExiledToCastFree { .. }
        | PendingChoice::RevealedCardToBattlefieldOrHand { .. }
        | PendingChoice::ChooseOwnSacrifices { .. }
        | PendingChoice::Devour { .. }
        | PendingChoice::ChooseManaColor { .. }
        | PendingChoice::ChooseCreatureType { .. }
        | PendingChoice::ChooseColor { .. }
        | PendingChoice::ChooseCopyTarget { .. }
        | PendingChoice::ChooseTokenToCopy { .. }
        | PendingChoice::ChooseCopyCardFromList { .. }
        | PendingChoice::ChooseAttachHost { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Effect, PlayerId};

    #[test]
    fn answer_rejects_wrong_intent_for_the_pending_discriminant() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::MayYesNo {
            player: PlayerId(0),
            source: 0,
            effect: Effect::DrawCards {
                count: crate::Amount::Fixed(1),
            },
        });
        // ChooseTargets is a real answer Intent, but not for MayYesNo.
        let err = answer(
            &mut game,
            Intent::ChooseTargets {
                player: PlayerId(0),
                targets: vec![],
            },
        );
        assert!(matches!(err, Err(Reject::IllegalChoice)));
        assert!(
            game.pending_choice.is_some(),
            "rejected answer must leave the pause intact"
        );
    }

    #[test]
    fn forced_is_none_for_an_open_may() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::MayYesNo {
            player: PlayerId(0),
            source: 0,
            effect: Effect::DrawCards {
                count: crate::Amount::Fixed(1),
            },
        });
        assert!(forced(&game).is_none());
    }

    #[test]
    fn forced_singleton_order_triggers() {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(PendingChoice::OrderTriggers {
            player: PlayerId(0),
            source: 0,
            effects: vec![Effect::DrawCards {
                count: crate::Amount::Fixed(1),
            }],
        });
        assert_eq!(
            forced(&game),
            Some(Intent::ChooseOrder {
                player: PlayerId(0),
                order: vec![0],
            })
        );
    }
}
