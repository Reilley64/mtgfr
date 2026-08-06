//! Legends (`leg`) section-C authoring wave — batch c2.

mod common;

use common::*;
use engine::*;

/// Stack both libraries so a test can roll through whole turns without decking anyone.
fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 10]);
    }
}

/// Roll forward to player 0's *next* upkeep (the constructor parks at Main1).
fn advance_to_your_next_upkeep(game: &mut Game) {
    pass_until_next_turn(game); // into player 1's turn
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
}

// ── Jacques le Vert: "Green creatures you control get +0/+2." ──────────────────────────

#[test]
fn jacques_le_vert_pumps_your_green_creatures_including_itself() {
    let mut game = Game::new();
    let jacques = game.spawn_on_battlefield(PlayerId(0), card("Jacques le Vert"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let lions = game.spawn_on_battlefield(PlayerId(0), card("Savannah Lions"));
    let their_bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (2, 4),
        "a green creature you control gets +0/+2"
    );
    assert_eq!(
        (game.power(jacques), game.toughness(jacques)),
        (3, 4),
        "Jacques is green himself, so he pumps himself too"
    );
    assert_eq!(
        (game.power(lions), game.toughness(lions)),
        (2, 1),
        "a white creature you control is untouched"
    );
    assert_eq!(
        (game.power(their_bears), game.toughness(their_bears)),
        (2, 2),
        "an opponent's green creature is untouched — this is a you-control anthem"
    );
}

// ── Kobold Taskmaster: "Other Kobold creatures you control get +1/+0." ─────────────────

#[test]
fn kobold_taskmaster_pumps_other_kobolds_but_not_itself() {
    let mut game = Game::new();
    let taskmaster = game.spawn_on_battlefield(PlayerId(0), card("Kobold Taskmaster"));
    let kobold = game.spawn_on_battlefield(PlayerId(0), card("Kobolds of Kher Keep"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let their_kobold = game.spawn_on_battlefield(PlayerId(1), card("Kobolds of Kher Keep"));

    assert_eq!(
        (game.power(kobold), game.toughness(kobold)),
        (1, 1),
        "another Kobold you control gets +1/+0"
    );
    assert_eq!(
        (game.power(taskmaster), game.toughness(taskmaster)),
        (1, 2),
        "\"other\" keeps the Taskmaster off its own anthem"
    );
    assert_eq!(game.power(bears), 2, "a non-Kobold is untouched");
    assert_eq!(
        game.power(their_kobold),
        0,
        "an opponent's Kobold is untouched"
    );
}

// ── Kobold Drill Sergeant: "Other Kobold creatures you control get +0/+1 and have trample." ──

#[test]
fn kobold_drill_sergeant_grants_toughness_and_trample_to_other_kobolds() {
    let mut game = Game::new();
    let sergeant = game.spawn_on_battlefield(PlayerId(0), card("Kobold Drill Sergeant"));
    let kobold = game.spawn_on_battlefield(PlayerId(0), card("Kobolds of Kher Keep"));

    assert_eq!(
        (game.power(kobold), game.toughness(kobold)),
        (0, 2),
        "another Kobold you control gets +0/+1"
    );
    assert!(
        game.has_keyword(kobold, Keyword::Trample),
        "and has trample"
    );
    assert_eq!(
        (game.power(sergeant), game.toughness(sergeant)),
        (1, 2),
        "\"other\" keeps the Sergeant off its own anthem"
    );
    assert!(
        !game.has_keyword(sergeant, Keyword::Trample),
        "the Sergeant grants trample to Kobolds other than itself"
    );
}

// ── Adun Oakenshield: "{B}{R}{G}, {T}: Return target creature card from your graveyard to
// your hand." ─────────────────────────────────────────────────────────────────────────

#[test]
fn adun_oakenshield_returns_a_creature_card_from_your_graveyard_to_hand() {
    let mut game = Game::new();
    let adun = game.spawn_on_battlefield(PlayerId(0), card("Adun Oakenshield"));
    let dead = game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_in_graveyard(PlayerId(1), card("Grizzly Bears"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: adun,
        ability_index: 0,
        target: Some(Target::Object(dead)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("{B}{R}{G}, {T} is payable and your own graveyard creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(dead),
        Zone::Hand,
        "the creature card came back to hand"
    );
    assert!(game.is_tapped(adun), "the ability taps Adun as a cost");
    assert_eq!(
        game.zone_of(theirs),
        Zone::Graveyard,
        "\"your graveyard\" — an opponent's creature card is not reachable"
    );
}

// ── Hell's Caretaker: "{T}, Sacrifice a creature: Return target creature card from your
// graveyard to the battlefield. Activate only during your upkeep." ─────────────────────

#[test]
fn hells_caretaker_reanimates_during_your_upkeep() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let caretaker = game.spawn_on_battlefield(PlayerId(0), card("Hell's Caretaker"));
    let fodder = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let dead = game.spawn_in_graveyard(PlayerId(0), card("Savannah Lions"));

    advance_to_your_next_upkeep(&mut game);

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: caretaker,
        ability_index: 0,
        target: Some(Target::Object(dead)),
        sacrifice: vec![fodder],
        discard_cost: vec![],
        x: 0,
    })
    .expect("your upkeep is the ability's window");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(fodder),
        Zone::Graveyard,
        "a creature was sacrificed to pay the cost"
    );
    assert_eq!(
        game.zone_of(dead),
        Zone::Battlefield,
        "the targeted graveyard creature came back onto the battlefield"
    );
}

#[test]
fn hells_caretaker_cannot_be_activated_outside_your_upkeep() {
    let mut game = Game::new();
    let caretaker = game.spawn_on_battlefield(PlayerId(0), card("Hell's Caretaker"));
    let fodder = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let dead = game.spawn_in_graveyard(PlayerId(0), card("Savannah Lions"));

    // The constructor parks at Main1 — a perfectly ordinary activation window for anything else.
    assert!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: caretaker,
            ability_index: 0,
            target: Some(Target::Object(dead)),
            sacrifice: vec![fodder],
            discard_cost: vec![],
            x: 0,
        })
        .is_err(),
        "\"Activate only during your upkeep\" closes the main phase"
    );
    assert_eq!(
        game.zone_of(fodder),
        Zone::Battlefield,
        "the rejected activation pays nothing"
    );
}

