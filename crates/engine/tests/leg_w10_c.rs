//! Legends (`leg`) grind, wave 10 slice C — counters and the untap step.
//!
//! Increments 26 (counter-gated untap suppression), 49 (block-with-toughness-filter then a
//! counter), 55 (a regenerate ability granted by a counter), 79 (skip the next two untap steps)
//! and 103 (countering an activated ability from an artifact source).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 60]);
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step, which is all a non-active seat needs to act at instant speed.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

fn cast(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
    x: u32,
) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
    game.fund_mana(player);
    game.submit(Intent::Cast {
        player,
        object,
        target,
        x,
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

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    cast(game, player, object, target, 0).unwrap();
    resolve_top_of_stack(game);
}

/// Roll forward through one whole table round, so the active player's next untap step has run.
fn round_trip(game: &mut Game) {
    for _ in 0..game.player_count() {
        pass_until_next_turn(game);
    }
}

/// Cast an Aura from `player`'s hand onto `host` on `player`'s own turn (sorcery speed), letting
/// the spell and its enters-the-battlefield trigger resolve.
fn cast_aura(
    game: &mut Game,
    player: PlayerId,
    aura: ObjectId,
    host: ObjectId,
    x: u32,
) -> ObjectId {
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Main1
    });
    cast(game, player, aura, Some(Target::Object(host)), x).unwrap();
    resolve_top_of_stack(game);
    // Two simultaneous enters triggers (Cocoon) pause on CR 603.3b ordering; any order will do.
    if let Some(PendingChoice::OrderTriggers { effects, .. }) = game.pending_choice() {
        let order = (0..effects.len()).collect();
        game.submit(Intent::ChooseOrder { player, order }).unwrap();
    }
    while !game.stack_is_empty() {
        resolve_top_of_stack(game);
    }
    // The Aura is a new object on the battlefield — hand back the id everything downstream reads.
    game.current_id(aura)
}

/// Roll forward to `player`'s next precombat main phase, so both their untap step and whatever
/// their upkeep triggered have already happened.
fn next_turn_of(game: &mut Game, player: PlayerId) {
    while game.active_player() == player {
        pass_until_next_turn(game);
    }
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Main1
    });
}

// ── increment 26: Venarian Gold, Cocoon ───────────────────────────────────────────────

#[test]
fn venarian_gold_sleeps_its_creature_for_one_untap_step_per_counter() {
    // "When this Aura enters, tap enchanted creature and put X sleep counters on it. Enchanted
    // creature doesn't untap during its controller's untap step if it has a sleep counter on it.
    // At the beginning of the upkeep of enchanted creature's controller, remove a sleep counter
    // from that creature."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let gold = game.spawn_in_hand(PlayerId(1), card("Venarian Gold"));

    cast_aura(&mut game, PlayerId(1), gold, bear, 2);
    assert!(game.is_tapped(bear), "the Aura taps what it enchants");
    assert_eq!(
        game.counters_of_kind(bear, CounterKind::Sleep),
        2,
        "X sleep counters go on the creature, not on the Aura"
    );

    next_turn_of(&mut game, PlayerId(0));
    assert!(game.is_tapped(bear), "two sleep counters, so no untapping");
    assert_eq!(
        game.counters_of_kind(bear, CounterKind::Sleep),
        1,
        "its controller's upkeep took one counter off"
    );

    next_turn_of(&mut game, PlayerId(0));
    assert!(
        game.is_tapped(bear),
        "one sleep counter still holds it down"
    );
    assert_eq!(game.counters_of_kind(bear, CounterKind::Sleep), 0);

    next_turn_of(&mut game, PlayerId(0));
    assert!(!game.is_tapped(bear), "out of counters, so it wakes up");
}

#[test]
fn venarian_gold_leaving_frees_the_creature_even_with_sleep_counters_left() {
    // The restriction is the *Aura's* static ability, not the counter's — destroy the Aura and the
    // creature untaps on schedule with its sleep counters still on it.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let gold = game.spawn_in_hand(PlayerId(1), card("Venarian Gold"));

    let gold = cast_aura(&mut game, PlayerId(1), gold, bear, 3);
    assert_eq!(game.counters_of_kind(bear, CounterKind::Sleep), 3);

    let disenchant = game.spawn_in_hand(PlayerId(0), card("Disenchant"));
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        disenchant,
        Some(Target::Object(gold)),
    );
    assert_eq!(game.zone_of(gold), Zone::Graveyard);

    next_turn_of(&mut game, PlayerId(0));
    assert!(
        game.counters_of_kind(bear, CounterKind::Sleep) > 0,
        "the counters outlive the Aura"
    );
    assert!(
        !game.is_tapped(bear),
        "with no Aura there is no ability telling it to stay tapped"
    );
}

