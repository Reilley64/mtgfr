//! Legends (`leg`) grind — increment 35: revealed-zone-visibility.
//!
//! Revelation ("Players play with their hands revealed.") and Field of Dreams ("Players play with
//! the top card of their libraries revealed.") are the only two cards in the set that widen the
//! server-side privacy gate. The gate itself lives in the projection layer (`crates/schema`), which
//! is where the per-viewer tests live; the engine only has to answer *whether* the table is playing
//! with a zone revealed, and answer `false` the instant the enchantment leaves the battlefield.

mod common;

use common::*;
use engine::*;

/// Pass priority once so the state-based-action sweep runs (CR 704.3).
fn sweep(game: &mut Game) {
    game.submit(Intent::PassPriority {
        player: game.priority_holder(),
    })
    .unwrap();
}

#[test]
fn an_empty_battlefield_reveals_nothing() {
    let game = Game::new();
    assert!(!game.hands_revealed_to_all());
    assert!(!game.library_tops_revealed_to_all());
}

#[test]
fn revelation_leaving_the_battlefield_hides_every_hand_again() {
    // Fail-closed: the reveal is a continuous effect of a battlefield permanent (CR 611.2c), so it
    // is gone the moment the permanent is. The world rule (CR 704.5k) is the shortest way off the
    // battlefield for a World enchantment — any newer World permanent evicts it.
    let mut game = Game::new();
    let revelation = game.spawn_on_battlefield(PlayerId(0), card("Revelation"));
    assert!(game.hands_revealed_to_all());

    game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    sweep(&mut game);

    assert_eq!(game.zone_of(revelation), Zone::Graveyard);
    assert!(
        !game.hands_revealed_to_all(),
        "hands go back to private the instant Revelation leaves the battlefield",
    );
}

#[test]
fn field_of_dreams_leaving_the_battlefield_hides_every_library_top_again() {
    let mut game = Game::new();
    let field = game.spawn_on_battlefield(PlayerId(0), card("Field of Dreams"));
    assert!(game.library_tops_revealed_to_all());

    game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    sweep(&mut game);

    assert_eq!(game.zone_of(field), Zone::Graveyard);
    assert!(
        !game.library_tops_revealed_to_all(),
        "library tops go back to private the instant Field of Dreams leaves the battlefield",
    );
}

#[test]
fn each_card_reveals_only_its_own_zone() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Revelation"));
    assert!(game.hands_revealed_to_all());
    assert!(
        !game.library_tops_revealed_to_all(),
        "Revelation says nothing about libraries",
    );

    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Field of Dreams"));
    assert!(game.library_tops_revealed_to_all());
    assert!(
        !game.hands_revealed_to_all(),
        "Field of Dreams says nothing about hands",
    );
}

#[test]
fn the_world_rule_lets_only_one_of_the_two_reveals_stand() {
    // CR 704.5k groups by neither name nor controller: two World enchantments can never share a
    // battlefield, so "hands and library tops revealed at once" is unreachable with these two.
    let mut game = Game::new();
    let revelation = game.spawn_on_battlefield(PlayerId(0), card("Revelation"));
    let field = game.spawn_on_battlefield(PlayerId(1), card("Field of Dreams"));

    sweep(&mut game);

    assert_eq!(game.zone_of(revelation), Zone::Graveyard, "the older loses");
    assert_eq!(game.zone_of(field), Zone::Battlefield);
    assert!(game.library_tops_revealed_to_all());
    assert!(
        !game.hands_revealed_to_all(),
        "the evicted Revelation stops revealing hands",
    );
}

#[test]
fn the_library_top_is_the_next_card_drawn() {
    // Field of Dreams reveals "the top card", so the projection needs the same card the draw would
    // take — and nothing beneath it.
    let mut game = Game::new();
    let p1 = PlayerId(1);
    let stacked = game.stack_library(p1, &[card("Shock"), card("Grizzly Bears"), card("Forest")]);

    assert_eq!(game.library_top(p1), Some(stacked[0]));
    game.draw_card(p1);
    assert_eq!(
        game.library_top(p1),
        Some(stacked[1]),
        "the reveal follows the draw down the library",
    );
}

#[test]
fn an_empty_library_has_no_top_card() {
    let game = Game::new();
    assert_eq!(game.library_top(PlayerId(0)), None);
}
