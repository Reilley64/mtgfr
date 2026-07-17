//! Exile-link and becomes-a-copy raises.

use crate::{EnterAsCopy, Game, ObjectId, PendingChoice, PlayerId};

pub(super) fn choose_exiled_with_card(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
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

pub(super) fn choose_exiled_with_card_to_cast(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
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

pub(super) fn enter_as_copy(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    marker: EnterAsCopy,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
        .permanent_ids(|_| true)
        .collect::<Vec<_>>()
        .into_iter()
        .filter(|&id| {
            id != source
                && match marker.of {
                    crate::CopyTargetKind::Creature => game.is_creature_on_battlefield(id),
                    crate::CopyTargetKind::Enchantment => game.is_enchantment_on_battlefield(id),
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

pub(super) fn choose_token_to_copy(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
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

pub(super) fn choose_copy_card_from_list(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    cards: &'static [ObjectId],
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = cards
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
