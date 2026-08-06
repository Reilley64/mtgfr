//! Legends (`leg`) grind — increment 119: divide-combat-damage-in-the-damage-step.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..10 {
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
    .unwrap();
    resolve_top_of_stack(game);
}

/// Spawn `count` 2/2 Grizzly Bears for the defending player.
fn bears(game: &mut Game, count: usize) -> Vec<ObjectId> {
    (0..count)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")))
        .collect()
}

/// Spawn `count` 0/1 Crookshank Kobolds for the defending player.
fn kobolds(game: &mut Game, count: usize) -> Vec<ObjectId> {
    (0..count)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Crookshank Kobolds")))
        .collect()
}

fn pending_assigner(game: &Game) -> PlayerId {
    let Some(PendingChoice::AssignCombatDamage { player, .. }) = game.pending_choice() else {
        panic!(
            "a multi-blocked attacker divides its damage; got {:?}",
            game.pending_choice()
        );
    };
    player
}

// ── CR 510.1a: the division happens in the combat damage step ──────────────────────────

#[test]
fn a_pump_cast_after_blockers_are_declared_is_divided_among_the_blockers() {
    // CR 509.2 chooses the damage *assignment order* at declare blockers; CR 510.1a divides the
    // actual amounts in the combat damage step, reading the attacker's power there. So a Giant
    // Growth cast in response to the blocks is damage the attacker can really spend on them.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(0), card("Giant Growth"));
    let blockers = bears(&mut game, 2);

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, blockers.iter().map(|&b| (b, attacker)).collect()).unwrap();
    assert!(
        game.pending_choice().is_none(),
        "declaring blockers no longer locks the division in; got {:?}",
        game.pending_choice(),
    );
    cast_and_resolve(&mut game, growth, Some(Target::Object(attacker)));
    assert_eq!(game.power(attacker), 5, "2/2 plus Giant Growth's +3/+3");

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    let player = pending_assigner(&game);
    assert!(
        game.submit(Intent::AssignDamage {
            player,
            assignment: vec![(blockers[0], 1), (blockers[1], 1)],
        })
        .is_err(),
        "the unpumped total is short of the power the attacker has in this step",
    );
    game.submit(Intent::AssignDamage {
        player,
        assignment: vec![(blockers[0], 2), (blockers[1], 3)],
    })
    .expect("the whole pumped 5 is divisible among the blockers");

    assert_eq!(
        (game.zone_of(blockers[0]), game.zone_of(blockers[1])),
        (Zone::Graveyard, Zone::Graveyard),
        "2 and 3 are both lethal to a 2/2 — the pump reaches the board, not just the P/T",
    );
}

#[test]
fn the_division_is_offered_only_the_blockers_still_blocking_in_the_damage_step() {
    // CR 510.1a reads "each creature blocking it" in the combat damage step: a blocker killed
    // while the blocks stood is not a creature to divide damage among, and with one blocker left
    // there is nothing to divide at all.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    let blockers = bears(&mut game, 2);

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, blockers.iter().map(|&b| (b, attacker)).collect()).unwrap();
    cast_and_resolve(&mut game, bolt, Some(Target::Object(blockers[0])));
    assert_eq!(
        game.zone_of(blockers[0]),
        Zone::Graveyard,
        "the Bolt killed one blocker"
    );

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(
        game.pending_choice().is_none(),
        "one blocker left is no division; got {:?}",
        game.pending_choice(),
    );
    assert_eq!(
        (game.zone_of(attacker), game.zone_of(blockers[1])),
        (Zone::Graveyard, Zone::Graveyard),
        "the attacker's whole 2 went to the one blocker left, with no choice raised, and it \
         traded with it",
    );
}

#[test]
fn every_multi_blocked_attacker_divides_before_any_combat_damage_is_dealt() {
    // CR 510.1: the divisions and the dealing are one turn-based action — both attackers divide,
    // with no priority in between, and then all of that damage is dealt at once.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let first = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blockers = kobolds(&mut game, 4);

    attack_with(&mut game, vec![first, second]);
    block_with(
        &mut game,
        vec![
            (blockers[0], first),
            (blockers[1], first),
            (blockers[2], second),
            (blockers[3], second),
        ],
    )
    .unwrap();

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    let player = pending_assigner(&game);
    game.submit(Intent::AssignDamage {
        player,
        assignment: vec![(blockers[0], 1), (blockers[1], 1)],
    })
    .expect("the first attacker's 2 power, split one apiece");
    assert!(
        game.zone_of(blockers[0]) == Zone::Battlefield,
        "no damage is dealt until the last attacker has divided",
    );

    let player = pending_assigner(&game);
    game.submit(Intent::AssignDamage {
        player,
        assignment: vec![(blockers[2], 1), (blockers[3], 1)],
    })
    .expect("the second attacker divides in the same turn-based action");

    for blocker in blockers {
        assert_eq!(
            game.zone_of(blocker),
            Zone::Graveyard,
            "1 damage is lethal to a 0/1 Kobold",
        );
    }
}

