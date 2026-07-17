//! Library / graveyard / hand look-and-select raises (scry, search, put land, …).

use crate::{CardFilter, Game, ObjectId, PendingChoice, PlayerId, RestDest, SearchDest, TopDest};

pub(super) fn arrange_top(
    game: &Game,
    player: PlayerId,
    count: u32,
    to_graveyard: bool,
) -> Option<PendingChoice> {
    let library = &game.players[player.0 as usize].library;
    let cards: Vec<ObjectId> = library.iter().take(count as usize).copied().collect();
    if cards.is_empty() {
        return None;
    }
    Some(PendingChoice::ArrangeTop {
        player,
        cards,
        to_graveyard,
    })
}

#[allow(clippy::too_many_arguments)] // mirrors PendingChoice::SelectFromTop fields
pub(super) fn select_from_top(
    game: &Game,
    player: PlayerId,
    count: u32,
    filter: CardFilter,
    up_to: u32,
    min: u32,
    dest: TopDest,
    dest_tapped: bool,
    rest: RestDest,
    mv_budget: Option<u32>,
) -> Option<PendingChoice> {
    let library = &game.players[player.0 as usize].library;
    let cards: Vec<ObjectId> = library.iter().take(count as usize).copied().collect();
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

pub(super) fn distribute_top(
    game: &Game,
    player: PlayerId,
    count: u32,
    to_hand: u32,
    to_bottom: u32,
    to_exile_may_play: u32,
) -> Option<PendingChoice> {
    let library = &game.players[player.0 as usize].library;
    let cards: Vec<ObjectId> = library.iter().take(count as usize).copied().collect();
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

pub(super) fn shuffle_from_graveyard(
    game: &Game,
    answerer: PlayerId,
    owner: PlayerId,
    source: ObjectId,
    max: u32,
) -> Option<PendingChoice> {
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

pub(super) fn search_library(
    game: &Game,
    player: PlayerId,
    filter: CardFilter,
    dest: SearchDest,
    tapped: bool,
    count: u8,
    overflow: Option<SearchDest>,
) -> Option<PendingChoice> {
    let matches: Vec<ObjectId> = game.players[player.0 as usize]
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

pub(super) fn put_land_from_hand(
    game: &Game,
    player: PlayerId,
    tapped: bool,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
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

pub(super) fn cast_creature_face_down(
    game: &Game,
    player: PlayerId,
    spent_mana: [u8; 6],
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
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
