//! Own-sacrifice / annihilator-style raises (skip when count covers all options).

use crate::{Game, ObjectId, PendingChoice, PermanentFilter, PlayerId};

pub(super) fn choose_own_sacrifices(
    game: &Game,
    player: PlayerId,
    source: ObjectId,
    filter: PermanentFilter,
    count: u32,
) -> Option<PendingChoice> {
    let options = game.edict_options(player, filter);
    if options.len() <= count as usize {
        return None;
    }
    Some(PendingChoice::ChooseOwnSacrifices {
        player,
        source,
        filter,
        count,
        options,
    })
}
