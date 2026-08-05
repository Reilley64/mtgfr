//! Legends (`leg`) grind, wave 11 slice D — the long tail of one-card increments.
//!
//! Increments 39 (`gabriel-angelfire`), 45 (`hazezon-tamar`), 50 (`johan`),
//! 63 (`primordial-ooze`), 68 (`rasputin-dreamweaver`), 73 (`rohgahh-of-kher-keep`),
//! 74 (`stangg-twin`), 93 (`wood-elemental`), 54 (`lesser-werewolf`) and
//! 36 (`firestorm-phoenix`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority across several turns can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..80 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

fn cast(game: &mut Game, player: PlayerId, object: ObjectId) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
    game.fund_mana(player);
    game.submit(Intent::Cast {
        player,
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
}

/// Roll forward to `player`'s next upkeep and on into their draw step, so anything the upkeep put
/// on the stack has resolved.
fn through_upkeep_of(game: &mut Game, player: PlayerId) {
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Upkeep
    });
    advance_until(game, |g| g.current_step() == Step::Draw);
}

/// Every battlefield permanent named `name`, whoever controls it.
fn named_on_battlefield(game: &Game, name: &str) -> usize {
    game.live_object_ids()
        .into_iter()
        .filter(|&id| game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == name)
        .count()
}

// ── increment 45: Hazezon Tamar ────────────────────────────────────────────────────────

/// Cast Hazezon from player 0's hand and let both the spell and its enters trigger resolve.
fn resolve_hazezon(game: &mut Game) -> ObjectId {
    let card_id = game.spawn_in_hand(PlayerId(0), card("Hazezon Tamar"));
    cast(game, PlayerId(0), card_id).expect("Hazezon is castable with a funded pool");
    resolve_top_of_stack(game);
    // The enters trigger goes on the stack behind the spell; resolving it only *schedules* the
    // tokens, it does not create them.
    resolve_top_of_stack(game);
    game.live_object_ids()
        .into_iter()
        .find(|&id| {
            game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == "Hazezon Tamar"
        })
        .expect("Hazezon resolved onto the battlefield")
}

#[test]
fn hazezon_waits_for_your_next_upkeep() {
    // "When Hazezon enters, create X 1/1 Sand Warrior creature tokens … at the beginning of your
    // next upkeep" — nothing arrives while the trigger is merely scheduled.
    let mut game = Game::new();
    stock_libraries(&mut game);
    for _ in 0..3 {
        game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    }
    resolve_hazezon(&mut game);

    assert_eq!(
        named_on_battlefield(&game, "Sand Warrior"),
        0,
        "the enters trigger only schedules — no tokens yet",
    );
    through_upkeep_of(&mut game, PlayerId(1));
    assert_eq!(
        named_on_battlefield(&game, "Sand Warrior"),
        0,
        "\"your next upkeep\" is Hazezon's controller's, not the next upkeep to begin",
    );
    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(
        named_on_battlefield(&game, "Sand Warrior"),
        3,
        "three lands controlled → three Sand Warriors",
    );
}

#[test]
fn hazezon_counts_lands_at_the_delayed_resolution() {
    // "where X is the number of lands you control **at that time**" — the count is taken when the
    // delayed trigger resolves, not when Hazezon entered.
    let mut game = Game::new();
    stock_libraries(&mut game);
    for _ in 0..2 {
        game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    }
    resolve_hazezon(&mut game);
    for _ in 0..3 {
        game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    }

    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(
        named_on_battlefield(&game, "Sand Warrior"),
        5,
        "five lands by the time the delayed trigger resolved",
    );
}

