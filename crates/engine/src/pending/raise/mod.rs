//! [`ChoiceRequest`] and `PendingChoice` construction for [`super::raise`].
//!
//! Dig-loop / multi-step effect kickoffs (cascade, reveal-until, dance, edict prep, …) still
//! live as non-`begin_*` helpers on [`crate::Game`] that emit dig events then raise — variants
//! here are pause payloads, not pure constructors for those flows (prep mutates via events
//! before the pause).

use std::sync::Arc;

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
// ponytail: every variant here carries an `Effect` (~6.3KB), so the spread the lint measures is
// noise against a floor set by `Effect` itself — same call as `Event`/`StackItem`.
#[allow(clippy::large_enum_variant)]
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
        strips_mana_on_decline: bool,
    },
    ChooseCreatureType {
        player: crate::PlayerId,
        source: crate::ObjectId,
        options: &'static [&'static str],
        /// Magical Hack / Sleight of Mind's second question (CR 612.1): what an answer to this
        /// pick should do next, rather than write a chosen creature type. `None` for every
        /// ordinary as-enters "choose a creature type".
        then: Option<crate::TextSwapPick>,
    },
    ChooseColor {
        player: crate::PlayerId,
        source: crate::ObjectId,
        use_: crate::ChosenColorUse,
    },
    ChooseMode {
        player: crate::PlayerId,
        source: crate::ObjectId,
        target: Option<crate::Target>,
        x: u32,
        modes: Arc<[crate::Effect]>,
        at_placement: bool,
        /// Whether the mode is being chosen for an *activated* ability as it goes on the stack
        /// (CR 601.2b — Cankerbloom). See [`crate::PendingChoice::ChooseMode`].
        activated: bool,
    },
    MayYesNo {
        player: crate::PlayerId,
        source: crate::ObjectId,
        effect: crate::Effect,
        /// [`crate::MayYesNoResume::Default`] for ability-level optional triggers;
        /// [`crate::MayYesNoResume::ResolveInline`] for mid-resolution may-search.
        resume: crate::MayYesNoResume,
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
    /// [`Effect::Choice(ChoiceEffect::MaySacrifice)`] — fewer than `count` legal permanents skips
    /// (the `otherwise` penalty is run by the caller before the raise, see `Game::run_effect`).
    MaySacrifice {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
        count: u8,
        then: &'static [crate::Effect],
        otherwise: &'static [crate::Effect],
    },
    /// [`Effect::Choice(ChoiceEffect::SacrificeAnyNumber)`](crate::ChoiceEffect::SacrificeAnyNumber)
    /// as-enters (Wood Elemental) — no matching permanent skips.
    SacrificeAnyNumber {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
    },
    /// [`CardDef::devour`] as-enters — no other creature skips.
    Devour {
        player: crate::PlayerId,
        source: crate::ObjectId,
        multiplier: u32,
    },
    /// [`Effect::Choice(ChoiceEffect::MayReturnFromGraveyard)`] — no legal card in any listed
    /// graveyard skips. `graveyards` is the queue of owners still owed a card: one entry for the
    /// ordinary single return, the same owner repeated for Recall's counted return, distinct
    /// owners for Glyph of Reincarnation's fan-out.
    MayReturnFromGraveyard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::CardFilter,
        mandatory: bool,
        to_battlefield: bool,
        graveyards: Vec<crate::PlayerId>,
    },
    /// [`Effect::Choice(ChoiceEffect::MayExileDiscardedNonlandMayPlay)`] — no card still in the
    /// graveyard skips (Conspiracy Theorist).
    MayExileDiscardedNonlandMayPlay {
        player: crate::PlayerId,
        source: crate::ObjectId,
        cards: &'static [crate::ObjectId],
    },
    /// [`Effect::Choice(ChoiceEffect::MayDiscard)`] — empty hand skips.
    MayDiscard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        then: &'static [crate::Effect],
    },
    /// [`Effect::Choice(ChoiceEffect::MayPutCounterOnCreature)`] — no battlefield creature skips.
    MayPutCounterOnCreature {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::Choice(ChoiceEffect::MayBlockAttackerOfYourChoice)`] — no attacker `blocker`
    /// could legally block skips.
    ChooseBlockTarget {
        player: crate::PlayerId,
        source: crate::ObjectId,
        blocker: crate::ObjectId,
    },
    /// [`Effect::Choice(ChoiceEffect::Discard)`] — empty (or zero-count) hand skips.
    Discard {
        player: crate::PlayerId,
        count: u32,
        or_one_matching: Option<crate::CardFilter>,
    },
    /// [`Effect::Choice(ChoiceEffect::PutFromHandOnTop)`] — empty (or zero-count) hand skips.
    PutFromHandOnTop {
        player: crate::PlayerId,
        count: u32,
        drawn_this_turn: bool,
        life_per_declined: u32,
    },
    /// [`Effect::Choice(ChoiceEffect::PayOrElse)`] — always pauses.
    PayOrElse {
        player: crate::PlayerId,
        source: crate::ObjectId,
        cost: crate::Cost,
        then: &'static [crate::Effect],
        otherwise: &'static [crate::Effect],
    },
    /// [`Effect::Choice(ChoiceEffect::SacrificeSelfUnlessReturnLand)`] — no candidates → `None` (caller sacrifices).
    SacrificeUnlessReturnLand {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
    },
    /// [`Effect::Dig(DigEffect::Scry)`] / [`Effect::Dig(DigEffect::Surveil)`] /
    /// [`Effect::Dig(DigEffect::RearrangeTargetPlayersTop)`] — empty library skips.
    ArrangeTop {
        player: crate::PlayerId,
        library: crate::PlayerId,
        count: u32,
        rest: crate::ArrangeRest,
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
    /// [`Effect::Choice(ChoiceEffect::PutCreatureFromHand)`] — no eligible hand creature skips.
    PutCreatureFromHand {
        player: crate::PlayerId,
        source: crate::ObjectId,
        subtypes: &'static [&'static str],
        keep: bool,
        defender: Option<crate::PlayerId>,
        /// Eureka's round-robin queue — see [`PendingChoice::PutCreatureFromHand`](crate::PendingChoice).
        round: Option<Vec<crate::PlayerId>>,
        permanent_cards: bool,
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
    /// sacrifices all). `player` answers; `owner` is whose battlefield the options come off, the
    /// same seat everywhere but Demonic Hordes' "a land of an opponent's choice".
    ChooseOwnSacrifices {
        player: crate::PlayerId,
        owner: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
        count: u32,
    },
    /// Next seat in a graveyard-exile fan-out (Augusta / Relic) — empty remaining skips.
    NextGraveyardExile {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
    },
    /// Next opponent in a discard fan-out (Syphon Mind) — empty-hand seats skipped.
    NextDiscardEdict {
        /// Balance's "down to the fewest": each seat discards its own excess over this hand size.
        /// `None` is the plain one-card-each fan-out (Syphon Mind).
        floor: Option<u32>,
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
    },
    /// Next seat in Tragic Arrogance's caster-keep fan-out — empty remaining skips.
    NextCasterKeep {
        remaining: Vec<crate::PlayerId>,
        caster: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Next graveyard in Glyph of Reincarnation's "for each creature that died this way"
    /// fan-out — a graveyard with no creature card is skipped, and an empty list skips entirely.
    /// Next seat in Nils' counter-target fan-out — empty remaining skips.
    NextCounterTarget {
        remaining: Vec<crate::PlayerId>,
        chooser: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// Next seat in a join-forces payment round — empty remaining skips. `prevent_up_to` is
    /// Power Leak's single-seat variant: `Some(cap)` turns what the seat pays into a prevention
    /// shield on that seat worth at most `cap`, instead of into the round's shared `X`.
    NextJoinForcesPayment {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
        prevent_up_to: Option<u8>,
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
        /// What the answered name is for — see [`crate::CardNameUse`]. Conundrum Sphinx's fan-out
        /// and Petra Sphinx's single seat differ only in this tail.
        use_: crate::CardNameUse,
    },
    /// Next seat in a multi-player sacrifice edict — no real choice left → `None` (caller runs
    /// follow-up).
    NextSacrificeEdict {
        remaining: Vec<crate::PlayerId>,
        keep_one: bool,
        filter: crate::PermanentFilter,
        count: u32,
        /// Balance's "down to the fewest": each seat sacrifices its own excess over this many
        /// matching permanents, overriding `count`. `None` is the ordinary fixed-count edict.
        floor: Option<u32>,
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
        count: u32,
        then: &'static [crate::Effect],
    },
    /// Herald dig / cascade / Creative Technique — empty `candidates` → `None` (caller bottoms).
    ChooseExiledDigToCastFree {
        player: crate::PlayerId,
        source: crate::ObjectId,
        candidates: Vec<crate::ObjectId>,
        exiled: Vec<crate::ObjectId>,
    },
    /// Raging River — the next defender in `defenders` divides their non-flying creatures. With
    /// `defenders` exhausted this rolls straight on to labeling `attackers`, and with nothing left
    /// to label either it yields `None`.
    SplitBlockersIntoPiles {
        source: crate::ObjectId,
        left: Vec<(crate::PlayerId, Vec<crate::ObjectId>)>,
        defenders: Vec<crate::PlayerId>,
        attackers: Vec<crate::ObjectId>,
    },
    /// Camouflage — `partial` is the defender midway through their division (what they have left
    /// to divide, and the piles they have named); `None` starts the next of `defenders` from the
    /// top. With `defenders` exhausted there is nothing left to ask, and this yields `None`.
    DivideBlockersIntoPiles {
        source: crate::ObjectId,
        partial: Option<(
            crate::PlayerId,
            Vec<crate::ObjectId>,
            Vec<Vec<crate::ObjectId>>,
        )>,
        defenders: Vec<crate::PlayerId>,
        attackers: Vec<crate::ObjectId>,
    },
    /// Word of Command — `player` picks from `subject`'s hand; an empty hand → `None`.
    ChooseCardInHandToPlay {
        player: crate::PlayerId,
        source: crate::ObjectId,
        subject: crate::PlayerId,
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
            count,
            then,
            otherwise,
        } => optional::may_sacrifice(game, player, source, filter, count, then, otherwise),
        ChoiceRequest::SacrificeAnyNumber {
            player,
            source,
            filter,
        } => optional::sacrifice_any_number(game, player, source, filter),
        ChoiceRequest::Devour {
            player,
            source,
            multiplier,
        } => optional::devour(game, player, source, multiplier),
        ChoiceRequest::MayReturnFromGraveyard {
            player,
            source,
            filter,
            mandatory,
            to_battlefield,
            graveyards,
        } => optional::may_return_from_graveyard(
            game,
            player,
            source,
            filter,
            mandatory,
            to_battlefield,
            graveyards,
        ),
        ChoiceRequest::MayExileDiscardedNonlandMayPlay {
            player,
            source,
            cards,
        } => optional::may_exile_discarded_nonland_may_play(game, player, source, cards),
        ChoiceRequest::MayDiscard {
            player,
            source,
            then,
        } => optional::may_discard(game, player, source, then),
        ChoiceRequest::MayPutCounterOnCreature { player, source } => {
            optional::may_put_counter_on_creature(game, player, source)
        }
        ChoiceRequest::ChooseBlockTarget {
            player,
            source,
            blocker,
        } => optional::choose_block_target(game, player, source, blocker),
        ChoiceRequest::Discard {
            player,
            count,
            or_one_matching,
        } => optional::discard(game, player, count, or_one_matching),
        ChoiceRequest::PutFromHandOnTop {
            player,
            count,
            drawn_this_turn,
            life_per_declined,
        } => {
            optional::put_from_hand_on_top(game, player, count, drawn_this_turn, life_per_declined)
        }
        ChoiceRequest::SacrificeUnlessReturnLand {
            player,
            source,
            filter,
        } => optional::sacrifice_unless_return_land(game, player, source, filter),
        ChoiceRequest::ArrangeTop {
            player,
            library,
            count,
            rest,
        } => library::arrange_top(game, player, library, count, rest),
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
        ChoiceRequest::PutCreatureFromHand {
            player,
            source,
            subtypes,
            keep,
            defender,
            round,
            permanent_cards,
        } => library::put_creature_from_hand(
            game,
            player,
            source,
            subtypes,
            keep,
            defender,
            round,
            permanent_cards,
        ),
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
            owner,
            source,
            filter,
            count,
        } => edict::choose_own_sacrifices(game, player, owner, source, filter, count),
        ChoiceRequest::NextGraveyardExile { remaining, source } => {
            fanout::next_graveyard_exile(game, remaining, source)
        }
        ChoiceRequest::NextDiscardEdict {
            remaining,
            source,
            floor,
        } => fanout::next_discard_edict(game, remaining, source, floor),
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
        ChoiceRequest::NextJoinForcesPayment {
            remaining,
            source,
            prevent_up_to,
        } => fanout::next_join_forces_payment(remaining, source, prevent_up_to),
        ChoiceRequest::NextVote {
            remaining,
            source,
            options,
        } => fanout::next_vote(remaining, source, options),
        ChoiceRequest::NextCardName {
            remaining,
            source,
            use_,
        } => fanout::next_card_name(remaining, source, use_),
        ChoiceRequest::NextSacrificeEdict {
            remaining,
            keep_one,
            filter,
            count,
            floor,
            follow_up,
            controller,
            source,
        } => fanout::next_sacrifice_edict(
            game, remaining, keep_one, filter, count, floor, follow_up, controller, source,
        ),
        ChoiceRequest::ChooseExiledDigToCastFree {
            player,
            source,
            candidates,
            exiled,
        } => dig::choose_exiled_dig_to_cast_free(player, source, candidates, exiled),
        ChoiceRequest::ChooseCardInHandToPlay {
            player,
            source,
            subject,
        } => dig::choose_card_in_hand_to_play(game, player, source, subject),
        ChoiceRequest::SplitBlockersIntoPiles {
            source,
            left,
            defenders,
            attackers,
        } => dig::split_blockers_into_piles(game, source, left, defenders, attackers),
        ChoiceRequest::DivideBlockersIntoPiles {
            source,
            partial,
            defenders,
            attackers,
        } => dig::divide_blockers_into_piles(game, source, partial, defenders, attackers),
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
        | ChoiceRequest::PayOrElse { .. }
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
