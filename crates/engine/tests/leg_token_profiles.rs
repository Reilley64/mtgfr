//! Legends (`leg`) grind — increment 97: token-profiles-without-a-scryfall-printing.

mod common;

use common::*;
use engine::*;

/// Enough cards that nobody decks out over the handful of turns these tests run.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Resolve the top of the stack by having every seat pass in succession (CR 117.4), returning
/// the events from the last pass — the one that actually resolves the spell/ability.
fn resolve_top_of_stack_events(game: &mut Game) -> Vec<Event> {
    let mut events = Vec::new();
    for _ in 0..game.player_count() {
        events = game
            .submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
    }
    events
}

fn created_token(events: &[Event]) -> ObjectId {
    events
        .iter()
        .find_map(|e| match e {
            Event::TokenCreated { token, .. } => Some(*token),
            _ => None,
        })
        .expect("activating the ability creates a token")
}

// ── Boris Devilboon — "Create a 1/1 black and red Demon creature token named Minor Demon." ──

#[test]
fn boris_devilboon_creates_a_minor_demon_token() {
    let mut game = Game::new();
    let boris = game.spawn_on_battlefield(PlayerId(0), card("Boris Devilboon"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: boris,
        ability_index: 0, // {2}{B}{R}, {T}: create a Minor Demon token.
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    let events = resolve_top_of_stack_events(&mut game);
    let demon = created_token(&events);

    assert_eq!(game.def_of(demon).name, "Minor Demon");
    assert_eq!(game.power(demon), 1);
    assert_eq!(game.toughness(demon), 1);
    assert!(
        game.colors_of(demon)[Color::Black.index()],
        "the token is black"
    );
    assert!(
        game.colors_of(demon)[Color::Red.index()],
        "the token is red"
    );
    assert_eq!(
        game.colors_of(demon).iter().filter(|&&c| c).count(),
        2,
        "the token is exactly black and red, no other colors"
    );
    assert!(game.def_of(demon).subtypes.contains(&"Demon"));
}

#[test]
fn the_minor_demon_token_ceases_to_exist_when_it_dies() {
    // CR 111.7: a token that leaves the battlefield ceases to exist rather than lingering
    // anywhere else — no Scryfall printing to key doesn't change that.
    let mut game = Game::new();
    let demon = game.spawn_on_battlefield(
        PlayerId(0),
        cards::get_token("leg-token-minor-demon").expect("Minor Demon token profile is loaded"),
    );

    let destroy = game.spawn_in_hand(PlayerId(0), card("Infernal Grasp"));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: destroy,
        target: Some(Target::Object(demon)),
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
    resolve_top_of_stack(&mut game);

    assert!(
        !game.live_object_ids().contains(&demon),
        "a dead token ceases to exist — it never lingers in the graveyard (CR 111.7)"
    );
}

// ── Serpent Generator — "Create a 1/1 colorless Snake artifact creature token. It has \"Whenever
// this creature deals damage to a player, that player gets a poison counter.\"" ──

#[test]
fn serpent_generator_creates_a_colorless_artifact_snake_token() {
    let mut game = Game::new();
    let generator = game.spawn_on_battlefield(PlayerId(0), card("Serpent Generator"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: generator,
        ability_index: 0, // {4}, {T}: create a poisonous Snake token.
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    let events = resolve_top_of_stack_events(&mut game);
    let snake = created_token(&events);

    assert_eq!(game.def_of(snake).name, "Snake");
    assert_eq!(game.power(snake), 1);
    assert_eq!(game.toughness(snake), 1);
    assert!(
        game.effective_types(snake).intersects(TypeSet::ARTIFACT),
        "the token is an artifact"
    );
    assert!(
        game.effective_types(snake).intersects(TypeSet::CREATURE),
        "the token is a creature"
    );
    assert!(
        game.colors_of(snake).iter().all(|&c| !c),
        "the token has no color pips and no stated colors — it's colorless (CR 111.4)"
    );
    assert!(game.def_of(snake).subtypes.contains(&"Snake"));
}

#[test]
fn the_serpent_generators_snake_token_poisons_the_player_it_damages() {
    // Same trigger shape as Pit Scorpion (increment #99): "Whenever this creature deals damage
    // to a player, that player gets a poison counter."
    let mut game = Game::with_players(4, 0);
    stock_libraries(&mut game);
    let snake = game.spawn_on_battlefield(
        PlayerId(0),
        cards::get_token("leg-token-snake-poison").expect("Snake (poison) token profile is loaded"),
    );

    attack_with(&mut game, vec![snake]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        1,
        "the damaged player got exactly one poison counter"
    );
}

// ── Master of the Hunt — "Create a 1/1 green Wolf creature token named Wolves of the Hunt." ──

#[test]
fn master_of_the_hunt_creates_a_wolves_of_the_hunt_token() {
    let mut game = Game::new();
    let master = game.spawn_on_battlefield(PlayerId(0), card("Master of the Hunt"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: master,
        ability_index: 0, // {2}{G}{G}: create a Wolves of the Hunt token.
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    let events = resolve_top_of_stack_events(&mut game);
    let wolf = created_token(&events);

    assert_eq!(game.def_of(wolf).name, "Wolves of the Hunt");
    assert_eq!(game.power(wolf), 1);
    assert_eq!(game.toughness(wolf), 1);
    assert!(
        game.colors_of(wolf)[Color::Green.index()],
        "the token is green"
    );
    assert!(game.def_of(wolf).subtypes.contains(&"Wolf"));
}

// ── #97's own change: `default_print` is optional for a token profile with no Scryfall printing ──

#[test]
fn pre_token_era_profiles_load_with_no_scryfall_printing() {
    // Legends predates printed token cards, so none of these three profiles has a real
    // `default_print` to key — the pool must load them anyway rather than panicking at process
    // start, and an empty `default_print` is the faithful "no printing exists" value (it already
    // renders as the card back client-side), not a gap to fill with a synthetic id.
    for id in [
        "leg-token-minor-demon",
        "leg-token-snake-poison",
        "leg-token-wolves-of-the-hunt",
    ] {
        let def = cards::get_token(id).unwrap_or_else(|| panic!("token profile {id} is loaded"));
        assert_eq!(
            def.default_print, "",
            "no Scryfall printing exists for {id}, so default_print stays empty"
        );
    }
}
