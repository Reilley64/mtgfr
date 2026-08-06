//! Legends (`leg`) section-C authoring wave — batch c3.

mod common;

use common::*;
use engine::*;

// ── Carrion Ants ─────────────────────────────────────────────────────────────────────────────

#[test]
fn carrion_ants_pumps_itself_for_one_generic() {
    // "{1}: This creature gets +1/+1 until end of turn." A repeatable generic-mana sink on a
    // 0/1 body.
    let mut game = Game::new();
    let ants = game.spawn_on_battlefield(PlayerId(0), card("Carrion Ants"));
    game.fund_mana(PlayerId(0));

    for _ in 0..3 {
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: ants,
            ability_index: 0,
            target: None,
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
        .expect("{1} is the whole cost");
        resolve_top_of_stack(&mut game);
    }

    assert_eq!(game.power(ants), 3, "three activations, +3/+3 total");
    assert_eq!(game.toughness(ants), 4);
}

#[test]
fn carrion_ants_pump_wears_off_at_end_of_turn() {
    // "…until end of turn." (CR 514.2)
    let mut game = Game::new();
    let ants = game.spawn_on_battlefield(PlayerId(0), card("Carrion Ants"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: ants,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);
    assert_eq!(game.power(ants), 1);

    pass_until_next_turn(&mut game);

    assert_eq!(game.power(ants), 0, "back to its printed 0/1");
    assert_eq!(game.toughness(ants), 1);
}

// ── Hyperion Blacksmith ──────────────────────────────────────────────────────────────────────

#[test]
fn hyperion_blacksmith_taps_an_opponents_artifact() {
    // "{T}: You may tap or untap target artifact an opponent controls." Mode 0 is the tap half,
    // and the target set is an opponent's artifacts only.
    let mut game = Game::new();
    let smith = game.spawn_on_battlefield(PlayerId(0), card("Hyperion Blacksmith"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Sol Ring"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Sol Ring"));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: smith,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("tapping the Blacksmith is the whole cost");
    assert!(game.is_tapped(smith), "{{T}} was paid");

    game.submit(Intent::ChooseMode {
        player: PlayerId(0),
        mode: 0, // tap
    })
    .expect("choosing the tap half is legal");

    let Some(PendingChoice::ChooseTarget { legal, .. }) = game.pending_choice() else {
        panic!(
            "the chosen mode asks for its own target; got {:?}",
            game.pending_choice()
        );
    };
    assert!(legal.contains(&Target::Object(theirs)));
    assert!(
        !legal.contains(&Target::Object(mine)),
        "\"an opponent controls\" excludes your own artifacts"
    );

    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(theirs)],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(theirs), "the opponent's Sol Ring is tapped");
}

#[test]
fn hyperion_blacksmith_untaps_an_opponents_artifact() {
    // The other half of "tap or untap" — mode 1 untaps instead.
    let mut game = Game::new();
    let smith = game.spawn_on_battlefield(PlayerId(0), card("Hyperion Blacksmith"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Sol Ring"));
    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: theirs,
    })
    .expect("the opponent taps their own Sol Ring for mana");
    assert!(game.is_tapped(theirs));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: smith,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    game.submit(Intent::ChooseMode {
        player: PlayerId(0),
        mode: 1, // untap
    })
    .unwrap();
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(theirs)],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert!(!game.is_tapped(theirs), "the untap half untapped it");
}

// ── Gwendlyn Di Corci ────────────────────────────────────────────────────────────────────────

#[test]
fn gwendlyn_di_corci_makes_target_player_discard_at_random() {
    // "{T}: Target player discards a card at random."
    let mut game = Game::new();
    let gwendlyn = game.spawn_on_battlefield(PlayerId(0), card("Gwendlyn Di Corci"));
    for _ in 0..2 {
        game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    }

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: gwendlyn,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(1))),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("it is player 0's own turn");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.hand(PlayerId(1)).len(),
        1,
        "the targeted player pitched one card"
    );
}

