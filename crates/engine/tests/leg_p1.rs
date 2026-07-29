//! Legends (`leg`) section-C authoring wave — batch p1.

mod common;

use common::*;
use engine::*;

// ── local drivers (game.rs keeps its own private copies of these) ─────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..20 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
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
    .expect("the Aura is castable");
    resolve_top_of_stack(game);
}

#[test]
fn relic_barrier_taps_an_artifact_and_nothing_else() {
    // "{T}: Tap target artifact."
    let mut game = Game::new();
    let barrier = game.spawn_on_battlefield(PlayerId(0), card("Relic Barrier"));
    let icy = game.spawn_on_battlefield(PlayerId(1), card("Icy Manipulator"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    activate(&mut game, barrier, 0, Some(Target::Object(icy))).expect("a tap and an artifact");
    assert!(game.is_tapped(barrier), "the {{T}} cost taps the Barrier");
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(icy), "the targeted artifact is tapped");

    // A creature is not an artifact — the ability fizzles on resolution (CR 608.2b).
    game.untap(barrier);
    activate(&mut game, barrier, 0, Some(Target::Object(bears))).unwrap();
    resolve_top_of_stack(&mut game);

    assert!(!game.is_tapped(bears), "a creature is not a legal target");
}

#[test]
fn lifeblood_pays_you_only_for_an_opponents_mountain() {
    // "Whenever a Mountain an opponent controls becomes tapped, you gain 1 life."
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Lifeblood"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let their_forest = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Mountain"));

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: theirs,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(0)),
        21,
        "an opponent's Mountain tapping pays the Lifeblood's controller"
    );

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: their_forest,
    })
    .unwrap();
    game.submit(Intent::TapForMana {
        player: PlayerId(0),
        object: mine,
    })
    .unwrap();
    assert_eq!(
        game.life(PlayerId(0)),
        21,
        "an opponent's Forest and your own Mountain are both outside the trigger"
    );
}

#[test]
fn divine_transformation_grows_the_creature_it_enchants() {
    // "Enchant creature / Enchanted creature gets +3/+3."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Divine Transformation"));

    cast_and_resolve(&mut game, aura, Some(Target::Object(giant)));

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (6, 6),
        "a 3/3 with +3/+3 is a 6/6"
    );
}

#[test]
fn immolation_trades_toughness_for_power() {
    // "Enchant creature / Enchanted creature gets +2/-2."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Immolation"));

    cast_and_resolve(&mut game, aura, Some(Target::Object(giant)));

    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (5, 1),
        "a 3/3 with +2/-2 is a 5/1"
    );

    // A 2/2 taken to 4/0 dies to the toughness state-based action (CR 704.5a).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Immolation"));

    cast_and_resolve(&mut game, aura, Some(Target::Object(bears)));

    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "0 toughness puts the enchanted creature into the graveyard"
    );
}

#[test]
fn blight_destroys_the_land_it_enchants_when_that_land_taps() {
    // "Enchant land / When enchanted land becomes tapped, destroy it."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let untouched = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let blight = game.spawn_in_hand(PlayerId(0), card("Blight"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, blight, Some(Target::Object(theirs)));

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: untouched,
    })
    .unwrap();
    assert_eq!(
        game.zone_of(untouched),
        Zone::Battlefield,
        "only the land the Aura is attached to is watched"
    );

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: theirs,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(theirs),
        Zone::Graveyard,
        "the enchanted land is destroyed when it becomes tapped"
    );
}

#[test]
fn spirit_link_pays_its_controller_for_the_hosts_damage() {
    // "Enchant creature / Whenever enchanted creature deals damage, you gain that much life."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Spirit Link"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, aura, Some(Target::Object(giant)));
    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (3, 3),
        "Spirit Link grants no stats of its own"
    );

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: vec![(giant, Defender::Player(PlayerId(1)))],
    })
    .unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.life(PlayerId(1)), 17, "the host deals 3 combat damage");
    assert_eq!(
        game.life(PlayerId(0)),
        23,
        "the Aura's controller gains that much life"
    );
}
