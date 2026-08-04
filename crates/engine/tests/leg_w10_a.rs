//! Legends (`leg`) grind, wave 10 slice A — the turn-scoped damage ledger and its readers.
//!
//! Increments 19 (`damage-dealt-this-turn-ledger`), 90 (`dealt-damage-to-opponent-this-turn`),
//! 112 (`damaged-player-discards-their-hand`), 113 (`gain-life-equal-to-mass-damage-dealt`),
//! 118 (`damage-to-any-player-trigger`) and 130 (`damage-cause-tracking`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 40]);
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step.
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
) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
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
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    cast(game, player, object, target).unwrap();
    resolve_top_of_stack(game);
}

// ── increment 113: Syphon Soul ────────────────────────────────────────────────────────
//
// "Syphon Soul deals 2 damage to each other player. You gain life equal to the damage dealt
// this way."

#[test]
fn syphon_soul_gains_life_equal_to_the_damage_dealt_to_every_other_player() {
    let mut game = Game::with_players(4, 7);
    stock_libraries(&mut game);
    let soul = game.spawn_in_hand(PlayerId(0), card("Syphon Soul"));
    let before = game.life(PlayerId(0));

    cast_and_resolve(&mut game, PlayerId(0), soul, None);

    for seat in 1..4 {
        assert_eq!(
            game.life(PlayerId(seat)),
            before - 2,
            "each other player takes 2"
        );
    }
    assert_eq!(
        game.life(PlayerId(0)),
        before + 6,
        "you gain life equal to all six points dealt this way"
    );
}

#[test]
fn syphon_soul_gains_nothing_for_damage_a_shield_ate() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    // Circle of Protection: Black stops a black source's damage outright, so nothing is dealt to
    // the shielded seat and Syphon Soul's rider sizes off zero. (CR 615, CR 120.8)
    let cop = game.spawn_on_battlefield(PlayerId(1), card("Circle of Protection: Black"));
    game.fund_mana(PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: cop,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    let soul = game.spawn_in_hand(PlayerId(0), card("Syphon Soul"));
    let before = game.life(PlayerId(0));
    let victim_before = game.life(PlayerId(1));

    cast_and_resolve(&mut game, PlayerId(0), soul, None);

    assert_eq!(
        game.life(PlayerId(1)),
        victim_before,
        "the Circle ate the whole hit"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        before,
        "no damage was dealt this way, so no life is gained"
    );
}

// ── increment 112: Nicol Bolas ────────────────────────────────────────────────────────
//
// "Whenever Nicol Bolas deals damage to an opponent, that player discards their hand."

#[test]
fn nicol_bolas_empties_the_hand_of_the_opponent_it_damaged() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let bolas = game.spawn_on_battlefield(PlayerId(0), card("Nicol Bolas"));
    let theirs: Vec<ObjectId> = (0..3)
        .map(|_| game.spawn_in_hand(PlayerId(1), card("Grizzly Bears")))
        .collect();
    let mine = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));

    attack_with(&mut game, vec![bolas]);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    for card in &theirs {
        assert_eq!(
            game.zone_of(*card),
            Zone::Graveyard,
            "the damaged opponent discarded their whole hand"
        );
    }
    assert_eq!(
        game.zone_of(mine),
        Zone::Hand,
        "Bolas's controller keeps their own hand — 'that player' is the damaged one"
    );
}

// ── increment 118: Pit Scorpion ───────────────────────────────────────────────────────
//
// "Whenever this creature deals damage to a player, that player gets a poison counter."

#[test]
fn pit_scorpion_poisons_the_opponent_it_damages_in_combat() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let scorpion = game.spawn_on_battlefield(PlayerId(0), card("Pit Scorpion"));

    attack_with(&mut game, vec![scorpion]);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        1,
        "the damaged player got a poison counter"
    );
}

#[test]
fn pit_scorpion_poisons_its_own_controller_when_a_redirect_sends_the_damage_there() {
    // The oracle says "a player", not "an opponent". Jade Monolith moves the Scorpion's combat
    // damage off the blocker and onto the Monolith's controller (CR 615.10) — who is also the
    // Scorpion's controller, and who gets poisoned all the same.
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let scorpion = game.spawn_on_battlefield(PlayerId(0), card("Pit Scorpion"));
    let monolith = game.spawn_on_battlefield(PlayerId(0), card("Jade Monolith"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));

    give_priority(&mut game, PlayerId(0));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: monolith,
        ability_index: 0,
        target: Some(Target::Object(blocker)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("the Monolith can shield any creature");
    resolve_top_of_stack(&mut game);

    attack_with(&mut game, vec![scorpion]);
    block_with(&mut game, vec![(blocker, scorpion)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.player_counters(PlayerId(0), PlayerCounterKind::Poison),
        1,
        "the Scorpion damaged its own controller, and 'a player' includes them"
    );
    assert_eq!(
        game.player_counters(PlayerId(1), PlayerCounterKind::Poison),
        0,
        "the blocker's controller took none of that damage"
    );
}

// ── increment 90: Whirling Dervish ────────────────────────────────────────────────────
//
// "At the beginning of each end step, if this creature dealt damage to an opponent this turn, put
// a +1/+1 counter on it."

#[test]
fn whirling_dervish_grows_on_the_end_step_after_it_connects() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let dervish = game.spawn_on_battlefield(PlayerId(0), card("Whirling Dervish"));

    attack_with(&mut game, vec![dervish]);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.power(dervish),
        2,
        "the end-step trigger saw the combat damage and put a +1/+1 counter on it"
    );
}

