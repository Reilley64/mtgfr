//! Legends (`leg`) grind — increment 102: counter-kinds-legends-prints.
//!
//! Osai Vultures' carrion counters (a named counter that only its own card reads) and Spirit
//! Shackle's -0/-2 counters (a P/T counter the layer-7d read must apply alongside +1/+1 and
//! -1/-1, and which CR 121.3 does *not* pair off against either of them).

mod common;

use common::*;
use engine::*;

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
    .unwrap();
    resolve_top_of_stack(game);
}

fn activate(
    game: &mut Game,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
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

/// Give every seat a library, so rolling the game through whole turns doesn't eliminate the other
/// players on an empty-library draw (which would leave player 0 the only seat and wedge
/// `advance_until`'s "the active player changed" predicate forever).
fn stock_libraries(game: &mut Game) {
    let deck = vec![card("Plains"); 40];
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &deck);
    }
}

/// Roll forward from player 0's turn to player 0's *next* precombat main phase.
fn advance_to_your_next_turn(game: &mut Game) {
    advance_until(game, |g| g.active_player() != PlayerId(0));
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
}

/// Kill a creature this turn, then let the end step's "if a creature died this turn" trigger
/// resolve — one carrion counter's worth. Leaves the game parked in the end step.
fn bank_a_carrion_counter(game: &mut Game) {
    let victim = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let shock = game.spawn_in_hand(PlayerId(0), card("Shock"));
    cast_and_resolve(game, shock, Some(Target::Object(victim)));
    assert_eq!(
        game.zone_of(victim),
        Zone::Graveyard,
        "2 damage kills a 2/2"
    );

    advance_until(game, |g| g.current_step() == Step::End);
    resolve_top_of_stack(game);
}

/// Tap `object` with an Icy Manipulator, untapping the Manipulator afterwards so the next call
/// can reuse it. (`{1}, {T}: Tap target artifact, creature, or land.`)
fn tap_with_icy(game: &mut Game, icy: ObjectId, object: ObjectId) {
    activate(game, icy, 0, Some(Target::Object(object))).expect("{1}, {T} is payable");
    resolve_top_of_stack(game); // the Manipulator's ability
    if !game.stack().is_empty() {
        resolve_top_of_stack(game); // whatever the tap itself triggered
    }
    game.untap(icy);
}

// ── Osai Vultures: carrion counters ───────────────────────────────────────────────────

#[test]
fn osai_vultures_banks_a_carrion_counter_when_a_creature_died_this_turn() {
    // "At the beginning of each end step, if a creature died this turn, put a carrion counter on
    // this creature."
    let mut game = Game::new();
    let vultures = game.spawn_on_battlefield(PlayerId(0), card("Osai Vultures"));

    bank_a_carrion_counter(&mut game);

    assert_eq!(
        game.counters_of_kind(vultures, CounterKind::Carrion),
        1,
        "one carrion counter for the end step a creature died in"
    );
    // A carrion counter is inert bookkeeping — it is not a P/T counter (CR 122.1).
    assert_eq!(
        (game.power(vultures), game.toughness(vultures)),
        (1, 1),
        "the counter itself changes nothing about the Bird"
    );
}

#[test]
fn osai_vultures_banks_nothing_in_an_end_step_with_no_death() {
    // The intervening-if clause: no creature died, so the ability never triggers at all
    // (CR 603.4).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vultures = game.spawn_on_battlefield(PlayerId(0), card("Osai Vultures"));

    advance_until(&mut game, |g| g.current_step() == Step::End);
    advance_to_your_next_turn(&mut game);

    assert_eq!(
        game.counters_of_kind(vultures, CounterKind::Carrion),
        0,
        "nothing died in any of those end steps"
    );
}

#[test]
fn osai_vultures_cannot_pay_the_pump_with_a_single_carrion_counter() {
    // "Remove two carrion counters from this creature: ..." — one banked counter cannot pay it.
    let mut game = Game::new();
    let vultures = game.spawn_on_battlefield(PlayerId(0), card("Osai Vultures"));

    bank_a_carrion_counter(&mut game);
    assert_eq!(game.counters_of_kind(vultures, CounterKind::Carrion), 1);

    assert!(
        activate(&mut game, vultures, 1, None).is_err(),
        "two carrion counters are the cost; one is not enough"
    );
}

#[test]
fn osai_vultures_spends_two_carrion_counters_to_swing_for_two() {
    // "Remove two carrion counters from this creature: This creature gets +1/+1 until end of
    // turn." A 1/1 flier that becomes a 2/2 deals 2 combat damage, not 1.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vultures = game.spawn_on_battlefield(PlayerId(0), card("Osai Vultures"));

    bank_a_carrion_counter(&mut game);
    advance_to_your_next_turn(&mut game);
    bank_a_carrion_counter(&mut game);
    advance_to_your_next_turn(&mut game);
    assert_eq!(
        game.counters_of_kind(vultures, CounterKind::Carrion),
        2,
        "one counter per end step a creature died in"
    );

    activate(&mut game, vultures, 1, None).expect("two carrion counters pay the cost");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.counters_of_kind(vultures, CounterKind::Carrion),
        0,
        "both counters were removed to pay the cost"
    );
    assert_eq!(
        (game.power(vultures), game.toughness(vultures)),
        (2, 2),
        "1/1 base plus +1/+1 until end of turn"
    );

    attack_with(&mut game, vec![vultures]);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.life(PlayerId(1)),
        18,
        "the pumped 2/2 flier connected for 2"
    );
}

