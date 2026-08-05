//! Legends (`leg`) section-C authoring wave — batch s1.

mod common;

use common::*;
use engine::*;

// ── local drivers (game.rs keeps its own private copies of these) ─────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..20 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

fn cast(game: &mut Game, object: ObjectId, target: Option<Target>) {
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
}

fn cast_mode(game: &mut Game, object: ObjectId, mode: usize, target: Option<Target>) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object,
        target: None,
        x: 0,
        modes: vec![(mode, target)],
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
}

fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) {
    cast(game, object, target);
    resolve_top_of_stack(game);
}

// ── the cards ─────────────────────────────────────────────────────────────────────────

#[test]
fn acid_rain_destroys_every_forest() {
    // "Destroy all Forests."
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let mountain = game.spawn_on_battlefield(PlayerId(0), card("Mountain"));
    let spell = game.spawn_in_hand(PlayerId(0), card("Acid Rain"));

    cast_and_resolve(&mut game, spell, None);

    assert_eq!(game.zone_of(mine), Zone::Graveyard, "your Forest dies too");
    assert_eq!(game.zone_of(theirs), Zone::Graveyard);
    assert_eq!(
        game.zone_of(mountain),
        Zone::Battlefield,
        "only Forests are destroyed"
    );
}

#[test]
fn hell_swarm_shrinks_every_creatures_power() {
    // "All creatures get -1/-0 until end of turn."
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let spell = game.spawn_in_hand(PlayerId(0), card("Hell Swarm"));

    cast_and_resolve(&mut game, spell, None);

    assert_eq!(game.power(mine), 1, "2 base - 1, on your own creature too");
    assert_eq!(game.power(theirs), 1);
    assert_eq!(game.toughness(mine), 2, "toughness is untouched");
    assert_eq!(game.toughness(theirs), 2);
}

#[test]
fn shield_wall_toughens_your_creatures_only() {
    // "Creatures you control get +0/+2 until end of turn."
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let spell = game.spawn_in_hand(PlayerId(0), card("Shield Wall"));

    cast_and_resolve(&mut game, spell, None);

    assert_eq!(game.toughness(mine), 4, "2 base + 2");
    assert_eq!(game.power(mine), 2, "power is untouched");
    assert_eq!(
        game.toughness(theirs),
        2,
        "an opponent's creature gets nothing"
    );
}

#[test]
fn darkness_prevents_combat_damage() {
    // "Prevent all combat damage that would be dealt this turn."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant")); // 3/3
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // 3/3
    let spell = game.spawn_in_hand(PlayerId(0), card("Darkness"));

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: vec![(attacker, Defender::Player(PlayerId(1)))],
    })
    .unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(1),
        blocks: vec![(blocker, attacker)],
    })
    .unwrap();
    cast_and_resolve(&mut game, spell, None);

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.marked_damage(attacker), 0);
    assert_eq!(game.marked_damage(blocker), 0);
    assert_eq!(
        game.zone_of(attacker),
        Zone::Battlefield,
        "the lethal trade is prevented on both sides"
    );
    assert_eq!(game.zone_of(blocker), Zone::Battlefield);
}

#[test]
fn pyrotechnics_divides_four_damage() {
    // "Pyrotechnics deals 4 damage divided as you choose among any number of targets."
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // 3/3
    let spell = game.spawn_in_hand(PlayerId(0), card("Pyrotechnics"));

    let before = game.life(PlayerId(1));
    cast(&mut game, spell, None);
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(giant), Target::Player(PlayerId(1))],
    })
    .expect("a creature and a player, within the {0,4} range");
    game.submit(Intent::DivideSpellDamage {
        player: PlayerId(0),
        assignment: vec![(Target::Object(giant), 1), (Target::Player(PlayerId(1)), 3)],
    })
    .expect("1 + 3 sums to 4, each target getting at least one");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.marked_damage(giant),
        1,
        "the creature's assigned share"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        before - 3,
        "the player's assigned share"
    );
}

