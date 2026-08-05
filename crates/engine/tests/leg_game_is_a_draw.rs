//! Legends (`leg`) grind — increment 27: game-is-a-draw.
//!
//! Divine Intervention's intervention counters count down its controller's upkeeps, and the removal
//! that empties it makes the game a draw (CR 104.4) — its own outcome, not every player losing.

mod common;

use common::*;
use engine::*;

const DI: CounterKind = CounterKind::Intervention;

/// Give every seat a library, so rolling the game through whole turns doesn't eliminate the other
/// players on an empty-library draw.
fn stock_libraries(game: &mut Game) {
    let deck = vec![card("Plains"); 60];
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &deck);
    }
}

/// A 4-player game with Divine Intervention resolved onto the battlefield under player 0.
fn divine_intervention_in_play() -> (Game, ObjectId) {
    let mut game = Game::with_players(4, 0);
    stock_libraries(&mut game);
    let card_id = game.spawn_in_hand(PlayerId(0), card("Divine Intervention"));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: card_id,
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
    .unwrap();
    let mut entered = None;
    for _ in 0..game.player_count() {
        let events = game
            .submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
        entered = entered.or_else(|| {
            events.iter().find_map(|e| match e {
                Event::PermanentEntered { permanent, .. } => Some(*permanent),
                _ => None,
            })
        });
    }
    let enchantment = entered.expect("Divine Intervention resolves onto the battlefield");
    (game, enchantment)
}

#[test]
fn divine_intervention_enters_with_two_intervention_counters() {
    let (game, enchantment) = divine_intervention_in_play();

    assert_eq!(
        game.counters_of_kind(enchantment, DI),
        2,
        "\"enters with two intervention counters on it\""
    );
    assert_eq!(game.outcome(), None, "nothing has ended the game yet");
}

#[test]
fn your_upkeep_removes_one_intervention_counter_without_drawing_the_game() {
    let (mut game, enchantment) = divine_intervention_in_play();

    // "At the beginning of your upkeep, remove an intervention counter from this enchantment."
    advance_until(&mut game, |g| g.counters_of_kind(enchantment, DI) == 1);

    assert_eq!(
        game.outcome(),
        None,
        "one counter is left — the last one has not come off"
    );
}

#[test]
fn removing_the_last_intervention_counter_makes_the_game_a_draw() {
    let (mut game, enchantment) = divine_intervention_in_play();

    advance_until(&mut game, |g| g.counters_of_kind(enchantment, DI) == 1);
    // The removal that empties the enchantment queues the draw trigger; it is on the stack now, so
    // the game is not over until it resolves (CR 603.2 — an ordinary triggered ability).
    advance_until(&mut game, |g| g.counters_of_kind(enchantment, DI) == 0);
    assert_eq!(
        game.outcome(),
        None,
        "the draw trigger is still on the stack"
    );

    resolve_top_of_stack(&mut game);

    // CR 104.4: a draw is its own outcome, not a loss for everyone.
    assert_eq!(game.outcome(), Some(GameOutcome::Draw));
    assert_eq!(game.winner(), None, "nobody won a drawn game");
    for p in 0..4u8 {
        assert!(
            !game.has_lost(PlayerId(p)),
            "seat {p} did not lose — the game was a draw"
        );
    }

    // CR 104.4b: the game ended for every player still in it, so nothing further is legal.
    assert_eq!(
        game.submit(Intent::PassPriority {
            player: game.priority_holder(),
        }),
        Err(Reject::WrongTiming),
        "the game is over — no intent is accepted"
    );
    assert_eq!(
        game.submit(Intent::Concede {
            player: PlayerId(1)
        }),
        Err(Reject::WrongTiming),
        "there is no game left to concede"
    );
}
