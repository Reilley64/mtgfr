//! Pending-choice lifecycle (ADR 0004): raise → answer → resume elsewhere.
//!
//! External seam for callers (`Game::submit`, effect/cast/trigger/combat/priority pause sites):
//! - [`raise`] / [`ChoiceRequest`] — typed raise for common effect/cast pause sites
//! - [`raise_choice`] — pause on an already-built [`PendingChoice`] (triggers/combat/TBAs)
//! - [`answer`] — apply a multiplexed answer [`Intent`] (does **not** resume sequences)
//! - [`forced`] — conservative singleton auto-answer
//!
//! [`resume_deferred_sequence`](crate::Game::resume_deferred_sequence) stays on submit /
//! resolution — Choice owns pause ↔ answer ↔ events only.
//!
//! Handlers and transitional `begin_*` raise helpers live in [`handlers`]. `pause_for` is
//! private to this module so other engine modules must not poke `PendingChoice` raw — use
//! [`raise`] / [`raise_choice`] / `begin_*` instead.
//!
//! ## Deferred (next increments)
//! - Collapse remaining `begin_*` into [`ChoiceRequest`] variants (effects still call
//!   `begin_*` for ~30 families).
//! - Optional internal `ChoiceHandler` per kind family (locality for new kinds).

mod handlers;

use crate::{Event, Game, Intent, PendingChoice, Reject};

/// Engine-internal raise request (not wire). Partial coverage of common effect/cast pause
/// sites; other raises still use `begin_*` on [`Game`] during migration.
#[derive(Debug, Clone)]
pub(crate) enum ChoiceRequest {
    ChooseTarget {
        player: crate::PlayerId,
        source: crate::ObjectId,
        effect: crate::Effect,
        legal: Vec<crate::Target>,
        optional: bool,
    },
    PayOrCounter {
        player: crate::PlayerId,
        cost: crate::Cost,
        spell: crate::ObjectId,
    },
    ChooseCreatureType {
        player: crate::PlayerId,
        source: crate::ObjectId,
        options: &'static [&'static str],
    },
    ChooseColor {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    ChooseMode {
        player: crate::PlayerId,
        source: crate::ObjectId,
        target: Option<crate::Target>,
        x: u32,
        modes: &'static [crate::Effect],
    },
    MayYesNo {
        player: crate::PlayerId,
        source: crate::ObjectId,
        effect: crate::Effect,
    },
    DivideSpellDamage {
        player: crate::PlayerId,
        spell: crate::ObjectId,
        targets: Vec<crate::Target>,
        total: i32,
    },
    DivideCounters {
        player: crate::PlayerId,
        spell: crate::ObjectId,
        targets: Vec<crate::ObjectId>,
        total: i32,
    },
    ChooseManaColor {
        player: crate::PlayerId,
        source: crate::ObjectId,
        amount: u8,
    },
}

/// Raise a Choice from resolution (or cast). Constructs [`PendingChoice`] and pauses.
pub(crate) fn raise(game: &mut Game, request: ChoiceRequest) {
    let choice = match request {
        ChoiceRequest::ChooseTarget {
            player,
            source,
            effect,
            legal,
            optional,
        } => PendingChoice::ChooseTarget {
            player,
            source,
            effect,
            legal,
            optional,
        },
        ChoiceRequest::PayOrCounter {
            player,
            cost,
            spell,
        } => PendingChoice::PayOrCounter {
            player,
            cost,
            spell,
        },
        ChoiceRequest::ChooseCreatureType {
            player,
            source,
            options,
        } => PendingChoice::ChooseCreatureType {
            player,
            source,
            options,
        },
        ChoiceRequest::ChooseColor { player, source } => {
            PendingChoice::ChooseColor { player, source }
        }
        ChoiceRequest::ChooseMode {
            player,
            source,
            target,
            x,
            modes,
        } => PendingChoice::ChooseMode {
            player,
            source,
            target,
            x,
            modes,
        },
        ChoiceRequest::MayYesNo {
            player,
            source,
            effect,
        } => PendingChoice::MayYesNo {
            player,
            source,
            effect,
        },
        ChoiceRequest::DivideSpellDamage {
            player,
            spell,
            targets,
            total,
        } => PendingChoice::DivideSpellDamage {
            player,
            spell,
            targets,
            total,
        },
        ChoiceRequest::DivideCounters {
            player,
            spell,
            targets,
            total,
        } => PendingChoice::DivideCounters {
            player,
            spell,
            targets,
            total,
        },
        ChoiceRequest::ChooseManaColor {
            player,
            source,
            amount,
        } => PendingChoice::ChooseManaColor {
            player,
            source,
            amount,
        },
    };
    game.pause_for(choice);
}