#[test]
fn gwendlyn_di_corci_cant_be_activated_on_an_opponents_turn() {
    // "Activate only during your turn." (CR 602.5b)
    let mut game = Game::new();
    let gwendlyn = game.spawn_on_battlefield(PlayerId(0), card("Gwendlyn Di Corci"));
    game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });
    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .unwrap();

    assert_eq!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: gwendlyn,
            ability_index: 0,
            target: Some(Target::Player(PlayerId(1))),
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        }),
        Err(Reject::WrongTiming),
        "Gwendlyn is inert on somebody else's turn — and it is the timing restriction that says so"
    );
    assert!(!game.is_tapped(gwendlyn), "and the cost was never paid");
}

// ── Lost Soul / Segovian Leviathan ───────────────────────────────────────────────────────────

#[test]
fn lost_soul_is_unblockable_while_the_defender_controls_a_swamp() {
    // "Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.)"
    let mut game = Game::new();
    let soul = game.spawn_on_battlefield(PlayerId(0), card("Lost Soul"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Swamp"));

    attack_with(&mut game, vec![soul]);

    assert!(
        block_with(&mut game, vec![(blocker, soul)]).is_err(),
        "the defender's Swamp switches swampwalk on"
    );
}

#[test]
fn segovian_leviathan_is_unblockable_while_the_defender_controls_an_island() {
    // "Islandwalk (This creature can't be blocked as long as defending player controls an
    // Island.)" — and it is blockable when they control none.
    let mut game = Game::new();
    let leviathan = game.spawn_on_battlefield(PlayerId(0), card("Segovian Leviathan"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![leviathan]);
    assert!(
        block_with(&mut game, vec![(blocker, leviathan)]).is_ok(),
        "islandwalk is inert against a defender with no Island"
    );

    let mut game = Game::new();
    let leviathan = game.spawn_on_battlefield(PlayerId(0), card("Segovian Leviathan"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Island"));

    attack_with(&mut game, vec![leviathan]);
    assert!(
        block_with(&mut game, vec![(blocker, leviathan)]).is_err(),
        "the defender's Island switches islandwalk on"
    );
}

// ── Tuknir Deathlock ─────────────────────────────────────────────────────────────────────────

#[test]
fn tuknir_deathlock_pumps_a_target_creature() {
    // "{R}{G}, {T}: Target creature gets +2/+2 until end of turn."
    let mut game = Game::new();
    let tuknir = game.spawn_on_battlefield(PlayerId(0), card("Tuknir Deathlock"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.fund_mana(PlayerId(0));

    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: tuknir,
        ability_index: 0,
        target: Some(Target::Object(bears)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("{R}{G} plus the tap");
    assert!(game.is_tapped(tuknir), "{{T}} is part of the cost");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.power(bears), 4, "+2/+2 on the target, not on Tuknir");
    assert_eq!(game.toughness(bears), 4);
    assert_eq!(game.power(tuknir), 2);
}

// ── Palladia-Mors ────────────────────────────────────────────────────────────────────────────

#[test]
fn palladia_mors_is_sacrificed_when_its_upkeep_goes_unpaid() {
    // "At the beginning of your upkeep, sacrifice Palladia-Mors unless you pay {R}{G}{W}."
    let mut game = Game::new();
    let dragon = game.spawn_on_battlefield(PlayerId(0), card("Palladia-Mors"));
    for seat in [PlayerId(0), PlayerId(1)] {
        game.stack_library(seat, &[card("Grizzly Bears"), card("Grizzly Bears")]);
    }

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    resolve_top_of_stack(&mut game);

    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: false,
        discard_cost: vec![],
    })
    .expect("declining {R}{G}{W} is legal");

    assert_eq!(
        game.zone_of(dragon),
        Zone::Graveyard,
        "the unpaid Elder Dragon is sacrificed"
    );
}

#[test]
fn palladia_mors_survives_a_paid_upkeep() {
    let mut game = Game::new();
    let dragon = game.spawn_on_battlefield(PlayerId(0), card("Palladia-Mors"));
    for seat in [PlayerId(0), PlayerId(1)] {
        game.stack_library(seat, &[card("Grizzly Bears"), card("Grizzly Bears")]);
    }

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    resolve_top_of_stack(&mut game);
    game.fund_mana(PlayerId(0)); // mana empties each step — fund it here, at the pause.

    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: true,
        discard_cost: vec![],
    })
    .expect("paying {R}{G}{W} is legal");

    assert_eq!(
        game.zone_of(dragon),
        Zone::Battlefield,
        "the paid Elder Dragon stays"
    );
}
