//! Legends (`leg`) grind — increment 1: rampage-n.

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

// ── rampage N (CR 702.23) ─────────────────────────────────────────────────────────────

#[test]
fn craw_giant_rampage_two_pumps_for_each_blocker_beyond_the_first() {
    // "Rampage 2 (Whenever this creature becomes blocked, it gets +2/+2 until end of turn for
    // each creature blocking it beyond the first.)" — three blockers is two beyond the first.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let blockers = bears(&mut game, 3);

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (10, 8),
        "6/4 plus 2 x rampage 2",
    );
}

#[test]
fn craw_giant_rampage_gives_nothing_against_a_lone_blocker() {
    // CR 702.23a counts blockers *beyond the first*, so a single blocker is worth zero.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let blocker = bears(&mut game, 1)[0];

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(blocker, giant)]).unwrap();
    assert_eq!(
        game.stack().len(),
        1,
        "rampage still triggers on becoming blocked"
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (6, 4),
        "printed 6/4, unchanged",
    );
}

#[test]
fn craw_giant_rampage_triggers_once_however_many_creatures_block() {
    // CR 702.23a is one trigger per instance of rampage, not one per blocker.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let blockers = bears(&mut game, 3);

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();

    assert_eq!(
        game.stack().len(),
        1,
        "one rampage trigger for three blockers"
    );
}

#[test]
fn aerathi_berserker_rampage_three_scales_by_its_own_n() {
    // "Rampage 3 (… it gets +3/+3 until end of turn for each creature blocking it beyond the
    // first.)" — two blockers is one beyond the first.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let berserker = game.spawn_on_battlefield(PlayerId(0), card("Aerathi Berserker"));
    let blockers = bears(&mut game, 2);

    attack_with(&mut game, vec![berserker]);
    block_with(
        &mut game,
        blockers.iter().map(|&b| (b, berserker)).collect(),
    )
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(berserker), game.toughness(berserker)),
        (5, 7),
        "2/4 plus 1 x rampage 3",
    );
}

#[test]
fn craw_giant_rampage_counts_the_blockers_alive_when_the_trigger_resolves() {
    // CR 702.23b: the bonus is calculated once, *when the triggered ability resolves*. Killing a
    // blocker with the trigger still on the stack shrinks the count it sees.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let blockers = bears(&mut game, 3);
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();
    cast_and_resolve(&mut game, bolt, Some(Target::Object(blockers[0])));
    assert_eq!(
        game.zone_of(blockers[0]),
        Zone::Graveyard,
        "the Bolt killed a blocker"
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (8, 6),
        "two blockers left, so 6/4 plus 1 x rampage 2",
    );
}

#[test]
fn craw_giant_rampage_bonus_survives_a_blocker_dying_after_it_resolves() {
    // CR 702.23b: once the trigger has resolved the bonus is locked in — removing a blocker
    // later in combat doesn't change it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let blockers = bears(&mut game, 3);
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();
    resolve_top_of_stack(&mut game);
    cast_and_resolve(&mut game, bolt, Some(Target::Object(blockers[0])));

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (10, 8),
        "still 6/4 plus 2 x rampage 2",
    );
}

#[test]
fn rampage_creatures_carry_their_printed_n() {
    let mut game = Game::new();
    for (name, n) in [
        ("Aerathi Berserker", 3),
        ("Chromium", 2),
        ("Craw Giant", 2),
        ("Frost Giant", 2),
        ("Hunding Gjornersen", 1),
        ("Marhault Elsdragon", 1),
        ("Wolverine Pack", 2),
    ] {
        let object = game.spawn_on_battlefield(PlayerId(0), card(name));
        assert!(
            game.has_keyword(object, Keyword::Rampage(n)),
            "{name} has rampage {n}",
        );
    }
}

// ── Rapid Fire ────────────────────────────────────────────────────────────────────────

#[test]
fn rapid_fire_grants_first_strike_and_rampage_two_to_a_creature_without_rampage() {
    // "Target creature gains first strike until end of turn. If it doesn't have rampage, that
    // creature gains rampage 2 until end of turn."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let rapid_fire = game.spawn_in_hand(PlayerId(0), card("Rapid Fire"));
    let blockers = bears(&mut game, 3);

    cast_and_resolve(&mut game, rapid_fire, Some(Target::Object(attacker)));
    assert!(
        game.has_keyword(attacker, Keyword::FirstStrike),
        "gains first strike"
    );
    assert!(
        game.has_keyword(attacker, Keyword::Rampage(2)),
        "gains rampage 2"
    );

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, blockers.iter().map(|&b| (b, attacker)).collect()).unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(attacker), game.toughness(attacker)),
        (6, 6),
        "2/2 plus 2 x rampage 2",
    );
}

#[test]
fn rapid_fire_does_not_stack_a_second_rampage_on_a_creature_that_already_has_one() {
    // "If it doesn't have rampage" — Craw Giant does, so it keeps only its printed rampage 2.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Craw Giant"));
    let rapid_fire = game.spawn_in_hand(PlayerId(0), card("Rapid Fire"));
    let blockers = bears(&mut game, 3);

    cast_and_resolve(&mut game, rapid_fire, Some(Target::Object(giant)));
    assert!(
        game.has_keyword(giant, Keyword::FirstStrike),
        "first strike lands regardless"
    );

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();
    assert_eq!(
        game.stack().len(),
        1,
        "one rampage instance, so one trigger"
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (10, 8),
        "6/4 plus 2 x rampage 2, not a doubled bonus",
    );
}

// ── the rampage bonus reaches the blockers (increment 119) ─────────────────────────────

/// Frost Giant is 4/4 rampage 2. Blocked by two 2/2 Bears its power becomes 6 (CR 702.23), and
/// CR 510.1a divides its combat damage in the combat damage step, reading the power it has then —
/// so all six points are the Giant's to spend on its blockers.
#[test]
fn rampage_bonus_is_divided_among_the_blockers_in_the_combat_damage_step() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Frost Giant"));
    let blockers = bears(&mut game, 2);

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, blockers.iter().map(|&b| (b, giant)).collect()).unwrap();
    resolve_top_of_stack(&mut game); // the rampage trigger
    assert_eq!(game.power(giant), 6, "4/4 plus 1 x rampage 2");

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    let Some(PendingChoice::AssignCombatDamage { player, .. }) = game.pending_choice() else {
        panic!("a multi-blocked attacker divides its damage in the combat damage step");
    };
    assert!(
        game.submit(Intent::AssignDamage {
            player,
            assignment: vec![(blockers[0], 2), (blockers[1], 2)],
        })
        .is_err(),
        "the unpumped total is short of the power the Giant has in this step",
    );

    game.submit(Intent::AssignDamage {
        player,
        assignment: vec![(blockers[0], 3), (blockers[1], 3)],
    })
    .expect("the whole rampage-pumped 6 is divisible");

    assert_eq!(
        (game.zone_of(blockers[0]), game.zone_of(blockers[1])),
        (Zone::Graveyard, Zone::Graveyard),
        "3 apiece is lethal to both 2/2 Bears — the rampage bonus lands as damage",
    );
}
