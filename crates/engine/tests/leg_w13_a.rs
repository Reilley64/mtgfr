//! Legends (`leg`) grind, wave 13 slice A — increment 24, Chains of Mephistopheles.
//!
//! "If a player would draw a card except the first one they draw in each of their draw steps,
//! that player discards a card instead. If the player discards a card this way, they draw a card.
//! If the player doesn't discard a card this way, they mill a card."
//!
//! A CR 614 replacement on *every* player's draws, so these tests hold the draw funnel
//! (`Game::draw_with_replacements`) to catching all of them: the turn-based draw step draw, a
//! spell's draws, an opponent's draws, and a table-wide wheel.

mod common;

use common::*;
use engine::*;

fn stock_libraries(game: &mut Game) -> Vec<Vec<ObjectId>> {
    (0..game.player_count() as u8)
        .map(|p| game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 40]))
        .collect()
}

/// `player` casts a spell, funding the mana first.
fn cast_by(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));
}

/// Answer every Chains discard pause the resolution raises, discarding the first card offered,
/// and return how many there were. Fails the test if a non-discard pause shows up instead.
fn answer_chains_discards(game: &mut Game) -> usize {
    let mut discards = 0;
    while let Some(choice) = game.pending_choice() {
        let PendingChoice::DiscardCards {
            player,
            hand,
            count,
            ..
        } = choice
        else {
            panic!("Chains replaces a draw with a discard, got {choice:?}");
        };
        assert_eq!(count, 1, "one draw is replaced by one discard");
        game.submit(Intent::Discard {
            player,
            cards: vec![hand[0]],
        })
        .expect("discarding an offered card is legal");
        discards += 1;
        assert!(
            discards < 20,
            "the replacement should not replace its own draw"
        );
    }
    discards
}

// ── Increment #24: Chains of Mephistopheles ────────────────────────────────────────────────────

#[test]
fn the_first_draw_of_a_players_own_draw_step_is_not_replaced() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let fodder = game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    let library_before = game.library_size(PlayerId(1));

    advance_until(&mut game, |g| g.current_step() == Step::Draw);
    assert_eq!(game.active_player(), PlayerId(1), "player 1's draw step");

    assert!(
        game.pending_choice().is_none(),
        "the exempt draw raises no discard, got {:?}",
        game.pending_choice()
    );
    assert_eq!(
        game.hand(PlayerId(1)).len(),
        2,
        "the turn-based draw landed in hand alongside the fodder"
    );
    assert_eq!(game.library_size(PlayerId(1)), library_before - 1);
    assert_eq!(game.zone_of(fodder), Zone::Hand, "nothing was discarded");
}

#[test]
fn a_draw_outside_the_draw_step_becomes_a_discard_then_a_draw() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));
    let fodder: Vec<ObjectId> = (0..3)
        .map(|_| game.spawn_in_hand(PlayerId(0), card("Grizzly Bears")))
        .collect();
    let library_before = game.library_size(PlayerId(0));

    cast_by(
        &mut game,
        PlayerId(0),
        recall,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);

    // "Target player draws three cards" is three draw events (CR 121.2), each replaced on its own.
    // Three, not four or more: the card Chains itself has them draw is not replaced again (CR 614.5).
    assert_eq!(answer_chains_discards(&mut game), 3);

    assert_eq!(
        game.hand(PlayerId(0)).len(),
        3,
        "three cards discarded, three drawn back — hand size unchanged"
    );
    assert_eq!(
        game.library_size(PlayerId(0)),
        library_before - 3,
        "each replacement still draws one card off the library"
    );
    for id in fodder {
        assert_eq!(game.zone_of(id), Zone::Graveyard, "discarded to Chains");
    }
}

#[test]
fn a_player_with_no_cards_to_discard_mills_instead() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));
    let library_before = game.library_size(PlayerId(1));

    // The opponent's draws are replaced too — Chains reads "if a player would draw", not "you".
    cast_by(
        &mut game,
        PlayerId(0),
        recall,
        Some(Target::Player(PlayerId(1))),
    );
    resolve_top_of_stack(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "an empty hand offers no discard to choose, got {:?}",
        game.pending_choice()
    );
    assert!(
        game.hand(PlayerId(1)).is_empty(),
        "every draw was replaced, so nothing reached hand"
    );
    assert_eq!(
        game.library_size(PlayerId(1)),
        library_before - 3,
        "three mills, one per replaced draw"
    );
}

