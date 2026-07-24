//! [`ChoiceRequest`] and `PendingChoice` construction for [`super::raise`].
//!
//! Dig-loop / multi-step effect kickoffs (cascade, reveal-until, dance, edict prep, …) still
//! live as non-`begin_*` helpers on [`crate::Game`] that emit dig events then raise — variants
//! here are pause payloads, not pure constructors for those flows (prep mutates via events
//! before the pause).

mod common;
mod copy;
mod dig;
mod edict;
mod fanout;
mod library;
mod optional;

use crate::{Game, PendingChoice};

/// Engine-internal raise request (not wire). Covers effect/cast pause sites, fan-out kickoffs,
/// and dig-loop pause payloads (prep/dig events stay at the call site — see module deferred notes).
#[derive(Debug, Clone)]
pub(crate) enum ChoiceRequest {
    ChooseTarget {
        player: crate::PlayerId,
        source: crate::ObjectId,
        effect: crate::Effect,
        legal: Vec<crate::Target>,
        count: crate::TargetCount,
        x: u32,
        activated: bool,
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
        until_end_of_turn: bool,
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
    /// [`Effect::Choice(ChoiceEffect::Proliferate)`] — empty counter-bearing board skips (no pause).
    Proliferate {
        player: crate::PlayerId,
        source: crate::ObjectId,
        /// Iterations still to run, including this one (`0` is a no-op).
        remaining: u8,
    },
    /// [`Effect::Choice(ChoiceEffect::PhaseOut)`] — no other creatures skips.
    PhaseOut {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::Choice(ChoiceEffect::MaySacrifice)`] — no legal permanent skips.
    MaySacrifice {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
        then: &'static [crate::Effect],
    },
    /// [`CardDef::devour`] as-enters — no other creature skips.
    Devour {
        player: crate::PlayerId,
        source: crate::ObjectId,
        multiplier: u32,
    },
    /// [`Effect::Choice(ChoiceEffect::MayReturnFromGraveyard)`] — no legal card skips.
    MayReturnFromGraveyard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::CardFilter,
    },
    /// [`Effect::Choice(ChoiceEffect::MayDiscard)`] — empty hand skips.
    MayDiscard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        then: &'static [crate::Effect],
    },
    /// [`Effect::Choice(ChoiceEffect::Discard)`] — empty (or zero-count) hand skips.
    Discard {
        player: crate::PlayerId,
        count: u32,
        or_one_matching: Option<crate::CardFilter>,
    },
    /// [`Effect::Choice(ChoiceEffect::PutFromHandOnTop)`] — empty (or zero-count) hand skips.
    PutFromHandOnTop { player: crate::PlayerId, count: u32 },
    /// [`Effect::Choice(ChoiceEffect::SacrificeSelfUnlessPay)`] — always pauses.
    SacrificeUnlessPay {
        player: crate::PlayerId,
        source: crate::ObjectId,
        cost: crate::Cost,
    },
    /// [`Effect::Choice(ChoiceEffect::SacrificeSelfUnlessReturnLand)`] — no candidates → `None` (caller sacrifices).
    SacrificeUnlessReturnLand {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
    },
    /// [`Effect::Dig(DigEffect::Scry)`] / [`Effect::Dig(DigEffect::Surveil)`] — empty library skips.
    ArrangeTop {
        player: crate::PlayerId,
        count: u32,
        to_graveyard: bool,
    },
    /// [`Effect::Dig(DigEffect::LookAtTop)`] — empty library skips.
    SelectFromTop {
        player: crate::PlayerId,
        count: u32,
        filter: crate::CardFilter,
        up_to: u32,
        min: u32,
        dest: crate::TopDest,
        dest_tapped: bool,
        rest: crate::RestDest,
        mv_budget: Option<u32>,
    },
    /// [`Effect::Dig(DigEffect::DistributeTop)`] — empty library skips.
    DistributeTop {
        player: crate::PlayerId,
        count: u32,
        to_hand: u32,
        to_bottom: u32,
        to_exile_may_play: u32,
    },
    /// [`Effect::ShuffleFromGraveyard`] — empty graveyard skips.
    ShuffleFromGraveyard {
        answerer: crate::PlayerId,
        owner: crate::PlayerId,
        source: crate::ObjectId,
        max: u32,
    },
    /// [`Effect::Dig(DigEffect::SearchLibrary)`] — always pauses (fail-to-find is a legal answer).
    SearchLibrary {
        player: crate::PlayerId,
        filter: crate::CardFilter,
        dest: crate::SearchDest,
        tapped: bool,
        count: u8,
        overflow: Option<crate::SearchDest>,
    },
    /// [`Effect::Choice(ChoiceEffect::PutLandFromHand)`] — no hand land skips.
    PutLandFromHand {
        player: crate::PlayerId,
        tapped: bool,
    },
    /// [`Effect::Choice(ChoiceEffect::PutCreatureFromHand)`] — no hand creature skips.
    PutCreatureFromHand {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::Choice(ChoiceEffect::CastCreatureFaceDown)`] — no payable creature skips.
    CastCreatureFaceDown {
        player: crate::PlayerId,
        spent_mana: [u8; 6],
    },
    /// [`Effect::Dig(DigEffect::CashOutExiledWithThis)`] — empty exile pile skips.
    ChooseExiledWithCard {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::Dig(DigEffect::CastExiledWithThisFree)`] — empty exile pile skips.
    ChooseExiledWithCardToCast {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`CardDef::enter_as_copy`] as-enters — no candidate skips.
    EnterAsCopy {
        player: crate::PlayerId,
        source: crate::ObjectId,
        marker: crate::EnterAsCopy,
    },
    /// [`Effect::Choice(ChoiceEffect::EachOtherTokenBecomesCopyOfChosen)`] — no token skips.
    ChooseTokenToCopy {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Copy-from-list pause (counter placement stays at the call site) — no candidate skips.
    ChooseCopyCardFromList {
        player: crate::PlayerId,
        source: crate::ObjectId,
        cards: &'static [crate::ObjectId],
    },
    /// [`Effect::Choice(ChoiceEffect::SacrificeOwn)`] / annihilator — `options.len() <= count` → `None` (caller
    /// sacrifices all).
    ChooseOwnSacrifices {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
        count: u32,
    },
    /// Next seat in a graveyard-exile fan-out (Augusta / Relic) — empty remaining skips.
    NextGraveyardExile {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
    },
    /// Next seat in Tragic Arrogance's caster-keep fan-out — empty remaining skips.
    NextCasterKeep {
        remaining: Vec<crate::PlayerId>,
        caster: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Next seat in Nils' counter-target fan-out — empty remaining skips.
    NextCounterTarget {
        remaining: Vec<crate::PlayerId>,
        chooser: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Next seat in a join-forces payment round — empty remaining skips.
    NextJoinForcesPayment {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
    },
    /// Next seat in a council's-dilemma vote — empty remaining skips.
    NextVote {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
        options: &'static [&'static str],
    },
    /// Next seat in Conundrum Sphinx's name-a-card fan-out — mandatory, empty remaining skips
    /// (same "every living seat, never skipped" posture as [`Self::NextVote`]).
    NextCardName {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
    },
    /// Next seat in a multi-player sacrifice edict — no real choice left → `None` (caller runs
    /// follow-up).
    NextSacrificeEdict {
        remaining: Vec<crate::PlayerId>,
        keep_one: bool,
        filter: crate::PermanentFilter,
        follow_up: &'static [crate::Effect],
        controller: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Priest of Forgotten Gods' "any number of target players" — always pauses.
    ChooseTargetPlayers {
        player: crate::PlayerId,
        source: crate::ObjectId,
        max: u8,
        legal: Vec<crate::PlayerId>,
        min: u8,
        keep_one: bool,
        filter: crate::PermanentFilter,
        life_loss: i32,
        then: &'static [crate::Effect],
    },
    /// Herald dig / cascade / Creative Technique — empty `candidates` → `None` (caller bottoms).
    ChooseExiledDigToCastFree {
        player: crate::PlayerId,
        source: crate::ObjectId,
        candidates: Vec<crate::ObjectId>,
        exiled: Vec<crate::ObjectId>,
    },
    /// Dance with Calamity push-your-luck — always pauses when raised.
    DanceExileMore {
        player: crate::PlayerId,
        source: crate::ObjectId,
        exiled: Vec<crate::ObjectId>,
        total_mv: u32,
        budget: u32,
    },
    /// Shared free-cast over an exile pile — no castable card → `None` (caller routes rest).
    ChooseExiledToCastFree {
        player: crate::PlayerId,
        source: crate::ObjectId,
        exiled: Vec<crate::ObjectId>,
        count: u8,
        rest_to_hand: bool,
    },
    /// Abstract Performance / Fact or Fiction "which opponent splits" — caller handles 0/1
    /// opponents (raise only when `legal.len() > 1`).
    ChooseSplittingOpponent {
        player: crate::PlayerId,
        source: crate::ObjectId,
        legal: Vec<crate::PlayerId>,
        then: crate::SplittingContinuation,
    },
    /// Opponent picks one of two exile piles (Abstract Performance).
    OpponentChoosesPile {
        player: crate::PlayerId,
        controller: crate::PlayerId,
        source: crate::ObjectId,
        pile_a: Vec<crate::ObjectId>,
        pile_b: Vec<crate::ObjectId>,
    },
    /// Opponent partitions revealed cards (Fact or Fiction).
    PartitionRevealed {
        player: crate::PlayerId,
        controller: crate::PlayerId,
        source: crate::ObjectId,
        revealed: Vec<crate::ObjectId>,
    },
    /// Opponent picks one revealed card to graveyard, rest to hand (Murmurs from Beyond).
    OpponentChoosesRevealedToGraveyard {
        player: crate::PlayerId,
        controller: crate::PlayerId,
        source: crate::ObjectId,
        revealed: Vec<crate::ObjectId>,
    },
    /// Controller picks which Fact-or-Fiction pile goes to hand.
    ChoosePileForHand {
        player: crate::PlayerId,
        source: crate::ObjectId,
        pile_a: Vec<crate::ObjectId>,
        pile_b: Vec<crate::ObjectId>,
    },
    /// Plargg and Nassari — empty `nonlands` → `None`.
    OpponentChoosesExiledNonland {
        player: crate::PlayerId,
        controller: crate::PlayerId,
        source: crate::ObjectId,
        nonlands: Vec<crate::ObjectId>,
        exiled: Vec<crate::ObjectId>,
    },
    /// Songbirds' Blessing reveal-until hit — always pauses when raised.
    RevealedCardToBattlefieldOrHand {
        player: crate::PlayerId,
        card: crate::ObjectId,
    },
    /// Deployed Aura/Equipment choose-host — empty candidates → `None`.
    ChooseAttachHost {
        player: crate::PlayerId,
        attachment: crate::ObjectId,
        candidates: Vec<crate::ObjectId>,
        optional: bool,
    },
}

/// Build a [`PendingChoice`] for `request`, or `None` when the raise is a no-op skip.
pub(super) fn choice_from_request(game: &Game, request: ChoiceRequest) -> Option<PendingChoice> {
    if let Some(choice) = common::map_identical(&request) {
        return Some(choice);
    }
    match request {
        ChoiceRequest::Proliferate {
            player,
            source,
            remaining,
        } => optional::proliferate(game, player, source, remaining),
        ChoiceRequest::PhaseOut { player, source } => optional::phase_out(game, player, source),
        ChoiceRequest::MaySacrifice {
            player,
            source,
            filter,
            then,
        } => optional::may_sacrifice(game, player, source, filter, then),
        ChoiceRequest::Devour {
            player,
            source,
            multiplier,
        } => optional::devour(game, player, source, multiplier),
        ChoiceRequest::MayReturnFromGraveyard {
            player,
            source,
            filter,
        } => optional::may_return_from_graveyard(game, player, source, filter),
        ChoiceRequest::MayDiscard {
            player,
            source,
            then,
        } => optional::may_discard(game, player, source, then),
        ChoiceRequest::Discard {
            player,
            count,
            or_one_matching,
        } => optional::discard(game, player, count, or_one_matching),
        ChoiceRequest::PutFromHandOnTop { player, count } => {
            optional::put_from_hand_on_top(game, player, count)
        }
        ChoiceRequest::SacrificeUnlessReturnLand {
            player,
            source,
            filter,
        } => optional::sacrifice_unless_return_land(game, player, source, filter),
        ChoiceRequest::ArrangeTop {
            player,
            count,
            to_graveyard,
        } => library::arrange_top(game, player, count, to_graveyard),
        ChoiceRequest::SelectFromTop {
            player,
            count,
            filter,
            up_to,
            min,
            dest,
            dest_tapped,
            rest,
            mv_budget,
        } => library::select_from_top(
            game,
            player,
            count,
            filter,
            up_to,
            min,
            dest,
            dest_tapped,
            rest,
            mv_budget,
        ),
        ChoiceRequest::DistributeTop {
            player,
            count,
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => library::distribute_top(game, player, count, to_hand, to_bottom, to_exile_may_play),
        ChoiceRequest::ShuffleFromGraveyard {
            answerer,
            owner,
            source,
            max,
        } => library::shuffle_from_graveyard(game, answerer, owner, source, max),
        ChoiceRequest::SearchLibrary {
            player,
            filter,
            dest,
            tapped,
            count,
            overflow,
        } => library::search_library(game, player, filter, dest, tapped, count, overflow),
        ChoiceRequest::PutLandFromHand { player, tapped } => {
            library::put_land_from_hand(game, player, tapped)
        }
        ChoiceRequest::PutCreatureFromHand { player, source } => {
            library::put_creature_from_hand(game, player, source)
        }
        ChoiceRequest::CastCreatureFaceDown { player, spent_mana } => {
            library::cast_creature_face_down(game, player, spent_mana)
        }
        ChoiceRequest::ChooseExiledWithCard { player, source } => {
            copy::choose_exiled_with_card(game, player, source)
        }
        ChoiceRequest::ChooseExiledWithCardToCast { player, source } => {
            copy::choose_exiled_with_card_to_cast(game, player, source)
        }
        ChoiceRequest::EnterAsCopy {
            player,
            source,
            marker,
        } => copy::enter_as_copy(game, player, source, marker),
        ChoiceRequest::ChooseTokenToCopy { player, source } => {
            copy::choose_token_to_copy(game, player, source)
        }
        ChoiceRequest::ChooseCopyCardFromList {
            player,
            source,
            cards,
        } => copy::choose_copy_card_from_list(game, player, source, cards),
        ChoiceRequest::ChooseOwnSacrifices {
            player,
            source,
            filter,
            count,
        } => edict::choose_own_sacrifices(game, player, source, filter, count),
        ChoiceRequest::NextGraveyardExile { remaining, source } => {
            fanout::next_graveyard_exile(game, remaining, source)
        }
        ChoiceRequest::NextCasterKeep {
            remaining,
            caster,
            source,
        } => fanout::next_caster_keep(game, remaining, caster, source),
        ChoiceRequest::NextCounterTarget {
            remaining,
            chooser,
            source,
        } => fanout::next_counter_target(game, remaining, chooser, source),
        ChoiceRequest::NextJoinForcesPayment { remaining, source } => {
            fanout::next_join_forces_payment(remaining, source)
        }
        ChoiceRequest::NextVote {
            remaining,
            source,
            options,
        } => fanout::next_vote(remaining, source, options),
        ChoiceRequest::NextCardName { remaining, source } => {
            fanout::next_card_name(remaining, source)
        }
        ChoiceRequest::NextSacrificeEdict {
            remaining,
            keep_one,
            filter,
            follow_up,
            controller,
            source,
        } => fanout::next_sacrifice_edict(
            game, remaining, keep_one, filter, follow_up, controller, source,
        ),
        ChoiceRequest::ChooseExiledDigToCastFree {
            player,
            source,
            candidates,
            exiled,
        } => dig::choose_exiled_dig_to_cast_free(player, source, candidates, exiled),
        ChoiceRequest::ChooseExiledToCastFree {
            player,
            source,
            exiled,
            count,
            rest_to_hand,
        } => dig::choose_exiled_to_cast_free(game, player, source, exiled, count, rest_to_hand),
        ChoiceRequest::ChooseSplittingOpponent {
            player,
            source,
            legal,
            then,
        } => dig::choose_splitting_opponent(player, source, legal, then),
        ChoiceRequest::OpponentChoosesExiledNonland {
            player,
            controller,
            source,
            nonlands,
            exiled,
        } => dig::opponent_chooses_exiled_nonland(player, controller, source, nonlands, exiled),
        ChoiceRequest::ChooseAttachHost {
            player,
            attachment,
            candidates,
            optional,
        } => dig::choose_attach_host(player, attachment, candidates, optional),
        // Identity variants are handled by [`common::map_identical`] above.
        ChoiceRequest::ChooseTarget { .. }
        | ChoiceRequest::PayOrCounter { .. }
        | ChoiceRequest::ChooseCreatureType { .. }
        | ChoiceRequest::ChooseColor { .. }
        | ChoiceRequest::ChooseMode { .. }
        | ChoiceRequest::MayYesNo { .. }
        | ChoiceRequest::DivideSpellDamage { .. }
        | ChoiceRequest::DivideCounters { .. }
        | ChoiceRequest::ChooseManaColor { .. }
        | ChoiceRequest::SacrificeUnlessPay { .. }
        | ChoiceRequest::ChooseTargetPlayers { .. }
        | ChoiceRequest::DanceExileMore { .. }
        | ChoiceRequest::OpponentChoosesPile { .. }
        | ChoiceRequest::PartitionRevealed { .. }
        | ChoiceRequest::OpponentChoosesRevealedToGraveyard { .. }
        | ChoiceRequest::ChoosePileForHand { .. }
        | ChoiceRequest::RevealedCardToBattlefieldOrHand { .. } => {
            unreachable!("identity ChoiceRequest variants handled by map_identical")
        }
    }
}
