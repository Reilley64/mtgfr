//! Multi-seat fan-out kickoffs (next remaining player with a real choice).

use crate::{Effect, Game, ObjectId, PendingChoice, PermanentFilter, PlayerId};

pub(super) fn next_graveyard_exile(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    source: ObjectId,
) -> Option<PendingChoice> {
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

/// Next opponent with a card to discard (Syphon Mind, "Each other player discards a card") —
/// skipping any with an empty hand, the discard twin of [`next_graveyard_exile`].
pub(super) fn next_discard_edict(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    source: ObjectId,
    floor: Option<u32>,
) -> Option<PendingChoice> {
    while !remaining.is_empty() {
        let player = remaining.remove(0);
        let options = game.hand_of(player);
        if options.is_empty() {
            continue;
        }
        let count = match floor {
            // Balance: a seat already at the smallest hand pitches nothing, so it is skipped
            // outright rather than asked for zero cards.
            Some(floor) if options.len() as u32 <= floor => continue,
            Some(floor) => options.len() as u32 - floor,
            None => 1,
        };
        return Some(PendingChoice::DiscardEdict {
            player,
            source,
            options,
            remaining,
            count,
            floor,
        });
    }
    None
}

pub(super) fn next_caster_keep(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    caster: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    while !remaining.is_empty() {
        let target_player = remaining.remove(0);
        let options = game.edict_options(
            target_player,
            PermanentFilter::of(crate::TypeSet::NONLAND),
            None,
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

pub(super) fn next_counter_target(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    chooser: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    while !remaining.is_empty() {
        let target_player = remaining.remove(0);
        let options: Vec<ObjectId> = game
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

/// Next seat in a join-forces payment round — mandatory to *address* (paying nothing is a legal
/// answer), so no seat is skipped, the same shape as [`next_vote`].
pub(super) fn next_join_forces_payment(
    mut remaining: Vec<PlayerId>,
    source: ObjectId,
    prevent_up_to: Option<u8>,
) -> Option<PendingChoice> {
    if remaining.is_empty() {
        return None;
    }
    let player = remaining.remove(0);
    Some(PendingChoice::JoinForcesPayment {
        player,
        source,
        remaining,
        prevent_up_to,
    })
}

pub(super) fn next_vote(
    mut remaining: Vec<PlayerId>,
    source: ObjectId,
    options: &'static [&'static str],
) -> Option<PendingChoice> {
    if remaining.is_empty() {
        return None;
    }
    let player = remaining.remove(0);
    Some(PendingChoice::CastVote {
        player,
        source,
        options,
        remaining,
    })
}

/// Next seat in Conundrum Sphinx's name-a-card fan-out (mandatory — naming is required even with
/// an empty library, so no seat is ever skipped, same shape as [`next_vote`]).
pub(super) fn next_card_name(
    mut remaining: Vec<PlayerId>,
    source: ObjectId,
) -> Option<PendingChoice> {
    if remaining.is_empty() {
        return None;
    }
    let player = remaining.remove(0);
    Some(PendingChoice::ChooseCardName {
        player,
        source,
        remaining,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn next_sacrifice_edict(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    keep_one: bool,
    filter: PermanentFilter,
    count: u32,
    floor: Option<u32>,
    follow_up: &'static [Effect],
    controller: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    while !remaining.is_empty() {
        let player = remaining.remove(0);
        let options = game.edict_options(player, filter, Some(source));
        if options.is_empty() || (keep_one && options.len() == 1) {
            continue;
        }
        let count = match floor {
            // Balance: a seat already at the fewest gives up nothing, so it is skipped outright
            // rather than asked for zero permanents.
            Some(floor) if options.len() as u32 <= floor => continue,
            Some(floor) => options.len() as u32 - floor,
            None => count,
        };
        return Some(PendingChoice::SacrificeEdict {
            player,
            options,
            keep_one,
            filter,
            remaining,
            count,
            floor,
            controller,
            source,
            follow_up,
        });
    }
    None
}