#[test]
fn only_the_first_draw_of_a_draw_step_is_exempt() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));

    advance_until(&mut game, |g| {
        g.current_step() == Step::Draw && g.active_player() == PlayerId(0)
    });
    let hand_before = game.hand(PlayerId(0)).len();
    let library_before = game.library_size(PlayerId(0));

    // Still in player 0's own draw step, but the exempt draw is already spent.
    cast_by(
        &mut game,
        PlayerId(0),
        recall,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(answer_chains_discards(&mut game), 3);
    assert_eq!(
        game.hand(PlayerId(0)).len(),
        hand_before - 1,
        "only Ancestral Recall left the hand; the three draws replaced themselves out"
    );
    assert_eq!(game.library_size(PlayerId(0)), library_before - 3);
}

#[test]
fn a_draw_taken_during_someone_elses_draw_step_is_replaced() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let recall = game.spawn_in_hand(PlayerId(0), card("Ancestral Recall"));

    advance_until(&mut game, |g| {
        g.current_step() == Step::Draw && g.active_player() == PlayerId(0)
    });
    let library_before = game.library_size(PlayerId(1));
    let hand_before = game.hand(PlayerId(1)).len();

    // Player 1 has drawn nothing during *this* draw step — but it is player 0's draw step, not
    // theirs, so none of their draws is the exempt one.
    cast_by(
        &mut game,
        PlayerId(0),
        recall,
        Some(Target::Player(PlayerId(1))),
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(answer_chains_discards(&mut game), 3, "all three replaced");
    assert_eq!(game.hand(PlayerId(1)).len(), hand_before);
    assert_eq!(game.library_size(PlayerId(1)), library_before - 3);
}

#[test]
fn timetwister_under_chains_mills_every_seat_instead_of_refilling_it() {
    let mut game = Game::with_players(2, 0);
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let timetwister = game.spawn_in_hand(PlayerId(0), card("Timetwister"));
    let libraries: Vec<usize> = (0..2).map(|p| game.library_size(PlayerId(p))).collect();

    cast_by(&mut game, PlayerId(0), timetwister, None);
    resolve_top_of_stack(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "every seat shuffled its hand away first, so no seat has a card to discard"
    );
    for p in 0..2u8 {
        let player = PlayerId(p);
        assert!(
            game.hand(player).is_empty(),
            "all seven draws were replaced for player {p}"
        );
        assert_eq!(
            game.library_size(player),
            libraries[p as usize] - 7,
            "seven mills for player {p}"
        );
    }
}

/// Three seats, each holding cards to discard, under someone's Chains. Vision Skeins is the
/// multi-seat batch: every one of the six draws pauses, so the batch has to resume *into the next
/// seat* five times rather than only within one seat's run.
fn three_seats_under_chains(fodder_each: usize) -> (Game, Vec<usize>) {
    let mut game = Game::with_players(3, 0);
    stock_libraries(&mut game);
    // Chains sits with player 1, not the active player: it replaces every seat's draws whoever
    // controls it, and keeping it off player 0 means the concede test can drop the *active* seat
    // (the one APNAP pauses first) without CR 800.4a taking the replacement off the board too.
    game.spawn_on_battlefield(PlayerId(1), card("Chains of Mephistopheles"));
    for p in 0..3u8 {
        for _ in 0..fodder_each {
            game.spawn_in_hand(PlayerId(p), card("Grizzly Bears"));
        }
    }
    let libraries = (0..3).map(|p| game.library_size(PlayerId(p))).collect();
    (game, libraries)
}

