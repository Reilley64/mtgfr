//! Legends (`leg`) grind — increment 11: mana-battery-counters.

mod common;

use common::*;
use engine::*;

fn activate(
    game: &mut Game,
    battery: ObjectId,
    ability_index: usize,
    x: u32,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: battery,
        ability_index,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x,
    })
}

/// "{2}, {T}: Put a charge counter on this artifact." — ability 0. The {2} is paid from Islands so
/// the battery's own colored mana can never be confused with the mana that funded its cost, and
/// the battery is untapped afterwards (a setup shortcut for the untap step a real turn would
/// bring, so a test can stack counters without rolling the game forward).
fn store_a_charge_counter(game: &mut Game, battery: ObjectId) {
    tap_basics(game, "Island", 2);
    activate(game, battery, 0, 0).expect("{2}, {T} is payable");
    resolve_top_of_stack(game); // not a mana ability — it uses the stack (CR 605.1a)
    game.untap(battery);
}

#[test]
fn black_mana_battery_stores_a_charge_counter() {
    // "{2}, {T}: Put a charge counter on this artifact."
    let mut game = Game::new();
    let battery = game.spawn_on_battlefield(PlayerId(0), card("Black Mana Battery"));
    tap_basics(&mut game, "Island", 2);

    activate(&mut game, battery, 0, 0).expect("{2}, {T} is payable from two Islands");

    assert!(game.is_tapped(battery), "the {{T}} cost taps the battery");
    assert_eq!(pool_total(&game, PlayerId(0)), 0, "the {{2}} was spent");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.counters_of_kind(battery, CounterKind::Charge), 1);
}

#[test]
fn black_mana_battery_adds_one_black_plus_one_per_charge_counter_removed() {
    // "{T}, Remove any number of charge counters from this artifact: Add {B}, then add an
    // additional {B} for each charge counter removed this way." Two counters removed is three
    // black mana, not two — the base {B} is added whatever the removal count.
    let mut game = Game::new();
    let battery = game.spawn_on_battlefield(PlayerId(0), card("Black Mana Battery"));
    store_a_charge_counter(&mut game, battery);
    store_a_charge_counter(&mut game, battery);
    assert_eq!(game.counters_of_kind(battery, CounterKind::Charge), 2);

    activate(&mut game, battery, 1, 2).expect("two charge counters are there to remove");

    assert!(game.is_tapped(battery), "the {{T}} cost taps the battery");
    assert_eq!(
        game.counters_of_kind(battery, CounterKind::Charge),
        0,
        "both charge counters were removed to pay the cost"
    );
    assert_eq!(
        game.mana_in_pool(PlayerId(0), Color::Black),
        3,
        "{{B}} plus an additional {{B}} for each of the two counters removed"
    );
    assert_eq!(pool_total(&game, PlayerId(0)), 3, "and nothing else");
}

#[test]
fn black_mana_battery_removing_no_charge_counters_still_adds_one_black() {
    // "Remove any number of charge counters" includes zero (CR 107.3c — a chosen X of 0 is always
    // legal), and the base "Add {B}" happens regardless.
    let mut game = Game::new();
    let battery = game.spawn_on_battlefield(PlayerId(0), card("Black Mana Battery"));

    activate(&mut game, battery, 1, 0).expect("removing no counters is a payable cost");

    assert_eq!(game.mana_in_pool(PlayerId(0), Color::Black), 1);
    assert_eq!(pool_total(&game, PlayerId(0)), 1, "and nothing else");
}

#[test]
fn black_mana_battery_cannot_remove_more_charge_counters_than_it_has() {
    // CR 602.2b: an activation cost that can't be fully paid makes the activation illegal —
    // removing two charge counters from a battery holding one is uncompletable.
    let mut game = Game::new();
    let battery = game.spawn_on_battlefield(PlayerId(0), card("Black Mana Battery"));
    store_a_charge_counter(&mut game, battery);

    assert_eq!(
        activate(&mut game, battery, 1, 2),
        Err(Reject::CannotActivate),
        "only one charge counter is on the battery"
    );
    assert_eq!(
        game.counters_of_kind(battery, CounterKind::Charge),
        1,
        "the rejected activation paid nothing"
    );
}

#[test]
fn each_mana_battery_adds_its_own_color() {
    // The five batteries are one cycle with one colored pip each — the mana produced is that
    // card's color, never generic colorless.
    for (name, color) in [
        ("White Mana Battery", Color::White),
        ("Blue Mana Battery", Color::Blue),
        ("Black Mana Battery", Color::Black),
        ("Red Mana Battery", Color::Red),
        ("Green Mana Battery", Color::Green),
    ] {
        let mut game = Game::new();
        let battery = game.spawn_on_battlefield(PlayerId(0), card(name));
        store_a_charge_counter(&mut game, battery);

        activate(&mut game, battery, 1, 1).expect("one charge counter is there to remove");

        assert_eq!(
            game.mana_in_pool(PlayerId(0), color),
            2,
            "{name} adds its own color: the base pip plus one per counter removed"
        );
        assert_eq!(
            pool_total(&game, PlayerId(0)),
            2,
            "{name}: and nothing else"
        );
    }
}
