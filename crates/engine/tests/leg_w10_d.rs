//! Legends (`leg`) grind, wave 10 slice D — the library/hand/graveyard spells.
//!
//! Increments 46 (Hellfire), 75 (Storm World), 85 (Visions), 92 (Winter Blast), 120 (Spurnmage
//! Advocate) and 69 (Recall).

mod common;

use common::*;
use engine::*;

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 30]);
    }
}

/// Player 0 casts a spell, funding the mana first.
fn cast(game: &mut Game, object: ObjectId, target: Option<Target>, x: u32) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object,
        target,
        x,
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

// ── Increment #46: Hellfire ────────────────────────────────────────────────────────────────────
// "Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of
// creatures that died this way."

#[test]
fn hellfire_spares_black_creatures_and_bills_its_controller_for_the_rest() {
    let mut game = Game::new();
    // Two nonblack creatures die; the black one survives, so X is 2 and Hellfire deals 5.
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bears2 = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let horror = game.spawn_on_battlefield(PlayerId(1), card("Cosmic Horror"));
    let hellfire = game.spawn_in_hand(PlayerId(0), card("Hellfire"));
    let before = game.life(PlayerId(0));

    cast(&mut game, hellfire, None, 0);
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(bears), Zone::Graveyard);
    assert_eq!(game.zone_of(bears2), Zone::Graveyard);
    assert_eq!(
        game.zone_of(horror),
        Zone::Battlefield,
        "a black creature is not destroyed"
    );
    assert_eq!(game.life(PlayerId(0)), before - 5, "X (2) plus 3");
}

#[test]
fn hellfire_with_no_creatures_still_deals_three() {
    // X is 0, and "X plus 3" is still 3 — the floor is the +3, not nothing.
    let mut game = Game::new();
    let hellfire = game.spawn_in_hand(PlayerId(0), card("Hellfire"));
    let before = game.life(PlayerId(0));

    cast(&mut game, hellfire, None, 0);
    resolve_top_of_stack(&mut game);

    assert_eq!(game.life(PlayerId(0)), before - 3);
}

// ── Increment #75: Storm World ─────────────────────────────────────────────────────────────────
// "At the beginning of each player's upkeep, this enchantment deals X damage to that player, where
// X is 4 minus the number of cards in their hand."

#[test]
fn storm_world_bills_an_empty_handed_player_four() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Storm World"));
    let before = game.life(PlayerId(1));

    // Player 1's upkeep, with an empty hand: 4 − 0 = 4.
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Upkeep
    });
    advance_until(&mut game, |g| g.current_step() == Step::Draw);

    // The draw step's card arrives after the upkeep trigger has resolved.
    assert_eq!(game.life(PlayerId(1)), before - 4);
}

#[test]
fn storm_world_deals_nothing_to_a_player_holding_four_or_more() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Storm World"));
    for _ in 0..5 {
        game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    }
    let before = game.life(PlayerId(1));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Upkeep
    });
    advance_until(&mut game, |g| g.current_step() == Step::Draw);

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "4 minus 5 is not −1 damage; the difference floors at zero and 0 damage is never dealt"
    );
}

#[test]
fn storm_world_is_a_world_enchantment() {
    assert!(card("Storm World").world);
}

// ── Increment #92: Winter Blast ────────────────────────────────────────────────────────────────
// "Tap X target creatures. Winter Blast deals 2 damage to each of those creatures with flying."

/// Player 0 casts `object` for `x` and answers the multi-target clause with `targets`.
fn cast_x_targeting(game: &mut Game, object: ObjectId, x: u32, targets: Vec<ObjectId>) {
    cast(game, object, None, x);
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: targets.into_iter().map(Target::Object).collect(),
    })
    .expect("a distinct, legal target set of exactly X");
}

