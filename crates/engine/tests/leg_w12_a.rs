//! Legends (`leg`) grind, wave 12 slice A — the draw path.
//!
//! Increments 150 (Visions' look-only pause), 135 (Backdraft's two chooser clauses),
//! 77 (Sylvan Library) and 24 (Chains of Mephistopheles).

mod common;

use common::*;
use engine::*;

fn stock_libraries(game: &mut Game) -> Vec<Vec<ObjectId>> {
    (0..game.player_count() as u8)
        .map(|p| game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 30]))
        .collect()
}

/// Player 0 tries to cast a spell, funding the mana first.
fn try_cast(
    game: &mut Game,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
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
}

/// Player 0 casts a spell, funding the mana first.
fn cast(game: &mut Game, object: ObjectId, target: Option<Target>) {
    try_cast(game, object, target).unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));
}

// ── Increment #150: Visions' look-only pause ───────────────────────────────────────────────────
// "Look at the top five cards of target player's library. You may then have that player shuffle
// that library." The look shows five cards and puts them back untouched — no reordering.

#[test]
fn visions_look_puts_the_top_five_back_in_the_order_they_were_dealt() {
    let mut game = Game::new();
    let libraries = stock_libraries(&mut game);
    let visions = game.spawn_in_hand(PlayerId(0), card("Visions"));
    let before = libraries[1].clone();

    cast(&mut game, visions, Some(Target::Player(PlayerId(1))));
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::ArrangeTop {
        player,
        library,
        cards,
        rest,
    }) = game.pending_choice()
    else {
        panic!("Visions pauses on a look, got {:?}", game.pending_choice());
    };
    assert_eq!(player, PlayerId(0), "the caster does the looking");
    assert_eq!(library, PlayerId(1), "at the targeted player's library");
    assert_eq!(cards.len(), 5, "the top five");
    assert_eq!(rest, ArrangeRest::LookOnly, "a look, not a rearrange");
    assert_eq!(cards, before[..5], "the top five, in library order");

    // The answer is a dismissal: whatever order it names is ignored, and the library is untouched.
    let reversed: Vec<ObjectId> = cards.iter().rev().copied().collect();
    game.submit(Intent::ArrangeTop {
        player: PlayerId(0),
        top: reversed,
        bottom: vec![],
    })
    .expect("dismissing the look is always legal");
    assert_eq!(
        game.library_top(PlayerId(1)),
        Some(before[0]),
        "a look-only pause puts every card back exactly where it was"
    );
    assert_eq!(game.library_size(PlayerId(1)), before.len());
}

#[test]
fn visions_still_offers_the_shuffle_after_the_look() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let visions = game.spawn_in_hand(PlayerId(0), card("Visions"));

    cast(&mut game, visions, Some(Target::Player(PlayerId(1))));
    resolve_top_of_stack(&mut game);
    let Some(PendingChoice::ArrangeTop { cards, .. }) = game.pending_choice() else {
        panic!("Visions pauses on a look");
    };
    game.submit(Intent::ArrangeTop {
        player: PlayerId(0),
        top: cards,
        bottom: vec![],
    })
    .expect("dismiss the look");

    assert!(
        matches!(game.pending_choice(), Some(PendingChoice::MayYesNo { .. })),
        "the shuffle clause follows the look, got {:?}",
        game.pending_choice()
    );
}

// ── Increment #77: Sylvan Library ─────────────────────────────────────────────────────────────
// "At the beginning of your draw step, you may draw two additional cards. If you do, choose two
// cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top
// of your library."

/// Player 0 with a Sylvan Library out, run forward to its draw-step trigger and taking the offer.
/// Leaves the game paused on the "choose two cards drawn this turn" prompt.
fn sylvan_library_offer(game: &mut Game) {
    game.spawn_on_battlefield(PlayerId(0), card("Sylvan Library"));
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::MayYesNo { .. }))
    });
    game.submit(Intent::AnswerMay {
        player: PlayerId(0),
        yes: true,
    })
    .expect("taking the two additional cards is legal");
    // A "may" trigger answered yes goes on the stack; the draw and the choice happen on resolution.
    resolve_top_of_stack(game);
}