#[test]
fn a_multi_seat_draw_batch_resumes_into_the_next_seat_after_each_pause() {
    let (mut game, libraries) = three_seats_under_chains(2);
    let skeins = game.spawn_in_hand(PlayerId(0), card("Vision Skeins"));
    let hands: Vec<usize> = (0..3).map(|p| game.hand(PlayerId(p)).len()).collect();

    cast_by(&mut game, PlayerId(0), skeins, None);
    resolve_top_of_stack(&mut game);

    // "Each player draws two cards" — six draws, every one of them replaced, and the batch parks
    // and re-enters between each.
    assert_eq!(
        answer_chains_discards(&mut game),
        6,
        "two per seat, replaced"
    );
    for p in 0..3u8 {
        let player = PlayerId(p);
        assert_eq!(
            game.library_size(player),
            libraries[p as usize] - 2,
            "player {p} took both of their granted draws"
        );
        // Discard one, draw one (CR 614.5 exempts the draw the replacement grants), twice over.
        let expected = hands[p as usize] - usize::from(p == 0);
        assert_eq!(
            game.hand(player).len(),
            expected,
            "player {p}'s hand is level after two discard-then-draws"
        );
    }
}

#[test]
fn conceding_on_a_chains_discard_still_pays_the_rest_of_the_batch() {
    let (mut game, libraries) = three_seats_under_chains(2);
    let skeins = game.spawn_in_hand(PlayerId(0), card("Vision Skeins"));

    cast_by(&mut game, PlayerId(0), skeins, None);
    resolve_top_of_stack(&mut game);

    // The batch is parked on player 0's first discard. They quit rather than answer it.
    let Some(PendingChoice::DiscardCards { player, .. }) = game.pending_choice() else {
        panic!("the first draw of the batch pauses for a discard");
    };
    assert_eq!(player, PlayerId(0), "APNAP puts the active player first");
    game.submit(Intent::Concede {
        player: PlayerId(0),
    })
    .expect("a player may always quit");

    // The quitter forfeits their own draws, but the two seats behind them in the batch are still
    // owed theirs — the batch must not strand on the choice that left with them.
    assert_eq!(answer_chains_discards(&mut game), 4, "two each for 1 and 2");
    for p in 1..3u8 {
        assert_eq!(
            game.library_size(PlayerId(p)),
            libraries[p as usize] - 2,
            "player {p} was paid in full"
        );
    }
}

#[test]
fn psychic_purge_discarded_to_chains_charges_the_chains_controller() {
    let mut game = Game::with_players(3, 0);
    stock_libraries(&mut game);
    // Player 0 controls Chains; player 1 casts the spell that asks player 2 to draw. Those are
    // different seats on purpose: the discard is caused by Chains' static ability, not by the
    // spell whose draw it replaced, so the 5 life is player 0's to lose.
    game.spawn_on_battlefield(PlayerId(0), card("Chains of Mephistopheles"));
    let recall = game.spawn_in_hand(PlayerId(1), card("Ancestral Recall"));
    let purge = game.spawn_in_hand(PlayerId(2), card("Psychic Purge"));
    let life_before: Vec<i32> = (0..3).map(|p| game.life(PlayerId(p))).collect();

    // Ancestral Recall is an instant, so player 1 only needs priority to hold it up.
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .expect("the active player may pass");
    cast_by(
        &mut game,
        PlayerId(1),
        recall,
        Some(Target::Player(PlayerId(2))),
    );
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::DiscardCards { player, .. }) = game.pending_choice() else {
        panic!("player 2's draw is replaced by a discard");
    };
    assert_eq!(player, PlayerId(2));
    game.submit(Intent::Discard {
        player: PlayerId(2),
        cards: vec![purge],
    })
    .expect("Psychic Purge is in hand and may be the discard");
    // "When a spell or ability an opponent controls causes you to discard this card, that player
    // loses 5 life." Drain the rest of the batch, then let the trigger resolve.
    answer_chains_discards(&mut game);
    while !game.stack_is_empty() {
        resolve_top_of_stack(&mut game);
    }

    assert_eq!(
        game.life(PlayerId(0)),
        life_before[0] - 5,
        "the Chains controller caused the discard"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        life_before[1],
        "the spell that asked for the draw did not cause the discard"
    );
}