#[test]
fn winter_blast_taps_every_target_but_only_burns_the_fliers() {
    let mut game = Game::new();
    // Air Elemental (4/4 flying) and Grizzly Bears (2/2) both get tapped; only the flier is dealt
    // the 2, and 2 doesn't kill a 4/4.
    let flier = game.spawn_on_battlefield(PlayerId(1), card("Air Elemental"));
    let ground = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let spare = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let blast = game.spawn_in_hand(PlayerId(0), card("Winter Blast"));

    cast_x_targeting(&mut game, blast, 2, vec![flier, ground]);
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(flier), "the flier is tapped");
    assert!(game.is_tapped(ground), "the ground creature is tapped too");
    assert!(
        !game.is_tapped(spare),
        "an untargeted creature is untouched"
    );
    assert_eq!(game.marked_damage(flier), 2, "2 damage to the flier");
    assert_eq!(
        game.marked_damage(ground),
        0,
        "no damage to a target without flying"
    );
}

#[test]
fn winter_blast_kills_a_small_flier_it_taps() {
    let mut game = Game::new();
    // A 1/1 flier dies to the 2; the tap and the burn ride the same chosen target.
    let bats = game.spawn_on_battlefield(PlayerId(1), card("Vampire Bats"));
    let spare = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let blast = game.spawn_in_hand(PlayerId(0), card("Winter Blast"));

    cast_x_targeting(&mut game, blast, 1, vec![bats]);
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(bats), Zone::Graveyard);
    assert_eq!(game.zone_of(spare), Zone::Battlefield);
}

// ── Increment #85: Visions ─────────────────────────────────────────────────────────────────────
// "Look at the top five cards of target player's library. You may then have that player shuffle
// that library." The look itself is a look-only pause (increment 150, `leg_w12_a.rs`).

#[test]
fn visions_looks_at_five_of_the_targeted_players_library_then_offers_a_shuffle() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let visions = game.spawn_in_hand(PlayerId(0), card("Visions"));

    cast(&mut game, visions, Some(Target::Player(PlayerId(1))), 0);
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::ArrangeTop {
        player,
        library,
        cards,
        ..
    }) = game.pending_choice()
    else {
        panic!(
            "Visions pauses on the look, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(player, PlayerId(0), "the caster does the looking");
    assert_eq!(library, PlayerId(1), "at the targeted player's library");
    assert_eq!(cards.len(), 5, "the top five");
}

// ── Increment #120: Spurnmage Advocate ─────────────────────────────────────────────────────────
// "{T}: Return two target cards from an opponent's graveyard to their hand. Destroy target
// attacking creature."

/// Player 0 activates `object`'s first ability, targeting `target` for the destroy clause.
fn activate(game: &mut Game, object: ObjectId, target: Option<Target>) {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index: 0,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap_or_else(|e| panic!("activation should be legal: {e:?}"));
}

#[test]
fn spurnmage_advocate_returns_two_graveyard_cards_and_destroys_an_attacker() {
    let mut game = Game::new();
    let advocate = game.spawn_on_battlefield(PlayerId(0), card("Spurnmage Advocate"));
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bolt = game.spawn_in_graveyard(PlayerId(1), card("Lightning Bolt"));
    let bears = game.spawn_in_graveyard(PlayerId(1), card("Grizzly Bears"));
    let spare = game.spawn_in_graveyard(PlayerId(1), card("Air Elemental"));

    attack_with(&mut game, vec![attacker]);
    activate(&mut game, advocate, Some(Target::Object(attacker)));
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(bolt), Target::Object(bears)],
    })
    .expect("two cards from an opponent's graveyard");
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(advocate), "{{T}} is the whole cost");
    assert_eq!(
        game.zone_of(attacker),
        Zone::Graveyard,
        "the attacking creature is destroyed"
    );
    assert_eq!(game.zone_of(bolt), Zone::Hand);
    assert_eq!(game.zone_of(bears), Zone::Hand);
    assert_eq!(
        game.zone_of(spare),
        Zone::Graveyard,
        "an unchosen graveyard card stays put"
    );
}

#[test]
fn spurnmage_advocate_returns_the_cards_to_their_owner_not_to_the_activator() {
    let mut game = Game::new();
    let advocate = game.spawn_on_battlefield(PlayerId(0), card("Spurnmage Advocate"));
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.spawn_in_graveyard(PlayerId(1), card("Lightning Bolt"));
    game.spawn_in_graveyard(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![attacker]);
    // Exactly two legal cards for a clause that takes exactly two: there is nothing to choose, so
    // the engine takes both without pausing.
    activate(&mut game, advocate, Some(Target::Object(attacker)));
    assert!(game.pending_choice().is_none(), "no choice to make");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.hand(PlayerId(1)).len(), 2, "returned to their hand");
    assert!(game.hand(PlayerId(0)).is_empty(), "not to yours");
}

