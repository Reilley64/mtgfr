//! Legends (`leg`) grind — increment 22: change base power and toughness.
//!
//! "Change the base power and toughness of … to X/Y" is CR 613.3's layer 7b: it *replaces* the
//! running base, and everything above it still applies on top — layer 7c pumps, layer 7d counters,
//! and finally layer 7e's power/toughness switch, which is the last thing to happen to a creature's
//! P/T. So these tests never read a base field back; they set a base, stack a counter or a pump or a
//! switch over it, and assert the board fact that falls out — who died, who connected, for how much.
//!
//! Five cards: Brine Hag (0/2 to everything that damaged it, on death), Halfdane (wears a target's
//! P/T until the end of your next upkeep), Sentinel (base *toughness* only, repeatable and free),
//! Wall of Tombstones (base toughness from the graveyard, snapshotted at resolution), and
//! Transmutation (the switch).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 20]);
    }
}

/// Cast `spell` from player 0's hand at `target`, funding its cost first, and resolve it.
fn cast_at(game: &mut Game, spell: ObjectId, target: Option<ObjectId>) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: spell,
        target: target.map(Target::Object),
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
    .expect("the spell is castable");
    resolve_top_of_stack(game);
}

/// Walk to `player`'s upkeep and answer the targeted trigger waiting there with `victim`.
fn take_upkeep_trigger(game: &mut Game, player: PlayerId, victim: ObjectId) {
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Upkeep
    });
    game.submit(Intent::ChooseTargets {
        player,
        targets: vec![Target::Object(game.current_id(victim))],
    })
    .expect("the upkeep trigger takes a target");
    resolve_top_of_stack(game);
}

/// Player 0 activates `object`'s free ability at `target` and lets it resolve.
fn activate_at(game: &mut Game, object: ObjectId, target: ObjectId) {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index: 0,
        target: Some(Target::Object(target)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("the ability is activatable");
    resolve_top_of_stack(game);
}

// ── Brine Hag ─────────────────────────────────────────────────────────────────────────
// "When this creature dies, change the base power and toughness of all creatures that dealt
// damage to it this turn to 0/2. (This effect lasts indefinitely.)"

#[test]
fn brine_hag_drags_the_creature_that_killed_it_down_to_two_toughness() {
    // The 3/3 Hill Giant blocks the 2/2 Hag: the Giant takes 2 damage and kills the Hag. The dies
    // trigger sets the Giant's *base* P/T to 0/2, and the 2 damage already marked on it is now
    // lethal (CR 704.5g) — it dies to the very next state-based sweep.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let hag = game.spawn_on_battlefield(PlayerId(0), card("Brine Hag"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));

    attack_with(&mut game, vec![hag]);
    block_with(&mut game, vec![(giant, hag)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.zone_of(hag), Zone::Graveyard, "the 3/3 killed the Hag");
    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "base 0/2 with 2 damage marked is lethal — the Hag took its killer with it"
    );
}

#[test]
fn brine_hag_leaves_a_creature_that_never_damaged_it_alone() {
    // Grizzly Bears attacks alongside the Hill Giant and is never blocked, so it deals the Hag no
    // damage and keeps its printed 2/2 — "dealt damage to it" is not "was in combat with it".
    let mut game = Game::new();
    stock_libraries(&mut game);
    let hag = game.spawn_on_battlefield(PlayerId(1), card("Brine Hag"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    attack_with(&mut game, vec![giant, bears]);
    block_with(&mut game, vec![(hag, giant)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(game.zone_of(hag), Zone::Graveyard, "the 3/3 killed the Hag");
    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "the blocked Giant did damage the Hag, so it went to base 0/2 and its marked 2 was lethal"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        18,
        "the unblocked Bears connected for its printed 2 before the Hag ever died"
    );

    // Next turn the untouched Bears still swings for its printed 2.
    let before = game.life(PlayerId(1));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
    attack_with(&mut game, vec![bears]);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        before - 2,
        "the Bears never damaged the Hag, so its printed 2/2 is untouched"
    );
}

#[test]
fn a_counter_stacks_on_top_of_brine_hags_zero_two() {
    // Layer 7d rides above 7b whatever the timestamps: the counter makes the Giant a 1/3 on top of
    // the base set, not a 0/2 — and 3 toughness survives the 2 damage it took killing the Hag.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let hag = game.spawn_on_battlefield(PlayerId(1), card("Brine Hag"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    game.add_plus_counter(giant);

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(hag, giant)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(giant),
        Zone::Battlefield,
        "0/2 plus a +1/+1 counter is a 1/3, which survives the 2 damage it is carrying"
    );

    let before = game.life(PlayerId(1));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
    attack_with(&mut game, vec![giant]);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        before - 1,
        "the counter's +1 sits on top of the base 0, so the Giant hits for exactly 1"
    );
}

// ── Halfdane ──────────────────────────────────────────────────────────────────────────
// "At the beginning of your upkeep, change Halfdane's base power and toughness to the power and
// toughness of target creature other than Halfdane until the end of your next upkeep."

#[test]
fn halfdane_wears_the_power_and_toughness_of_its_target() {
    // Halfdane is a printed 3/3; pointed at a 6/4 Craw Wurm it attacks for 6.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let halfdane = game.spawn_on_battlefield(PlayerId(0), card("Halfdane"));
    let wurm = game.spawn_on_battlefield(PlayerId(1), card("Craw Wurm"));

    take_upkeep_trigger(&mut game, PlayerId(0), wurm);
    attack_with(&mut game, vec![halfdane]);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.life(PlayerId(1)),
        14,
        "Halfdane swung with the Wurm's 6 power, not its own printed 3"
    );
}