#[test]
fn whirling_dervish_stays_put_on_an_end_step_it_dealt_no_damage() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let dervish = game.spawn_on_battlefield(PlayerId(0), card("Whirling Dervish"));

    advance_until(&mut game, |g| g.current_step() == Step::End);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.power(dervish),
        1,
        "the intervening-if failed — no damage to an opponent this turn, no counter"
    );
}

// ── increment 19: Blazing Effigy ──────────────────────────────────────────────────────
//
// "When this creature dies, it deals X damage to target creature, where X is 3 plus the amount of
// damage dealt to this creature this turn by other sources named Blazing Effigy."

#[test]
fn blazing_effigy_adds_the_damage_a_sibling_effigy_dealt_it() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let first = game.spawn_on_battlefield(PlayerId(0), card("Blazing Effigy"));
    let second = game.spawn_on_battlefield(PlayerId(0), card("Blazing Effigy"));
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Pelakka Wurm"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    // Bolt the first Effigy. Its dies trigger deals a flat 3 to the second (0/3), killing it —
    // and the Bolt's own 3 must not count, since Lightning Bolt is not named Blazing Effigy.
    cast(&mut game, PlayerId(0), bolt, Some(Target::Object(first))).unwrap();
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(second)],
    })
    .expect("the first Effigy's dies trigger picks the second");
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(second),
        Zone::Graveyard,
        "3 damage from a sibling Effigy is lethal to a 0/3"
    );

    // The second Effigy's own trigger now reads 3 + the 3 its sibling dealt it.
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(victim)],
    })
    .expect("the second Effigy's dies trigger picks the Wurm");
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.marked_damage(victim),
        6,
        "3 plus the 3 dealt to it this turn by another source named Blazing Effigy"
    );
}


// ── increment 19: Backdraft ───────────────────────────────────────────────────────────
//
// "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that
// player equal to half the damage dealt by one of those sorcery spells this turn, rounded down."

#[test]
fn backdraft_deals_half_the_damage_the_chosen_players_sorcery_dealt() {
    let mut game = Game::with_players(4, 7);
    stock_libraries(&mut game);
    // Syphon Soul is a sorcery that deals 2 to each of the other three seats — 6 damage.
    let soul = game.spawn_in_hand(PlayerId(1), card("Syphon Soul"));
    cast_and_resolve(&mut game, PlayerId(1), soul, None);

    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));
    let before = game.life(PlayerId(1));
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        backdraft,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before - 3,
        "half of the 6 that sorcery dealt, rounded down"
    );
}

#[test]
fn backdraft_deals_nothing_to_a_player_whose_sorceries_dealt_no_damage() {
    let mut game = Game::with_players(2, 7);
    stock_libraries(&mut game);
    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));
    let before = game.life(PlayerId(1));

    cast_and_resolve(
        &mut game,
        PlayerId(0),
        backdraft,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "no sorcery, no damage — 0 is never dealt (CR 120.8)"
    );
}

// ── increment 19: Reverberation ───────────────────────────────────────────────────────
//
// "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's
// controller instead."

#[test]
fn reverberation_turns_a_sorcerys_damage_back_on_its_caster() {
    let mut game = Game::with_players(4, 7);
    stock_libraries(&mut game);
    // Syphon Soul is P1's sorcery: 2 to each other player, and it gains its controller life equal
    // to the damage dealt this way.
    let soul = game.spawn_in_hand(PlayerId(1), card("Syphon Soul"));
    let reverb = game.spawn_in_hand(PlayerId(0), card("Reverberation"));
    let before: Vec<i32> = (0..4).map(|p| game.life(PlayerId(p))).collect();

    cast(&mut game, PlayerId(1), soul, None).expect("P1 casts the sorcery");
    cast(&mut game, PlayerId(0), reverb, Some(Target::Object(soul)))
        .expect("Reverberation targets the sorcery on the stack");
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);

    for seat in [0, 2, 3] {
        assert_eq!(
            game.life(PlayerId(seat)),
            before[seat as usize],
            "the sorcery's damage never reached the other players"
        );
    }
    assert_eq!(
        game.life(PlayerId(1)),
        before[1] - 6 + 6,
        "all six points came back at the caster, who still gains life for damage dealt this way"
    );
}
