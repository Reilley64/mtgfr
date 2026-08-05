//! Legends (`leg`) section-C authoring wave — batch s2.

mod common;

use common::*;
use engine::*;

// ── local drivers (game.rs keeps its own private copies of these) ─────────────────────

/// A [`Intent::Cast`] with every optional-cost knob at its decline default.
fn cast_intent(
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
    x: u32,
    modes: Vec<(usize, Option<Target>)>,
) -> Intent {
    Intent::Cast {
        player,
        object,
        target,
        x,
        modes,
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
    }
}

/// Put `object` on the stack for `player` (funding the mana first) without resolving it.
fn cast(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    game.fund_mana(player);
    game.submit(cast_intent(player, object, target, 0, vec![]))
        .unwrap();
}

/// Cast and resolve for player 0 — the ordinary "this spell does its thing" driver.
fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) {
    cast(game, PlayerId(0), object, target);
    resolve_top_of_stack(game);
}

/// The topmost spell on the stack (the one just cast).
fn top_spell(game: &Game) -> ObjectId {
    match game.stack().last().expect("a spell is on the stack") {
        StackEntry::Spell(id) => *id,
        other => panic!("expected a spell on top of the stack, got {other:?}"),
    }
}

/// Keep every seat's library stocked so passing whole turns can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..20 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Roll to `player`'s own precombat main phase, where they may cast a sorcery-speed spell.
fn advance_to_main1_of(game: &mut Game, player: PlayerId) {
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Main1
    });
}

/// Player 1 casts `object` on their own main phase and then passes, handing player 0 the
/// response window a counterspell needs.
fn opponent_casts_then_passes(game: &mut Game, object: ObjectId) -> ObjectId {
    game.fund_mana(PlayerId(1));
    game.submit(cast_intent(PlayerId(1), object, None, 0, vec![]))
        .unwrap();
    let spell = top_spell(game);
    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .unwrap();
    spell
}

// ── the cards ─────────────────────────────────────────────────────────────────────────

#[test]
fn cleanse_destroys_only_the_black_creatures() {
    // "Destroy all black creatures."
    let mut game = Game::new();
    let zombie = game.spawn_on_battlefield(PlayerId(1), card("Walking Dead"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let mine = game.spawn_on_battlefield(PlayerId(1), card("Howling Mine"));
    let cleanse = game.spawn_in_hand(PlayerId(0), card("Cleanse"));

    cast_and_resolve(&mut game, cleanse, None);

    assert_eq!(
        game.zone_of(zombie),
        Zone::Graveyard,
        "the black creature died"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the green one did not"
    );
    assert_eq!(
        game.zone_of(mine),
        Zone::Battlefield,
        "a noncreature permanent is untouched"
    );
}

#[test]
fn remove_soul_only_counters_a_creature_spell() {
    // "Counter target creature spell."
    let mut game = Game::new();
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    let remove = game.spawn_in_hand(PlayerId(0), card("Remove Soul"));

    cast(
        &mut game,
        PlayerId(0),
        bolt,
        Some(Target::Player(PlayerId(1))),
    );
    let bolt_spell = top_spell(&game);
    game.fund_mana(PlayerId(0));
    assert_eq!(
        game.submit(cast_intent(
            PlayerId(0),
            remove,
            Some(Target::Object(bolt_spell)),
            0,
            vec![],
        )),
        Err(Reject::IllegalTarget),
        "an instant spell is not a creature spell"
    );
    resolve_top_of_stack(&mut game);

    cast(&mut game, PlayerId(0), bear, None);
    let bear_spell = top_spell(&game);
    cast_and_resolve(&mut game, remove, Some(Target::Object(bear_spell)));
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "the countered creature spell went to the graveyard, never the battlefield"
    );
}

#[test]
fn boomerang_returns_any_permanent_to_its_owners_hand() {
    // "Return target permanent to its owner's hand."
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(1), card("Howling Mine"));
    let boomerang = game.spawn_in_hand(PlayerId(0), card("Boomerang"));

    cast_and_resolve(&mut game, boomerang, Some(Target::Object(mine)));

    assert_eq!(
        game.zone_of(mine),
        Zone::Hand,
        "a noncreature permanent is a legal target and goes home"
    );
}