// ── Wall of Opposition: "{1}: This creature gets +1/+0 until end of turn." ─────────────

#[test]
fn wall_of_opposition_pumps_itself_for_one_generic() {
    let mut game = Game::new();
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Opposition"));
    game.fund_mana(PlayerId(0));

    for _ in 0..3 {
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: wall,
            ability_index: 0,
            target: None,
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game);
    }

    assert_eq!(
        (game.power(wall), game.toughness(wall)),
        (3, 6),
        "+1/+0 three times over a base 0/6"
    );
    assert!(
        !game.is_tapped(wall),
        "the ability has no tap in its cost — a Wall with defender can pump repeatedly"
    );
}

// ── Axelrod Gunnarson: "Whenever a creature dealt damage by Axelrod Gunnarson this turn
// dies, you gain 1 life and Axelrod Gunnarson deals 1 damage to target player or
// planeswalker." ──────────────────────────────────────────────────────────────────────

#[test]
fn axelrod_gunnarson_drains_when_a_creature_it_damaged_dies() {
    let mut game = Game::new();
    let axelrod = game.spawn_on_battlefield(PlayerId(0), card("Axelrod Gunnarson"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![axelrod]);
    block_with(&mut game, vec![(blocker, axelrod)]).unwrap();
    advance_until(&mut game, |g| g.pending_choice().is_some());

    let Some(PendingChoice::ChooseTarget { legal, .. }) = game.pending_choice() else {
        panic!("the death-watch trigger pauses to choose a player or planeswalker");
    };
    assert!(
        legal.contains(&Target::Player(PlayerId(1))),
        "\"target player\" reaches the opponent"
    );
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Player(PlayerId(1))],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(blocker),
        Zone::Graveyard,
        "the blocker died to Axelrod's combat damage"
    );
    assert_eq!(game.life(PlayerId(0)), 21, "you gain 1 life");
    assert_eq!(
        game.life(PlayerId(1)),
        16,
        "3 trampled over the 2/2 blocker, then the trigger's 1 damage"
    );
}

// ── Vaevictis Asmadi: "At the beginning of your upkeep, sacrifice Vaevictis Asmadi unless
// you pay {B}{R}{G}." + three firebreathing abilities ──────────────────────────────────

#[test]
fn vaevictis_asmadi_is_sacrificed_when_the_upkeep_cost_goes_unpaid() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vaevictis = game.spawn_on_battlefield(PlayerId(0), card("Vaevictis Asmadi"));

    advance_to_your_next_upkeep(&mut game);
    resolve_top_of_stack(&mut game);

    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: false,
        discard_cost: vec![],
    })
    .expect("declining is legal");

    assert_eq!(
        game.zone_of(vaevictis),
        Zone::Graveyard,
        "an unpaid black-red-green upkeep cost sacrifices the Elder Dragon"
    );
}

#[test]
fn vaevictis_asmadi_paid_off_survives_and_pumps_in_three_colors() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vaevictis = game.spawn_on_battlefield(PlayerId(0), card("Vaevictis Asmadi"));

    advance_to_your_next_upkeep(&mut game);
    resolve_top_of_stack(&mut game);
    game.fund_mana(PlayerId(0)); // mana empties each step — fund it here, at the pause.

    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: true,
        discard_cost: vec![],
    })
    .expect("paying {B}{R}{G} is legal");
    assert_eq!(
        game.zone_of(vaevictis),
        Zone::Battlefield,
        "the paid upkeep keeps the Elder Dragon around"
    );

    // One activation of each of the three firebreathing abilities ({B}, {R}, {G}).
    game.fund_mana(PlayerId(0));
    for ability_index in 1..=3 {
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: vaevictis,
            ability_index,
            target: None,
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
        .unwrap();
        resolve_top_of_stack(&mut game);
    }

    assert_eq!(
        (game.power(vaevictis), game.toughness(vaevictis)),
        (10, 7),
        "+1/+0 from each of the black, red, and green abilities over a base 7/7"
    );
}

// ── Devouring Deep: "Islandwalk" ───────────────────────────────────────────────────────

#[test]
fn devouring_deep_is_unblockable_while_the_defender_controls_an_island() {
    let mut game = Game::new();
    let fish = game.spawn_on_battlefield(PlayerId(0), card("Devouring Deep"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Island"));

    attack_with(&mut game, vec![fish]);
    assert!(
        block_with(&mut game, vec![(blocker, fish)]).is_err(),
        "islandwalk stops the block while the defender controls an Island"
    );
}
