//! Legends (`leg`) grind — increment 115: player-draws-trigger-context.

mod common;

use common::*;
use engine::*;

/// Keep every seat's library deep enough that a draw step (or an Ancestral Recall) can't deck
/// anybody over the turns these tests roll through.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..40 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object,
        target,
        x: 0,
        modes: vec![],
        discard_cost: vec![],
        graveyard_exile: vec![],
        sacrifice_cost: vec![],
        kicked: false,
        bought_back: false,
        evoked: false,
        strive_count: 0,
        replicate_count: 0,
        multikicker_count: 0,
        alternative_cost: false,
    })
    .expect("the spell is castable");
    resolve_top_of_stack(game);
}

/// Drain the stack, item by item (CR 117.4) — the draw triggers land on it as the draw spell
/// finishes resolving.
fn resolve_whole_stack(game: &mut Game) {
    let mut guard = 0;
    while !game.stack().is_empty() {
        resolve_top_of_stack(game);
        guard += 1;
        assert!(guard < 100, "stack failed to drain");
    }
}

#[test]
fn underworld_dreams_deals_one_damage_per_card_an_opponent_draws() {
    // "Whenever an opponent draws a card, this enchantment deals 1 damage to that player."
    // CR 121.3/603.2: drawing three cards is three draws, so the watch fires three times — once
    // per card — and each ping lands on the *drawing* player, not on the Dreams' controller.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Underworld Dreams"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));

    cast_and_resolve(&mut game, recall, Some(Target::Player(PlayerId(1))));
    resolve_whole_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(1)),
        17,
        "each of the opponent's three draws deals them 1 damage"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "the damage lands on the drawing opponent, not on the Dreams' controller"
    );
}

#[test]
fn underworld_dreams_ignores_its_controllers_own_draws() {
    // "Whenever an *opponent* draws a card" — your own draws are outside the watch.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Underworld Dreams"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));

    cast_and_resolve(&mut game, recall, Some(Target::Player(PlayerId(0))));
    resolve_whole_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "drawing your own three cards doesn't trigger your own Underworld Dreams"
    );
    assert_eq!(game.life(PlayerId(1)), 20, "and nobody else takes damage");
}

#[test]
fn underworld_dreams_does_not_fire_on_the_skipped_first_draw() {
    // CR 103.8a: the starting player of a two-player game skips their first draw step. No card is
    // drawn, so there is nothing for the watch to see — but their draw step two turns later is a
    // real draw and does get pinged.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(1), card("Underworld Dreams"));

    game.begin_first_turn();
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "the skipped first draw draws no card, so Underworld Dreams doesn't fire"
    );

    pass_until_next_turn(&mut game); // P1's turn — their own draw is not an opponent's.
    pass_until_next_turn(&mut game); // back to P0, who draws for real this time.
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        game.life(PlayerId(0)),
        19,
        "P0's real draw-step draw takes 1 damage from the opponent's Underworld Dreams"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "P1's own draw-step draw never fires their own Underworld Dreams"
    );
}
