//! Legends (`leg`) grind — increment 99: fill-player-for-counters-on-player.

mod common;

use common::*;
use engine::*;

/// Enough cards that nobody decks out over the handful of turns these tests run.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

#[test]
fn pit_scorpion_poisons_the_player_it_connected_with() {
    // "Whenever this creature deals damage to a player, that player gets a poison counter."
    // *That* player — the one who took the damage, not the Scorpion's controller and not the
    // rest of the table (CR 122.1).
    let mut game = Game::with_players(4, 0);
    stock_libraries(&mut game);
    let scorpion = game.spawn_on_battlefield(PlayerId(0), card("Pit Scorpion"));

    attack_with(&mut game, vec![scorpion]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        1,
        "the damaged player got exactly one poison counter"
    );
    for seat in [PlayerId(0), PlayerId(2), PlayerId(3)] {
        assert_eq!(
            game.player_counters(seat, PlayerCounterKind::Poison),
            0,
            "only the damaged player gets poisoned"
        );
    }
}

#[test]
fn pit_scorpion_stacks_a_second_poison_counter_on_a_second_connection() {
    // One trigger per damage event (CR 603.3), and "gets a poison counter" adds rather than sets,
    // so two swings across two of the controller's turns leave two counters.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let scorpion = game.spawn_on_battlefield(PlayerId(0), card("Pit Scorpion"));

    attack_with(&mut game, vec![scorpion]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    attack_with(&mut game, vec![scorpion]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        2,
        "each connection adds its own poison counter"
    );
}

#[test]
fn pit_scorpion_does_not_poison_the_player_whose_blocker_ate_the_damage() {
    // The trigger watches damage dealt to a *player*; combat damage assigned to a blocking
    // creature is not that, so a blocked Scorpion poisons no one.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let scorpion = game.spawn_on_battlefield(PlayerId(0), card("Pit Scorpion"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![scorpion]);
    block_with(&mut game, vec![(bears, scorpion)]).expect("the Bears can block a 1/1");
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        0,
        "damage dealt to a blocker never reaches the defending player"
    );
}
