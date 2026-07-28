//! Shared helpers for schema unit tests.

#[cfg(test)]
use engine::{Game, PlayerId};

#[cfg(test)]
pub(crate) fn def(name: &str) -> engine::CardDef {
    cards::get_by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}"))
}

#[cfg(test)]
pub(crate) fn card_id(name: &str) -> engine::CardId {
    engine::intern_card_def(def(name))
}

#[cfg(test)]
pub(crate) fn refresh_via_mana_tap(game: &mut Game, tapland: engine::ObjectId) {
    game.submit(engine::Intent::TapForMana {
        player: PlayerId(0),
        object: tapland,
    })
    .unwrap();
}

#[cfg(test)]
pub(crate) fn pass_until_choice(game: &mut Game) {
    while game.pending_choice().is_none() {
        game.submit(engine::Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
    }
}

/// The two-pass [`resolve_top_of_stack`] only clears the stack at a two-player table; past that,
/// pass until it is actually empty.
#[cfg(test)]
pub(crate) fn resolve_top_of_stack_multiplayer(game: &mut Game) {
    while !game.stack().is_empty() {
        let player = game.priority_holder();
        game.submit(engine::Intent::PassPriority { player })
            .unwrap();
    }
}

#[cfg(test)]
pub(crate) fn resolve_top_of_stack(game: &mut Game) {
    game.submit(engine::Intent::PassPriority {
        player: game.priority_holder(),
    })
    .unwrap();
    game.submit(engine::Intent::PassPriority {
        player: game.priority_holder(),
    })
    .unwrap();
}