#[test]
fn a_first_striking_attacker_divides_in_the_first_strike_damage_step() {
    // Rapid Fire: "Target creature gains first strike until end of turn. If it doesn't have
    // rampage, that creature gains rampage 2 until end of turn." CR 510.5 gives first strike its
    // own combat damage step, and CR 510.1a divides there — so the rampage-pumped 4 is spent on
    // the blockers before either of them can deal damage back.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let rapid_fire = game.spawn_in_hand(PlayerId(0), card("Rapid Fire"));
    let blockers = bears(&mut game, 2);

    cast_and_resolve(&mut game, rapid_fire, Some(Target::Object(attacker)));
    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, blockers.iter().map(|&b| (b, attacker)).collect()).unwrap();
    resolve_top_of_stack(&mut game); // the rampage trigger
    assert_eq!(
        game.power(attacker),
        4,
        "2/2 plus rampage 2 for the one blocker beyond the first"
    );

    advance_until(&mut game, |g| {
        g.current_step() == Step::FirstStrikeCombatDamage
    });
    let player = pending_assigner(&game);
    game.submit(Intent::AssignDamage {
        player,
        assignment: vec![(blockers[0], 2), (blockers[1], 2)],
    })
    .expect("the first-strike batch divides the attacker's current power");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        (game.zone_of(blockers[0]), game.zone_of(blockers[1])),
        (Zone::Graveyard, Zone::Graveyard),
        "both blockers die to first-strike damage",
    );
    assert_eq!(
        game.marked_damage(attacker),
        0,
        "dead blockers never reach the normal combat damage step",
    );
}

#[test]
fn a_concede_during_a_division_still_deals_the_rest_of_the_batch() {
    // A player may quit whatever the game is waiting on (CR 104.3a), even a division that is theirs
    // to make — CR 702.22j gives a banding blocker's controller the attacker's division. The
    // division goes with them, and so do their creatures (CR 800.4a), but the combat damage step's
    // turn-based action belongs to the game: the attacker aimed at a third seat still connects.
    let mut game = Game::with_players(4, 0);
    for player in 0..4 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
    let at_bander = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let at_third = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wolves = game.spawn_on_battlefield(PlayerId(1), card("Timber Wolves")); // banding
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let before = game.life(PlayerId(2));

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: vec![
            (at_bander, Defender::Player(PlayerId(1))),
            (at_third, Defender::Player(PlayerId(2))),
        ],
    })
    .unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(1),
        blocks: vec![(wolves, at_bander), (bears, at_bander)],
    })
    .unwrap();
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(2),
        blocks: vec![],
    })
    .unwrap();

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assert_eq!(
        pending_assigner(&game),
        PlayerId(1),
        "the banding blocker's controller divides"
    );
    game.submit(Intent::Concede {
        player: PlayerId(1),
    })
    .unwrap();

    assert!(
        game.pending_choice().is_none(),
        "the quitter's division left with them; got {:?}",
        game.pending_choice(),
    );
    assert_eq!(
        game.life(PlayerId(2)),
        before - 2,
        "the batch is still dealt: the other attacker's damage isn't forfeited too",
    );
}

#[test]
fn a_zero_power_attacker_is_asked_for_no_division() {
    // CR 510.1a: a creature with 0 or less power assigns no combat damage, so a multi-blocked one
    // has nothing to divide. Asking anyway would park the damage step on a choice whose only
    // legal answer is all zeroes — and, once the power is negative, on one with no legal answer.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let kobold = game.spawn_on_battlefield(PlayerId(0), card("Crookshank Kobolds"));
    let blockers = bears(&mut game, 2);

    attack_with(&mut game, vec![kobold]);
    block_with(&mut game, blockers.iter().map(|&b| (b, kobold)).collect()).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(kobold),
        Zone::Graveyard,
        "the 0/1 Kobold dies to its blockers without ever dividing damage",
    );
    for blocker in blockers {
        assert_eq!(
            game.marked_damage(blocker),
            0,
            "a 0-power attacker deals nothing"
        );
    }
}
