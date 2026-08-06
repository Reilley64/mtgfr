//! Legends (`leg`) grind — increments 51 (Kismet), 57 (Mirror Universe), 72 (Reset), 84
//! ("Activate no more than twice each turn") and 91 (Winds of Change).
//!
//! Two of these are wave 8 holes rather than new work: `max_activations_per_turn` shipped with
//! Vampire Bats and `LifeEffect::Exchange` shipped without Mirror Universe, and in both cases
//! nothing asserted the behaviour, so either could have stopped working without a test going red.
//! The cap is an activation restriction (CR 602.2b), so the third activation is refused outright
//! rather than countered on resolution — and the ledger it counts against is per-turn, so a new
//! turn restores both uses.

mod common;

use common::*;
use engine::*;

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 20]);
    }
}

/// The live card ids in `player`'s hand.
fn hand_ids(game: &Game, player: PlayerId) -> Vec<ObjectId> {
    game.live_object_ids()
        .into_iter()
        .filter(|&id| game.zone_of(id) == Zone::Hand && game.owner_of(id) == player)
        .collect()
}

/// Player 0 casts a targetless spell.
fn cast(game: &mut Game, object: ObjectId) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object,
        target: None,
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
    .unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));
}

/// Player 0 activates `object`'s only ability, funding the mana first.
fn firebreathe(game: &mut Game, object: ObjectId) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

#[test]
fn vampire_bats_refuses_a_third_activation_in_one_turn() {
    // Two are allowed and each lands its +1/+0, so the 1/1 is a 3/1. The third is refused as the
    // activation is announced, not countered later.
    let mut game = Game::new();
    let bats = game.spawn_on_battlefield(PlayerId(0), card("Vampire Bats"));

    firebreathe(&mut game, bats).expect("the first activation is free of the cap");
    resolve_top_of_stack(&mut game);
    firebreathe(&mut game, bats).expect("the second reaches the cap without exceeding it");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.power(bats),
        3,
        "two +1/+0 pumps landed on the printed 1"
    );

    assert_eq!(
        firebreathe(&mut game, bats),
        Err(Reject::CannotActivate),
        "\"Activate no more than twice each turn\" — the third is refused"
    );
    assert_eq!(game.power(bats), 3, "the refused activation pumped nothing");
}

#[test]
fn vampire_bats_gets_its_two_activations_back_next_turn() {
    // The cap counts activations *this turn*, so it resets — and the pumps from the previous turn
    // are gone at cleanup, putting the Bats back on its printed 1 power before the new pair.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bats = game.spawn_on_battlefield(PlayerId(0), card("Vampire Bats"));

    firebreathe(&mut game, bats).expect("first of turn one");
    resolve_top_of_stack(&mut game);
    firebreathe(&mut game, bats).expect("second of turn one");
    resolve_top_of_stack(&mut game);
    firebreathe(&mut game, bats).expect_err("third of turn one is refused");

    // Round the table — stepping off player 0's turn first, or the predicate below is already true
    // and we never leave the turn we activated in.
    advance_until(&mut game, |g| g.active_player() != PlayerId(0));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });

    assert_eq!(game.power(bats), 1, "last turn's pumps wore off at cleanup");

    firebreathe(&mut game, bats).expect("a new turn restores the first use");
    resolve_top_of_stack(&mut game);
    firebreathe(&mut game, bats).expect("and the second");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.power(bats), 3, "both fresh activations landed");
    assert_eq!(
        firebreathe(&mut game, bats),
        Err(Reject::CannotActivate),
        "the cap applies again on the new turn"
    );
}

// ── Winds of Change (increment 91) ──────────────────────────────────────────────────────────

