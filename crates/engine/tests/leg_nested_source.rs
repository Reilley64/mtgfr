//! Legends (`leg`) grind — increment 98: nested-effects-lose-their-source.

mod common;

use common::*;
use engine::*;

/// Roll to player 0's own upkeep with the Cosmic Horror trigger on the stack. Libraries are
/// stacked first so nobody decks on the way there.
fn to_own_upkeep(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &[card("Grizzly Bears"), card("Grizzly Bears")]);
    }
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
}

fn pay_upkeep(game: &mut Game, pay: bool) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay,
        discard_cost: vec![],
    })
}

// ── Cosmic Horror: "At the beginning of your upkeep, destroy this creature unless you pay
// {3}{B}{B}{B}. If this creature is destroyed this way, it deals 7 damage to you." ─────────
//
// The pay-or-else penalty acts on the ability's own source through a *nested* `target = "this"`
// effect — `TargetSpec::ThisPermanent` (CR 115: a fixed reference, never a chosen target), which
// only ability placement used to settle.

#[test]
fn cosmic_horror_destroys_itself_when_the_upkeep_cost_goes_unpaid() {
    let mut game = Game::new();
    let horror = game.spawn_on_battlefield(PlayerId(0), card("Cosmic Horror"));

    to_own_upkeep(&mut game);
    resolve_top_of_stack(&mut game);
    pay_upkeep(&mut game, false).expect("declining is legal");

    assert_eq!(
        game.zone_of(horror),
        Zone::Graveyard,
        "the unpaid upkeep destroys this creature"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        13,
        "destroyed this way, it deals 7 damage to you"
    );
}

#[test]
fn cosmic_horror_paid_off_survives_and_deals_no_damage() {
    let mut game = Game::new();
    let horror = game.spawn_on_battlefield(PlayerId(0), card("Cosmic Horror"));

    to_own_upkeep(&mut game);
    resolve_top_of_stack(&mut game);
    game.fund_mana(PlayerId(0)); // mana empties each step — fund it here, at the pause.
    pay_upkeep(&mut game, true).expect("paying {3}{B}{B}{B} is legal");

    assert_eq!(
        game.zone_of(horror),
        Zone::Battlefield,
        "the paid upkeep destroys nothing"
    );
    assert_eq!(game.life(PlayerId(0)), 20, "and burns nobody");
}

#[test]
fn a_regenerated_cosmic_horror_was_not_destroyed_this_way_and_deals_no_damage() {
    let mut game = Game::new();
    let horror = game.spawn_on_battlefield(PlayerId(0), card("Cosmic Horror"));
    let regenerator = game.spawn_on_battlefield(PlayerId(0), card("Horror of Horrors"));
    let swamp = game.spawn_on_battlefield(PlayerId(0), card("Swamp"));

    to_own_upkeep(&mut game);
    // In response to the upkeep trigger: "Sacrifice a Swamp: Regenerate target black creature."
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: regenerator,
        ability_index: 0,
        target: Some(Target::Object(horror)),
        sacrifice: vec![swamp],
        discard_cost: vec![],
        x: 0,
    })
    .expect("Cosmic Horror is a black creature");
    resolve_top_of_stack(&mut game); // the regeneration shield
    resolve_top_of_stack(&mut game); // the upkeep trigger
    pay_upkeep(&mut game, false).expect("declining is legal");

    assert_eq!(
        game.zone_of(horror),
        Zone::Battlefield,
        "the regeneration shield replaced the destruction (CR 701.15)"
    );
    assert_eq!(
        game.regeneration_shields(horror),
        0,
        "the destruction really was attempted — the shield was spent replacing it (CR 701.15)"
    );
    assert!(game.is_tapped(horror), "regenerating taps it (CR 701.15a)");
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "it was never destroyed this way, so the 7-damage rider never happens"
    );
}
