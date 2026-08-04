//! Legends (`leg`) grind, wave 11 slice B — combat restrictions, round 2.
//!
//! Increments 111 (`activate-only-before-the-combat-damage-step`),
//! 100 (`blocks-and-becomes-blocked-by-as-separate-triggers`),
//! 14 (`becomes-blocked-color-change`), 56 (`must-block-by-filter`),
//! 88 (`cant-be-targeted-by-wall-only-effects`) and 34 (`feint`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority across several turns can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..60 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Try `object`'s ability at index 0 with mana funded from thin air, returning the verdict.
fn try_activate(game: &mut Game, player: PlayerId, object: ObjectId) -> Result<Vec<Event>, Reject> {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

// ── increment 111: Angus Mackenzie ────────────────────────────────────────────────────

#[test]
fn angus_mackenzie_fogs_while_the_window_is_open() {
    // "{G}{W}{U}, {T}: Prevent all combat damage that would be dealt this turn. Activate only
    // before the combat damage step." Declare blockers is inside the window.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    let defender_before = game.life(PlayerId(1));
    attack_with(&mut game, vec![bear]);
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    try_activate(&mut game, PlayerId(0), angus).expect("declare blockers is before combat damage");
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        defender_before,
        "the fog prevented the bear's 2 damage",
    );
}

#[test]
fn an_unfogged_bear_does_reach_the_defending_player() {
    // The control for the test above: same board, same attack, Angus never activated. Without it
    // "life is unchanged" would also pass on an attack that never happened.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let _angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    let defender_before = game.life(PlayerId(1));
    attack_with(&mut game, vec![bear]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        defender_before - 2,
        "an unprevented 2/2 deals its 2",
    );
}

#[test]
fn angus_mackenzie_is_shut_out_from_the_combat_damage_step_on() {
    // The window is "before the combat damage step" — it closes at the first one, and stays shut
    // for the rest of the turn (the second main phase is still after it).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assert_eq!(
        try_activate(&mut game, PlayerId(0), angus),
        Err(Reject::WrongTiming),
        "the combat damage step itself is already too late",
    );

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    assert_eq!(
        try_activate(&mut game, PlayerId(0), angus),
        Err(Reject::WrongTiming),
        "and the window does not reopen later in the turn",
    );
}

#[test]
fn angus_mackenzie_is_open_before_combat_ever_starts() {
    // "Before the combat damage step" is the whole turn up to it, not just combat: upkeep counts.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert!(
        try_activate(&mut game, PlayerId(0), angus).is_ok(),
        "the precombat main phase is before the combat damage step",
    );
}
