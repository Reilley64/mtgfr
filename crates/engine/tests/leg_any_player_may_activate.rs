//! Legends (`leg`) grind — increment 25: any-player-may-activate.
//!
//! `AbilityToml::activator` (`Activator::{Controller, Opponents, AnyPlayer, Owner}`) replaces the
//! old `only_owner_may_activate` bool so a printed ability can widen (or narrow) who may activate
//! it past CR 602.2's controller-only baseline. Land's Edge ("Any player may activate this
//! ability") and Clergy of the Holy Nimbus's second ability ("Only your opponents may activate
//! this ability") are the two `leg` cards that need it.

mod common;

use common::*;
use engine::*;

/// Land's Edge on the battlefield under player 0, with a land and a non-land card in player 1's
/// hand to discard.
fn lands_edge_in_play() -> (Game, ObjectId, ObjectId, ObjectId) {
    let mut game = Game::new();
    let edge = game.spawn_on_battlefield(PlayerId(0), card("Land's Edge"));
    let land = game.spawn_in_hand(PlayerId(1), card("Forest"));
    let nonland = game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    (game, edge, land, nonland)
}

/// CR 602.2a: the activator becomes the ability's controller on the stack — an opponent
/// activating Land's Edge pays its own discard cost from *their own* hand, not the enchantment
/// controller's.
#[test]
fn an_opponent_activating_lands_edge_discards_from_their_own_hand() {
    let (mut game, edge, land, _nonland) = lands_edge_in_play();

    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: edge,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(0))),
        sacrifice: vec![],
        discard_cost: vec![land],
        x: 0,
    })
    .expect("\"any player may activate this ability\" lets a non-controller pay the cost");

    assert_eq!(
        game.zone_of(land),
        Zone::Graveyard,
        "the discarded card left the activator's hand"
    );
    assert_eq!(
        game.owner_of(land),
        PlayerId(1),
        "the activator discarded from their own hand, not the enchantment controller's"
    );
}

/// "If the discarded card was a land card, this enchantment deals 2 damage to target player or
/// planeswalker." A land discard deals the 2 damage.
#[test]
fn lands_edge_deals_two_damage_when_the_discarded_card_was_a_land() {
    let (mut game, edge, land, _nonland) = lands_edge_in_play();
    let life_before = game.life(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: edge,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(0))),
        sacrifice: vec![],
        discard_cost: vec![land],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 2,
        "the discarded card was a land — 2 damage to the target"
    );
}

/// The conditional half of the ability (CR 120.8 — 0 damage is never dealt): discarding a
/// non-land pays the cost but deals no damage.
#[test]
fn lands_edge_deals_no_damage_when_the_discarded_card_was_not_a_land() {
    let (mut game, edge, _land, nonland) = lands_edge_in_play();
    let life_before = game.life(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: edge,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(0))),
        sacrifice: vec![],
        discard_cost: vec![nonland],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(0)),
        life_before,
        "the discarded card was not a land — no damage is dealt"
    );
}

/// "Any player may activate this ability" — the controller may still activate their own Land's
/// Edge too; the widening adds activators, it does not remove the baseline one.
#[test]
fn lands_edges_own_controller_may_also_activate_it() {
    let (mut game, edge, land, _nonland) = lands_edge_in_play();
    let owner_land = game.spawn_in_hand(PlayerId(0), card("Forest"));
    let _ = land;

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: edge,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(1))),
        sacrifice: vec![],
        discard_cost: vec![owner_land],
        x: 0,
    })
    .expect("\"any player\" includes the controller");
}

/// The available-actions list (`Game::meaningful_actions`, which every priority scan and
/// auto-pass check routes through) offers Land's Edge's activation to a non-controller — the
/// other half of threading `Activator` through, alongside the legality gate proven above.
#[test]
fn lands_edge_activation_is_a_meaningful_action_for_a_non_controller() {
    let (game, edge, _land, _nonland) = lands_edge_in_play();

    let actions = game.meaningful_actions(PlayerId(1));
    assert!(
        actions.contains(&MeaningfulAction::Activate {
            source: edge,
            ability: 0
        }),
        "a non-controller with a card to discard sees Land's Edge as activatable: {actions:?}"
    );
}

/// Clergy of the Holy Nimbus on the battlefield under player 0.
fn clergy_in_play() -> (Game, ObjectId) {
    let mut game = Game::new();
    let clergy = game.spawn_on_battlefield(PlayerId(0), card("Clergy of the Holy Nimbus"));
    game.fund_mana(PlayerId(1));
    (game, clergy)
}

/// "Only your opponents may activate this ability" — an opponent may pay {1} and activate it.
#[test]
fn clergys_second_ability_is_activatable_by_an_opponent() {
    let (mut game, clergy) = clergy_in_play();

    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: clergy,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("\"only your opponents may activate this ability\" admits an opponent");
}

/// The other half of "only your opponents" — Clergy's own controller may not activate it.
#[test]
fn clergys_own_controller_may_not_activate_its_second_ability() {
    let (mut game, clergy) = clergy_in_play();
    game.fund_mana(PlayerId(0));

    assert_eq!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: clergy,
            ability_index: 0,
            target: None,
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        }),
        Err(Reject::CannotActivate),
        "\"only your opponents\" excludes this creature's own controller"
    );
}

/// The available-actions list agrees: Clergy's ability is listed for an opponent, but never for
/// its own controller — the `meaningful_actions`/auto-pass half of the same gate.
#[test]
fn clergys_second_ability_is_meaningful_only_for_opponents() {
    let (game, clergy) = clergy_in_play();

    let opponent_actions = game.meaningful_actions(PlayerId(1));
    assert!(
        opponent_actions.contains(&MeaningfulAction::Activate {
            source: clergy,
            ability: 0
        }),
        "an opponent with mana up sees it as activatable: {opponent_actions:?}"
    );

    let controller_actions = game.meaningful_actions(PlayerId(0));
    assert!(
        !controller_actions.contains(&MeaningfulAction::Activate {
            source: clergy,
            ability: 0
        }),
        "its own controller never sees it as activatable: {controller_actions:?}"
    );
}