#[test]
fn untamed_wilds_fetches_a_basic_land_untapped() {
    // "Search your library for a basic land card, put that card onto the battlefield, then
    // shuffle."
    let mut game = Game::new();
    let lib = game.stack_library(PlayerId(0), &[card("Shock"), card("Forest")]);
    let forest = lib[1];
    let spell = game.spawn_in_hand(PlayerId(0), card("Untamed Wilds"));

    cast_and_resolve(&mut game, spell, None);

    assert_eq!(
        game.pending_choice(),
        Some(PendingChoice::SearchLibrary {
            player: PlayerId(0),
            matches: vec![forest],
            dest: SearchDest::Battlefield,
            tapped: false,
            remaining: 1,
            overflow: None,
        }),
        "only the basic land is on offer, and it is not put in tapped",
    );
    game.submit(Intent::SearchLibrary {
        player: PlayerId(0),
        choice: Some(forest),
    })
    .unwrap();

    assert_eq!(game.zone_of(forest), Zone::Battlefield);
    let permanent = game.current_id(forest);
    assert!(!game.is_tapped(permanent), "Untamed Wilds does not tap it");
}

#[test]
fn storm_seeker_deals_damage_equal_to_hand_size() {
    // "Storm Seeker deals damage to target player equal to the number of cards in that player's
    // hand."
    let mut game = Game::new();
    for _ in 0..3 {
        game.spawn_in_hand(PlayerId(1), card("Shock"));
    }
    game.spawn_in_hand(PlayerId(0), card("Shock")); // the caster's own hand must not count
    let spell = game.spawn_in_hand(PlayerId(0), card("Storm Seeker"));

    let before = game.life(PlayerId(1));
    cast_and_resolve(&mut game, spell, Some(Target::Player(PlayerId(1))));

    assert_eq!(
        game.life(PlayerId(1)),
        before - 3,
        "three cards in the targeted player's hand, three damage"
    );
}

#[test]
fn jovial_evil_deals_twice_the_white_creature_count() {
    // "Jovial Evil deals X damage to target opponent, where X is twice the number of white
    // creatures that player controls."
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Savannah Lions")); // white
    game.spawn_on_battlefield(PlayerId(1), card("Savannah Lions")); // white
    game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")); // green — not counted
    game.spawn_on_battlefield(PlayerId(0), card("Savannah Lions")); // yours — not counted
    let spell = game.spawn_in_hand(PlayerId(0), card("Jovial Evil"));

    let before = game.life(PlayerId(1));
    cast_and_resolve(&mut game, spell, Some(Target::Player(PlayerId(1))));

    assert_eq!(
        game.life(PlayerId(1)),
        before - 4,
        "two white creatures that player controls, doubled"
    );
}

#[test]
fn disharmony_steals_an_attacker_out_of_combat() {
    // "Untap target attacking creature and remove it from combat. Gain control of that creature
    // until end of turn."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let spell = game.spawn_in_hand(PlayerId(0), card("Disharmony"));

    pass_until_next_turn(&mut game); // → player 1's turn, so its creature can attack player 0
    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(attacker, Defender::Player(PlayerId(0)))],
    })
    .unwrap();
    assert!(game.is_tapped(attacker), "attacking tapped it");

    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .unwrap();
    cast_and_resolve(&mut game, spell, Some(Target::Object(attacker)));

    assert!(!game.is_tapped(attacker), "the creature untaps");
    assert!(
        !game.attackers().contains(&attacker),
        "it is removed from combat"
    );
    assert_eq!(
        game.controller_of(attacker),
        PlayerId(0),
        "the caster gains control of it"
    );

    let before = game.life(PlayerId(0));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(0)),
        before,
        "a creature removed from combat deals no combat damage"
    );
}

#[test]
fn flash_flood_destroys_a_red_permanent() {
    // "• Destroy target red permanent."
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // red
    let spell = game.spawn_in_hand(PlayerId(0), card("Flash Flood"));

    cast_mode(&mut game, spell, 0, Some(Target::Object(giant)));
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(giant), Zone::Graveyard);
}

#[test]
fn flash_flood_bounces_a_mountain() {
    // "• Return target Mountain to its owner's hand."
    let mut game = Game::new();
    let mountain = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let spell = game.spawn_in_hand(PlayerId(0), card("Flash Flood"));

    cast_mode(&mut game, spell, 1, Some(Target::Object(mountain)));
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(mountain),
        Zone::Hand,
        "the Mountain goes back to its owner's hand"
    );
}
