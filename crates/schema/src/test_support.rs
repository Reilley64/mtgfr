//! Shared helpers for schema unit tests.

#[cfg(test)]
use engine::{Game, PlayerId};

#[cfg(test)]
pub(crate) fn def(name: &str) -> engine::CardDef {
    cards::get_by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}"))
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
