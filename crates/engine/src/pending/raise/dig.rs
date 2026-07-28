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