#[test]
fn cocoon_holds_its_creature_down_then_hatches_it() {
    // "When this Aura enters, tap enchanted creature and put three pupa counters on this Aura.
    // Enchanted creature doesn't untap during your untap step if this Aura has a pupa counter on
    // it. At the beginning of your upkeep, remove a pupa counter from this Aura. If you can't,
    // sacrifice it, put a +1/+1 counter on enchanted creature, and that creature gains flying."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let cocoon = game.spawn_in_hand(PlayerId(0), card("Cocoon"));

    let cocoon = cast_aura(&mut game, PlayerId(0), cocoon, bear, 0);
    assert!(game.is_tapped(bear), "the Aura taps what it enchants");
    assert_eq!(
        game.counters_of_kind(cocoon, CounterKind::Pupa),
        3,
        "the pupa counters go on the Aura, not on the creature"
    );

    for left in [2, 1, 0] {
        next_turn_of(&mut game, PlayerId(0));
        assert!(game.is_tapped(bear), "still cocooned");
        assert_eq!(game.counters_of_kind(cocoon, CounterKind::Pupa), left);
    }

    next_turn_of(&mut game, PlayerId(0));
    assert!(
        !game.is_tapped(bear),
        "no pupa counters left to hold it down"
    );
    assert_eq!(
        game.zone_of(cocoon),
        Zone::Graveyard,
        "the upkeep that couldn't remove a counter sacrificed the Aura"
    );
    assert_eq!(game.plus_counters(bear), 1, "and left a +1/+1 counter");
    assert!(
        game.has_keyword(bear, Keyword::Flying),
        "the hatched creature keeps flying with the Aura gone"
    );
}

// ── increment 55: Life Matrix ─────────────────────────────────────────────────────────

/// Put `object`'s ability at `ability_index` on the stack for `player`, funding whatever it costs,
/// and leave it there — which is where increment 103 needs to reach it.
fn activate_only(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    index: usize,
    target: Option<Target>,
) {
    give_priority(game, player);
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
}

/// Activate `object`'s ability at `ability_index` for `player`, funding whatever it costs.
fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    index: usize,
    target: Option<Target>,
) {
    activate_only(game, player, object, index, target);
    resolve_top_of_stack(game);
}

#[test]
fn life_matrix_counter_carries_its_regenerate_ability_after_the_matrix_is_gone() {
    // "{4}, {T}: Put a matrix counter on target creature and that creature gains 'Remove a matrix
    // counter from this creature: Regenerate this creature.' Activate only during your upkeep."
    // The grant has no duration, so it survives the Matrix leaving (CR 400.7).
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let matrix = game.spawn_on_battlefield(PlayerId(0), card("Life Matrix"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    assert!(
        game.ability_at(bear, 0).is_none(),
        "an unmatrixed creature has no granted ability to activate"
    );

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    activate(
        &mut game,
        PlayerId(0),
        matrix,
        0,
        Some(Target::Object(bear)),
    );
    assert_eq!(game.counters_of_kind(bear, CounterKind::Matrix), 1);

    let disenchant = game.spawn_in_hand(PlayerId(0), card("Disenchant"));
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        disenchant,
        Some(Target::Object(matrix)),
    );
    assert_eq!(game.zone_of(matrix), Zone::Graveyard);

    activate(&mut game, PlayerId(0), bear, 0, None);
    assert_eq!(
        game.counters_of_kind(bear, CounterKind::Matrix),
        0,
        "the counter is the ability's cost"
    );
    assert_eq!(
        game.regeneration_shields(bear),
        1,
        "the granted ability still regenerates with the Matrix in the graveyard"
    );
}

// ── increment 79: Telekinesis ─────────────────────────────────────────────────────────

#[test]
fn telekinesis_holds_a_creature_down_for_two_untap_steps() {
    // "Tap target creature. … It doesn't untap during its controller's next two untap steps."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let telekinesis = game.spawn_in_hand(PlayerId(1), card("Telekinesis"));

    cast_and_resolve(
        &mut game,
        PlayerId(1),
        telekinesis,
        Some(Target::Object(bear)),
    );
    assert!(game.is_tapped(bear), "the spell taps its target");

    round_trip(&mut game);
    assert!(
        game.is_tapped(bear),
        "the first of the two skipped untap steps left it tapped"
    );

    round_trip(&mut game);
    assert!(
        game.is_tapped(bear),
        "the second skipped untap step left it tapped too"
    );

    round_trip(&mut game);
    assert!(
        !game.is_tapped(bear),
        "the third untap step is an ordinary one — both marks are spent"
    );
}

// ── increment 103: Rust, Ayesha Tanaka ────────────────────────────────────────────────

/// Set the table up with Rod of Ruin's ping on the stack, aimed at `victim`: an artifact's
/// activated ability whose outcome is a life total, so "countered" needs no follow-up choice.
fn rod_ability_on_stack(game: &mut Game, controller: PlayerId, victim: PlayerId) -> ObjectId {
    let rod = game.spawn_on_battlefield(controller, card("Rod of Ruin"));
    activate_only(game, controller, rod, 0, Some(Target::Player(victim)));
    rod
}

#[test]
fn rust_counters_an_activated_ability_from_an_artifact_source() {
    // "Counter target activated ability from an artifact source."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let rust = game.spawn_in_hand(PlayerId(1), card("Rust"));
    let rod = rod_ability_on_stack(&mut game, PlayerId(0), PlayerId(1));
    let life = game.life(PlayerId(1));

    cast_and_resolve(&mut game, PlayerId(1), rust, Some(Target::Object(rod)));
    assert!(
        game.stack_is_empty(),
        "Rust took the ping off the stack with it"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        life,
        "the countered ability dealt no damage"
    );
}

#[test]
fn rust_cannot_target_an_activated_ability_from_a_creature() {
    // "from an artifact source" is a targeting restriction, so a Prodigal Sorcerer ping is not a
    // legal target however much its controller would like it countered.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let rust = game.spawn_in_hand(PlayerId(1), card("Rust"));
    let sorcerer = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    activate_only(
        &mut game,
        PlayerId(0),
        sorcerer,
        0,
        Some(Target::Player(PlayerId(1))),
    );

    assert!(
        cast(
            &mut game,
            PlayerId(1),
            rust,
            Some(Target::Object(sorcerer)),
            0
        )
        .is_err(),
        "a creature's activated ability is not an artifact source"
    );
}

#[test]
fn ayesha_tanaka_counters_an_artifact_ability_unless_its_controller_pays_white() {
    // "{T}: Counter target activated ability from an artifact source unless that ability's
    // controller pays {W}."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let ayesha = game.spawn_on_battlefield(PlayerId(1), card("Ayesha Tanaka"));
    let rod = rod_ability_on_stack(&mut game, PlayerId(0), PlayerId(1));
    let life = game.life(PlayerId(1));

    activate(&mut game, PlayerId(1), ayesha, 0, Some(Target::Object(rod)));
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::PayOrCounter {
                player: PlayerId(0),
                ..
            })
        ),
        "the *ability's* controller gets the chance to pay {{W}}"
    );

    game.fund_mana(PlayerId(0));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: true,
        discard_cost: vec![],
    })
    .unwrap();
    while !game.stack_is_empty() {
        resolve_top_of_stack(&mut game);
    }
    assert_eq!(
        game.life(PlayerId(1)),
        life - 1,
        "paying {{W}} saved the ability, so the ping landed"
    );
}