#[test]
fn winds_of_change_redraws_each_player_their_own_hand_size() {
    // "Each player shuffles the cards from their hand into their library, then draws that many
    // cards." Timetwister's count is a flat seven for the table; this one is per player, read
    // before that player shuffles — so three seats holding different numbers of cards each get
    // their own number back. Graveyards are untouched, and Winds of Change is on the stack while
    // everyone shuffles, so it is not swept up (CR 608.2m).
    let mut game = Game::with_players(3, 0);
    let p0_hand = [
        game.spawn_in_hand(PlayerId(0), card("Forest")),
        game.spawn_in_hand(PlayerId(0), card("Forest")),
    ];
    let p1_hand = game.spawn_in_hand(PlayerId(1), card("Forest"));
    let p2_hand: Vec<ObjectId> = (0..4)
        .map(|_| game.spawn_in_hand(PlayerId(2), card("Forest")))
        .collect();
    let p2_yard = game.spawn_in_graveyard(PlayerId(2), card("Forest"));
    stock_libraries(&mut game);

    let winds = game.spawn_in_hand(PlayerId(0), card("Winds of Change"));
    cast(&mut game, winds);
    advance_until(&mut game, |g| g.stack_is_empty());

    for (player, expected) in [(0, 2), (1, 1), (2, 4)] {
        assert_eq!(
            hand_ids(&game, PlayerId(player)).len(),
            expected,
            "player {player} redraws exactly what they shuffled away, not the table's average"
        );
    }
    for id in p0_hand.into_iter().chain([p1_hand]).chain(p2_hand) {
        // Library, or back in hand if the redraw turned it up again — never a graveyard.
        assert!(
            matches!(game.zone_of(id), Zone::Library | Zone::Hand),
            "the shuffled cards went into libraries, not graveyards"
        );
    }
    assert_eq!(
        game.zone_of(p2_yard),
        Zone::Graveyard,
        "only hands are shuffled in — the graveyard is left alone"
    );
    assert_eq!(
        game.zone_of(winds),
        Zone::Graveyard,
        "Winds of Change was on the stack while everyone shuffled"
    );
}

#[test]
fn winds_of_change_leaves_an_empty_handed_player_empty_handed() {
    // "That many" is zero for a player holding nothing, so they draw nothing — the symmetry does
    // not hand them a free card.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let winds = game.spawn_in_hand(PlayerId(0), card("Winds of Change"));

    cast(&mut game, winds);
    advance_until(&mut game, |g| g.stack_is_empty());

    assert!(
        hand_ids(&game, PlayerId(1)).is_empty(),
        "an empty hand shuffles nothing in and draws nothing back"
    );
    assert!(
        hand_ids(&game, PlayerId(0)).is_empty(),
        "the caster's hand held only Winds of Change, which was on the stack"
    );
}

// ── Reset (increment 72) ────────────────────────────────────────────────────────────────────

/// Roll to `step` of player 1's turn and hand priority to player 0 — the active player receives
/// it first in every step (CR 117.3a), so player 0's window opens only once they pass.
fn opponents_step(game: &mut Game, step: Step) {
    advance_until(game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == step
    });
    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .expect("the active player passes priority first");
}

/// Try to cast `spell` for player 0 right now, funding the mana first.
fn try_cast(game: &mut Game, spell: ObjectId) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: spell,
        target: None,
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

#[test]
fn reset_is_shut_on_your_own_turn_and_through_an_opponents_upkeep() {
    // "Cast this spell only during an opponent's turn after their upkeep step" is two
    // restrictions in one sentence, and each shuts the window on its own.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let reset = game.spawn_in_hand(PlayerId(0), card("Reset"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        try_cast(&mut game, reset),
        Err(Reject::WrongTiming),
        "it is player 0's own turn — the seat half of the restriction"
    );

    opponents_step(&mut game, Step::Upkeep);
    assert_eq!(
        try_cast(&mut game, reset),
        Err(Reject::WrongTiming),
        "the right seat, but the upkeep is still the wrong step"
    );
}

#[test]
fn reset_untaps_only_your_lands_from_the_opponents_draw_step_on() {
    // The window opens the step after the upkeep, and "lands you control" leaves the active
    // player's own mana exactly where it was.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let my_creature = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let reset = game.spawn_in_hand(PlayerId(0), card("Reset"));

    // Tapped *after* arriving, or player 1's own untap step would have handed their land back.
    opponents_step(&mut game, Step::Draw);
    for id in [mine, my_creature, theirs] {
        game.tap(id);
    }

    try_cast(&mut game, reset).expect("an opponent's draw step is after their upkeep");
    advance_until(&mut game, |g| g.stack_is_empty());

    assert!(!game.is_tapped(mine), "my land untapped");
    assert!(
        game.is_tapped(my_creature),
        "\"all lands\" is not \"all permanents\" — my creature stays tapped"
    );
    assert!(
        game.is_tapped(theirs),
        "\"you control\" — the active player's land is untouched"
    );
}

// ── Mirror Universe (increment 57) ──────────────────────────────────────────────────────────