#[test]
fn spurnmage_advocate_needs_two_cards_in_one_opponents_graveyard() {
    // One card is not two, and your own graveyard is not an opponent's — the ability can't be
    // activated at all (CR 601.2c: every clause needs its full complement of legal targets).
    let mut game = Game::new();
    let advocate = game.spawn_on_battlefield(PlayerId(0), card("Spurnmage Advocate"));
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.spawn_in_graveyard(PlayerId(1), card("Lightning Bolt"));
    game.spawn_in_graveyard(PlayerId(0), card("Lightning Bolt"));

    attack_with(&mut game, vec![attacker]);
    assert_eq!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: advocate,
            ability_index: 0,
            target: Some(Target::Object(attacker)),
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        }),
        Err(Reject::IllegalTarget)
    );
}

// ── Recall (#69) ────────────────────────────────────────────────────────────────────────────────

#[test]
fn recall_returns_one_graveyard_card_per_card_it_discarded() {
    // "Discard X cards, then return a card from your graveyard to your hand for each card
    // discarded this way. Exile Recall." X=2 discards two and returns two, one prompt at a time —
    // and the discarded cards are themselves in the graveyard by then, so they are fair game.
    let mut game = Game::new();
    let recall = game.spawn_in_hand(PlayerId(0), card("Recall"));
    let pitch_a = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    let pitch_b = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    let buried = game.spawn_in_graveyard(PlayerId(0), card("Lightning Bolt"));
    let spare = game.spawn_in_graveyard(PlayerId(0), card("Forest"));

    cast(&mut game, recall, None, 2);
    resolve_top_of_stack(&mut game);

    let Some(PendingChoice::DiscardCards {
        player: PlayerId(0),
        count: 2,
        ..
    }) = game.pending_choice()
    else {
        panic!("X=2 discards two: {:?}", game.pending_choice());
    };
    game.submit(Intent::Discard {
        player: PlayerId(0),
        cards: vec![pitch_a, pitch_b],
    })
    .expect("discarding exactly X cards from hand is legal");

    // Two discards owe two returns, prompted one at a time.
    for want in [buried, spare] {
        assert!(
            matches!(
                game.pending_choice(),
                Some(PendingChoice::MayReturnFromGraveyard {
                    player: PlayerId(0),
                    ..
                })
            ),
            "a return prompt per card discarded: {:?}",
            game.pending_choice()
        );
        game.submit(Intent::ChooseSacrifices {
            player: PlayerId(0),
            sacrifices: vec![want],
        })
        .expect("returning a card from your own graveyard is legal");
    }

    assert!(game.pending_choice().is_none(), "two returns, then done");
    assert_eq!(game.zone_of(buried), Zone::Hand);
    assert_eq!(game.zone_of(spare), Zone::Hand);
    assert_eq!(game.zone_of(pitch_a), Zone::Graveyard, "discarded");
    assert_eq!(game.zone_of(pitch_b), Zone::Graveyard, "discarded");
    assert_eq!(game.zone_of(recall), Zone::Exile, "Exile Recall.");
}

#[test]
fn recall_for_zero_discards_nothing_returns_nothing_and_still_exiles() {
    // X=0: no discard, so "for each card discarded this way" is zero and the spell never pauses —
    // but it still exiles itself instead of going to the graveyard.
    let mut game = Game::new();
    let recall = game.spawn_in_hand(PlayerId(0), card("Recall"));
    let buried = game.spawn_in_graveyard(PlayerId(0), card("Lightning Bolt"));

    cast(&mut game, recall, None, 0);
    resolve_top_of_stack(&mut game);

    assert!(game.pending_choice().is_none(), "nothing to choose");
    assert_eq!(game.zone_of(buried), Zone::Graveyard, "nothing returned");
    assert_eq!(game.zone_of(recall), Zone::Exile, "Exile Recall.");
}