#[test]
fn hazezon_leaving_exiles_every_sand_warrior() {
    // "When Hazezon leaves the battlefield, exile all Sand Warriors."
    let mut game = Game::new();
    stock_libraries(&mut game);
    for _ in 0..3 {
        game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    }
    let hazezon = resolve_hazezon(&mut game);
    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(named_on_battlefield(&game, "Sand Warrior"), 3);

    let terror = game.spawn_in_hand(PlayerId(0), card("Terror"));
    give_priority(&mut game, PlayerId(0));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: terror,
        target: Some(Target::Object(hazezon)),
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
    .expect("Terror can kill Hazezon");
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(hazezon),
        Zone::Graveyard,
        "Hazezon died to the removal",
    );
    assert_eq!(
        named_on_battlefield(&game, "Sand Warrior"),
        0,
        "the leaves trigger exiled the whole band",
    );
}

// ── increment 50: Johan ───────────────────────────────────────────────────────────────

/// Johan plus a bear that can swing, both already past summoning sickness.
fn johan_board(game: &mut Game) -> (ObjectId, ObjectId) {
    stock_libraries(game);
    let johan = game.spawn_on_battlefield(PlayerId(0), card("Johan"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    (johan, bear)
}

/// Roll to Johan's begin-combat "you may" and answer it.
fn answer_johan(game: &mut Game, yes: bool) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::MayYesNo { .. }))
    });
    game.submit(Intent::AnswerMay {
        player: PlayerId(0),
        yes,
    })
    .unwrap();
}

#[test]
fn johan_taken_leaves_your_attackers_untapped() {
    // "If you do, attacking doesn't cause creatures you control to tap this combat if Johan is
    // untapped."
    let mut game = Game::new();
    let (_johan, bear) = johan_board(&mut game);

    answer_johan(&mut game, true);
    attack_with(&mut game, vec![bear]);
    assert!(
        !game.is_tapped(bear),
        "attacking didn't cause the bear to tap",
    );
}

#[test]
fn johan_declined_still_taps_your_attackers() {
    // The whole clause is one "you may" — decline and nothing changes.
    let mut game = Game::new();
    let (_johan, bear) = johan_board(&mut game);

    answer_johan(&mut game, false);
    attack_with(&mut game, vec![bear]);
    assert!(
        game.is_tapped(bear),
        "a declined trigger grants no pseudo-vigilance"
    );
}

#[test]
fn johan_cant_attack_once_the_trigger_is_taken() {
    // "you may have Johan gain \"Johan can't attack\" until end of combat."
    let mut game = Game::new();
    let (johan, _bear) = johan_board(&mut game);

    answer_johan(&mut game, true);
    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    assert!(
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![(johan, Defender::Player(PlayerId(1)))],
        })
        .is_err(),
        "Johan gained \"Johan can't attack\"",
    );
}

#[test]
fn a_tapped_johan_taxes_your_attackers_again() {
    // "… if Johan is untapped" — the pseudo-vigilance is conditional, read at the declaration.
    let mut game = Game::new();
    let (johan, bear) = johan_board(&mut game);

    // Tapped in the first main phase, before combat begins — the trigger's own clause reads
    // Johan's state at the declaration, not at resolution.
    let energy_tap = game.spawn_in_hand(PlayerId(0), card("Energy Tap"));
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    give_priority(&mut game, PlayerId(0));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: energy_tap,
        target: Some(Target::Object(johan)),
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
    .expect("Energy Tap can tap Johan");
    resolve_top_of_stack(&mut game);
    assert!(game.is_tapped(johan), "Johan is tapped now");

    answer_johan(&mut game, true);
    attack_with(&mut game, vec![bear]);
    assert!(
        game.is_tapped(bear),
        "a tapped Johan doesn't hold the tap back",
    );
}

// ── increment 63: Primordial Ooze ─────────────────────────────────────────────────────

/// Roll to the Ooze's upkeep pay-or-else and answer it.
fn answer_ooze(game: &mut Game, pay: bool) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::PayOrElse { .. }))
    });
    game.fund_mana(PlayerId(0));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay,
        discard_cost: Vec::new(),
    })
    .unwrap();
}

