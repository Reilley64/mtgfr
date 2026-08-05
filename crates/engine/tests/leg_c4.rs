//! Legends (`leg`) section-C authoring wave — batch c4.

mod common;

use common::*;
use engine::*;

// ── local drivers (game.rs keeps its own private copies of these) ─────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

fn activate(game: &mut Game, object: ObjectId, ability_index: usize, target: Option<Target>) {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
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

// ── the cards ─────────────────────────────────────────────────────────────────────────

#[test]
fn princess_lucrezia_taps_for_blue() {
    // "{T}: Add {U}."
    let mut game = Game::new();
    let princess = game.spawn_on_battlefield(PlayerId(0), card("Princess Lucrezia"));

    activate(&mut game, princess, 0, None);

    assert_eq!(game.mana_in_pool(PlayerId(0), Color::Blue), 1);
    assert!(game.is_tapped(princess), "the mana ability taps her");
}

#[test]
fn walking_dead_regenerates_for_black() {
    // "{B}: Regenerate this creature."
    let mut game = Game::new();
    let zombie = game.spawn_on_battlefield(PlayerId(0), card("Walking Dead"));
    let shock = game.spawn_in_hand(PlayerId(0), card("Shock"));

    game.fund_mana(PlayerId(0));
    activate(&mut game, zombie, 0, None);
    resolve_top_of_stack(&mut game);
    assert_eq!(game.regeneration_shields(zombie), 1, "one shield granted");

    // 2 damage is lethal to a 1/1, so the shield stands in for the destroy (CR 701.15b).
    cast_and_resolve(&mut game, shock, Some(Target::Object(zombie)));
    assert_eq!(
        game.zone_of(zombie),
        Zone::Battlefield,
        "the shield replaced the destroy"
    );
    assert!(game.is_tapped(zombie), "regeneration taps the creature");
    assert_eq!(game.marked_damage(zombie), 0, "regeneration heals damage");
}

#[test]
fn ghosts_of_the_damned_shrink_a_creatures_power() {
    // "{T}: Target creature gets -1/-0 until end of turn."
    let mut game = Game::new();
    let ghosts = game.spawn_on_battlefield(PlayerId(0), card("Ghosts of the Damned"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    activate(&mut game, ghosts, 0, Some(Target::Object(bear)));
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(ghosts), "the {{T}} cost tapped the Spirit");
    assert_eq!(game.power(bear), 1, "2 base - 1");
    assert_eq!(game.toughness(bear), 2, "toughness is untouched");
}

#[test]
fn amrou_kithkin_turns_away_power_three_blockers() {
    // "This creature can't be blocked by creatures with power 3 or greater."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let kithkin = game.spawn_on_battlefield(PlayerId(0), card("Amrou Kithkin"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // 3/3
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")); // 2/2

    attack_with(&mut game, vec![kithkin]);
    assert_eq!(
        block_with(&mut game, vec![(giant, kithkin)]),
        Err(Reject::IllegalDeclaration),
        "a 3-power blocker is exactly what the Kithkin turns away",
    );
    assert!(
        block_with(&mut game, vec![(bear, kithkin)]).is_ok(),
        "a 2-power blocker is still legal",
    );
}

#[test]
fn kei_takahashi_shields_a_creature_from_the_next_two_damage() {
    // "{T}: Prevent the next 2 damage that would be dealt to target creature this turn."
    let mut game = Game::new();
    let kei = game.spawn_on_battlefield(PlayerId(0), card("Kei Takahashi"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let first = game.spawn_in_hand(PlayerId(0), card("Shock"));
    let second = game.spawn_in_hand(PlayerId(0), card("Shock"));

    activate(&mut game, kei, 0, Some(Target::Object(bear)));
    resolve_top_of_stack(&mut game);

    cast_and_resolve(&mut game, first, Some(Target::Object(bear)));
    assert_eq!(game.marked_damage(bear), 0, "the shield ate both points");
    assert_eq!(game.zone_of(bear), Zone::Battlefield);

    // Two points of shield paid for two points of damage — the next Shock lands.
    cast_and_resolve(&mut game, second, Some(Target::Object(bear)));
    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "the shield is spent, so the second Shock kills the 2/2"
    );
}

#[test]
fn cat_warriors_walk_past_a_forest() {
    // "Forestwalk (This creature can't be blocked as long as defending player controls a Forest.)"
    let mut game = Game::new();
    stock_libraries(&mut game);
    let cats = game.spawn_on_battlefield(PlayerId(0), card("Cat Warriors"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Forest"));

    attack_with(&mut game, vec![cats]);
    assert_eq!(
        block_with(&mut game, vec![(bear, cats)]),
        Err(Reject::IllegalDeclaration),
        "the defending player controls a Forest, so nothing may block",
    );
}

#[test]
fn cat_warriors_are_blockable_without_a_forest() {
    // Forestwalk is conditional on the *defending player's* lands, not on the walker.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let cats = game.spawn_on_battlefield(PlayerId(0), card("Cat Warriors"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(0), card("Forest")); // the attacker's own Forest is irrelevant

    attack_with(&mut game, vec![cats]);
    assert!(
        block_with(&mut game, vec![(bear, cats)]).is_ok(),
        "no Forest on the defending side means no evasion",
    );
}

#[test]
fn emerald_dragonfly_buys_first_strike() {
    // "{G}{G}: This creature gains first strike until end of turn."
    let mut game = Game::new();
    let fly = game.spawn_on_battlefield(PlayerId(0), card("Emerald Dragonfly"));
    assert!(game.has_keyword(fly, Keyword::Flying), "printed flying");
    assert!(!game.has_keyword(fly, Keyword::FirstStrike));

    game.fund_mana(PlayerId(0));
    activate(&mut game, fly, 0, None);
    resolve_top_of_stack(&mut game);

    assert!(game.has_keyword(fly, Keyword::FirstStrike));
}

#[test]
fn mountain_yeti_walks_mountains() {
    // "Mountainwalk (This creature can't be blocked as long as defending player controls a
    // Mountain.)"
    let mut game = Game::new();
    stock_libraries(&mut game);
    let yeti = game.spawn_on_battlefield(PlayerId(0), card("Mountain Yeti"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Mountain"));

    attack_with(&mut game, vec![yeti]);
    assert_eq!(
        block_with(&mut game, vec![(bear, yeti)]),
        Err(Reject::IllegalDeclaration),
        "the defending player controls a Mountain",
    );
}

#[test]
fn mountain_yeti_cant_be_blocked_by_white() {
    // "Protection from white" — the block-side half of CR 702.16e, independent of mountainwalk.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let yeti = game.spawn_on_battlefield(PlayerId(0), card("Mountain Yeti"));
    let lions = game.spawn_on_battlefield(PlayerId(1), card("Savannah Lions"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![yeti]);
    assert_eq!(
        block_with(&mut game, vec![(lions, yeti)]),
        Err(Reject::IllegalDeclaration),
        "a white creature can't block a creature with protection from white",
    );
    assert!(
        block_with(&mut game, vec![(bear, yeti)]).is_ok(),
        "a green blocker is unaffected",
    );
}