#[test]
fn force_spike_counters_a_spell_whose_controller_declines_to_pay_one() {
    // "Counter target spell unless its controller pays {1}."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    let spike = game.spawn_in_hand(PlayerId(0), card("Force Spike"));

    advance_to_main1_of(&mut game, PlayerId(1));
    let bear_spell = opponent_casts_then_passes(&mut game, bear);
    cast(
        &mut game,
        PlayerId(0),
        spike,
        Some(Target::Object(bear_spell)),
    );
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::PayOrCounter { player, cost, .. }) = game.pending_choice() else {
        panic!("Force Spike pauses the bear's controller on a pay-or-counter choice");
    };
    assert_eq!(player, PlayerId(1), "the target spell's controller chooses");
    assert_eq!(cost.generic, 1, "the ransom is {{1}}");

    game.submit(Intent::PayOptionalCost {
        player,
        pay: false,
        discard_cost: vec![],
    })
    .unwrap();

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "declining the {{1}} lets the counter through"
    );
}

#[test]
fn force_spike_is_paid_off_and_the_spell_resolves() {
    // "Counter target spell unless its controller pays {1}."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    let spike = game.spawn_in_hand(PlayerId(0), card("Force Spike"));

    advance_to_main1_of(&mut game, PlayerId(1));
    let bear_spell = opponent_casts_then_passes(&mut game, bear);
    cast(
        &mut game,
        PlayerId(0),
        spike,
        Some(Target::Object(bear_spell)),
    );
    resolve_top_of_stack(&mut game);

    game.submit(Intent::PayOptionalCost {
        player: PlayerId(1),
        pay: true,
        discard_cost: vec![],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "paying the {{1}} leaves the spell to resolve"
    );
}

#[test]
fn holy_day_prevents_the_turns_combat_damage() {
    // "Prevent all combat damage that would be dealt this turn."
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let holy_day = game.spawn_in_hand(PlayerId(0), card("Holy Day"));
    let before = game.life(PlayerId(1));

    cast_and_resolve(&mut game, holy_day, None);
    attack_with(&mut game, vec![bear]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "the unblocked bear's combat damage was prevented"
    );
}

#[test]
fn divine_offering_destroys_an_artifact_and_gains_its_mana_value() {
    // "Destroy target artifact. You gain life equal to its mana value."
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(1), card("Howling Mine"));
    let offering = game.spawn_in_hand(PlayerId(0), card("Divine Offering"));
    let before = game.life(PlayerId(0));

    cast_and_resolve(&mut game, offering, Some(Target::Object(mine)));

    assert_eq!(
        game.zone_of(mine),
        Zone::Graveyard,
        "the artifact was destroyed"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        before + 2,
        "Howling Mine's mana value is 2"
    );
}

#[test]
fn great_defender_adds_toughness_equal_to_the_targets_mana_value() {
    // "Target creature gets +0/+X until end of turn, where X is its mana value."
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let defender = game.spawn_in_hand(PlayerId(0), card("Great Defender"));

    cast_and_resolve(&mut game, defender, Some(Target::Object(bear)));

    assert_eq!(game.power(bear), 2, "power is untouched");
    assert_eq!(
        game.toughness(bear),
        4,
        "2 base toughness + Grizzly Bears' mana value of 2"
    );
}

#[test]
fn typhoon_bills_each_opponent_for_their_own_islands() {
    // "Typhoon deals damage to each opponent equal to the number of Islands that player controls."
    let mut game = Game::new();
    for _ in 0..3 {
        game.spawn_on_battlefield(PlayerId(1), card("Island"));
    }
    // The caster's own Islands are never counted — the amount reads the damaged seat.
    game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let typhoon = game.spawn_in_hand(PlayerId(0), card("Typhoon"));
    let opponent_before = game.life(PlayerId(1));
    let caster_before = game.life(PlayerId(0));

    cast_and_resolve(&mut game, typhoon, None);

    assert_eq!(
        game.life(PlayerId(1)),
        opponent_before - 3,
        "three Islands, three damage"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        caster_before,
        "the caster is not an opponent"
    );
}