/// Pause on an already-built [`PendingChoice`]. Production sites outside this module
/// (triggers, combat, turn-based discard, cast targeting) must use this instead of writing
/// `pending_choice` directly.
pub(crate) fn raise_choice(game: &mut Game, choice: PendingChoice) {
    game.pause_for(choice);
}

/// Whether `intent` is an answer to a pending Choice (not cast / pass / concede / …).
pub(crate) fn is_answer(intent: &Intent) -> bool {
    intent.is_choice_answer()
}

/// Apply `intent` as the answer to the current [`PendingChoice`].
///
/// Caller guarantees: a choice is pending, [`is_answer`], and
/// `intent.actor() == choice.player()` (`submit`'s existing gate).
///
/// Does **not** run `resume_deferred_sequence` / `after_events` — `submit` owns the
/// post-intent pipeline.
pub(crate) fn answer(game: &mut Game, intent: Intent) -> Result<Vec<Event>, Reject> {
    match intent {
        Intent::ChooseOrder { player, order } => game.choose_order(player, order),
        Intent::ChooseTargets { player, targets } => game.choose_targets(player, targets),
        Intent::ChooseTargetPlayers { player, players } => {
            game.choose_target_players(player, players)
        }
        // AnswerMay's yes/no wire shape also drives Dance with Calamity's exile-another loop.
        Intent::AnswerMay { player, yes } => {
            if matches!(
                game.pending_choice,
                Some(PendingChoice::DanceExileMore { .. })
            ) {
                game.dance_exile_more(player, yes)
            } else {
                game.answer_may(player, yes)
            }
        }
        // Pay-or-counter / pay-or-sacrifice reuse PayOptionalCost's wire shape.
        Intent::PayOptionalCost { player, pay } => {
            if matches!(
                game.pending_choice,
                Some(PendingChoice::PayOrCounter { .. })
            ) {
                game.pay_or_counter(player, pay)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::PayEchoOrSacrifice { .. })
            ) {
                game.pay_echo(player, pay)
            } else {
                game.pay_optional_cost(player, pay)
            }
        }
        Intent::AssignDamage { player, assignment } => {
            if matches!(
                game.pending_choice,
                Some(PendingChoice::DivideCounters { .. })
            ) {
                game.divide_counters(player, assignment)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::DivideMovedCounters { .. })
            ) {
                game.divide_moved_counters(player, assignment)
            } else {
                game.assign_damage(player, assignment)
            }
        }
        Intent::DivideSpellDamage { player, assignment } => {
            game.divide_spell_damage(player, assignment)
        }
        Intent::ArrangeTop {
            player,
            top,
            bottom,
        } => game.arrange_top(player, top, bottom),
        Intent::SelectFromTop { player, cards } => game.select_from_top(player, cards),
        Intent::DistributeTop {
            player,
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => game.distribute_top(player, to_hand, to_bottom, to_exile_may_play),
        Intent::ShuffleFromGraveyard { player, cards } => {
            game.shuffle_from_graveyard(player, cards)
        }
        Intent::SearchLibrary { player, choice } => game.search_library(player, choice),
        Intent::ChooseSacrifices { player, sacrifices } => {
            if matches!(
                game.pending_choice,
                Some(PendingChoice::MaySacrifice { .. })
            ) {
                game.answer_may_sacrifice(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::MayReturnFromGraveyard { .. })
            ) {
                game.answer_may_return_from_graveyard(player, sacrifices)
            } else if matches!(game.pending_choice, Some(PendingChoice::MayDiscard { .. })) {
                game.answer_may_discard(player, sacrifices)
            } else if matches!(game.pending_choice, Some(PendingChoice::Proliferate { .. })) {
                game.answer_proliferate(player, sacrifices)
            } else if matches!(game.pending_choice, Some(PendingChoice::PhaseOut { .. })) {
                game.answer_phase_out(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::ChooseOwnSacrifices { .. })
            ) {
                game.choose_own_sacrifices(player, sacrifices)
            } else if matches!(game.pending_choice, Some(PendingChoice::Devour { .. })) {
                game.answer_devour(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::ChooseExiledToCastFree { .. })
            ) {
                game.choose_exiled_to_cast_free(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::ExileFromGraveyard { .. })
            ) {
                game.choose_graveyard_exile(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::CasterKeepPermanents { .. })
            ) {
                game.answer_caster_keep(player, sacrifices)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::ChooseCounterTargetForPlayer { .. })
            ) {
                game.answer_choose_counter_target(player, sacrifices)
            } else {
                game.choose_sacrifices(player, sacrifices)
            }
        }
        Intent::Discard { player, cards } => game.answer_discard(player, cards),
        Intent::DeclineUntap {
            player,
            keep_tapped,
        } => game.answer_decline_untap(player, keep_tapped),
        Intent::PutLandFromHand { player, choice } => game.put_land_from_hand(player, choice),
        Intent::ChooseExiledWithCard { player, choice }
            if matches!(
                game.pending_choice,
                Some(PendingChoice::OpponentChoosesExiledNonland { .. })
            ) =>
        {
            game.choose_opponent_exiled_nonland(player, choice)
        }
        Intent::ChooseExiledWithCard { player, choice } => {
            game.choose_exiled_with_card(player, choice)
        }
        Intent::ChooseExiledWithCardToCast { player, choice } => {
            game.choose_exiled_with_card_to_cast(player, choice)
        }
        Intent::ChooseExiledDigToCastFree { player, choice } => {
            game.choose_exiled_dig_to_cast_free(player, choice)
        }
        Intent::ChooseOpponentPile { player, pile } => game.choose_opponent_pile(player, pile),
        Intent::RevealedCardToBattlefieldOrHand { player, choice } => {
            game.revealed_card_to_battlefield_or_hand(player, choice)
        }
        Intent::ChooseMode { player, mode }
            if matches!(game.pending_choice, Some(PendingChoice::CastVote { .. })) =>
        {
            game.answer_vote(player, mode)
        }
        Intent::ChooseMode { player, mode } => game.answer_choose_mode(player, mode),
        Intent::ChooseTriggerModes { player, modes } => {
            game.answer_choose_trigger_modes(player, modes)
        }
        Intent::ChooseManaColor { player, color } => game.choose_mana_color(player, color),
        Intent::ChooseCreatureType { player, subtype } => {
            game.choose_creature_type(player, subtype)
        }
        Intent::ChooseColor { player, color } => game.choose_color(player, color),
        Intent::ChooseCopyTarget { player, copy }
            if matches!(
                game.pending_choice,
                Some(PendingChoice::ChooseTokenToCopy { .. })
            ) =>
        {
            game.answer_each_other_token_becomes_copy(player, copy)
        }
        Intent::ChooseCopyTarget { player, copy }
            if matches!(
                game.pending_choice,
                Some(PendingChoice::ChooseCopyCardFromList { .. })
            ) =>
        {
            game.answer_choose_copy_card_from_list(player, copy)
        }
        Intent::ChooseCopyTarget { player, copy } => game.answer_enter_as_copy(player, copy),
        Intent::ChooseAttachHost { player, host } => game.choose_attach_host(player, host),
        Intent::ChooseTopOrBottom { player, top } => {
            game.choose_countered_spell_destination(player, top)
        }
        _ => Err(Reject::IllegalChoice),
    }
}