#[test]
fn halfdane_keeps_its_counters_on_top_of_the_borrowed_body() {
    // The borrowed 6/4 is a layer-7b base set, so a +1/+1 counter still applies above it: 7/5.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let halfdane = game.spawn_on_battlefield(PlayerId(0), card("Halfdane"));
    let wurm = game.spawn_on_battlefield(PlayerId(1), card("Craw Wurm"));
    game.add_plus_counter(halfdane);

    take_upkeep_trigger(&mut game, PlayerId(0), wurm);
    attack_with(&mut game, vec![halfdane]);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.life(PlayerId(1)),
        13,
        "6 borrowed power plus the counter's 1 — the counter was not overwritten by the set"
    );
}

#[test]
fn halfdane_reverts_when_its_next_upkeep_finds_nothing_to_target() {
    // "Until the end of your next upkeep" is a real duration, not just "until the trigger fires
    // again": Shock kills the only other creature, so the second upkeep's trigger has no legal
    // target, the borrowed 2/2 ends anyway, and Halfdane attacks as its printed 3/3.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let halfdane = game.spawn_on_battlefield(PlayerId(0), card("Halfdane"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let shock = game.spawn_in_hand(PlayerId(0), card("Shock"));

    take_upkeep_trigger(&mut game, PlayerId(0), bears);
    cast_at(&mut game, shock, Some(bears));
    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "2 damage kills the 2/2"
    );

    // Round the table back to player 0's next upkeep, and past it into their main phase.
    advance_until(&mut game, |g| g.active_player() == PlayerId(1));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![halfdane]);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        before - 3,
        "the borrowed body expired at the end of Halfdane's next upkeep"
    );
}

// ── Sentinel ──────────────────────────────────────────────────────────────────────────
// "{0}: Change this creature's base toughness to 1 plus the power of target creature blocking or
// blocked by this creature. (This effect lasts indefinitely.)"

#[test]
fn sentinel_sets_only_its_toughness_and_survives_what_it_blocks() {
    // A 1/1 Sentinel blocking a 3/3 sets its base toughness to 1 + 3 = 4 and lives. Its base
    // *power* is untouched — it deals its printed 1, so the Hill Giant walks away too.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let sentinel = game.spawn_on_battlefield(PlayerId(0), card("Sentinel"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(giant, Defender::Player(PlayerId(0)))],
    })
    .expect("the Giant attacks");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(0),
        blocks: vec![(sentinel, giant)],
    })
    .expect("the Sentinel blocks");

    activate_at(&mut game, sentinel, giant);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(sentinel),
        Zone::Battlefield,
        "base toughness 4 survives the Giant's 3 damage"
    );
    assert_eq!(
        game.zone_of(giant),
        Zone::Battlefield,
        "the Sentinel's base *power* was never changed, so it dealt its printed 1"
    );
}

