//! Library / graveyard / hand look-and-select raises (scry, search, put land, …).

use crate::{
    ArrangeRest, CardFilter, Game, ObjectId, PendingChoice, PlayerId, RestDest, SearchDest, TopDest,
};

pub(super) fn arrange_top(
    game: &Game,
    player: PlayerId,
    library: PlayerId,
    count: u32,
    rest: ArrangeRest,
) -> Option<PendingChoice> {
    let cards: Vec<ObjectId> = game.players[library.0 as usize]
        .library
        .iter()
        .take(count as usize)
        .copied()
        .collect();
    if cards.is_empty() {
        return None;
    }
    Some(PendingChoice::ArrangeTop {
        player,
        library,
        cards,
        rest,
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
    // Stranglehold's "Your opponents can't search libraries" (CR 701.19): a denied search never
    // raises a pause at all, so it neither finds a card nor shuffles (the shuffle is tied to the
    // search that didn't happen).
    if game.opponent_search_denied(player) {
        return None;
    }
    let matches: Vec<ObjectId> = game.players[player.0 as usize]
        .library
        .iter()
        .copied()
        .filter(|&id| filter.matches(&game.def_of(id)))
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

/// Cauldron Dance's "you may put a creature card from your hand onto the battlefield": the
/// creature sibling of [`put_land_from_hand`] (CR 608.2b's "may" — no eligible creature in hand
/// raises no choice). `source` is carried on the choice so the answer can later schedule the
/// end-step sacrifice against this same resolving ability. `subtypes` restricts eligibility
/// (Kaalia: Angel/Demon/Dragon; empty = any creature); `keep` and `defender` ride along for the
/// answer (no end-step sacrifice; enter tapped and attacking that opponent).
pub(super) fn put_creature_from_hand(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    subtypes: &'static [&'static str],
    keep: bool,
    defender: Option<PlayerId>,
) -> Option<PendingChoice> {
    let candidates: Vec<ObjectId> = game
        .hand_of(player)
        .into_iter()
        .filter(|&id| matches!(game.def_of(id).kind, crate::CardKind::Creature { .. }))
        .filter(|&id| {
            subtypes.is_empty()
                || subtypes
                    .iter()
                    .any(|want| game.def_of(id).subtypes.contains(want))
        })
        .collect();
    if candidates.is_empty() {
        return None;
    }
    Some(PendingChoice::PutCreatureFromHand {
        player,
        source,
        candidates,
        keep,
        defender,
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