#[test]
fn primordial_ooze_grows_every_upkeep() {
    // "At the beginning of your upkeep, put a +1/+1 counter on this creature."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let ooze = game.spawn_on_battlefield(PlayerId(0), card("Primordial Ooze"));

    answer_ooze(&mut game, true);
    assert_eq!(game.power(ooze), 2, "one +1/+1 counter on a printed 1/1");
    answer_ooze(&mut game, true);
    assert_eq!(game.power(ooze), 3, "a second upkeep, a second counter");
}

#[test]
fn declining_the_ooze_taps_it_and_burns_you() {
    // "If you don't, tap this creature and it deals X damage to you", X being the counters on it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let ooze = game.spawn_on_battlefield(PlayerId(0), card("Primordial Ooze"));
    let life = game.life(PlayerId(0));

    answer_ooze(&mut game, false);
    assert!(game.is_tapped(ooze), "declining taps the Ooze");
    assert_eq!(
        game.life(PlayerId(0)),
        life - 1,
        "X is the one counter it just got, not a chosen value",
    );
}

#[test]
fn the_ooze_tax_climbs_with_its_counters() {
    // "where X is the number of +1/+1 counters on it" — read live, so the second decline costs 2.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Primordial Ooze"));

    answer_ooze(&mut game, true);
    let life = game.life(PlayerId(0));
    answer_ooze(&mut game, false);
    assert_eq!(
        game.life(PlayerId(0)),
        life - 2,
        "two counters by the second upkeep",
    );
}

#[test]
fn primordial_ooze_attacks_each_combat_if_able() {
    // "This creature attacks each combat if able."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let ooze = game.spawn_on_battlefield(PlayerId(0), card("Primordial Ooze"));

    answer_ooze(&mut game, true);
    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    assert!(
        game.required_attacks(PlayerId(0))
            .iter()
            .any(|&(a, _)| a == ooze),
        "the Ooze is a required attacker",
    );
}

// ── increment 68: Rasputin Dreamweaver ────────────────────────────────────────────────

/// Rasputin cast and resolved onto player 0's battlefield, plus stocked libraries — cast rather
/// than spawned, because "enters with seven dream counters on it" is a CR 614.1c replacement that
/// only the real ETB path applies.
fn rasputin_board() -> (Game, ObjectId) {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let card_id = game.spawn_in_hand(PlayerId(0), card("Rasputin Dreamweaver"));
    cast(&mut game, PlayerId(0), card_id).expect("Rasputin is castable with a funded pool");
    resolve_top_of_stack(&mut game);
    let rasputin = game
        .live_object_ids()
        .into_iter()
        .find(|&id| {
            game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == "Rasputin Dreamweaver"
        })
        .expect("Rasputin resolved onto the battlefield");
    (game, rasputin)
}

/// Spend one dream counter on `ability_index` (1 = add {C}, 2 = the prevention shield; index 0 is
/// the enters-with-counters static).
fn spend_a_dream(game: &mut Game, rasputin: ObjectId, ability_index: usize) {
    give_priority(game, PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: rasputin,
        ability_index,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("a dream counter pays the whole activation cost");
}

#[test]
fn rasputin_enters_with_seven_dream_counters() {
    // "Rasputin enters with seven dream counters on it."
    let (game, rasputin) = rasputin_board();
    assert_eq!(game.counters_of_kind(rasputin, CounterKind::Dream), 7);
}

#[test]
fn a_dream_counter_pays_for_a_colorless_mana() {
    // "Remove a dream counter from Rasputin: Add {C}."
    let (mut game, rasputin) = rasputin_board();
    let before = pool_total(&game, PlayerId(0));

    spend_a_dream(&mut game, rasputin, 1);
    assert_eq!(
        game.counters_of_kind(rasputin, CounterKind::Dream),
        6,
        "the counter is the cost",
    );
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        before + 1,
        "a mana ability adds without using the stack",
    );
}

