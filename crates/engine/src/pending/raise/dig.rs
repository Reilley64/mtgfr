//! Dig / cascade / free-cast / pile-split raises (prep events stay at the call site).

use crate::{Game, ObjectId, PendingChoice, PlayerId, SplittingContinuation};

pub(super) fn choose_exiled_dig_to_cast_free(
    player: PlayerId,
    source: ObjectId,
    candidates: Vec<ObjectId>,
    exiled: Vec<ObjectId>,
) -> Option<PendingChoice> {
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

/// Raging River's ritual, in one builder so both halves and the hand-off between them read in
/// order: ask the next defender to divide, and once every defender has, ask the ability's
/// controller to label the next attacking creature. A defender with no non-flying creatures has
/// nothing to divide and is skipped; when both lists run out there is nothing to pause on.
pub(super) fn split_blockers_into_piles(
    game: &Game,
    source: ObjectId,
    left: Vec<(PlayerId, Vec<ObjectId>)>,
    defenders: Vec<PlayerId>,
    attackers: Vec<ObjectId>,
) -> Option<PendingChoice> {
    let mut defenders = defenders;
    while let Some(player) = defenders.first().copied() {
        defenders.remove(0);
        let options: Vec<ObjectId> = game
            .controlled_battlefield(player)
            .into_iter()
            // "all creatures without flying they control" — printed as flying, not as evasion,
            // so reach doesn't keep a creature out of the piles.
            .filter(|&c| {
                game.is_creature_on_battlefield(c) && !game.has_keyword(c, crate::Keyword::Flying)
            })
            .collect();
        if options.is_empty() {
            continue;
        }
        return Some(PendingChoice::SplitBlockersIntoPiles {
            player,
            source,
            options,
            left,
            defenders,
            attackers,
        });
    }
    let mut remaining = attackers;
    let attacker = remaining.first().copied()?;
    remaining.remove(0);
    Some(PendingChoice::ChoosePileForAttacker {
        player: game.controller_of(attacker),
        source,
        attacker,
        left,
        remaining,
    })
}

/// Camouflage's division: ask the defender in `partial` for their next pile, or start the next of
/// `defenders` from the top. Unlike Raging River this is asked once *per attacker aimed at that
/// seat*, so a defender facing three attackers answers three times, each over what the earlier
/// piles left them. A defender with no creatures has nothing to divide and is skipped.
pub(super) fn divide_blockers_into_piles(
    game: &Game,
    source: ObjectId,
    partial: Option<(PlayerId, Vec<ObjectId>, Vec<Vec<ObjectId>>)>,
    defenders: Vec<PlayerId>,
    attackers: Vec<ObjectId>,
) -> Option<PendingChoice> {
    let mut defenders = defenders;
    let mut partial = partial;
    loop {
        let (player, options, piles) = match partial.take() {
            Some(open) => open,
            None => {
                let player = defenders.first().copied()?;
                defenders.remove(0);
                // "any number of creatures they control" — no flying exemption here, unlike Raging
                // River; a flyer that can't block what its pile was dealt simply doesn't block.
                let options: Vec<ObjectId> = game
                    .controlled_battlefield(player)
                    .into_iter()
                    .filter(|&c| game.is_creature_on_battlefield(c))
                    .collect();
                (player, options, Vec::new())
            }
        };
        if options.is_empty() {
            continue;
        }
        let needed = attackers
            .iter()
            .filter(|&&a| game.defending_player_of(a) == Some(player))
            .count();
        return Some(PendingChoice::DivideBlockersIntoPiles {
            player,
            source,
            options,
            piles,
            needed: needed as u8,
            defenders,
            attackers,
        });
    }
}

/// Word of Command's "choose a card from it": every card in `subject`'s hand is on offer — the
/// printed text puts no filter on the pick, and "plays that card if able" is what sorts out the
/// unplayable ones at the answer. An empty hand leaves nothing to choose.
pub(super) fn choose_card_in_hand_to_play(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    subject: PlayerId,
) -> Option<PendingChoice> {
    let options = game.hand_of(subject);
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::ChooseCardInHandToPlay {
        player,
        source,
        subject,
        options,
    })
}

pub(super) fn choose_exiled_to_cast_free(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    exiled: Vec<ObjectId>,
    count: u8,
    rest_to_hand: bool,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = exiled
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

pub(super) fn choose_splitting_opponent(
    player: PlayerId,
    source: ObjectId,
    legal: Vec<PlayerId>,
    then: SplittingContinuation,
) -> Option<PendingChoice> {
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

pub(super) fn opponent_chooses_exiled_nonland(
    player: PlayerId,
    controller: PlayerId,
    source: ObjectId,
    nonlands: Vec<ObjectId>,
    exiled: Vec<ObjectId>,
) -> Option<PendingChoice> {
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

pub(super) fn choose_attach_host(
    player: PlayerId,
    attachment: ObjectId,
    candidates: Vec<ObjectId>,
    optional: bool,
) -> Option<PendingChoice> {
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