/// The single legal answer when the pending choice is *forced*; else `None`.
///
/// Conservative: never force May / Pay / Scry / fail-to-find / keep-one edicts.
pub(crate) fn forced(game: &Game) -> Option<Intent> {
    let choice = game.pending_choice.as_ref()?;
    match choice {
        PendingChoice::DiscardToHandSize {
            player,
            hand,
            count,
        }
        | PendingChoice::DiscardCards {
            player,
            hand,
            count,
        } => (*count == hand.len()).then(|| Intent::Discard {
            player: *player,
            cards: hand.clone(),
        }),
        PendingChoice::ChooseTarget {
            player,
            legal,
            optional,
            ..
        } => match (legal[..].len(), *optional) {
            (1, false) => Some(Intent::ChooseTargets {
                player: *player,
                targets: vec![legal[0]],
            }),
            _ => None,
        },
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
        _ => None,
    }
}

impl Game {
    /// Begin waiting on `choice` before resolution can continue.
    /// Private to [`pending`]: effects/cast use [`raise`] or `begin_*`.
    fn pause_for(&mut self, choice: PendingChoice) {
        self.pending_choice = Some(choice);
    }

    /// Take the pending choice for validation; invalid answers must call [`Self::restore_pause`].
    fn take_pending_choice(&mut self) -> Option<PendingChoice> {
        self.pending_choice.take()
    }

    /// Put back a pending choice after rejecting an invalid answer.
    fn restore_pause(&mut self, choice: PendingChoice) {
        self.pending_choice = Some(choice);
    }

    /// Clear the pause after a valid answer.
    pub(crate) fn finish_answer(&mut self) {
        self.pending_choice = None;
    }
}