// ── Spirit Shackle: -0/-2 counters ────────────────────────────────────────────────────

#[test]
fn spirit_shackle_puts_a_minus_zero_minus_two_counter_when_its_host_becomes_tapped() {
    // "Enchant creature / Whenever enchanted creature becomes tapped, put a -0/-2 counter on it."
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // 3/3
    let icy = game.spawn_on_battlefield(PlayerId(0), card("Icy Manipulator"));
    let shackle = game.spawn_in_hand(PlayerId(0), card("Spirit Shackle"));

    cast_and_resolve(&mut game, shackle, Some(Target::Object(giant)));
    tap_with_icy(&mut game, icy, giant);

    assert_eq!(
        game.counters_of_kind(giant, CounterKind::MinusZeroMinusTwo),
        1,
        "becoming tapped put one -0/-2 counter on the enchanted creature"
    );
    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (3, 1),
        "3/3 base, toughness only reduced — layer 7d (CR 613.4)"
    );
    assert_eq!(
        game.zone_of(giant),
        Zone::Battlefield,
        "toughness 1 is still alive"
    );
}

#[test]
fn spirit_shackle_kills_its_host_once_the_counters_reach_its_toughness() {
    // Two taps, two -0/-2 counters: a 3/3 becomes 3/-1 and is put into its owner's graveyard as a
    // state-based action (CR 704.5f).
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant")); // 3/3
    let icy = game.spawn_on_battlefield(PlayerId(0), card("Icy Manipulator"));
    let shackle = game.spawn_in_hand(PlayerId(0), card("Spirit Shackle"));

    cast_and_resolve(&mut game, shackle, Some(Target::Object(giant)));
    tap_with_icy(&mut game, icy, giant);
    game.untap(giant);
    tap_with_icy(&mut game, icy, giant);

    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "3/3 with two -0/-2 counters has 0 or less toughness and dies"
    );
}

#[test]
fn minus_zero_minus_two_counters_do_not_annihilate_plus_one_plus_one_counters() {
    // CR 121.3 removes a +1/+1 counter and a -1/-1 counter in pairs. A -0/-2 counter is a
    // different kind, so nothing is paired off — both sit on the creature and both apply.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant")); // 3/3
    let shell = game.spawn_on_battlefield(PlayerId(0), card("Shambling Shell"));
    let icy = game.spawn_on_battlefield(PlayerId(0), card("Icy Manipulator"));
    let shackle = game.spawn_in_hand(PlayerId(0), card("Spirit Shackle"));

    // "Sacrifice this creature: Put a +1/+1 counter on target creature."
    activate(&mut game, shell, 0, Some(Target::Object(giant))).expect("the sacrifice pays it");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.plus_counters(giant), 1);

    cast_and_resolve(&mut game, shackle, Some(Target::Object(giant)));
    tap_with_icy(&mut game, icy, giant);

    assert_eq!(
        game.plus_counters(giant),
        1,
        "the +1/+1 counter is untouched — CR 121.3 pairs it only with a -1/-1 counter"
    );
    assert_eq!(
        game.counters_of_kind(giant, CounterKind::MinusZeroMinusTwo),
        1,
        "the -0/-2 counter is untouched"
    );
    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (4, 2),
        "3/3, plus 1/1 from the +1/+1 counter, minus 0/2 from the -0/-2 counter"
    );
}

#[test]
fn minus_zero_minus_two_counters_stack_with_minus_one_minus_one_counters() {
    // Layer 7d applies every counter kind on the permanent, so a -1/-1 counter and a -0/-2
    // counter together take a 3/3 to 2/0 — dead.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant")); // 3/3
    let icy = game.spawn_on_battlefield(PlayerId(0), card("Icy Manipulator"));
    let shackle = game.spawn_in_hand(PlayerId(0), card("Spirit Shackle"));
    let clasp = game.spawn_in_hand(PlayerId(0), card("Contagion Clasp"));

    cast_and_resolve(&mut game, shackle, Some(Target::Object(giant)));
    tap_with_icy(&mut game, icy, giant);
    assert_eq!(
        (game.power(giant), game.toughness(giant)),
        (3, 1),
        "the -0/-2 counter alone leaves it alive at 3/1"
    );

    // "When this artifact enters, put a -1/-1 counter on target creature."
    cast_and_resolve(&mut game, clasp, None);
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(giant)],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "3/3 minus 1/1 minus 0/2 is 2/0 — it dies"
    );
}