#[test]
fn mana_drain_counters_then_refunds_the_mana_next_main_phase() {
    // "Counter target spell. At the beginning of your next main phase, add an amount of {C} equal
    // to that spell's mana value."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angel = game.spawn_in_hand(PlayerId(1), card("Serra Angel"));
    let drain = game.spawn_in_hand(PlayerId(0), card("Mana Drain"));

    advance_to_main1_of(&mut game, PlayerId(1));
    let angel_spell = opponent_casts_then_passes(&mut game, angel);
    cast(
        &mut game,
        PlayerId(0),
        drain,
        Some(Target::Object(angel_spell)),
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(angel),
        Zone::Graveyard,
        "Serra Angel was countered"
    );

    // Roll to the caster's own next main phase and let the delayed trigger resolve.
    advance_to_main1_of(&mut game, PlayerId(0));
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.colorless_in_pool(PlayerId(0)),
        5,
        "Serra Angel's mana value is 5"
    );
}

#[test]
fn active_volcano_destroys_a_blue_permanent() {
    // "Choose one — • Destroy target blue permanent. • Return target Island to its owner's hand."
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let elemental = game.spawn_on_battlefield(PlayerId(1), card("Air Elemental"));
    let volcano = game.spawn_in_hand(PlayerId(0), card("Active Volcano"));

    game.fund_mana(PlayerId(0));
    assert_eq!(
        game.submit(cast_intent(
            PlayerId(0),
            volcano,
            None,
            0,
            vec![(0, Some(Target::Object(bear)))],
        )),
        Err(Reject::IllegalTarget),
        "a green creature is not a blue permanent"
    );

    game.submit(cast_intent(
        PlayerId(0),
        volcano,
        None,
        0,
        vec![(0, Some(Target::Object(elemental)))],
    ))
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(elemental),
        Zone::Graveyard,
        "the blue creature died"
    );
}

#[test]
fn active_volcano_bounces_an_island() {
    // "Choose one — • Destroy target blue permanent. • Return target Island to its owner's hand."
    let mut game = Game::new();
    let island = game.spawn_on_battlefield(PlayerId(1), card("Island"));
    let forest = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let volcano = game.spawn_in_hand(PlayerId(0), card("Active Volcano"));

    game.fund_mana(PlayerId(0));
    assert_eq!(
        game.submit(cast_intent(
            PlayerId(0),
            volcano,
            None,
            0,
            vec![(1, Some(Target::Object(forest)))],
        )),
        Err(Reject::IllegalTarget),
        "a Forest is not an Island"
    );

    game.submit(cast_intent(
        PlayerId(0),
        volcano,
        None,
        0,
        vec![(1, Some(Target::Object(island)))],
    ))
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(island), Zone::Hand, "the Island went home");
}

#[test]
fn alabaster_potion_gains_the_target_player_x_life() {
    // "Choose one — • Target player gains X life. • Prevent the next X damage that would be dealt
    // to any target this turn."
    let mut game = Game::new();
    let potion = game.spawn_in_hand(PlayerId(0), card("Alabaster Potion"));
    let before = game.life(PlayerId(1));

    game.fund_mana(PlayerId(0));
    game.submit(cast_intent(
        PlayerId(0),
        potion,
        None,
        4,
        vec![(0, Some(Target::Player(PlayerId(1))))],
    ))
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(1)),
        before + 4,
        "X = 4 life to the chosen seat"
    );
}

#[test]
fn alabaster_potion_shields_a_target_from_the_next_x_damage() {
    // "Choose one — • Target player gains X life. • Prevent the next X damage that would be dealt
    // to any target this turn."
    let mut game = Game::new();
    let potion = game.spawn_in_hand(PlayerId(0), card("Alabaster Potion"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    let before = game.life(PlayerId(1));

    game.fund_mana(PlayerId(0));
    game.submit(cast_intent(
        PlayerId(0),
        potion,
        None,
        2,
        vec![(1, Some(Target::Player(PlayerId(1))))],
    ))
    .unwrap();
    resolve_top_of_stack(&mut game);

    cast_and_resolve(&mut game, bolt, Some(Target::Player(PlayerId(1))));

    assert_eq!(
        game.life(PlayerId(1)),
        before - 1,
        "2 of Lightning Bolt's 3 damage was prevented"
    );
}
