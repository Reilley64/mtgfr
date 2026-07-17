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
//! Handlers and dig-loop kickoff helpers live in [`handlers`]. `pause_for` is private to this
//! module so other engine modules must not poke `PendingChoice` raw — use [`raise`] /
//! [`raise_choice`] instead.
//!
//! ## Deferred (next increments)
//! - Optional internal `ChoiceHandler` per kind family (locality for new kinds).
//! - Dig-loop / multi-step effect kickoffs (cascade, reveal-until, dance, edict prep, …) still
//!   live as non-`begin_*` helpers on [`Game`] that emit dig events then [`raise`] — they are
//!   not pure `ChoiceRequest` constructors because prep mutates via events before the pause.

mod handlers;

use crate::{Event, Game, Intent, PendingChoice, Reject};

/// Engine-internal raise request (not wire). Covers effect/cast pause sites, fan-out kickoffs,
/// and dig-loop pause payloads (prep/dig events stay at the call site — see module deferred notes).
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
    /// [`Effect::Proliferate`] — empty counter-bearing board skips (no pause).
    Proliferate {
        player: crate::PlayerId,
        source: crate::ObjectId,
        /// Iterations still to run, including this one (`0` is a no-op).
        remaining: u8,
    },
    /// [`Effect::PhaseOut`] — no other creatures skips.
    PhaseOut {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::MaySacrifice`] — no legal permanent skips.
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
    /// [`Effect::MayReturnFromGraveyard`] — no legal card skips.
    MayReturnFromGraveyard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::CardFilter,
    },
    /// [`Effect::MayDiscard`] — empty hand skips.
    MayDiscard {
        player: crate::PlayerId,
        source: crate::ObjectId,
        then: &'static [crate::Effect],
    },
    /// [`Effect::Discard`] — empty (or zero-count) hand skips.
    Discard {
        player: crate::PlayerId,
        count: u32,
        or_one_matching: Option<crate::CardFilter>,
    },
    /// [`Effect::SacrificeSelfUnlessPay`] — always pauses.
    SacrificeUnlessPay {
        player: crate::PlayerId,
        source: crate::ObjectId,
        cost: crate::Cost,
    },
    /// [`Effect::SacrificeSelfUnlessReturnLand`] — no candidates → `None` (caller sacrifices).
    SacrificeUnlessReturnLand {
        player: crate::PlayerId,
        source: crate::ObjectId,
        filter: crate::PermanentFilter,
    },
    /// [`Effect::Scry`] / [`Effect::Surveil`] — empty library skips.
    ArrangeTop {
        player: crate::PlayerId,
        count: u32,
        to_graveyard: bool,
    },
    /// [`Effect::LookAtTop`] — empty library skips.
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
    /// [`Effect::DistributeTop`] — empty library skips.
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
    /// [`Effect::SearchLibrary`] — always pauses (fail-to-find is a legal answer).
    SearchLibrary {
        player: crate::PlayerId,
        filter: crate::CardFilter,
        dest: crate::SearchDest,
        tapped: bool,
        count: u8,
        overflow: Option<crate::SearchDest>,
    },
    /// [`Effect::PutLandFromHand`] — no hand land skips.
    PutLandFromHand {
        player: crate::PlayerId,
        tapped: bool,
    },
    /// [`Effect::CastCreatureFaceDown`] — no payable creature skips.
    CastCreatureFaceDown {
        player: crate::PlayerId,
        spent_mana: [u8; 6],
    },
    /// [`Effect::CashOutExiledWithThis`] — empty exile pile skips.
    ChooseExiledWithCard {
        player: crate::PlayerId,
        source: crate::ObjectId,
    },
    /// [`Effect::CastExiledWithThisFree`] — empty exile pile skips.
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
    /// [`Effect::EachOtherTokenBecomesCopyOfChosen`] — no token skips.
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
    /// [`Effect::SacrificeOwn`] / annihilator — `options.len() <= count` → `None` (caller
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
    /// Next seat in a council's-dilemma vote — empty remaining skips.
    NextVote {
        remaining: Vec<crate::PlayerId>,
        source: crate::ObjectId,
        options: &'static [&'static str],
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

/// Raise a Choice from resolution (or cast). Constructs [`PendingChoice`] and pauses.
/// Some variants skip when there is nothing to choose (empty board / hand).
pub(crate) fn raise(game: &mut Game, request: ChoiceRequest) {
    let Some(choice) = choice_from_request(game, request) else {
        return;
    };
    game.pause_for(choice);
}

/// Build a [`PendingChoice`] for `request`, or `None` when the raise is a no-op skip.
fn choice_from_request(game: &Game, request: ChoiceRequest) -> Option<PendingChoice> {
    match request {
        ChoiceRequest::ChooseTarget {
            player,
            source,
            effect,
            legal,
            optional,
        } => Some(PendingChoice::ChooseTarget {
            player,
            source,
            effect,
            legal,
            optional,
        }),
        ChoiceRequest::PayOrCounter {
            player,
            cost,
            spell,
        } => Some(PendingChoice::PayOrCounter {
            player,
            cost,
            spell,
        }),
        ChoiceRequest::ChooseCreatureType {
            player,
            source,
            options,
        } => Some(PendingChoice::ChooseCreatureType {
            player,
            source,
            options,
        }),
        ChoiceRequest::ChooseColor { player, source } => {
            Some(PendingChoice::ChooseColor { player, source })
        }
        ChoiceRequest::ChooseMode {
            player,
            source,
            target,
            x,
            modes,
        } => Some(PendingChoice::ChooseMode {
            player,
            source,
            target,
            x,
            modes,
        }),
        ChoiceRequest::MayYesNo {
            player,
            source,
            effect,
        } => Some(PendingChoice::MayYesNo {
            player,
            source,
            effect,
        }),
        ChoiceRequest::DivideSpellDamage {
            player,
            spell,
            targets,
            total,
        } => Some(PendingChoice::DivideSpellDamage {
            player,
            spell,
            targets,
            total,
        }),
        ChoiceRequest::DivideCounters {
            player,
            spell,
            targets,
            total,
        } => Some(PendingChoice::DivideCounters {
            player,
            spell,
            targets,
            total,
        }),
        ChoiceRequest::ChooseManaColor {
            player,
            source,
            amount,
        } => Some(PendingChoice::ChooseManaColor {
            player,
            source,
            amount,
        }),
        ChoiceRequest::Proliferate {
            player,
            source,
            remaining,
        } => {
            if remaining == 0 {
                return None;
            }
            let options: Vec<crate::ObjectId> = game
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    let p = game.permanent(id);
                    p.plus_counters > 0 || p.kind_counters.iter().any(|&c| c > 0)
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            Some(PendingChoice::Proliferate {
                player,
                source,
                options,
                remaining: remaining - 1,
            })
        }
        ChoiceRequest::PhaseOut { player, source } => {
            let options: Vec<crate::ObjectId> = game
                .battlefield()
                .into_iter()
                .filter(|&id| {
                    id != source
                        && game.controller_of(id) == player
                        && matches!(game.def_of(id).kind, crate::CardKind::Creature { .. })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            Some(PendingChoice::PhaseOut {
                player,
                source,
                options,
            })
        }
        ChoiceRequest::MaySacrifice {
            player,
            source,
            filter,
            then,
        } => {
            let options = game.edict_options(player, filter);
            if options.is_empty() {
                return None;
            }
            Some(PendingChoice::MaySacrifice {
                player,
                source,
                options,
                then,
            })
        }
        ChoiceRequest::Devour {
            player,
            source,
            multiplier,
        } => {
            let options: Vec<crate::ObjectId> = game
                .edict_options(player, crate::PermanentFilter::of(crate::TypeSet::CREATURE))
                .into_iter()
                .filter(|&id| id != source)
                .collect();
            if options.is_empty() {
                return None;
            }
            Some(PendingChoice::Devour {
                player,
                source,
                multiplier,
                options,
            })
        }
        ChoiceRequest::MayReturnFromGraveyard {
            player,
            source,
            filter,
        } => {
            let options: Vec<crate::ObjectId> = game
                .live_object_ids()
                .into_iter()
                .filter(|&id| {
                    game.zone_of(id) == crate::Zone::Graveyard
                        && game.owner_of(id) == player
                        && filter.matches(game.def_of(id))
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            Some(PendingChoice::MayReturnFromGraveyard {
                player,
                source,
                options,
            })
        }
        ChoiceRequest::MayDiscard {
            player,
            source,
            then,
        } => {
            let hand = game.hand_of(player);
            if hand.is_empty() {
                return None;
            }
            Some(PendingChoice::MayDiscard {
                player,
                source,
                options: hand,
                then,
            })
        }
        ChoiceRequest::Discard {
            player,
            count,
            or_one_matching,
        } => {
            let hand = game.hand_of(player);
            let count = (count as usize).min(hand.len());
            if count == 0 {
                return None;
            }
            Some(PendingChoice::DiscardCards {
                player,
                hand,
                count,
                or_one_matching,
            })
        }
        ChoiceRequest::SacrificeUnlessPay {
            player,
            source,
            cost,
        } => Some(PendingChoice::SacrificeUnlessPay {
            player,
            source,
            cost,
        }),
        ChoiceRequest::SacrificeUnlessReturnLand {
            player,
            source,
            filter,
        } => {
            let candidates = game.edict_options(player, filter);
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::SacrificeUnlessReturnLand {
                player,
                source,
                candidates,
            })
        }
        ChoiceRequest::ArrangeTop {
            player,
            count,
            to_graveyard,
        } => {
            let library = &game.players[player.0 as usize].library;
            let cards: Vec<crate::ObjectId> =
                library.iter().take(count as usize).copied().collect();
            if cards.is_empty() {
                return None;
            }
            Some(PendingChoice::ArrangeTop {
                player,
                cards,
                to_graveyard,
            })
        }
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
        } => {
            let library = &game.players[player.0 as usize].library;
            let cards: Vec<crate::ObjectId> =
                library.iter().take(count as usize).copied().collect();
            if cards.is_empty() {
                return None;
            }
            Some(PendingChoice::SelectFromTop {
                player,
                cards,
                filter,
                up_to,
                min,
                dest,
                dest_tapped,
                rest,
                mv_budget,
            })
        }
        ChoiceRequest::DistributeTop {
            player,
            count,
            to_hand,
            to_bottom,
            to_exile_may_play,
        } => {
            let library = &game.players[player.0 as usize].library;
            let cards: Vec<crate::ObjectId> =
                library.iter().take(count as usize).copied().collect();
            if cards.is_empty() {
                return None;
            }
            // ponytail: no pool card yet distributes into a library shorter than its total slots;
            // if (CR 400.3) one ever does, slots are filled hand→bottom→exile in priority order and
            // any excess slot (CR 117, CR 406.5, CR 402.5) is silently dropped (CR 120.3-style "as
            // many as possible" with no printed tie-break).
            let mut looked_at = cards.len() as u32;
            let to_hand = to_hand.min(looked_at);
            looked_at -= to_hand;
            let to_bottom = to_bottom.min(looked_at);
            looked_at -= to_bottom;
            let to_exile_may_play = to_exile_may_play.min(looked_at);
            Some(PendingChoice::DistributeTop {
                player,
                cards,
                to_hand,
                to_bottom,
                to_exile_may_play,
            })
        }
        ChoiceRequest::ShuffleFromGraveyard {
            answerer,
            owner,
            source,
            max,
        } => {
            let candidates = game.graveyard_of(owner);
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ShuffleFromGraveyard {
                player: answerer,
                owner,
                source,
                candidates,
                max,
            })
        }
        ChoiceRequest::SearchLibrary {
            player,
            filter,
            dest,
            tapped,
            count,
            overflow,
        } => {
            let matches: Vec<crate::ObjectId> = game.players[player.0 as usize]
                .library
                .iter()
                .copied()
                .filter(|&id| filter.matches(game.def_of(id)))
                .collect();
            Some(PendingChoice::SearchLibrary {
                player,
                matches,
                dest,
                tapped,
                remaining: count,
                overflow,
            })
        }
        ChoiceRequest::PutLandFromHand { player, tapped } => {
            let candidates: Vec<crate::ObjectId> = game
                .hand_of(player)
                .into_iter()
                .filter(|&id| matches!(game.def_of(id).kind, crate::CardKind::Land { .. }))
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::PutLandFromHand {
                player,
                tapped,
                candidates,
            })
        }
        ChoiceRequest::CastCreatureFaceDown { player, spent_mana } => {
            let candidates: Vec<crate::ObjectId> = game
                .hand_of(player)
                .into_iter()
                .filter(|&id| matches!(game.def_of(id).kind, crate::CardKind::Creature { .. }))
                .filter(|&id| game.def_of(id).cost.payable_from_multiset(&spent_mana))
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::CastCreatureFaceDown { player, candidates })
        }
        ChoiceRequest::ChooseExiledWithCard { player, source } => {
            let candidates: Vec<crate::ObjectId> = game
                .exile_links
                .with_source
                .iter()
                .filter(|&&(s, _)| s == source)
                .map(|&(_, card)| card)
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseExiledWithCard {
                player,
                source,
                candidates,
            })
        }
        ChoiceRequest::ChooseExiledWithCardToCast { player, source } => {
            let candidates: Vec<crate::ObjectId> = game
                .exile_links
                .with_source
                .iter()
                .filter(|&&(s, _)| s == source)
                .map(|&(_, card)| card)
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseExiledWithCardToCast {
                player,
                source,
                candidates,
            })
        }
        ChoiceRequest::EnterAsCopy {
            player,
            source,
            marker,
        } => {
            let candidates: Vec<crate::ObjectId> = game
                .permanent_ids(|_| true)
                .collect::<Vec<_>>()
                .into_iter()
                .filter(|&id| {
                    id != source
                        && match marker.of {
                            crate::CopyTargetKind::Creature => game.is_creature_on_battlefield(id),
                            crate::CopyTargetKind::Enchantment => {
                                game.is_enchantment_on_battlefield(id)
                            }
                        }
                })
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseCopyTarget {
                player,
                source,
                candidates,
                until_eot: marker.until_eot,
                extra_counters: marker.extra_counters,
                gains_haste: marker.gains_haste,
            })
        }
        ChoiceRequest::ChooseTokenToCopy { player, source } => {
            let candidates: Vec<crate::ObjectId> = game
                .permanent_ids(|p| p.token)
                .collect::<Vec<_>>()
                .into_iter()
                .filter(|&id| game.controller_of(id) == player)
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseTokenToCopy {
                player,
                source,
                candidates,
            })
        }
        ChoiceRequest::ChooseCopyCardFromList {
            player,
            source,
            cards,
        } => {
            let candidates: Vec<crate::ObjectId> = cards
                .iter()
                .copied()
                .filter(|&id| {
                    game.def_of(id)
                        .kind
                        .types()
                        .intersects(crate::TypeSet::CREATURE.union(crate::TypeSet::ARTIFACT))
                })
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseCopyCardFromList {
                player,
                source,
                candidates,
            })
        }
        ChoiceRequest::ChooseOwnSacrifices {
            player,
            source,
            filter,
            count,
        } => {
            let options = game.edict_options(player, filter);
            if options.len() <= count as usize {
                return None;
            }
            Some(PendingChoice::ChooseOwnSacrifices {
                player,
                source,
                filter,
                count,
                options,
            })
        }
        ChoiceRequest::NextGraveyardExile { remaining, source } => {
            let mut remaining = remaining;
            while !remaining.is_empty() {
                let player = remaining.remove(0);
                let options = game.graveyard_cards(player);
                if options.is_empty() {
                    continue;
                }
                return Some(PendingChoice::ExileFromGraveyard {
                    player,
                    source,
                    options,
                    remaining,
                });
            }
            None
        }
        ChoiceRequest::NextCasterKeep {
            remaining,
            caster,
            source,
        } => {
            let mut remaining = remaining;
            while !remaining.is_empty() {
                let target_player = remaining.remove(0);
                let options = game.edict_options(
                    target_player,
                    crate::PermanentFilter::of(crate::TypeSet::NONLAND),
                );
                if options.is_empty() {
                    continue;
                }
                return Some(PendingChoice::CasterKeepPermanents {
                    caster,
                    source,
                    target_player,
                    options,
                    remaining,
                });
            }
            None
        }
        ChoiceRequest::NextCounterTarget {
            remaining,
            chooser,
            source,
        } => {
            let mut remaining = remaining;
            while !remaining.is_empty() {
                let target_player = remaining.remove(0);
                let options: Vec<crate::ObjectId> = game
                    .controlled_battlefield(target_player)
                    .into_iter()
                    .filter(|&id| game.is_creature_on_battlefield(id))
                    .collect();
                if options.is_empty() {
                    continue;
                }
                return Some(PendingChoice::ChooseCounterTargetForPlayer {
                    chooser,
                    source,
                    target_player,
                    options,
                    remaining,
                });
            }
            None
        }
        ChoiceRequest::NextVote {
            remaining,
            source,
            options,
        } => {
            if remaining.is_empty() {
                return None;
            }
            let mut remaining = remaining;
            let player = remaining.remove(0);
            Some(PendingChoice::CastVote {
                player,
                source,
                options,
                remaining,
            })
        }
        ChoiceRequest::NextSacrificeEdict {
            remaining,
            keep_one,
            filter,
            follow_up,
            controller,
            source,
        } => {
            let mut remaining = remaining;
            while !remaining.is_empty() {
                let player = remaining.remove(0);
                let options = game.edict_options(player, filter);
                if options.is_empty() || (keep_one && options.len() == 1) {
                    continue;
                }
                return Some(PendingChoice::SacrificeEdict {
                    player,
                    options,
                    keep_one,
                    filter,
                    remaining,
                    controller,
                    source,
                    follow_up,
                });
            }
            None
        }
        ChoiceRequest::ChooseTargetPlayers {
            player,
            source,
            max,
            legal,
            min,
            keep_one,
            filter,
            life_loss,
            then,
        } => Some(PendingChoice::ChooseTargetPlayers {
            player,
            source,
            max,
            legal,
            min,
            keep_one,
            filter,
            life_loss,
            then,
        }),
        ChoiceRequest::ChooseExiledDigToCastFree {
            player,
            source,
            candidates,
            exiled,
        } => {
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseExiledDigToCastFree {
                player,
                source,
                candidates,
                exiled,
            })
        }
        ChoiceRequest::DanceExileMore {
            player,
            source,
            exiled,
            total_mv,
            budget,
        } => Some(PendingChoice::DanceExileMore {
            player,
            source,
            exiled,
            total_mv,
            budget,
        }),
        ChoiceRequest::ChooseExiledToCastFree {
            player,
            source,
            exiled,
            count,
            rest_to_hand,
        } => {
            let candidates: Vec<crate::ObjectId> = exiled
                .iter()
                .copied()
                .filter(|&id| !matches!(game.def_of(id).kind, crate::CardKind::Land { .. }))
                .collect();
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseExiledToCastFree {
                player,
                source,
                candidates,
                exiled,
                count,
                rest_to_hand,
            })
        }
        ChoiceRequest::ChooseSplittingOpponent {
            player,
            source,
            legal,
            then,
        } => {
            if legal.len() <= 1 {
                return None;
            }
            Some(PendingChoice::ChooseSplittingOpponent {
                player,
                source,
                legal,
                then,
            })
        }
        ChoiceRequest::OpponentChoosesPile {
            player,
            controller,
            source,
            pile_a,
            pile_b,
        } => Some(PendingChoice::OpponentChoosesPile {
            player,
            controller,
            source,
            pile_a,
            pile_b,
        }),
        ChoiceRequest::PartitionRevealed {
            player,
            controller,
            source,
            revealed,
        } => Some(PendingChoice::PartitionRevealed {
            player,
            controller,
            source,
            revealed,
        }),
        ChoiceRequest::ChoosePileForHand {
            player,
            source,
            pile_a,
            pile_b,
        } => Some(PendingChoice::ChoosePileForHand {
            player,
            source,
            pile_a,
            pile_b,
        }),
        ChoiceRequest::OpponentChoosesExiledNonland {
            player,
            controller,
            source,
            nonlands,
            exiled,
        } => {
            if nonlands.is_empty() {
                return None;
            }
            Some(PendingChoice::OpponentChoosesExiledNonland {
                player,
                controller,
                source,
                nonlands,
                exiled,
            })
        }
        ChoiceRequest::RevealedCardToBattlefieldOrHand { player, card } => {
            Some(PendingChoice::RevealedCardToBattlefieldOrHand { player, card })
        }
        ChoiceRequest::ChooseAttachHost {
            player,
            attachment,
            candidates,
            optional,
        } => {
            if candidates.is_empty() {
                return None;
            }
            Some(PendingChoice::ChooseAttachHost {
                player,
                attachment,
                candidates,
                optional,
            })
        }
    }
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
                Some(PendingChoice::PayOrControllerDraws { .. })
            ) {
                game.pay_or_controller_draws(player, pay)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::PayEchoOrSacrifice { .. })
            ) {
                game.pay_echo(player, pay)
            } else if matches!(
                game.pending_choice,
                Some(PendingChoice::SacrificeUnlessPay { .. })
            ) {
                game.pay_sacrifice_unless(player, pay)
            } else {
                game.pay_optional_cost(player, pay)
            }
        }
        // Decree of Justice's cycling rider "you may pay {X}" — the only `PayCost` whose cost
        // carries a chosen `{X}` (CR 107.3), so it gets its own wire shape rather than widening
        // `PayOptionalCost`'s bare bool.
        Intent::PayOptionalCostX { player, pay, x } => {
            game.pay_optional_cost_with_x(player, pay, x)
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
                Some(PendingChoice::PartitionRevealed { .. })
            ) {
                game.partition_revealed(player, sacrifices)
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
        Intent::CastCreatureFaceDown { player, choice } => {
            game.cast_creature_face_down(player, choice)
        }
        Intent::ReturnLandOrSacrifice { player, land } => {
            game.return_land_or_sacrifice(player, land)
        }
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
        Intent::ChooseOpponentPile { player, pile }
            if matches!(
                game.pending_choice,
                Some(PendingChoice::ChoosePileForHand { .. })
            ) =>
        {
            game.choose_pile_for_hand(player, pile)
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
    /// Private to [`pending`]: effects/cast use [`raise`] / [`raise_choice`].
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