/// Player 0 activates `object`'s only ability at `opponent`, funding the mana first.
fn swap_with(game: &mut Game, object: ObjectId, opponent: PlayerId) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index: 0,
        target: Some(Target::Player(opponent)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

#[test]
fn mirror_universe_exchanges_life_totals_with_the_targeted_opponent() {
    // "Exchange life totals with target opponent" (CR 118.7): both seats end on the other's
    // number, and the untargeted third seat is not part of the trade. The artifact sacrifices
    // itself as a cost, so it is in the graveyard before the ability even resolves.
    let mut game = Game::with_players(3, 0);
    stock_libraries(&mut game);
    let mirror = game.spawn_on_battlefield(PlayerId(0), card("Mirror Universe"));
    game.set_life(PlayerId(0), 3);
    game.set_life(PlayerId(1), 34);
    game.set_life(PlayerId(2), 21);

    // The game opens in a main phase, so the first upkeep player 0 gets priority in is the one on
    // their *next* turn, a full round away.
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    swap_with(&mut game, mirror, PlayerId(1)).expect("your own upkeep is the window");
    assert_eq!(
        game.zone_of(mirror),
        Zone::Graveyard,
        "\"Sacrifice this artifact\" is a cost, paid on announcement"
    );

    advance_until(&mut game, |g| g.stack_is_empty());

    assert_eq!(game.life(PlayerId(0)), 34, "took the opponent's total");
    assert_eq!(game.life(PlayerId(1)), 3, "and handed over their own");
    assert_eq!(
        game.life(PlayerId(2)),
        21,
        "the untargeted opponent is not part of the exchange"
    );
}

#[test]
fn mirror_universe_is_shut_outside_your_upkeep() {
    // "Activate only during your upkeep" — the main phase of the same turn is already too late.
    let mut game = Game::with_players(2, 0);
    let mirror = game.spawn_on_battlefield(PlayerId(0), card("Mirror Universe"));
    game.set_life(PlayerId(0), 3);
    game.set_life(PlayerId(1), 34);

    advance_until(&mut game, |g| g.current_step() == Step::Main1);

    assert_eq!(
        swap_with(&mut game, mirror, PlayerId(1)),
        Err(Reject::WrongTiming),
        "the upkeep has passed"
    );
    assert_eq!(game.life(PlayerId(0)), 3, "no exchange happened");
}

// ── Kismet (increment 51) ───────────────────────────────────────────────────────────────────

#[test]
fn kismet_taps_the_permanents_your_opponents_play_but_not_your_own() {
    // "Artifacts, creatures, and lands your opponents control enter tapped" (CR 614.13). Player 1
    // is the one under the lock: their land arrives tapped, while player 0's identical land — and
    // Kismet's own controller's creature — arrive untapped.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Kismet"));
    let my_land = game.spawn_in_hand(PlayerId(0), card("Forest"));
    let their_land = game.spawn_in_hand(PlayerId(1), card("Forest"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    game.submit(Intent::PlayLand {
        player: PlayerId(0),
        object: my_land,
    })
    .expect("player 0 may play a land in their main phase");
    assert!(
        !game.is_tapped(game.current_id(my_land)),
        "\"your opponents control\" — Kismet's own controller is untouched"
    );

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });
    game.submit(Intent::PlayLand {
        player: PlayerId(1),
        object: their_land,
    })
    .expect("player 1 may play a land in their main phase");
    assert!(
        game.is_tapped(game.current_id(their_land)),
        "the opponent's land enters tapped"
    );
}

#[test]
fn kismet_leaves_an_opponents_enchantment_alone() {
    // "Artifacts, creatures, and lands" is the whole list — an enchantment resolving under the
    // same opponent is not on it, so nothing about it changes.
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Kismet"));
    let bears = game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    let greed = game.spawn_in_hand(PlayerId(1), card("Greed"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });
    for spell in [bears, greed] {
        game.fund_mana(PlayerId(1));
        game.submit(Intent::Cast {
            player: PlayerId(1),
            object: spell,
            target: None,
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
        .expect("a main-phase cast with the mana funded");
        advance_until(&mut game, |g| g.stack_is_empty());
    }

    assert!(
        game.is_tapped(game.current_id(bears)),
        "a creature is on Kismet's list"
    );
    assert!(
        !game.is_tapped(game.current_id(greed)),
        "an enchantment is not"
    );
}