/// Point a Prodigal Sorcerer's "{T}: This creature deals 1 damage to any target" at `victim`.
fn ping(game: &mut Game, sorcerer: ObjectId, victim: ObjectId) {
    give_priority(game, PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: sorcerer,
        ability_index: 0,
        target: Some(Target::Object(victim)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("the Sorcerer's only cost is tapping");
    resolve_top_of_stack(game);
}

#[test]
fn one_point_of_damage_kills_an_unshielded_rasputin() {
    // The control for the shield test below: Rasputin is a 4/1, so a single ping is lethal.
    let (mut game, rasputin) = rasputin_board();
    let sorcerer = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));

    ping(&mut game, sorcerer, rasputin);
    assert_ne!(game.zone_of(rasputin), Zone::Battlefield);
}

#[test]
fn a_dream_counter_buys_a_point_of_prevention() {
    // "Remove a dream counter from Rasputin: Prevent the next 1 damage that would be dealt to
    // Rasputin this turn."
    let (mut game, rasputin) = rasputin_board();
    let sorcerer = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    spend_a_dream(&mut game, rasputin, 2);
    resolve_top_of_stack(&mut game);

    ping(&mut game, sorcerer, rasputin);
    assert_eq!(
        game.zone_of(rasputin),
        Zone::Battlefield,
        "the shield ate the only point of damage",
    );
}

#[test]
fn rasputin_regrows_a_dream_counter_at_your_upkeep() {
    // "At the beginning of your upkeep, if Rasputin started the turn untapped, put a dream
    // counter on it."
    let (mut game, rasputin) = rasputin_board();
    spend_a_dream(&mut game, rasputin, 1);
    assert_eq!(game.counters_of_kind(rasputin, CounterKind::Dream), 6);

    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(
        game.counters_of_kind(rasputin, CounterKind::Dream),
        7,
        "an untapped Rasputin regrows one",
    );
}

#[test]
fn a_rasputin_that_started_the_turn_tapped_regrows_nothing() {
    // "if Rasputin started the turn untapped" — tapped as the turn begins, the trigger never
    // fires, even though the untap step untaps it moments later.
    let (mut game, rasputin) = rasputin_board();
    spend_a_dream(&mut game, rasputin, 1);
    // Tap it on the turn before, so the untap step finds it tapped.
    advance_until(&mut game, |g| g.active_player() == PlayerId(1));
    game.tap(rasputin);

    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(
        game.counters_of_kind(rasputin, CounterKind::Dream),
        6,
        "the intervening-if reads the turn's start, not the upkeep",
    );
}

#[test]
fn rasputin_cant_have_more_than_seven_dream_counters() {
    // "Rasputin can't have more than seven dream counters on it."
    let (mut game, rasputin) = rasputin_board();
    through_upkeep_of(&mut game, PlayerId(0));
    assert_eq!(
        game.counters_of_kind(rasputin, CounterKind::Dream),
        7,
        "the upkeep trigger can't push it past the maximum",
    );
}

// ── increment 39: Gabriel Angelfire ───────────────────────────────────────────────────

/// Advance to player 0's upkeep and answer Gabriel's keyword choice with `mode`.
fn upkeep_and_choose(game: &mut Game, mode: usize) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::ChooseMode { .. }))
    });
    game.submit(Intent::ChooseMode {
        player: PlayerId(0),
        mode,
    })
    .unwrap();
}

#[test]
fn gabriel_angelfire_gains_the_chosen_keyword() {
    // "At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3.
    // Gabriel Angelfire gains that ability until your next upkeep."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let gabriel = game.spawn_on_battlefield(PlayerId(0), card("Gabriel Angelfire"));

    assert!(
        !game.has_keyword(gabriel, Keyword::Flying),
        "printed Gabriel is a vanilla 4/4",
    );
    upkeep_and_choose(&mut game, 0);
    assert!(
        game.has_keyword(gabriel, Keyword::Flying),
        "mode 0 is flying",
    );
    assert!(
        !game.has_keyword(gabriel, Keyword::Trample),
        "only the chosen ability is granted",
    );
}

