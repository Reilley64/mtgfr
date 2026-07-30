//! Legends (`leg`) grind — increment 21: blood-lust-conditional-pump.
//!
//! Blood Lust: "If target creature has toughness 5 or greater, it gets +4/-4 until end of turn.
//! Otherwise, it gets +4/-X until end of turn, where X is its toughness minus 1." X is computed
//! once at resolution (CR 608.2g) and locked in as a fixed `TempBoost` delta (CR 613.4) — later
//! changes to the target's toughness stack on top of that locked value rather than recomputing it.

mod common;

use common::*;
use engine::*;

/// Cast Blood Lust from player 0's hand at `target`, funding its own cost first.
fn cast_blood_lust(game: &mut Game, blood_lust: ObjectId, target: ObjectId) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: blood_lust,
        target: Some(Target::Object(target)),
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

#[test]
fn blood_lust_never_kills_a_one_toughness_creature() {
    // Toughness 1 is below the 5-or-greater threshold, so X = 1 - 1 = 0: the Merfolk gets +4/-0
    // and lives at 1 toughness, not -4/-4's -3.
    let mut game = Game::new();
    let merfolk = game.spawn_on_battlefield(PlayerId(0), card("Merfolk of the Pearl Trident"));
    let blood_lust = game.spawn_in_hand(PlayerId(0), card("Blood Lust"));

    cast_blood_lust(&mut game, blood_lust, merfolk);
    assert_eq!(
        game.zone_of(merfolk),
        Zone::Battlefield,
        "a 1/1 must not die to its own toughness minus 1"
    );

    attack_with(&mut game, vec![merfolk]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        15,
        "the pumped 5 power connected unblocked"
    );
}

#[test]
fn blood_lust_gives_a_flat_minus_four_toughness_at_the_five_toughness_threshold() {
    // Toughness 5 meets "5 or greater", so this is the flat +4/-4 branch, not the X branch —
    // Barktooth Warbeard (6/5, vanilla) ends at 10/1, not dead.
    let mut game = Game::new();
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let blood_lust = game.spawn_in_hand(PlayerId(0), card("Blood Lust"));

    cast_blood_lust(&mut game, blood_lust, barktooth);
    assert_eq!(
        game.zone_of(barktooth),
        Zone::Battlefield,
        "5 toughness minus 4 is 1, not lethal"
    );

    attack_with(&mut game, vec![barktooth]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        10,
        "the pumped 10 power connected unblocked"
    );
}

#[test]
fn blood_lust_toughness_delta_stays_locked_when_the_target_later_gains_toughness() {
    // The Merfolk's +4/-0 is computed once, at resolution, from its toughness at that moment (1).
    // Two +1/+1 counters land afterward and stack on top of that locked delta (5/1 -> 7/3), rather
    // than the whole pump being recomputed from the creature's new, higher toughness.
    let mut game = Game::new();
    let merfolk = game.spawn_on_battlefield(PlayerId(0), card("Merfolk of the Pearl Trident"));
    let blood_lust = game.spawn_in_hand(PlayerId(0), card("Blood Lust"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    cast_blood_lust(&mut game, blood_lust, merfolk);
    game.add_plus_counter(merfolk);
    game.add_plus_counter(merfolk);

    attack_with(&mut game, vec![merfolk]);
    block_with(&mut game, vec![(bears, merfolk)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "the 7-power Merfolk killed the 2/2 blocker"
    );
    assert_eq!(
        game.zone_of(merfolk),
        Zone::Battlefield,
        "3 toughness (1 locked by Blood Lust, +2 from counters after) survives 2 damage — a \
         recomputed formula reading the post-counter toughness would have shrunk the pump instead"
    );
}

#[test]
fn blood_lust_toughness_delta_stays_locked_when_the_target_later_loses_toughness() {
    // Barktooth's flat -4 is locked in at 6/5 -> 10/1. A -1/-1 counter placed afterward stacks on
    // top of that locked delta and brings it to 0 toughness — proving Blood Lust is a one-time
    // pump, not a continuous "toughness can't go below 1" effect that would keep saving it.
    let mut game = Game::new();
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let blood_lust = game.spawn_in_hand(PlayerId(0), card("Blood Lust"));

    cast_blood_lust(&mut game, blood_lust, barktooth);
    game.add_kind_counter(barktooth, CounterKind::MinusOneMinusOne);
    // Any action prompts a state-based-action sweep.
    game.submit(Intent::PassPriority {
        player: game.priority_holder(),
    })
    .unwrap();

    assert_eq!(
        game.zone_of(barktooth),
        Zone::Graveyard,
        "1 toughness locked by Blood Lust, minus 1 from the counter, is lethal (CR 704.5g)"
    );
}
