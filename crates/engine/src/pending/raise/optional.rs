//! Optional / skip-if-empty raises (board, hand, graveyard filters).

use crate::{
    CardFilter, Effect, Game, ObjectId, PendingChoice, PermanentFilter, PlayerCounterKind,
    PlayerId, ProliferateTarget,
};

/// Every permanent and player that currently has a counter (CR 701.27 — proliferate can only
/// grow a counter of a kind "already there"). Permanents first, then players in seat order, so
/// the offered set is deterministic.
pub(super) fn proliferate(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    remaining: u8,
) -> Option<PendingChoice> {
    if remaining == 0 {
        return None;
    }
    let permanents = game.battlefield().into_iter().filter(|&id| {
        let p = game.permanent(id);
        // A planeswalker's loyalty is loyalty counters (CR 306.5b), so it always has counters.
        p.plus_counters > 0 || p.loyalty > 0 || p.kind_counters.iter().any(|&c| c > 0)
    });
    let players = (0..game.player_count() as u8).map(PlayerId).filter(|&p| {
        !game.has_lost(p)
            && PlayerCounterKind::ALL
                .iter()
                .any(|&kind| game.player_counters(p, kind) > 0)
    });
    let options: Vec<ProliferateTarget> = permanents
        .map(ProliferateTarget::Permanent)
        .chain(players.map(ProliferateTarget::Player))
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
    let options = game.edict_options(player, filter, Some(source));
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
        .edict_options(player, PermanentFilter::of(crate::TypeSet::CREATURE), None)
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
    mandatory: bool,
) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
        .live_object_ids()
        .into_iter()
        .filter(|&id| {
            game.zone_of(id) == crate::Zone::Graveyard
                && game.owner_of(id) == player
                && filter.matches(&game.def_of(id))
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::MayReturnFromGraveyard {
        player,
        source,
        options,
        mandatory,
    })
}

pub(super) fn may_exile_discarded_nonland_may_play(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    cards: &'static [ObjectId],
) -> Option<PendingChoice> {
    // Only the discarded nonland cards still sitting in this player's graveyard are eligible — a
    // prior effect may have already moved one out (CR 400.7). No survivor skips the pause.
    let options: Vec<ObjectId> = cards
        .iter()
        .copied()
        .filter(|&id| game.zone_of(id) == crate::Zone::Graveyard && game.owner_of(id) == player)
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::MayExileDiscardedToPlay {
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

pub(super) fn may_put_counter_on_creature(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
        .battlefield()
        .into_iter()
        .filter(|&id| matches!(game.def_of(id).kind, crate::CardKind::Creature { .. }))
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::MayPutCounterOnCreature {
        player,
        source,
        options,
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

pub(super) fn put_from_hand_on_top(
    game: &Game,
    player: PlayerId,
    count: u32,
) -> Option<PendingChoice> {
    let hand = game.hand_of(player);
    let count = (count as usize).min(hand.len());
    if count == 0 {
        return None;
    }
    Some(PendingChoice::PutFromHandOnTop {
        player,
        hand,
        count,
    })
}

pub(super) fn sacrifice_unless_return_land(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: PermanentFilter,
) -> Option<PendingChoice> {
    let candidates = game.edict_options(player, filter, Some(source));
    if candidates.is_empty() {
        return None;
    }
    Some(PendingChoice::SacrificeUnlessReturnLand {
        player,
        source,
        candidates,
    })
}