#[test]
fn gabriel_angelfire_can_choose_rampage_3() {
    // The fourth mode is a keyword with a value, not a bare one.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let gabriel = game.spawn_on_battlefield(PlayerId(0), card("Gabriel Angelfire"));

    upkeep_and_choose(&mut game, 3);
    assert!(
        game.has_keyword(gabriel, Keyword::Rampage(3)),
        "mode 3 is rampage 3",
    );
}

#[test]
fn gabriels_grant_survives_the_turn_it_was_made_in() {
    // "until your next upkeep" outlives cleanup — unlike every until-end-of-turn grant.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let gabriel = game.spawn_on_battlefield(PlayerId(0), card("Gabriel Angelfire"));

    upkeep_and_choose(&mut game, 0);
    pass_until_next_turn(&mut game);
    assert!(
        game.has_keyword(gabriel, Keyword::Flying),
        "the grant is still live on the next player's turn",
    );
}

#[test]
fn gabriels_grant_ends_as_his_next_upkeep_begins() {
    // "until *your* next upkeep": the old grant is gone by the time the new trigger resolves, so
    // choosing trample the second time around leaves Gabriel with trample and not flying.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let gabriel = game.spawn_on_battlefield(PlayerId(0), card("Gabriel Angelfire"));

    upkeep_and_choose(&mut game, 0);
    upkeep_and_choose(&mut game, 2);
    assert!(
        game.has_keyword(gabriel, Keyword::Trample),
        "the second upkeep's choice is live",
    );
    assert!(
        !game.has_keyword(gabriel, Keyword::Flying),
        "the first upkeep's grant expired as this upkeep began",
    );
}

// ── increment 73: Rohgahh of Kher Keep ────────────────────────────────────────────────

/// Rohgahh plus one Kobolds of Kher Keep for each seat, so "all creatures named Kobolds of Kher
/// Keep" has something to sweep on both sides of the table.
fn rohgahh_board() -> (Game, ObjectId, ObjectId, ObjectId) {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let rohgahh = game.spawn_on_battlefield(PlayerId(0), card("Rohgahh of Kher Keep"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Kobolds of Kher Keep"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Kobolds of Kher Keep"));
    (game, rohgahh, mine, theirs)
}

/// Roll to Rohgahh's upkeep pay-or-else and answer it.
fn answer_rohgahh(game: &mut Game, pay: bool) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::PayOrElse { .. }))
    });
    game.fund_mana(PlayerId(0));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay,
        discard_cost: Vec::new(),
    })
    .unwrap();
}

#[test]
fn your_kobolds_of_kher_keep_get_plus_two_two() {
    // "Creatures you control named Kobolds of Kher Keep get +2/+2."
    let (game, _rohgahh, mine, theirs) = rohgahh_board();
    assert_eq!((game.power(mine), game.toughness(mine)), (2, 3));
    assert_eq!(
        (game.power(theirs), game.toughness(theirs)),
        (0, 1),
        "an opponent's Kobolds are not yours",
    );
}

#[test]
fn paying_rrr_keeps_rohgahh_and_his_kobolds() {
    // "At the beginning of your upkeep, you may pay {R}{R}{R}."
    let (mut game, rohgahh, mine, _theirs) = rohgahh_board();

    answer_rohgahh(&mut game, true);
    assert!(!game.is_tapped(rohgahh), "the toll was paid");
    assert_eq!(game.controller_of(rohgahh), PlayerId(0));
    assert_eq!(game.controller_of(mine), PlayerId(0));
}

