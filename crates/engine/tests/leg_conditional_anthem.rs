//! Legends (`leg`) grind — increment 13: board-state-conditional-anthem.
//!
//! Four cards whose static P/T boost is gated on something the anthem has to re-read live
//! (CR 613.4): a count of creatures *you* control (Angelic Voices), a count an *opponent*
//! controls (Beasts of Bogardan, Ivory Guardians), and the affected creature's own combat state
//! (Arcades Sabboth). Every test asserts the board fact the boost buys — a death, a life total —
//! and every gate is asserted turning *off* again, not only on.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

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
    .expect("a legal cast");
    resolve_top_of_stack(game);
}

/// Shock `target` from player 0's hand — 2 damage, lethal to a 2/2 and survivable at 2/3.
fn shock(game: &mut Game, target: ObjectId) {
    let shock = game.spawn_in_hand(PlayerId(0), card("Shock"));
    cast_and_resolve(game, shock, Some(Target::Object(target)));
}

/// One priority pass, which is where state-based actions get checked (CR 704.3) — how a test
/// makes the engine notice that a permanent dropped into place turned an anthem off.
fn check_state(game: &mut Game) {
    let player = game.priority_holder();
    game.submit(Intent::PassPriority { player }).unwrap();
}

/// Swing player 0's `attackers` at player 1, run combat out, and report player 1's life loss.
fn damage_dealt_to_defender(game: &mut Game, attackers: Vec<ObjectId>) -> i32 {
    let before = game.life(PlayerId(1));
    attack_with(game, attackers);
    advance_until(game, |g| g.current_step() == Step::End);
    before - game.life(PlayerId(1))
}

// ── Angelic Voices ────────────────────────────────────────────────────────────────────
// "Creatures you control get +1/+1 as long as you control no nonartifact, nonwhite creatures."

#[test]
fn angelic_voices_pumps_until_a_nonwhite_creature_joins_your_board() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Angelic Voices"));
    let unicorn = game.spawn_on_battlefield(PlayerId(0), card("Pearled Unicorn"));

    shock(&mut game, unicorn);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Battlefield,
        "2 damage is not lethal to the pumped 3/3"
    );
    assert_eq!(game.marked_damage(unicorn), 2);

    // A green creature is a nonartifact, nonwhite creature, so the gate closes, the +1/+1 falls
    // away, and the already-marked damage becomes lethal (CR 704.5g).
    game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    check_state(&mut game);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Graveyard,
        "the Unicorn is a 2/2 again with 2 damage marked on it"
    );
}

#[test]
fn angelic_voices_looks_past_artifact_and_white_creatures() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Angelic Voices"));
    // Darksteel Myr is a colorless *artifact* creature: nonwhite, but the clause counts only
    // nonartifact ones.
    game.spawn_on_battlefield(PlayerId(0), card("Darksteel Myr"));
    let unicorn = game.spawn_on_battlefield(PlayerId(0), card("Pearled Unicorn"));

    shock(&mut game, unicorn);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Battlefield,
        "an artifact creature and a white creature both leave the gate open"
    );
}

#[test]
fn angelic_voices_counts_only_creatures_you_control() {
    let mut game = Game::new();
    // An opponent's green creature is not one *you* control, so it never closes the gate.
    game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(0), card("Angelic Voices"));
    let unicorn = game.spawn_on_battlefield(PlayerId(0), card("Pearled Unicorn"));

    shock(&mut game, unicorn);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Battlefield,
        "\"you control no ...\" reads your own board only"
    );
}

// ── Beasts of Bogardan ────────────────────────────────────────────────────────────────
// "Protection from red
//  This creature gets +1/+1 as long as an opponent controls a nontoken white permanent."

#[test]
fn beasts_of_bogardan_hits_for_four_while_an_opponent_controls_a_nontoken_white_permanent() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Pearled Unicorn"));
    let beasts = game.spawn_on_battlefield(PlayerId(0), card("Beasts of Bogardan"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![beasts]),
        4,
        "the printed 3/3 swings as a 4/4"
    );
}

#[test]
fn beasts_of_bogardan_hits_for_three_with_no_white_permanent_across_the_table() {
    let mut game = Game::new();
    let beasts = game.spawn_on_battlefield(PlayerId(0), card("Beasts of Bogardan"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![beasts]),
        3,
        "nothing white across the table, so the printed 3/3 swings"
    );
}

#[test]
fn beasts_of_bogardan_ignores_a_white_token() {
    let mut game = Game::new();
    game.spawn_token_on_battlefield(PlayerId(1), card("Pearled Unicorn"));
    let beasts = game.spawn_on_battlefield(PlayerId(0), card("Beasts of Bogardan"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![beasts]),
        3,
        "the clause reads \"nontoken white permanent\" — a token does not qualify"
    );
}

#[test]
fn beasts_of_bogardan_loses_the_boost_when_the_white_permanent_dies_mid_combat() {
    let mut game = Game::new();
    let unicorn = game.spawn_on_battlefield(PlayerId(1), card("Pearled Unicorn"));
    let beasts = game.spawn_on_battlefield(PlayerId(0), card("Beasts of Bogardan"));
    let before = game.life(PlayerId(1));

    attack_with(&mut game, vec![beasts]);
    assert_eq!(game.power(beasts), 4, "declared as a 4/4");
    // Combat damage is read off live power as the damage step begins (CR 510.1a), so killing the
    // last nontoken white permanent before then closes the gate in time.
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    shock(&mut game, unicorn);
    assert_eq!(game.zone_of(unicorn), Zone::Graveyard);

    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        before - game.life(PlayerId(1)),
        3,
        "the gate closed mid-combat, so only the printed 3 power connected"
    );
}