#[test]
fn sylvan_library_offers_only_the_cards_drawn_this_turn() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    // A card that was in hand before the draw step is not "drawn this turn" and is not offered.
    let held = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));

    sylvan_library_offer(&mut game);

    let Some(PendingChoice::PutFromHandOnTop {
        player,
        hand,
        count,
        life_per_declined,
    }) = game.pending_choice()
    else {
        panic!(
            "Sylvan Library pauses on the two-card choice, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(player, PlayerId(0));
    assert_eq!(count, 2, "choose two of them");
    assert_eq!(life_per_declined, 4, "pay 4 life for each one you keep");
    assert!(
        !hand.contains(&held),
        "a card already in hand was not drawn this turn"
    );
    assert_eq!(
        hand.len(),
        3,
        "the draw step's own card plus the two additional ones"
    );
}

#[test]
fn sylvan_library_charges_four_life_for_each_card_kept() {
    let mut game = Game::new();
    stock_libraries(&mut game);

    sylvan_library_offer(&mut game);
    let life_before = game.life(PlayerId(0));
    let hand_before = game.hand(PlayerId(0)).len();
    let library_before = game.library_size(PlayerId(0));

    // Keep both: pay 4 life twice.
    game.submit(Intent::PutFromHandOnTop {
        player: PlayerId(0),
        cards: vec![],
    })
    .expect("putting none back and paying for both is a legal answer");

    assert_eq!(game.life(PlayerId(0)), life_before - 8);
    assert_eq!(
        game.hand(PlayerId(0)).len(),
        hand_before,
        "nothing went back"
    );
    assert_eq!(game.library_size(PlayerId(0)), library_before);
}

#[test]
fn sylvan_library_puts_the_first_named_card_on_top_and_charges_for_the_other() {
    let mut game = Game::new();
    stock_libraries(&mut game);

    sylvan_library_offer(&mut game);
    let Some(PendingChoice::PutFromHandOnTop { hand, .. }) = game.pending_choice() else {
        panic!("Sylvan Library pauses on the two-card choice");
    };
    let life_before = game.life(PlayerId(0));
    let library_before = game.library_size(PlayerId(0));

    // One back, one paid for.
    game.submit(Intent::PutFromHandOnTop {
        player: PlayerId(0),
        cards: vec![hand[0]],
    })
    .expect("one back and one paid for is a legal answer");

    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 4,
        "4 for the one kept"
    );
    assert_eq!(game.library_size(PlayerId(0)), library_before + 1);
    assert!(
        !game.hand(PlayerId(0)).contains(&hand[0]),
        "the named card left the hand"
    );
}

// ── Increment #135: Backdraft's "a player who cast one or more sorcery spells this turn" ───────
// "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that
// player equal to half the damage dealt by one of those sorcery spells this turn, rounded down."

#[test]
fn backdraft_cannot_be_aimed_at_a_player_who_cast_no_sorcery_this_turn() {
    let mut game = Game::new();
    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));

    // Nobody has cast a sorcery this turn, so there is no one to choose.
    assert_eq!(
        try_cast(&mut game, backdraft, Some(Target::Player(PlayerId(1)))),
        Err(Reject::IllegalTarget),
        "an opponent who cast no sorcery cannot be chosen"
    );
    assert_eq!(
        try_cast(&mut game, backdraft, Some(Target::Player(PlayerId(0)))),
        Err(Reject::IllegalTarget),
        "nor can the caster themselves"
    );
}

#[test]
fn backdraft_reaches_the_one_player_who_cast_a_sorcery_this_turn() {
    let mut game = Game::new();
    let breath = game.spawn_in_hand(PlayerId(0), card("Breath of Darigaaz"));
    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));

    // Kicked: 4 damage to each player — one sorcery of player 0's dealing 8 across two rows.
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: breath,
        target: None,
        x: 0,
        modes: vec![],
        discard_cost: vec![],
        graveyard_exile: vec![],
        sacrifice_cost: vec![],
        kicked: true,
        bought_back: false,
        evoked: false,
        strive_count: 0,
        replicate_count: 0,
        multikicker_count: 0,
        alternative_cost: false,
    })
    .expect("a kicked Breath of Darigaaz is castable");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        try_cast(&mut game, backdraft, Some(Target::Player(PlayerId(1)))),
        Err(Reject::IllegalTarget),
        "player 1 cast no sorcery, so they still cannot be chosen"
    );
    let life_before = game.life(PlayerId(0));
    cast(&mut game, backdraft, Some(Target::Player(PlayerId(0))));
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 4,
        "half the 8 damage Breath of Darigaaz dealt this turn, rounded down"
    );
}