#[test]
fn declining_taps_rohgahh_and_every_kobold() {
    // "If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, …"
    let (mut game, rohgahh, mine, theirs) = rohgahh_board();

    answer_rohgahh(&mut game, false);
    assert!(game.is_tapped(rohgahh));
    assert!(game.is_tapped(mine));
    assert!(
        game.is_tapped(theirs),
        "the clause says all of them, not just yours",
    );
}

#[test]
fn declining_hands_them_to_an_opponent() {
    // "… then an opponent gains control of them."
    let (mut game, rohgahh, mine, _theirs) = rohgahh_board();

    answer_rohgahh(&mut game, false);
    assert_eq!(
        game.controller_of(rohgahh),
        PlayerId(1),
        "the only opponent takes Rohgahh",
    );
    assert_eq!(game.controller_of(mine), PlayerId(1));
}

#[test]
fn the_controller_picks_which_opponent_takes_them() {
    // "an opponent gains control of them" — with more than one opponent alive the controller
    // chooses, and one opponent takes the whole set.
    let mut game = Game::with_players(4, 0);
    stock_libraries(&mut game);
    let rohgahh = game.spawn_on_battlefield(PlayerId(0), card("Rohgahh of Kher Keep"));
    let kobold = game.spawn_on_battlefield(PlayerId(0), card("Kobolds of Kher Keep"));

    answer_rohgahh(&mut game, false);
    let Some(PendingChoice::ChooseSplittingOpponent { player, legal, .. }) = game.pending_choice()
    else {
        panic!(
            "expected the choose-an-opponent pause, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(player, PlayerId(0), "Rohgahh's controller chooses");
    assert_eq!(legal, vec![PlayerId(1), PlayerId(2), PlayerId(3)]);

    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Player(PlayerId(2))],
    })
    .unwrap();
    assert_eq!(game.controller_of(rohgahh), PlayerId(2));
    assert_eq!(
        game.controller_of(kobold),
        PlayerId(2),
        "one opponent takes them all, not one apiece",
    );
}

// ── increment 74: Stangg ──────────────────────────────────────────────────────────────

/// Cast Stangg and let both the spell and its enters trigger resolve; returns Stangg and his Twin.
fn stangg_board() -> (Game, ObjectId, ObjectId) {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let card_id = game.spawn_in_hand(PlayerId(0), card("Stangg"));
    cast(&mut game, PlayerId(0), card_id).expect("Stangg is castable with a funded pool");
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);
    let stangg = game
        .live_object_ids()
        .into_iter()
        .find(|&id| game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == "Stangg")
        .expect("Stangg resolved onto the battlefield");
    let twin = game
        .live_object_ids()
        .into_iter()
        .find(|&id| game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == "Stangg Twin")
        .expect("the enters trigger created the Twin");
    (game, stangg, twin)
}

/// Destroy `victim` with a Terror from player 0's hand, resolving both the spell and whatever it
/// triggers.
fn terror(game: &mut Game, victim: ObjectId) {
    let terror = game.spawn_in_hand(PlayerId(0), card("Terror"));
    give_priority(game, PlayerId(0));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: terror,
        target: Some(Target::Object(victim)),
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
    .expect("Terror can kill a red-green creature");
    resolve_top_of_stack(game);
    resolve_top_of_stack(game);
}

#[test]
fn stangg_enters_with_his_twin() {
    // "When Stangg enters, create Stangg Twin, a legendary 3/4 red and green Human Warrior
    // creature token."
    let (game, _stangg, twin) = stangg_board();
    assert_eq!((game.power(twin), game.toughness(twin)), (3, 4));
    let def = game.def_of(twin);
    assert!(def.legendary, "the Twin is legendary");
    assert_eq!(&*def.subtypes, ["Human", "Warrior"]);
}

