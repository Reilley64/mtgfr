//! Legends (`leg`) section-C authoring wave — batch c5.

mod common;

use common::*;
use engine::*;

/// Pass until the stack is empty, so a spell plus the triggers it put on top all resolve.
fn resolve_stack(game: &mut Game) {
    let mut guard = 0;
    while !game.stack().is_empty() {
        game.submit(Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
        guard += 1;
        assert!(guard < 100, "the stack did not drain within a sane bound");
    }
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .expect("the spell is castable");
    resolve_stack(game);
}

fn activate(
    game: &mut Game,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

#[test]
fn riven_turnbull_taps_for_black() {
    let mut game = Game::new();
    let riven = game.spawn_on_battlefield(PlayerId(0), card("Riven Turnbull"));

    activate(&mut game, riven, 0, None).expect("{T}: Add {B}");

    assert_eq!(
        game.mana_in_pool(PlayerId(0), Color::Black),
        1,
        "{{T}}: Add {{B}} leaves one black mana in the pool"
    );
    assert!(
        game.is_tapped(riven),
        "the mana ability taps Riven Turnbull"
    );
}

#[test]
fn fire_sprites_converts_green_into_red() {
    let mut game = Game::new();
    let sprites = game.spawn_on_battlefield(PlayerId(0), card("Fire Sprites"));
    tap_basics(&mut game, "Forest", 1);

    activate(&mut game, sprites, 0, None).expect("{G}, {T}: Add {R}");

    assert_eq!(
        game.mana_in_pool(PlayerId(0), Color::Red),
        1,
        "the ability adds {{R}}"
    );
    assert_eq!(
        game.mana_in_pool(PlayerId(0), Color::Green),
        0,
        "the {{G}} in the activation cost was spent"
    );
    assert!(game.is_tapped(sprites), "the ability taps Fire Sprites");
}

#[test]
fn fire_sprites_cannot_activate_without_green_mana() {
    let mut game = Game::new();
    let sprites = game.spawn_on_battlefield(PlayerId(0), card("Fire Sprites"));

    assert!(
        activate(&mut game, sprites, 0, None).is_err(),
        "the {{G}} in the activation cost is unpaid"
    );
}

#[test]
fn pavel_maliki_pumps_himself_until_end_of_turn() {
    let mut game = Game::new();
    let pavel = game.spawn_on_battlefield(PlayerId(0), card("Pavel Maliki"));
    tap_basics(&mut game, "Swamp", 1);
    tap_basics(&mut game, "Mountain", 1);

    activate(&mut game, pavel, 0, None).expect("{B}{R}: Pavel Maliki gets +1/+0");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(pavel), game.toughness(pavel)),
        (6, 3),
        "printed 5/3 plus +1/+0"
    );

    pass_until_next_turn(&mut game);
    assert_eq!(
        (game.power(pavel), game.toughness(pavel)),
        (5, 3),
        "the pump wore off at end of turn"
    );
}

#[test]
fn ramses_overdark_destroys_an_enchanted_creature() {
    let mut game = Game::new();
    let ramses = game.spawn_on_battlefield(PlayerId(0), card("Ramses Overdark"));
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let flight = game.spawn_in_hand(PlayerId(0), card("Flight"));
    cast_and_resolve(&mut game, PlayerId(0), flight, Some(Target::Object(victim)));

    activate(&mut game, ramses, 0, Some(Target::Object(victim)))
        .expect("{T}: Destroy target enchanted creature");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(victim),
        Zone::Graveyard,
        "the enchanted creature was destroyed"
    );
    assert!(game.is_tapped(ramses), "the ability taps Ramses Overdark");
}

#[test]
fn ramses_overdark_cannot_target_an_unenchanted_creature() {
    let mut game = Game::new();
    let ramses = game.spawn_on_battlefield(PlayerId(0), card("Ramses Overdark"));
    let bare = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    assert!(
        !game
            .legal_targets(ramses, Some(0))
            .contains(&Target::Object(bare)),
        "a creature with no Aura on it isn't an enchanted creature"
    );
}

#[test]
fn dakkon_blackblades_power_and_toughness_track_your_lands() {
    let mut game = Game::new();
    let dakkon = game.spawn_on_battlefield(PlayerId(0), card("Dakkon Blackblade"));

    assert_eq!(
        (game.power(dakkon), game.toughness(dakkon)),
        (0, 0),
        "no lands, so */* reads 0/0"
    );

    game.spawn_on_battlefield(PlayerId(0), card("Swamp"));
    game.spawn_on_battlefield(PlayerId(0), card("Plains"));
    game.spawn_on_battlefield(PlayerId(0), card("Island"));
    assert_eq!(
        (game.power(dakkon), game.toughness(dakkon)),
        (3, 3),
        "three lands you control"
    );

    game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    assert_eq!(
        (game.power(dakkon), game.toughness(dakkon)),
        (3, 3),
        "an opponent's land isn't a land you control"
    );
}

#[test]
fn kobold_overlord_gives_your_other_kobolds_first_strike() {
    let mut game = Game::new();
    let overlord = game.spawn_on_battlefield(PlayerId(0), card("Kobold Overlord"));
    let yours = game.spawn_on_battlefield(PlayerId(0), card("Crimson Kobolds"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Crimson Kobolds"));
    let non_kobold = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert!(
        game.has_keyword(overlord, Keyword::FirstStrike),
        "Kobold Overlord has printed first strike"
    );
    assert!(
        game.has_keyword(yours, Keyword::FirstStrike),
        "another Kobold you control gains first strike"
    );
    assert!(
        !game.has_keyword(theirs, Keyword::FirstStrike),
        "a Kobold an opponent controls gains nothing"
    );
    assert!(
        !game.has_keyword(non_kobold, Keyword::FirstStrike),
        "a non-Kobold you control gains nothing"
    );
}

#[test]
fn solkanar_has_swampwalk() {
    let mut game = Game::new();
    let solkanar = game.spawn_on_battlefield(PlayerId(0), card("Sol'kanar the Swamp King"));

    assert!(
        game.has_keyword(solkanar, Keyword::Landwalk(BasicLandType::Swamp)),
        "Sol'kanar has swampwalk"
    );
}

#[test]
fn solkanar_gains_life_when_any_player_casts_a_black_spell() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Sol'kanar the Swamp King"));
    let start = game.life(PlayerId(0));

    // Your own black spell.
    let ritual = game.spawn_in_hand(PlayerId(0), card("Dark Ritual"));
    cast_and_resolve(&mut game, PlayerId(0), ritual, None);
    assert_eq!(
        game.life(PlayerId(0)),
        start + 1,
        "your black spell gains you 1 life"
    );

    // An opponent's black spell.
    let theirs = game.spawn_in_hand(PlayerId(1), card("Dark Ritual"));
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();
    cast_and_resolve(&mut game, PlayerId(1), theirs, None);
    assert_eq!(
        game.life(PlayerId(0)),
        start + 2,
        "an opponent's black spell gains you 1 life too"
    );
}

#[test]
fn solkanar_ignores_a_nonblack_spell() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Sol'kanar the Swamp King"));
    let start = game.life(PlayerId(0));

    let bears = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast_and_resolve(&mut game, PlayerId(0), bears, None);

    assert_eq!(
        game.life(PlayerId(0)),
        start,
        "a green spell isn't a black spell"
    );
}
