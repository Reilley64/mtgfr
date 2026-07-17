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

pub(super) fn next_caster_keep(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    caster: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    while !remaining.is_empty() {
        let target_player = remaining.remove(0);
        let options =
            game.edict_options(target_player, PermanentFilter::of(crate::TypeSet::NONLAND));
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

pub(super) fn next_sacrifice_edict(
    game: &Game,
    mut remaining: Vec<PlayerId>,
    keep_one: bool,
    filter: PermanentFilter,
    follow_up: &'static [Effect],
    controller: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
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
