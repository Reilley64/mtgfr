//! Legends (`leg`) section-C authoring wave — batch p2.

mod common;

use common::*;
use engine::*;

// ── local drivers (game.rs keeps its own private copies of these) ─────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        game.stack_library(PlayerId(player), &vec![card("Grizzly Bears"); 12]);
    }
}

fn activate(
    game: &mut Game,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
    sacrifice: Vec<ObjectId>,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index,
        target,
        sacrifice,
        discard_cost: vec![],
        x: 0,
    })
}

fn cast(game: &mut Game, object: ObjectId, target: Option<Target>) -> Result<Vec<Event>, Reject> {
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
}

/// Cast `object` off an unlimited pool and resolve it.
fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) {
    game.fund_mana(PlayerId(0));
    cast(game, object, target).unwrap();
    resolve_top_of_stack(game);
}

/// Roll forward to player 0's *next* upkeep (the constructor parks at Main1).
fn advance_to_your_next_upkeep(game: &mut Game) {
    pass_until_next_turn(game); // into player 1's turn
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
}

// ── Greed: "{B}, Pay 2 life: Draw a card." ─────────────────────────────────────────────

#[test]
fn greed_draws_a_card_for_black_and_two_life() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let greed = game.spawn_on_battlefield(PlayerId(0), card("Greed"));
    game.fund_mana(PlayerId(0));
    let hand_before = game.hand(PlayerId(0)).len();
    let library_before = game.library_size(PlayerId(0));
    let life_before = game.life(PlayerId(0));

    activate(&mut game, greed, 0, None, vec![]).expect("{B} and 2 life are payable");
    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 2,
        "the 2 life is paid as a cost, on activation, not on resolution"
    );

    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.hand(PlayerId(0)).len(),
        hand_before + 1,
        "the ability drew a card"
    );
    assert_eq!(
        game.library_size(PlayerId(0)),
        library_before - 1,
        "the card came off the library"
    );
}

// ── Planar Gate: "Creature spells you cast cost {2} less to cast." ────────────────────

#[test]
fn planar_gate_shaves_two_generic_off_a_creature_spell_only() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Planar Gate"));

    // Scathe Zombies is {2}{B}; {2} off leaves the {B} pip, payable by a single Swamp.
    let zombies = game.spawn_in_hand(PlayerId(0), card("Scathe Zombies"));
    tap_basics(&mut game, "Swamp", 1);
    cast(&mut game, zombies, None).expect("the reducer leaves only {B} to pay");
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(zombies),
        Zone::Battlefield,
        "the discounted creature spell resolved"
    );

    // Bad Moon is {1}{B} — a noncreature spell, so the Gate does nothing for it.
    let moon = game.spawn_in_hand(PlayerId(0), card("Bad Moon"));
    tap_basics(&mut game, "Swamp", 1);
    assert!(
        cast(&mut game, moon, None).is_err(),
        "an enchantment spell gets no discount from a creature-spell reducer"
    );
}

// ── Life Chisel: "Sacrifice a creature: You gain life equal to the sacrificed creature's
// toughness. Activate only during your upkeep." ───────────────────────────────────────

#[test]
fn life_chisel_gains_the_sacrificed_creatures_toughness_during_your_upkeep() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let chisel = game.spawn_on_battlefield(PlayerId(0), card("Life Chisel"));
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Stone")); // 0/8
    let life_before = game.life(PlayerId(0));

    advance_to_your_next_upkeep(&mut game);
    activate(&mut game, chisel, 0, None, vec![wall]).expect("your upkeep is the ability's window");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(wall),
        Zone::Graveyard,
        "a creature was sacrificed to pay the cost"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        life_before + 8,
        "life gained equals the sacrificed creature's toughness (8), not its power (0)"
    );
}

#[test]
fn life_chisel_cannot_be_activated_outside_your_upkeep() {
    let mut game = Game::new();
    let chisel = game.spawn_on_battlefield(PlayerId(0), card("Life Chisel"));
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Stone"));

    // The constructor parks at Main1 — an ordinary activation window for anything else.
    assert!(
        activate(&mut game, chisel, 0, None, vec![wall]).is_err(),
        "\"Activate only during your upkeep\" closes the main phase"
    );
    assert_eq!(
        game.zone_of(wall),
        Zone::Battlefield,
        "the rejected activation sacrifices nothing"
    );
}

