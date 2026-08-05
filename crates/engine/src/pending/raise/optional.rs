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
    count: u8,
    then: &'static [Effect],
    otherwise: &'static [Effect],
) -> Option<PendingChoice> {
    let options = game.edict_options(player, filter, Some(source));
    // Can't make the offer (Mold Demon on one Swamp): no prompt. The penalty isn't lost — the
    // caller ran `otherwise` instead of reaching this raise at all.
    if options.len() < count.max(1) as usize {
        return None;
    }
    Some(PendingChoice::MaySacrifice {
        player,
        source,
        options,
        count: count.max(1),
        then,
        otherwise,
    })
}

/// Wood Elemental's as-enters "sacrifice any number of untapped Forests": every matching permanent
/// its controller has, minus the source itself (a permanent can't be sacrificed to its own entry
/// cost — and no printing of this shape matches its own type anyway). `None` when nothing matches,
/// so the entry never pauses on an empty prompt.
pub(super) fn sacrifice_any_number(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: PermanentFilter,
) -> Option<PendingChoice> {
    let options: Vec<ObjectId> = game
        .edict_options(player, filter, Some(source))
        .into_iter()
        .filter(|&id| id != source)
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::SacrificeAnyNumber {
        player,
        source,
        options,
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

/// The next graveyard still owed a card by a return-from-graveyard effect: `graveyards` is the
/// queue of owners, and the first that actually holds a matching card becomes this prompt (the
/// rest ride along on the choice to be prompted for in turn). One entry is the ordinary
/// single return (Deadly Brew's rider); the *same* owner repeated is Recall's "a card ... for each
/// card discarded this way" (each prompt recomputes the options, so an already-returned card is
/// gone from the next one); distinct owners are Glyph of Reincarnation's per-graveyard fan-out.
/// `None` when no listed graveyard holds a matching card at all — CR 700.2's "as much as
/// possible" leaves nothing to do, so the resolution never pauses.
pub(super) fn may_return_from_graveyard(
    game: &Game,
    chooser: PlayerId,
    source: ObjectId,
    filter: CardFilter,
    mandatory: bool,
    to_battlefield: bool,
    graveyards: Vec<PlayerId>,
) -> Option<PendingChoice> {
    let mut rest = graveyards.into_iter();
    while let Some(owner) = rest.next() {
        let options: Vec<ObjectId> = game
            .live_object_ids()
            .into_iter()
            .filter(|&id| {
                game.zone_of(id) == crate::Zone::Graveyard
                    && game.owner_of(id) == owner
                    && filter.matches(&game.def_of(id))
            })
            .collect();
        if options.is_empty() {
            continue;
        }
        return Some(PendingChoice::MayReturnFromGraveyard {
            player: chooser,
            source,
            options,
            filter,
            mandatory,
            to_battlefield,
            then_graveyards: rest.collect(),
        });
    }
    None
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

/// False Orders' "you may have it block an attacking creature of your choice": every declared
/// attacker `blocker` could legally have been declared as blocking (CR 509.1a — its own
/// controller's legality, not the spell controller's), so the re-aim can't launder a block that
/// was never available. No such attacker skips the pause.
pub(super) fn choose_block_target(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    blocker: ObjectId,
) -> Option<PendingChoice> {
    let defender = game
        .as_permanent(blocker)
        .map(|_| game.controller_of(blocker))?;
    let options: Vec<ObjectId> = game
        .attackers()
        .into_iter()
        .filter(|&attacker| game.can_block(defender, blocker, attacker))
        .collect();
    if options.is_empty() {
        return None;
    }
    Some(PendingChoice::ChooseBlockTarget {
        player,
        source,
        blocker,
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
    drawn_this_turn: bool,
    life_per_declined: u32,
) -> Option<PendingChoice> {
    let mut hand = game.hand_of(player);
    // Sylvan Library chooses among "cards in your hand drawn this turn" only — a card drawn and
    // already played this turn is no longer in hand, so the intersection is the candidate set.
    if drawn_this_turn {
        hand.retain(|c| game.drawn_this_turn.contains(c));
    }
    let count = (count as usize).min(hand.len());
    if count == 0 {
        return None;
    }
    Some(PendingChoice::PutFromHandOnTop {
        player,
        hand,
        count,
        life_per_declined,
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
