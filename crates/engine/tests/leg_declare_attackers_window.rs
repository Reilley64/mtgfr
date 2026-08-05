//! Legends (`leg`) grind — increment 114: cast-during-any-players-declare-attackers.

mod common;

use common::*;
use engine::*;

fn cast(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(player);
    game.submit(Intent::Cast {
        player,
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
}

/// Stack every library so a test can roll through whole turns without decking anyone.
fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 10]);
    }
}

/// Roll to player 1's declare attackers step, swing `attacker` at player 2, and pass priority
/// around to player 0 — a seat that is neither the attacking player nor the defending one.
fn attack_on_player_ones_turn(game: &mut Game, attacker: ObjectId) {
    stock_libraries(game);
    pass_until_next_turn(game);
    assert_eq!(game.active_player(), PlayerId(1), "player 1 is attacking");
    advance_until(game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(attacker, Defender::Player(PlayerId(2)))],
    })
    .expect("player 1 declares an attacker");
    while game.priority_holder() != PlayerId(0) {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
    assert_eq!(
        game.current_step(),
        Step::DeclareAttackers,
        "still inside the declare attackers step",
    );
}

/// "Cast this spell only during the declare attackers step." (CR 601.3e) — no "your" in the
/// printed restriction, so every player's declare attackers step is a window (CR 506.3: the step
/// belongs to the turn, but priority in it goes around the table).
#[test]
fn teleport_is_castable_during_another_players_declare_attackers_step() {
    let mut game = Game::with_players(4, 0);
    let attacker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let teleport = game.spawn_in_hand(PlayerId(0), card("Teleport"));

    attack_on_player_ones_turn(&mut game, attacker);

    cast(
        &mut game,
        PlayerId(0),
        teleport,
        Some(Target::Object(attacker)),
    )
    .expect("someone else's declare attackers step is Teleport's window too");
}

/// "Target creature can't be blocked this turn." — cast in another player's combat, the creature
/// Teleport resolved on walks past the defending player's blocker.
#[test]
fn teleport_makes_its_target_unblockable_for_the_turn() {
    let mut game = Game::with_players(4, 0);
    let attacker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(2), card("Grizzly Bears"));
    let teleport = game.spawn_in_hand(PlayerId(0), card("Teleport"));
    let before = game.life(PlayerId(2));

    attack_on_player_ones_turn(&mut game, attacker);
    cast(
        &mut game,
        PlayerId(0),
        teleport,
        Some(Target::Object(attacker)),
    )
    .expect("the attacking creature is a legal target");
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    assert!(
        game.submit(Intent::DeclareBlockers {
            player: PlayerId(2),
            blocks: vec![(blocker, attacker)],
        })
        .is_err(),
        "the defending player can't block a creature Teleport made unblockable",
    );

    game.submit(Intent::DeclareBlockers {
        player: PlayerId(2),
        blocks: vec![],
    })
    .expect("declaring no blockers is still legal");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(2)),
        before - 2,
        "the unblockable attacker connects",
    );
}

/// "…only during the declare attackers step": one step, not the whole turn — a main phase is
/// closed even on the caster's own turn.
#[test]
fn teleport_is_locked_out_of_a_main_phase() {
    let mut game = Game::with_players(4, 0);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let teleport = game.spawn_in_hand(PlayerId(0), card("Teleport"));

    assert!(
        cast(
            &mut game,
            PlayerId(0),
            teleport,
            Some(Target::Object(bears)),
        )
        .is_err(),
        "no attackers have been declared, so Teleport has no window",
    );
}

/// Camouflage prints "Cast this spell only during **your** declare attackers step" — the narrow
/// half of the same family, and still closed in someone else's combat.
#[test]
fn camouflage_is_still_locked_out_of_another_players_declare_attackers_step() {
    let mut game = Game::with_players(4, 0);
    let attacker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let camo = game.spawn_in_hand(PlayerId(0), card("Camouflage"));

    attack_on_player_ones_turn(&mut game, attacker);

    assert!(
        cast(&mut game, PlayerId(0), camo, None).is_err(),
        "Camouflage hides your own attackers, so someone else's combat is never its window",
    );
}
