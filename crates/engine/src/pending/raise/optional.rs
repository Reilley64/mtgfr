//! Optional / skip-if-empty raises (board, hand, graveyard filters).

use crate::{CardFilter, Effect, Game, ObjectId, PendingChoice, PermanentFilter, PlayerId};

pub(super) fn proliferate(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    remaining: u8,
) -> Option<PendingChoice> {
    if remaining == 0 {
        return None;
    }
    let options: Vec<ObjectId> = game
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

pub(super) fn phase_out(game: &Game, player: PlayerId, source: ObjectId) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
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

pub(super) fn may_sacrifice(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: PermanentFilter,
    then: &'static [Effect],
) -> Option<PendingChoice> {
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

pub(super) fn devour(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    multiplier: u32,
) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
        .edict_options(player, PermanentFilter::of(crate::TypeSet::CREATURE))
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

pub(super) fn may_return_from_graveyard(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: CardFilter,
) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
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

pub(super) fn may_discard(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    then: &'static [Effect],
) -> Option<PendingChoice> {
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

pub(super) fn discard(
    game: &Game,
    player: PlayerId,
    count: u32,
    or_one_matching: Option<CardFilter>,
) -> Option<PendingChoice> {
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

pub(super) fn sacrifice_unless_return_land(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: PermanentFilter,
) -> Option<PendingChoice> {
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