// ── Ivory Guardians ───────────────────────────────────────────────────────────────────
// "Protection from red
//  Creatures named Ivory Guardians get +1/+1 as long as an opponent controls a nontoken red
//  permanent."

#[test]
fn ivory_guardians_pump_only_creatures_named_ivory_guardians() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));
    let guardians = game.spawn_on_battlefield(PlayerId(0), card("Ivory Guardians"));
    let unicorn = game.spawn_on_battlefield(PlayerId(0), card("Pearled Unicorn"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![guardians, unicorn]),
        6,
        "the Guardians swing as a 4/4; the Unicorn is not named Ivory Guardians and stays a 2/2"
    );
}

#[test]
fn a_second_ivory_guardians_stacks_its_boost_on_the_first() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));
    let first = game.spawn_on_battlefield(PlayerId(0), card("Ivory Guardians"));
    game.spawn_on_battlefield(PlayerId(0), card("Ivory Guardians"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![first]),
        5,
        "each Guardians buffs every creature with the name, itself included"
    );
}

#[test]
fn ivory_guardians_lose_the_boost_when_the_red_permanent_dies_mid_combat() {
    let mut game = Game::new();
    let ogre = game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));
    let guardians = game.spawn_on_battlefield(PlayerId(0), card("Ivory Guardians"));
    let before = game.life(PlayerId(1));

    attack_with(&mut game, vec![guardians]);
    assert_eq!(game.power(guardians), 4, "declared as a 4/4");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    shock(&mut game, ogre);
    assert_eq!(game.zone_of(ogre), Zone::Graveyard);

    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        before - game.life(PlayerId(1)),
        3,
        "no nontoken red permanent left across the table, so only 3 power connected"
    );
}

#[test]
fn ivory_guardians_stay_printed_size_with_no_red_permanent_across_the_table() {
    let mut game = Game::new();
    let guardians = game.spawn_on_battlefield(PlayerId(0), card("Ivory Guardians"));

    assert_eq!(
        damage_dealt_to_defender(&mut game, vec![guardians]),
        3,
        "an empty opposing board leaves the gate closed"
    );
}

// ── Arcades Sabboth ───────────────────────────────────────────────────────────────────
// "Each untapped creature you control gets +0/+2 as long as it's not attacking."

#[test]
fn arcades_sabboth_toughens_an_untapped_creature_until_it_attacks() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));
    // Zephyr Falcon has vigilance, so attacking leaves it untapped — the only way to tell the
    // "not attacking" half of the clause apart from the "untapped" half.
    let damaged = game.spawn_on_battlefield(PlayerId(0), card("Zephyr Falcon"));
    let control = game.spawn_on_battlefield(PlayerId(0), card("Zephyr Falcon"));

    shock(&mut game, damaged);
    assert_eq!(
        game.zone_of(damaged),
        Zone::Battlefield,
        "2 damage is not lethal to the shielded 1/3"
    );

    attack_with(&mut game, vec![damaged, control]);
    assert!(
        !game.is_tapped(control),
        "vigilance keeps an attacking Falcon untapped, so `untapped_only` is not what fires here"
    );
    assert_eq!(game.toughness(control), 1, "attacking drops the +0/+2");
    assert_eq!(
        game.zone_of(damaged),
        Zone::Graveyard,
        "the boost fell off an untapped attacker, so its 2 marked damage turned lethal"
    );
}

#[test]
fn arcades_sabboth_skips_a_tapped_creature() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));
    let unicorn = game.spawn_on_battlefield(PlayerId(0), card("Pearled Unicorn"));

    shock(&mut game, unicorn);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Battlefield,
        "a 2/4 survives 2 damage"
    );

    game.tap(unicorn);
    check_state(&mut game);
    assert_eq!(
        game.zone_of(unicorn),
        Zone::Graveyard,
        "\"Each untapped creature\" — tapping ends the boost and the marked damage is lethal"
    );
}

#[test]
fn arcades_sabboth_flies_pumps_itself_and_toughens_itself() {
    let mut game = Game::new();
    let arcades = game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));

    assert!(game.has_keyword(arcades, Keyword::Flying));
    // The clause is "Each untapped creature you control", which the Dragon itself satisfies.
    assert_eq!(game.toughness(arcades), 9, "printed 7/7 plus its own +0/+2");

    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: arcades,
        ability_index: 2,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("{W} is payable");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.power(arcades), 7, "the pump is toughness only");
    assert_eq!(game.toughness(arcades), 10, "+0/+1 stacks on the anthem");
}

#[test]
fn arcades_sabboth_boosts_only_creatures_you_control() {
    let mut game = Game::new();
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Pearled Unicorn"));
    game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));

    shock(&mut game, theirs);
    assert_eq!(
        game.zone_of(theirs),
        Zone::Graveyard,
        "\"Each untapped creature *you control*\" stops at your own board, so the 2/2 dies"
    );
}