#[test]
fn sentinels_later_activation_wins_on_timestamp() {
    // The ability is free and repeatable, so two indefinite base-toughness sets can be live at
    // once; CR 613.7 says the later timestamp wins. Setting off the 2/2 Bears second leaves the
    // Sentinel at toughness 3, which the 3-power Giant it is blocking now kills.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let sentinel = game.spawn_on_battlefield(PlayerId(0), card("Sentinel"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![
            (giant, Defender::Player(PlayerId(0))),
            (bears, Defender::Player(PlayerId(0))),
        ],
    })
    .expect("both attack");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(0),
        blocks: vec![(sentinel, giant)],
    })
    .expect("the Sentinel blocks the Giant");

    activate_at(&mut game, sentinel, giant);
    activate_at(&mut game, sentinel, bears);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(sentinel),
        Zone::Graveyard,
        "the second set (1 + the Bears' 2) replaced the first (1 + the Giant's 3)"
    );
}

// ── Wall of Tombstones ────────────────────────────────────────────────────────────────
// "Defender / At the beginning of your upkeep, change this creature's base toughness to 1 plus the
// number of creature cards in your graveyard. (This effect lasts indefinitely.)"

#[test]
fn wall_of_tombstones_grows_with_the_creatures_in_your_graveyard() {
    // Two creature cards in the graveyard make the printed 0/1 Wall a 0/3, which holds off a 2/2.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Tombstones"));
    game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));
    game.spawn_in_graveyard(PlayerId(0), card("Hill Giant"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(bears, Defender::Player(PlayerId(0)))],
    })
    .expect("the Bears attack");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(0),
        blocks: vec![(wall, bears)],
    })
    .expect("defender does not stop a creature blocking");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(wall),
        Zone::Battlefield,
        "1 + 2 creature cards in the graveyard is 3 toughness, which eats the 2/2's damage"
    );
}

#[test]
fn wall_of_tombstones_snapshots_the_count_at_resolution() {
    // CR 613.4b: the amount is locked in when the ability resolves. Creatures that hit the
    // graveyard afterwards do not push the Wall's toughness up until the *next* upkeep, so a 3/3
    // still kills the 0/1 Wall the turn it was set with an empty graveyard.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Tombstones"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    resolve_top_of_stack(&mut game);
    for _ in 0..4 {
        game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));
    }

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(giant, Defender::Player(PlayerId(0)))],
    })
    .expect("the Giant attacks");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(0),
        blocks: vec![(wall, giant)],
    })
    .expect("defender does not stop a creature blocking");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(wall),
        Zone::Graveyard,
        "the Wall is still the 0/1 it was set to with an empty graveyard — the four creature cards \
         that arrived after resolution change nothing until the next upkeep"
    );
}

// ── Transmutation ─────────────────────────────────────────────────────────────────────
// "Switch target creature's power and toughness until end of turn."

#[test]
fn transmutation_switches_after_the_counters_have_applied() {
    // CR 613.4e: the switch is the *last* thing that happens to a creature's P/T. A 6/4 Craw Wurm
    // with a -0/-2 counter is a 6/2 by the time the switch runs, so it ends up a 2/6 — not the 4/4
    // a switch applied down in layer 7b (4/6, then the counter) would have produced.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wurm = game.spawn_on_battlefield(PlayerId(0), card("Craw Wurm"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let transmutation = game.spawn_in_hand(PlayerId(0), card("Transmutation"));
    game.add_kind_counter(wurm, CounterKind::MinusZeroMinusTwo);

    cast_at(&mut game, transmutation, Some(wurm));
    attack_with(&mut game, vec![wurm]);
    block_with(&mut game, vec![(giant, wurm)]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(giant),
        Zone::Battlefield,
        "the switched Wurm hit for 2, not the 4 a pre-counter switch would have given it"
    );
    assert_eq!(
        game.zone_of(wurm),
        Zone::Battlefield,
        "6 toughness after the switch shrugs off the Giant's 3"
    );
}

#[test]
fn transmutation_wears_off_at_end_of_turn() {
    // "Until end of turn" — the 2/6 is a 6/2 again next turn, and connects for its printed 6.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wurm = game.spawn_on_battlefield(PlayerId(0), card("Craw Wurm"));
    let transmutation = game.spawn_in_hand(PlayerId(0), card("Transmutation"));

    cast_at(&mut game, transmutation, Some(wurm));
    // Round the table back to player 0's next turn, past the cleanup that ends the switch.
    advance_until(&mut game, |g| g.active_player() == PlayerId(1));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![wurm]);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        before - 6,
        "the switch ended at cleanup, so the Wurm is a 6/4 again"
    );
}
