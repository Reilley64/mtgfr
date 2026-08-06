//! Legends (`leg`) grind — increment 2: world-supertype.

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
fn concordant_crossroads_carries_the_world_supertype() {
    // "World Enchantment" (CR 205.4a) — the supertype the world rule keys off.
    assert!(card("Concordant Crossroads").world);
    assert!(
        !card("Bad Moon").world,
        "an ordinary enchantment is not World"
    );
}

#[test]
fn lone_world_enchantment_stays_on_the_battlefield() {
    // CR 704.5k only fires at two or more.
    let mut game = Game::new();
    let crossroads = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));

    sweep(&mut game);

    assert_eq!(game.zone_of(crossroads), Zone::Battlefield);
}

#[test]
fn world_rule_puts_all_but_the_newest_world_enchantment_into_graveyards() {
    // CR 704.5k: two permanents with the World supertype — all but the one that has had it for
    // the shortest amount of time are put into their owners' graveyards. Unlike the legend rule
    // (CR 704.5j) there is no choice and no name matching.
    let mut game = Game::new();
    let older = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    let newer = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));

    sweep(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "the world rule is not a choice"
    );
    assert_eq!(
        game.zone_of(newer),
        Zone::Battlefield,
        "the newest survives"
    );
    assert_eq!(game.zone_of(older), Zone::Graveyard);
}

#[test]
fn world_rule_spans_controllers() {
    // CR 704.5k is a global rule — unlike the legend rule it does not group by controller.
    let mut game = Game::new();
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Concordant Crossroads"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));

    sweep(&mut game);

    assert_eq!(game.zone_of(mine), Zone::Battlefield, "the newest survives");
    assert_eq!(
        game.zone_of(theirs),
        Zone::Graveyard,
        "an opponent's older World enchantment dies to my newer one"
    );
}

#[test]
fn world_rule_leaves_only_the_newest_of_three() {
    let mut game = Game::new();
    let first = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Concordant Crossroads"));
    let third = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));

    sweep(&mut game);

    assert_eq!(game.zone_of(third), Zone::Battlefield);
    assert_eq!(game.zone_of(first), Zone::Graveyard);
    assert_eq!(game.zone_of(second), Zone::Graveyard);
}

#[test]
fn world_rule_ignores_enchantments_without_the_supertype() {
    let mut game = Game::new();
    let crossroads = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    let bad_moon = game.spawn_on_battlefield(PlayerId(0), card("Bad Moon"));

    sweep(&mut game);

    assert_eq!(game.zone_of(crossroads), Zone::Battlefield);
    assert_eq!(game.zone_of(bad_moon), Zone::Battlefield);
}

#[test]
fn a_cast_world_enchantment_supplants_the_one_already_out() {
    // The real-game shape: the second copy resolves and the resident one dies on the sweep that
    // follows resolution, without anyone getting a choice.
    let mut game = Game::new();
    let resident = game.spawn_on_battlefield(PlayerId(0), card("Concordant Crossroads"));
    let in_hand = game.spawn_in_hand(PlayerId(0), card("Concordant Crossroads"));

    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: in_hand,
        target: None,
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
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(resident),
        Zone::Graveyard,
        "the resident World enchantment is the older one"
    );
}
