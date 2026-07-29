//! Legends (`leg`) section-C authoring wave — batch c1.

mod common;

use common::*;
use engine::*;

fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

#[test]
fn sunastian_falconer_taps_for_two_colorless() {
    // "{T}: Add {C}{C}."
    let mut game = Game::new();
    let falconer = game.spawn_on_battlefield(PlayerId(0), card("Sunastian Falconer"));

    activate(&mut game, PlayerId(0), falconer, 0, None).expect("the mana ability activates");

    assert!(game.is_tapped(falconer), "the {{T}} cost taps the Falconer");
    assert_eq!(
        game.colorless_in_pool(PlayerId(0)),
        2,
        "two colorless mana, not two of any color"
    );
    assert_eq!(pool_total(&game, PlayerId(0)), 2, "and nothing else");
}

#[test]
fn spinal_villain_destroys_a_blue_creature_and_cannot_touch_a_green_one() {
    // "{T}: Destroy target blue creature."
    let mut game = Game::new();
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    // A green Grizzly Bears is not a blue creature, so the ability fizzles for want of a legal
    // target on resolution (CR 608.2b).
    activate(
        &mut game,
        PlayerId(0),
        villain,
        0,
        Some(Target::Object(bears)),
    )
    .unwrap();
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(bears),
        Zone::Battlefield,
        "the green Bears survive"
    );

    let mut game = Game::new();
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    let drake = game.spawn_on_battlefield(PlayerId(1), card("Azure Drake"));

    activate(
        &mut game,
        PlayerId(0),
        villain,
        0,
        Some(Target::Object(drake)),
    )
    .expect("a blue Azure Drake is a legal target");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.zone_of(drake), Zone::Graveyard, "the blue Drake dies");
}

#[test]
fn ragnar_regenerates_a_target_creature() {
    // "{G}{W}{U}, {T}: Regenerate target creature."
    let mut game = Game::new();
    let ragnar = game.spawn_on_battlefield(PlayerId(0), card("Ragnar"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.fund_mana(PlayerId(0));

    activate(
        &mut game,
        PlayerId(0),
        ragnar,
        0,
        Some(Target::Object(bears)),
    )
    .expect("{G}{W}{U} and a tap");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.regeneration_shields(bears),
        1,
        "the target gets one regeneration shield"
    );
    assert!(game.is_tapped(ragnar), "the {{T}} cost taps Ragnar");
}

#[test]
fn pradesh_gypsies_strip_two_power_from_a_creature() {
    // "{1}{G}, {T}: Target creature gets -2/-0 until end of turn."
    let mut game = Game::new();
    let gypsies = game.spawn_on_battlefield(PlayerId(0), card("Pradesh Gypsies"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.fund_mana(PlayerId(0));

    activate(
        &mut game,
        PlayerId(0),
        gypsies,
        0,
        Some(Target::Object(bears)),
    )
    .expect("{1}{G} and a tap");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.power(bears), 0, "2/2 becomes 0/2");
    assert_eq!(game.toughness(bears), 2, "toughness is untouched");
}

#[test]
fn righteous_avengers_walk_past_a_defender_who_controls_a_plains() {
    // "Plainswalk (This creature can't be blocked as long as defending player controls a Plains.)"
    let mut game = Game::new();
    let avengers = game.spawn_on_battlefield(PlayerId(0), card("Righteous Avengers"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![avengers]);
    assert!(
        block_with(&mut game, vec![(blocker, avengers)]).is_ok(),
        "plainswalk is inert against a defender who controls no Plains"
    );

    let mut game = Game::new();
    let avengers = game.spawn_on_battlefield(PlayerId(0), card("Righteous Avengers"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Plains"));

    attack_with(&mut game, vec![avengers]);
    assert!(
        block_with(&mut game, vec![(blocker, avengers)]).is_err(),
        "the defender's Plains turns plainswalk on"
    );
}

#[test]
fn killer_bees_pump_themselves_once_per_green_mana() {
    // "{G}: This creature gets +1/+1 until end of turn."
    let mut game = Game::new();
    let bees = game.spawn_on_battlefield(PlayerId(0), card("Killer Bees"));
    game.fund_mana(PlayerId(0));

    assert_eq!((game.power(bees), game.toughness(bees)), (0, 1), "base 0/1");

    for expected in [(1, 2), (2, 3)] {
        activate(&mut game, PlayerId(0), bees, 0, None).expect("{G} buys a pump");
        resolve_top_of_stack(&mut game);
        assert_eq!((game.power(bees), game.toughness(bees)), expected);
    }
}

#[test]
fn pixie_queen_grants_flying_to_a_target_creature() {
    // "{G}{G}{G}, {T}: Target creature gains flying until end of turn."
    let mut game = Game::new();
    let queen = game.spawn_on_battlefield(PlayerId(0), card("Pixie Queen"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.fund_mana(PlayerId(0));

    assert!(
        !game.has_keyword(bears, Keyword::Flying),
        "Grizzly Bears are grounded"
    );

    activate(
        &mut game,
        PlayerId(0),
        queen,
        0,
        Some(Target::Object(bears)),
    )
    .expect("{G}{G}{G} and a tap");
    resolve_top_of_stack(&mut game);

    assert!(game.has_keyword(bears, Keyword::Flying), "and now they fly");
    assert!(game.is_tapped(queen), "the {{T}} cost taps the Queen");
}