// ── Horror of Horrors: "Sacrifice a Swamp: Regenerate target black creature." ─────────

#[test]
fn horror_of_horrors_regenerates_a_black_creature_for_a_swamp() {
    let mut game = Game::new();
    let horror = game.spawn_on_battlefield(PlayerId(0), card("Horror of Horrors"));
    let swamp = game.spawn_on_battlefield(PlayerId(0), card("Swamp"));
    let spare = game.spawn_on_battlefield(PlayerId(0), card("Swamp"));
    let zombies = game.spawn_on_battlefield(PlayerId(0), card("Scathe Zombies")); // black
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")); // green

    // A green Grizzly Bears is not "target black creature", so the ability fizzles on
    // resolution for having no legal target (CR 608.2b).
    activate(
        &mut game,
        horror,
        0,
        Some(Target::Object(bears)),
        vec![spare],
    )
    .expect("activation isn't re-validated; the target check happens on resolution");
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.regeneration_shields(bears),
        0,
        "the green creature was never a legal target"
    );

    activate(
        &mut game,
        horror,
        0,
        Some(Target::Object(zombies)),
        vec![swamp],
    )
    .expect("a black creature is, and a Swamp pays the cost");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(swamp),
        Zone::Graveyard,
        "the Swamp was sacrificed to pay the cost"
    );
    assert_eq!(
        game.regeneration_shields(zombies),
        1,
        "the black creature has a regeneration shield"
    );
}

// ── Fortified Area: "Wall creatures you control get +1/+0 and have banding." ──────────

#[test]
fn fortified_area_pumps_and_bands_your_walls_only() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Fortified Area"));
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Stone")); // 0/8
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")); // 2/2, not a Wall
    let their_wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));

    assert_eq!(
        (game.power(wall), game.toughness(wall)),
        (1, 8),
        "your Wall gets +1/+0"
    );
    assert!(game.has_keyword(wall, Keyword::Banding), "and has banding");
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (2, 2),
        "a creature that isn't a Wall is untouched"
    );
    assert!(
        !game.has_keyword(bears, Keyword::Banding),
        "and gets no banding"
    );
    assert_eq!(
        (game.power(their_wall), game.toughness(their_wall)),
        (0, 8),
        "\"you control\" keeps an opponent's Wall off the anthem"
    );
}

// ── Giant Strength: "Enchant creature / Enchanted creature gets +2/+2." ───────────────

#[test]
fn giant_strength_pumps_the_creature_it_enchants() {
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")); // 2/2
    let aura = game.spawn_in_hand(PlayerId(0), card("Giant Strength"));

    cast_and_resolve(&mut game, aura, Some(Target::Object(bears)));

    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (4, 4),
        "the enchanted creature gets +2/+2"
    );
}

// ── Eternal Warrior: "Enchant creature / Enchanted creature has vigilance." ───────────

#[test]
fn eternal_warrior_gives_its_host_vigilance() {
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let other = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Eternal Warrior"));

    cast_and_resolve(&mut game, aura, Some(Target::Object(bears)));

    assert!(
        game.has_keyword(bears, Keyword::Vigilance),
        "the enchanted creature has vigilance"
    );
    assert!(
        !game.has_keyword(other, Keyword::Vigilance),
        "an unenchanted creature does not"
    );

    // Vigilance in practice: attacking doesn't tap it (CR 702.20b).
    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: vec![(bears, Defender::Player(PlayerId(1)))],
    })
    .unwrap();
    assert!(
        !game.is_tapped(bears),
        "a vigilant attacker doesn't tap to attack"
    );
}

// ── The Brute: "Enchant creature / Enchanted creature gets +1/+0. / {R}{R}{R}:
// Regenerate enchanted creature." ─────────────────────────────────────────────────────

#[test]
fn the_brute_pumps_its_host_and_regenerates_it_for_three_red() {
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")); // 2/2
    let brute = game.spawn_in_hand(PlayerId(0), card("The Brute"));

    cast_and_resolve(&mut game, brute, Some(Target::Object(bears)));
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (3, 2),
        "the enchanted creature gets +1/+0"
    );

    let attached = game.attachments(bears)[0];
    game.fund_mana(PlayerId(0));
    activate(&mut game, attached, 1, None, vec![]).expect("{R}{R}{R} is payable");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.regeneration_shields(bears),
        1,
        "the enchanted creature has a regeneration shield"
    );
}
