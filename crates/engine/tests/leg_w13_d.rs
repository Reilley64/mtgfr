//! Legends (`leg`) grind, wave 13 slice D — amounts and the resolution frame.
//!
//! Increment 176 (Backdraft's "one of those sorcery spells" chooser).

mod common;

use common::*;
use engine::*;

/// Player 0 casts a spell, funding the mana first. Returns the *spell* object now on the stack —
/// a new id, which is what the damage ledger records and what the pick is made from.
fn cast(game: &mut Game, object: ObjectId, target: Option<Target>, kicked: bool) -> ObjectId {
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
        kicked,
        bought_back: false,
        evoked: false,
        strive_count: 0,
        replicate_count: 0,
        multikicker_count: 0,
        alternative_cost: false,
    })
    .unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));

    match game.stack().last() {
        Some(StackEntry::Spell(id)) => *id,
        other => panic!("the cast spell should be on the stack, got {other:?}"),
    }
}

/// Player 0 casts Syphon Soul (2 damage to each other player — 2 in a two-seat game) and a kicked
/// Breath of Darigaaz (4 damage to each player — 8 across two seats), leaving two of their own
/// damaging sorceries in this turn's ledger. Returns their spell object ids —
/// `(syphon_soul, breath)`.
fn cast_two_damaging_sorceries(game: &mut Game) -> (ObjectId, ObjectId) {
    let card_in_hand = game.spawn_in_hand(PlayerId(0), card("Syphon Soul"));
    let soul = cast(game, card_in_hand, None, false);
    resolve_top_of_stack(game);

    let card_in_hand = game.spawn_in_hand(PlayerId(0), card("Breath of Darigaaz"));
    let breath = cast(game, card_in_hand, None, true);
    resolve_top_of_stack(game);

    (soul, breath)
}

// ── Increment #176: Backdraft's "one of those sorcery spells" ──────────────────────────────────
// "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that
// player equal to half the damage dealt by one of those sorcery spells this turn, rounded down."

#[test]
fn backdrafts_controller_picks_which_of_the_chosen_players_sorceries_is_halved() {
    let mut game = Game::new();
    let (soul, breath) = cast_two_damaging_sorceries(&mut game);
    let card_in_hand = game.spawn_in_hand(PlayerId(0), card("Backdraft"));

    // Aimed at themselves, where the *smaller* sorcery is the pick they want.
    let backdraft = cast(
        &mut game,
        card_in_hand,
        Some(Target::Player(PlayerId(0))),
        false,
    );
    let life_before = game.life(PlayerId(0));
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::ChooseDamageSource {
        player,
        candidates,
        source,
    }) = game.pending_choice()
    else {
        panic!(
            "Backdraft pauses to pick one of those sorceries, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(player, PlayerId(0), "the spell's controller picks");
    assert_eq!(source, backdraft, "the prompt names the Backdraft spell");
    assert_eq!(
        candidates,
        vec![soul, breath],
        "both of the chosen player's damaging sorceries are offered"
    );

    game.submit(Intent::ChooseCopyTarget {
        player: PlayerId(0),
        copy: Some(soul),
    })
    .expect("picking one of the offered sorceries is legal");

    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 1,
        "half the 2 damage Syphon Soul dealt, not half the 8 Breath of Darigaaz dealt"
    );
}

#[test]
fn backdraft_can_still_pick_the_biggest_hitting_sorcery() {
    let mut game = Game::new();
    let (_, breath) = cast_two_damaging_sorceries(&mut game);
    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));

    cast(
        &mut game,
        backdraft,
        Some(Target::Player(PlayerId(0))),
        false,
    );
    let life_before = game.life(PlayerId(0));
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseCopyTarget {
        player: PlayerId(0),
        copy: Some(breath),
    })
    .expect("picking one of the offered sorceries is legal");

    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 4,
        "half the 8 damage Breath of Darigaaz dealt"
    );
}

#[test]
fn backdraft_rejects_a_sorcery_that_is_not_on_offer() {
    let mut game = Game::new();
    cast_two_damaging_sorceries(&mut game);
    let card_in_hand = game.spawn_in_hand(PlayerId(0), card("Backdraft"));

    let backdraft = cast(
        &mut game,
        card_in_hand,
        Some(Target::Player(PlayerId(0))),
        false,
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.submit(Intent::ChooseCopyTarget {
            player: PlayerId(0),
            copy: Some(backdraft),
        }),
        Err(Reject::IllegalChoice),
        "Backdraft is not one of the chosen player's damaging sorceries"
    );
    assert_eq!(
        game.submit(Intent::ChooseCopyTarget {
            player: PlayerId(0),
            copy: None,
        }),
        Err(Reject::IllegalChoice),
        "the pick is mandatory — there is no decline"
    );
}

#[test]
fn backdraft_does_not_pause_when_only_one_sorcery_dealt_damage() {
    let mut game = Game::new();
    let soul = game.spawn_in_hand(PlayerId(0), card("Syphon Soul"));
    cast(&mut game, soul, None, false);
    resolve_top_of_stack(&mut game);

    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));
    cast(
        &mut game,
        backdraft,
        Some(Target::Player(PlayerId(0))),
        false,
    );
    let life_before = game.life(PlayerId(0));
    resolve_top_of_stack(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "one candidate is no choice at all — the pick is settled without asking"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        life_before - 1,
        "half the 2 damage Syphon Soul dealt"
    );
}

#[test]
fn backdraft_deals_nothing_when_the_chosen_players_sorcery_dealt_no_damage() {
    let mut game = Game::new();
    // Armageddon is a sorcery that deals no damage, so it makes player 0 choosable but leaves no
    // ledger row to halve.
    let armageddon = game.spawn_in_hand(PlayerId(0), card("Armageddon"));
    cast(&mut game, armageddon, None, false);
    resolve_top_of_stack(&mut game);

    let backdraft = game.spawn_in_hand(PlayerId(0), card("Backdraft"));
    cast(
        &mut game,
        backdraft,
        Some(Target::Player(PlayerId(0))),
        false,
    );
    let life_before = game.life(PlayerId(0));
    resolve_top_of_stack(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "no damaging sorcery means nothing to pick"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        life_before,
        "half of nothing is nothing"
    );
}