#[test]
fn ayesha_tanaka_counters_the_ability_when_its_controller_declines() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let ayesha = game.spawn_on_battlefield(PlayerId(1), card("Ayesha Tanaka"));
    let rod = rod_ability_on_stack(&mut game, PlayerId(0), PlayerId(1));
    let life = game.life(PlayerId(1));

    activate(&mut game, PlayerId(1), ayesha, 0, Some(Target::Object(rod)));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay: false,
        discard_cost: vec![],
    })
    .unwrap();
    while !game.stack_is_empty() {
        resolve_top_of_stack(&mut game);
    }
    assert_eq!(
        game.life(PlayerId(1)),
        life,
        "declining lets the ability be countered, so no damage landed"
    );
}

// ── increment 49: Infinite Authority ──────────────────────────────────────────────────

#[test]
fn infinite_authority_petrifies_a_small_blocker_then_grows_its_creature() {
    // "Whenever enchanted creature blocks or becomes blocked by a creature with toughness 3 or
    // less, destroy the other creature at end of combat. At the beginning of the next end step,
    // if that creature was destroyed this way, put a +1/+1 counter on the first creature."
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let authority = game.spawn_in_hand(PlayerId(0), card("Infinite Authority"));
    cast_aura(&mut game, PlayerId(0), authority, bear, 0);
    // Wall of Wood is 0/3 — small enough to be watched, tough enough to survive the bear's two damage, so the
    // end-of-combat destroy is the only thing that could have killed it.
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));

    attack_with(&mut game, vec![bear]);
    block_with(&mut game, vec![(wall, bear)]).expect("a 0/3 Wall can block a 2/2");
    resolve_top_of_stack(&mut game); // the granted "becomes blocked" trigger schedules the destroy

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(wall),
        Zone::Graveyard,
        "toughness 3 or less, so it turns to stone once combat is over"
    );
    assert_eq!(
        game.plus_counters(bear),
        0,
        "the reward waits for the end step"
    );

    advance_until(&mut game, |g| g.current_step() == Step::End);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.plus_counters(bear),
        1,
        "…and then the first creature gets its +1/+1 counter"
    );
}

#[test]
fn infinite_authority_ignores_a_blocker_with_toughness_four() {
    // "with toughness 3 or less" — a 0/8 Wall blocks the same creature and nothing happens, so the
    // +1/+1 counter's intervening-if never comes due either.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let authority = game.spawn_in_hand(PlayerId(0), card("Infinite Authority"));
    cast_aura(&mut game, PlayerId(0), authority, bear, 0);
    let ox = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));

    attack_with(&mut game, vec![bear]);
    block_with(&mut game, vec![(ox, bear)]).expect("a 0/8 Wall can block a 2/2");

    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(game.zone_of(ox), Zone::Battlefield, "too tough to petrify");
    assert_eq!(
        game.plus_counters(bear),
        0,
        "nothing was destroyed this way, so no counter"
    );
}