#[test]
fn stangg_leaving_exiles_the_twin() {
    // "Exile that token when Stangg leaves the battlefield."
    let (mut game, stangg, twin) = stangg_board();

    terror(&mut game, stangg);
    assert_eq!(game.zone_of(stangg), Zone::Graveyard);
    // A token that leaves the battlefield ceases to exist (CR 111.7), so it is gone from the game
    // rather than sitting in exile.
    assert!(
        !game.live_object_ids().contains(&twin),
        "the Twin is exiled with him",
    );
}

#[test]
fn the_twin_leaving_sacrifices_stangg() {
    // "Sacrifice Stangg when that token leaves the battlefield."
    let (mut game, stangg, twin) = stangg_board();

    terror(&mut game, twin);
    assert_eq!(
        game.zone_of(stangg),
        Zone::Graveyard,
        "Stangg is sacrificed after his Twin dies",
    );
}

// ── increment 93: Wood Elemental ──────────────────────────────────────────────────────

/// `forests` untapped Forests, then Wood Elemental cast and resolved — stopping on the as-enters
/// "sacrifice any number of untapped Forests" choice when there is one to make.
fn wood_elemental_board(forests: usize) -> (Game, Vec<ObjectId>) {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let lands: Vec<ObjectId> = (0..forests)
        .map(|_| game.spawn_on_battlefield(PlayerId(0), card("Forest")))
        .collect();
    let card_id = game.spawn_in_hand(PlayerId(0), card("Wood Elemental"));
    cast(&mut game, PlayerId(0), card_id).expect("Wood Elemental is castable with a funded pool");
    resolve_top_of_stack(&mut game);
    (game, lands)
}

/// The Wood Elemental on the battlefield, if it is still there.
fn wood_elemental(game: &Game) -> Option<ObjectId> {
    game.live_object_ids().into_iter().find(|&id| {
        game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == "Wood Elemental"
    })
}

#[test]
fn sacrificing_two_forests_makes_a_two_two() {
    // "As this creature enters, sacrifice any number of untapped Forests. Its power and toughness
    // are each equal to the number of Forests sacrificed as it entered."
    let (mut game, forests) = wood_elemental_board(3);

    game.submit(Intent::ChooseSacrifices {
        player: PlayerId(0),
        sacrifices: vec![forests[0], forests[1]],
    })
    .expect("any number of the untapped Forests may be sacrificed");

    let elemental = wood_elemental(&game).expect("a 2/2 survives the state-based check");
    assert_eq!((game.power(elemental), game.toughness(elemental)), (2, 2));
    assert_eq!(game.zone_of(forests[0]), Zone::Graveyard);
    assert_eq!(game.zone_of(forests[1]), Zone::Graveyard);
    assert_eq!(
        game.zone_of(forests[2]),
        Zone::Battlefield,
        "only the Forests named are sacrificed",
    );
}

#[test]
fn sacrificing_nothing_leaves_a_zero_zero() {
    // "any number" includes none — and a 0/0 dies to state-based actions (CR 704.5a).
    let (mut game, _forests) = wood_elemental_board(2);

    game.submit(Intent::ChooseSacrifices {
        player: PlayerId(0),
        sacrifices: vec![],
    })
    .expect("declining is a legal answer");

    assert!(
        wood_elemental(&game).is_none(),
        "0/0 with no Forests sacrificed dies immediately",
    );
}

#[test]
fn its_size_is_frozen_at_the_count_it_entered_with() {
    // "…the number of Forests sacrificed as it entered" — not the number of Forests you control
    // now, so a later Forest doesn't grow it.
    let (mut game, forests) = wood_elemental_board(3);

    game.submit(Intent::ChooseSacrifices {
        player: PlayerId(0),
        sacrifices: vec![forests[0], forests[1]],
    })
    .unwrap();
    let elemental = wood_elemental(&game).expect("a 2/2 survives the state-based check");
    game.spawn_on_battlefield(PlayerId(0), card("Forest"));

    assert_eq!(
        (game.power(elemental), game.toughness(elemental)),
        (2, 2),
        "the count was locked in as it entered",
    );
}
