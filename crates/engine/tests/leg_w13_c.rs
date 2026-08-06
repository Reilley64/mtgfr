//! Legends (`leg`) grind, wave 13 slice C — the trigger system.
//!
//! Increments 116 (Spiritual Sanctuary's condition scoped to the triggering player),
//! 101 (Ichneumon Druid's filtered per-turn cast tally) and 64 (Psychic Purge's
//! discarded-from-hand trigger).

mod common;

use common::*;
use engine::*;

/// Cast one of `player`'s hand cards with the mana handed to them, then let it resolve.
fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .expect("the spell is castable");
    resolve_top_of_stack(game);
}

// Spiritual Sanctuary: "At the beginning of each player's upkeep, if that player controls a
// Plains, they gain 1 life." CR 603.4's intervening-if read from the *triggering* player's
// seat, not the enchantment controller's — the Sanctuary's own Plains never pays an opponent.

#[test]
fn spiritual_sanctuary_reads_the_plains_of_whoever_the_upkeep_belongs_to() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Spiritual Sanctuary"));
    game.spawn_on_battlefield(PlayerId(0), card("Plains"));
    game.spawn_on_battlefield(PlayerId(1), card("Swamp"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }

    // Turn one's upkeep is behind us, so the first fire we see is the opponent's — and they
    // have no Plains, so the condition fails from their seat even though the Sanctuary's
    // controller is standing on one.
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "no Plains of their own, so no life on their upkeep"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "and the payoff never reaches the Sanctuary's controller on someone else's upkeep"
    );

    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        game.life(PlayerId(0)),
        21,
        "their upkeep, their Plains, their life"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "once per upkeep, not per player"
    );
}

#[test]
fn spiritual_sanctuary_pays_an_opponent_who_has_the_plains() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Spiritual Sanctuary"));
    game.spawn_on_battlefield(PlayerId(1), card("Plains"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }

    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        game.life(PlayerId(1)),
        21,
        "the Sanctuary is symmetrical — an opponent's Plains pays the opponent"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "while its controller, with no Plains, gets nothing on their own upkeep either"
    );
}

// Ichneumon Druid: "Whenever an opponent casts an instant spell other than the first instant
// spell that player casts each turn, this creature deals 4 damage to that player." The exemption
// is per-turn, per-player, and counts *instants* — a sorcery cast first must not spend it.

#[test]
fn ichneumon_druid_spares_the_turns_first_instant_and_burns_every_one_after() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Ichneumon Druid"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }

    // The opponent's own turn, so they hold priority in their main phase.
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);

    let first = game.spawn_in_hand(PlayerId(1), card("Shock"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        first,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "the first instant that player casts this turn is the exempt one"
    );

    let second = game.spawn_in_hand(PlayerId(1), card("Shock"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        second,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        16,
        "the second instant is billed 4 — \"that player\", not the Druid's controller"
    );

    let third = game.spawn_in_hand(PlayerId(1), card("Shock"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        third,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        12,
        "\"other than the first\" is open-ended — every instant after it fires"
    );
}

#[test]
fn ichneumon_druid_counts_instants_only_and_starts_over_each_turn() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Ichneumon Druid"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }

    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);

    // A sorcery first: it is a spell, but not an *instant* spell, so it must not spend the
    // exemption the instant behind it is owed.
    let sorcery = game.spawn_in_hand(PlayerId(1), card("Psychic Purge"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        sorcery,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    let instant = game.spawn_in_hand(PlayerId(1), card("Shock"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        instant,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "the sorcery is not an instant — this is still the turn's first instant spell"
    );

    // Around the table and back: the tally is turn-scoped, so the exemption is fresh.
    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let next_turn = game.spawn_in_hand(PlayerId(1), card("Shock"));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        next_turn,
        Some(Target::Player(PlayerId(0))),
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "a new turn earns a new exempt first instant"
    );
}

// Psychic Purge: "When a spell or ability an opponent controls causes you to discard this card,
// that player loses 5 life." A triggered ability that functions from hand (CR 603.6d), gated on
// the *cause* being an opponent's — your own discard outlet never fires it.

/// Player 0's Disrupting Scepter ("{3}, {T}: Target player discards a card") makes `victim`
/// discard, on player 0's own end step (its activation is turn-restricted).
fn scepter_discard(game: &mut Game, scepter: ObjectId, controller: PlayerId, victim: PlayerId) {
    advance_until(game, |g| g.current_step() == Step::End);
    game.fund_mana(controller);
    game.submit(Intent::ActivateAbility {
        player: controller,
        object: scepter,
        ability_index: 0,
        target: Some(Target::Player(victim)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("your own end step is still your turn");
    resolve_top_of_stack(game);
}

#[test]
fn psychic_purge_bills_the_opponent_whose_ability_discarded_it() {
    let mut game = Game::new();
    let scepter = game.spawn_on_battlefield(PlayerId(0), card("Disrupting Scepter"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }
    let purge = game.spawn_in_hand(PlayerId(1), card("Psychic Purge"));

    scepter_discard(&mut game, scepter, PlayerId(0), PlayerId(1));
    game.submit(Intent::Discard {
        player: PlayerId(1),
        cards: vec![purge],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(purge),
        Zone::Graveyard,
        "the Purge is the card that was discarded"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        15,
        "\"that player\" is whoever controlled the ability that caused the discard"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "the discarding player pays nothing"
    );
}

#[test]
fn psychic_purge_stays_silent_when_you_discard_it_to_your_own_ability() {
    let mut game = Game::new();
    let scepter = game.spawn_on_battlefield(PlayerId(0), card("Disrupting Scepter"));
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 5]);
    }
    let purge = game.spawn_in_hand(PlayerId(0), card("Psychic Purge"));

    scepter_discard(&mut game, scepter, PlayerId(0), PlayerId(0));
    game.submit(Intent::Discard {
        player: PlayerId(0),
        cards: vec![purge],
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(purge), Zone::Graveyard, "still discarded");
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "\"a spell or ability an *opponent* controls\" — your own outlet is not that"
    );
}
