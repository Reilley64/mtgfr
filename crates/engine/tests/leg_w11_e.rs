//! Legends (`leg`) grind, wave 11 slice E — targeting and cost vocabulary.
//!
//! Increments 58 (name a card), 59 (spend mana as any type), 61 (X target creatures),
//! 80 (the opponent chooses the target), 106 (sacrifice filtered permanents as an alternative
//! cost), 110/145 (per-effect targets), 129 (unbounded "one or more target" clauses) and
//! 131 (a second target clause for spells).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Cast an instant/sorcery from `player`'s hand, auto-funded, with no announced target — the
/// multi-target clause pauses for [`Intent::ChooseTargets`].
fn cast_spell(game: &mut Game, player: PlayerId, name: &str, x: u32) -> ObjectId {
    game.fund_mana(player);
    let spell = game.spawn_in_hand(player, card(name));
    game.submit(Intent::Cast {
        player,
        object: spell,
        target: None,
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
    .unwrap_or_else(|e| panic!("{name} is castable: {e:?}"));
    spell
}

fn choose_targets(game: &mut Game, player: PlayerId, targets: &[ObjectId]) {
    game.submit(Intent::ChooseTargets {
        player,
        targets: targets.iter().map(|&id| Target::Object(id)).collect(),
    })
    .expect("a legal set of targets");
}

fn is_color(game: &Game, object: ObjectId, color: Color) -> bool {
    game.colors_of(object)[color as usize]
}

fn is_green(game: &Game, object: ObjectId) -> bool {
    is_color(game, object, Color::Green)
}

// ── increment 129: unbounded "one or more target" clauses ─────────────────────────────

/// CR 601.2c: "one or more target creatures" lets the caster name every creature on the
/// battlefield. Seven bodies is past the six the fixed-width target list used to stop at.
#[test]
fn sylvan_paradise_recolors_seven_targets() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..7)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")))
        .collect();

    cast_spell(&mut game, PlayerId(0), "Sylvan Paradise", 0);
    choose_targets(&mut game, PlayerId(0), &bears);
    resolve_top_of_stack(&mut game);

    for (i, &bear) in bears.iter().enumerate() {
        assert!(is_green(&game, bear), "chosen creature {i} became green");
    }
}

/// The cycle's other four cards share the clause, so one of them proves the increment is in the
/// DSL rather than in one script: Touch of Darkness is the black member and takes seven too.
#[test]
fn touch_of_darkness_recolors_seven_targets() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..7)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")))
        .collect();

    cast_spell(&mut game, PlayerId(0), "Touch of Darkness", 0);
    choose_targets(&mut game, PlayerId(0), &bears);
    resolve_top_of_stack(&mut game);

    for (i, &bear) in bears.iter().enumerate() {
        assert!(
            is_color(&game, bear, Color::Black),
            "chosen creature {i} became black",
        );
    }
}

/// The clamp CR 601.2c calls "the maximum possible number" — an unbounded clause on a board with
/// fewer creatures than the engine's width takes every one of them and stops, rather than
/// demanding the caster produce targets that do not exist.
#[test]
fn an_unbounded_clause_settles_for_every_creature_that_exists() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..2)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")))
        .collect();

    cast_spell(&mut game, PlayerId(0), "Sylvan Paradise", 0);
    choose_targets(&mut game, PlayerId(0), &bears);
    resolve_top_of_stack(&mut game);

    for &bear in &bears {
        assert!(is_green(&game, bear), "a two-creature board is a legal set");
    }
}

// ── increment 58: choose a card name ──────────────────────────────────────────────────

/// Activate `object`'s ability `index` for `player`, auto-funded, with `x` and one chosen target.
fn activate(game: &mut Game, player: PlayerId, object: ObjectId, target: Target, x: u32) {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target: Some(target),
        sacrifice: vec![],
        discard_cost: vec![],
        x,
    })
    .expect("the ability is activatable");
}

fn name_card(game: &mut Game, player: PlayerId, name: &str) {
    game.submit(Intent::ChooseCardName {
        player,
        name: name.to_owned(),
    })
    .expect("a card name is always a legal answer");
}

/// CR 201.2/703.2j: Petra Sphinx's chooser is the *targeted* player, not the Sphinx's controller,
/// and a hit puts the revealed card into that player's hand.
#[test]
fn petra_sphinx_hit_puts_the_top_card_in_the_named_players_hand() {
    let mut game = Game::new();
    let sphinx = game.spawn_on_battlefield(PlayerId(0), card("Petra Sphinx"));
    let top = game.stack_library(PlayerId(1), &[card("Grizzly Bears")])[0];

    activate(
        &mut game,
        PlayerId(0),
        sphinx,
        Target::Player(PlayerId(1)),
        0,
    );
    resolve_top_of_stack(&mut game);
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::ChooseCardName { player, .. }) if player == PlayerId(1)
        ),
        "the targeted player names the card, got {:?}",
        game.pending_choice(),
    );

    name_card(&mut game, PlayerId(1), "Grizzly Bears");
    assert_eq!(game.zone_of(top), Zone::Hand, "a hit goes to hand");
    assert_eq!(game.library_size(PlayerId(1)), 0, "and leaves the library");
}

/// The miss half: "If it doesn't, the player puts it into their graveyard" — the graveyard, not
/// Conundrum Sphinx's bottom of the library.
#[test]
fn petra_sphinx_miss_puts_the_top_card_in_the_named_players_graveyard() {
    let mut game = Game::new();
    let sphinx = game.spawn_on_battlefield(PlayerId(0), card("Petra Sphinx"));
    let top = game.stack_library(PlayerId(1), &[card("Grizzly Bears")])[0];

    activate(
        &mut game,
        PlayerId(0),
        sphinx,
        Target::Player(PlayerId(1)),
        0,
    );
    resolve_top_of_stack(&mut game);
    name_card(&mut game, PlayerId(1), "Black Lotus");

    assert!(
        game.hand(PlayerId(1)).is_empty(),
        "a miss never reaches hand"
    );
    assert_eq!(
        game.zone_of(top),
        Zone::Graveyard,
        "a miss goes to the graveyard, not the bottom of the library",
    );
}

/// Nebuchadnezzar: "Choose a card name. Target opponent reveals X cards at random from their hand.
/// Then that player discards all cards with that name revealed this way." The *controller* names
/// the card, and a hand of nothing but the named card is emptied whatever the random pick was.
#[test]
fn nebuchadnezzar_discards_every_revealed_card_with_the_named_name() {
    let mut game = Game::new();
    let neb = game.spawn_on_battlefield(PlayerId(0), card("Nebuchadnezzar"));
    let bears: Vec<_> = (0..3)
        .map(|_| game.spawn_in_hand(PlayerId(1), card("Grizzly Bears")))
        .collect();

    activate(&mut game, PlayerId(0), neb, Target::Player(PlayerId(1)), 3);
    resolve_top_of_stack(&mut game);
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::ChooseCardName { player, .. }) if player == PlayerId(0)
        ),
        "Nebuchadnezzar's controller names the card, got {:?}",
        game.pending_choice(),
    );

    name_card(&mut game, PlayerId(0), "Grizzly Bears");
    assert!(
        game.hand(PlayerId(1)).is_empty(),
        "all three revealed Bears are discarded",
    );
    for bear in bears {
        assert_eq!(game.zone_of(bear), Zone::Graveyard);
    }
}

/// X bounds the reveal, so an unrevealed copy of the named card is never discarded: X = 1 against
/// a three-Bear hand reveals one card and discards exactly that one.
#[test]
fn nebuchadnezzar_only_discards_what_x_revealed() {
    let mut game = Game::new();
    let neb = game.spawn_on_battlefield(PlayerId(0), card("Nebuchadnezzar"));
    for _ in 0..3 {
        game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    }

    activate(&mut game, PlayerId(0), neb, Target::Player(PlayerId(1)), 1);
    resolve_top_of_stack(&mut game);
    name_card(&mut game, PlayerId(0), "Grizzly Bears");

    assert_eq!(
        game.hand(PlayerId(1)).len(),
        2,
        "only X cards were revealed, so only X can be discarded",
    );
}

/// A name nothing in hand matches discards nothing (CR 201.3 — a player may even name a card that
/// doesn't exist).
#[test]
fn nebuchadnezzar_misses_discard_nothing() {
    let mut game = Game::new();
    let neb = game.spawn_on_battlefield(PlayerId(0), card("Nebuchadnezzar"));
    for _ in 0..3 {
        game.spawn_in_hand(PlayerId(1), card("Grizzly Bears"));
    }

    activate(&mut game, PlayerId(0), neb, Target::Player(PlayerId(1)), 3);
    resolve_top_of_stack(&mut game);
    name_card(&mut game, PlayerId(0), "Black Lotus");

    assert_eq!(game.hand(PlayerId(1)).len(), 3, "a miss discards nothing");
}

// ── increment 59: spend mana as though it were mana of any type ───────────────────────

/// Cast `name` from `player`'s hand with **no** auto-funding, so the pool the test built is the
/// only mana available — the point of every North Star assertion below.
fn try_cast(game: &mut Game, player: PlayerId, name: &str) -> Result<Vec<Event>, Reject> {
    let spell = game.spawn_in_hand(player, card(name));
    game.submit(Intent::Cast {
        player,
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

/// Activate North Star's `{4}, {T}` and let it resolve, leaving the relaxation on the player.
fn light_north_star(game: &mut Game, star: ObjectId) {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: star,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("{4} is on the table and North Star is untapped");
    resolve_top_of_stack(game);
}

/// The control: white mana can't pay Grizzly Bears' `{G}` pip on its own (CR 202.1).
#[test]
fn off_color_mana_cannot_pay_a_colored_pip_without_north_star() {
    let mut game = Game::new();
    tap_basics(&mut game, "Plains", 2);

    assert!(
        try_cast(&mut game, PlayerId(0), "Grizzly Bears").is_err(),
        "{{1}}{{G}} is not payable out of two white",
    );
}

/// North Star: "For one spell this turn, you may spend mana as though it were mana of any type to
/// pay that spell's mana cost" (CR 609.4b) — six Plains pay the `{4}` activation and then Grizzly
/// Bears' `{1}{G}` out of the two white left over.
#[test]
fn north_star_lets_white_mana_pay_a_green_pip() {
    let mut game = Game::new();
    let star = game.spawn_on_battlefield(PlayerId(0), card("North Star"));
    tap_basics(&mut game, "Plains", 6);

    light_north_star(&mut game, star);
    assert!(
        try_cast(&mut game, PlayerId(0), "Grizzly Bears").is_ok(),
        "the relaxation covers every colored pip",
    );
}

/// "For **one** spell this turn": the relaxation is spent by the first spell cast under it, so a
/// second off-color spell out of the same pool is refused.
#[test]
fn north_star_relaxation_is_spent_by_the_first_spell() {
    let mut game = Game::new();
    let star = game.spawn_on_battlefield(PlayerId(0), card("North Star"));
    tap_basics(&mut game, "Plains", 8);

    light_north_star(&mut game, star);
    try_cast(&mut game, PlayerId(0), "Grizzly Bears").expect("the first spell spends it");
    assert!(
        try_cast(&mut game, PlayerId(0), "Grizzly Bears").is_err(),
        "one spell only, even with mana left over",
    );
}

/// The relaxation is turn-scoped (CR 500.8): an unused one is gone by the next turn.
#[test]
fn north_star_relaxation_does_not_survive_the_turn() {
    let mut game = Game::new();
    let star = game.spawn_on_battlefield(PlayerId(0), card("North Star"));
    // Everyone needs something to draw, or the table decks itself before the turn comes back.
    for seat in 0..2 {
        let deck: Vec<_> = (0..10).map(|_| card("Plains")).collect();
        game.stack_library(PlayerId(seat), &deck);
    }
    tap_basics(&mut game, "Plains", 4);

    light_north_star(&mut game, star);
    for _ in 0..2 {
        pass_until_next_turn(&mut game);
    }
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    tap_basics(&mut game, "Plains", 2);
    assert!(
        try_cast(&mut game, PlayerId(0), "Grizzly Bears").is_err(),
        "the permission wore off with the turn",
    );
}

// ── increment 61: X target creatures ──────────────────────────────────────────────────

/// Part Water: "X target creatures gain islandwalk until end of turn." The target *count* is the
/// announced X (CR 601.2c), so X = 3 names three creatures and all three get the evasion.
#[test]
fn part_water_grants_islandwalk_to_x_target_creatures() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..3)
        .map(|_| game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")))
        .collect();
    let bystander = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    cast_spell(&mut game, PlayerId(0), "Part Water", 3);
    choose_targets(&mut game, PlayerId(0), &bears);
    resolve_top_of_stack(&mut game);

    for (i, &bear) in bears.iter().enumerate() {
        assert!(
            game.has_keyword(bear, Keyword::Landwalk(BasicLandType::Island)),
            "chosen creature {i} gained islandwalk",
        );
    }
    assert!(
        !game.has_keyword(bystander, Keyword::Landwalk(BasicLandType::Island)),
        "an unchosen creature gained nothing",
    );
}

/// X bounds the clause: X = 1 takes exactly one target, so naming two is refused.
#[test]
fn part_water_refuses_more_targets_than_x() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..2)
        .map(|_| game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears")))
        .collect();

    cast_spell(&mut game, PlayerId(0), "Part Water", 1);
    assert!(
        game.submit(Intent::ChooseTargets {
            player: PlayerId(0),
            targets: bears.iter().map(|&id| Target::Object(id)).collect(),
        })
        .is_err(),
        "X = 1 is one target, not two",
    );
}

/// The other half of the increment: Winter Blast's "Tap X target creatures" shares the axis, so
/// the count comes off the same announced X.
#[test]
fn winter_blast_taps_x_target_creatures() {
    let mut game = Game::new();
    let bears: Vec<ObjectId> = (0..3)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears")))
        .collect();
    // A fourth body, so X = 3 is a real pick out of four rather than a forced set of three.
    game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    cast_spell(&mut game, PlayerId(0), "Winter Blast", 3);
    choose_targets(&mut game, PlayerId(0), &bears);
    resolve_top_of_stack(&mut game);

    for (i, &bear) in bears.iter().enumerate() {
        assert!(game.is_tapped(bear), "chosen creature {i} is tapped");
    }
}

// --- Increment 80: `opponent-chooses-the-target` (The Abyss) ----------------------------------
//
// "At the beginning of each player's upkeep, destroy target nonartifact creature that player
// controls **of their choice**." The trigger belongs to The Abyss's controller, but the target is
// picked by the *upkeep* player out of their own board — the first ability in the pool where the
// chooser is not the ability's controller.

/// Nobody decks before the test finishes.
fn stock_libraries(game: &mut Game) {
    let deck = vec![card("Plains"); 10];
    for seat in 0..2 {
        game.stack_library(PlayerId(seat), &deck);
    }
}

/// Roll to the next player's upkeep, stopping the moment a target choice comes up.
fn advance_to_upkeep_choice(game: &mut Game) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::ChooseTarget { .. }))
    });
}

#[test]
fn the_abyss_lets_the_upkeep_player_choose_which_of_their_creatures_dies() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("The Abyss"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let ape = game.spawn_on_battlefield(PlayerId(1), card("Barbary Apes"));

    advance_to_upkeep_choice(&mut game);

    let Some(PendingChoice::ChooseTarget { player, legal, .. }) = game.pending_choice() else {
        panic!("The Abyss's upkeep trigger must pause on a target choice");
    };
    assert_eq!(
        player,
        PlayerId(1),
        "the upkeep player picks, not The Abyss's controller"
    );
    let legal = legal.clone();
    assert_eq!(legal.len(), 2, "only the upkeep player's own two creatures");
    for want in [Target::Object(bear), Target::Object(ape)] {
        assert!(legal.contains(&want), "{want:?} is a legal choice");
    }

    choose_targets(&mut game, PlayerId(1), &[ape]);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(ape),
        Zone::Graveyard,
        "the chosen creature dies"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the one they kept lives"
    );
}

/// The Abyss's controller never gets the choice — and never loses a creature to their own
/// enchantment on someone else's upkeep. Their board is not even in the legal set.
#[test]
fn the_abyss_never_offers_the_controllers_own_creatures_on_an_opponents_upkeep() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("The Abyss"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_to_upkeep_choice(&mut game);

    let Some(PendingChoice::ChooseTarget { legal, .. }) = game.pending_choice() else {
        panic!("The Abyss's upkeep trigger must pause on a target choice");
    };
    assert_eq!(
        legal.clone(),
        vec![Target::Object(theirs)],
        "the controller's own creature is off the table on an opponent's upkeep"
    );
    assert_ne!(legal.clone(), vec![Target::Object(mine)]);
}

/// "**nonartifact** creature" — an artifact creature is never a legal choice, so an upkeep player
/// whose only body is a Juggernaut loses nothing (CR 603.3c drops the trigger outright).
#[test]
fn the_abyss_skips_an_artifact_creature() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("The Abyss"));
    let juggernaut = game.spawn_on_battlefield(PlayerId(1), card("Juggernaut"));

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });

    assert_eq!(
        game.zone_of(juggernaut),
        Zone::Battlefield,
        "an artifact creature is not a nonartifact creature"
    );
}

// --- Increment 110: per-effect targets (Psionic Entity) ---------------------------------------
//
// "{T}: This creature deals 2 damage to any target **and 3 damage to itself**." A sequence's steps
// share one chosen target, so the self-damage half used to land on whatever the first half hit.
// A step naming a fixed reference (CR 115 — "itself") is settled from the source instead.

#[test]
fn psionic_entity_pings_the_chosen_target_and_burns_itself() {
    let mut game = Game::new();
    let entity = game.spawn_on_battlefield(PlayerId(0), card("Psionic Entity"));
    // A 5/5 survives, so the 2 stays visible as marked damage.
    let boars = game.spawn_on_battlefield(PlayerId(1), card("Durkwood Boars"));

    activate(&mut game, PlayerId(0), entity, Target::Object(boars), 0);
    resolve_top_of_stack(&mut game);

    assert_eq!(game.marked_damage(boars), 2, "the chosen target takes 2");
    assert_eq!(
        game.zone_of(entity),
        Zone::Graveyard,
        "3 damage to a 2/2 Illusion kills it",
    );
}

/// The two halves are independent: pointed at a player, the self-damage still lands on the Entity
/// rather than following the first half's target.
#[test]
fn psionic_entity_burns_itself_even_when_it_targets_a_player() {
    let mut game = Game::new();
    let entity = game.spawn_on_battlefield(PlayerId(0), card("Psionic Entity"));
    let before = game.life(PlayerId(1));

    activate(
        &mut game,
        PlayerId(0),
        entity,
        Target::Player(PlayerId(1)),
        0,
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(game.life(PlayerId(1)), before - 2, "the player takes 2");
    assert_eq!(
        game.zone_of(entity),
        Zone::Graveyard,
        "and the Entity still takes its own 3",
    );
}

// Increment 145 rides the same `run_sequence` change: Cocoon's one printed enters trigger ("tap
// enchanted creature **and** put three pupa counters on this Aura") is now scripted as the single
// ability Magic prints instead of two split `etb` triggers. Its behaviour is already asserted by
// `cocoon_holds_its_creature_down_then_hatches_it` in `leg_w10_c.rs`, which is left as the
// regression test rather than duplicated here.

// ── increment 131: a second target clause for spells ──────────────────────────────────

/// Two Walls in front of the Giant, a third in front of the Bears — so "target creature that
/// **target Wall** blocked this turn" has both a real choice and a wrong answer.
/// Returns `(giant, bears, [the Giant's two Walls], the Bears' Wall)`.
fn three_walls_two_attackers(game: &mut Game) -> (ObjectId, ObjectId, [ObjectId; 2], ObjectId) {
    stock_libraries(game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let stone = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let air = game.spawn_on_battlefield(PlayerId(1), card("Wall of Air"));
    let wood = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));

    attack_with(game, vec![giant, bears]);
    block_with(game, vec![(stone, giant), (air, giant), (wood, bears)]).unwrap();
    // The defender casts at instant speed, so hand them priority first.
    while game.priority_holder() != PlayerId(1) {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
    (giant, bears, [stone, air], wood)
}

#[test]
fn glyph_of_delusion_only_offers_walls_that_blocked_the_chosen_creature() {
    // "target creature that **target Wall** blocked this turn" — two independent clauses (CR
    // 601.2c), chosen in printed order, where the first narrows the second.
    let mut game = Game::new();
    let (giant, bears, [stone, air], wood) = three_walls_two_attackers(&mut game);

    cast_spell(&mut game, PlayerId(1), "Glyph of Delusion", 0);
    let Some(PendingChoice::ChooseTarget { clause, legal, .. }) = game.pending_choice() else {
        panic!("clause 0 picks the creature");
    };
    assert_eq!(clause, 0, "the creature is printed first");
    assert_eq!(legal.len(), 2, "both blocked attackers qualify: {legal:?}");
    assert!(legal.contains(&Target::Object(giant)));
    assert!(legal.contains(&Target::Object(bears)));

    choose_targets(&mut game, PlayerId(1), &[giant]);
    let Some(PendingChoice::ChooseTarget { clause, legal, .. }) = game.pending_choice() else {
        panic!("clause 1 picks the Wall");
    };
    assert_eq!(clause, 1, "the Wall is the second clause");
    assert_eq!(legal.len(), 2, "only the Giant's two Walls: {legal:?}");
    assert!(legal.contains(&Target::Object(stone)));
    assert!(legal.contains(&Target::Object(air)));
    assert!(
        !legal.contains(&Target::Object(wood)),
        "the Wall that blocked the Bears never blocked the Giant",
    );
}

#[test]
fn glyph_of_delusion_refuses_a_wall_that_blocked_a_different_creature() {
    // The acceptance criterion: naming the Wall in front of the *other* attacker is illegal.
    let mut game = Game::new();
    let (giant, _, _, wood) = three_walls_two_attackers(&mut game);

    cast_spell(&mut game, PlayerId(1), "Glyph of Delusion", 0);
    choose_targets(&mut game, PlayerId(1), &[giant]);
    let rejected = game.submit(Intent::ChooseTargets {
        player: PlayerId(1),
        targets: vec![Target::Object(wood)],
    });

    assert!(
        rejected.is_err(),
        "the Wall of Wood blocked the Bears, so it is no legal second target for the Giant",
    );
}

#[test]
fn glyph_of_delusion_still_lands_its_counters_with_both_clauses_named() {
    // "where X is the power of that blocked creature" — the second clause restricts, it does not
    // take counters of its own.
    let mut game = Game::new();
    let (giant, _, [stone, _], _) = three_walls_two_attackers(&mut game);

    cast_spell(&mut game, PlayerId(1), "Glyph of Delusion", 0);
    choose_targets(&mut game, PlayerId(1), &[giant]);
    choose_targets(&mut game, PlayerId(1), &[stone]);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.counters_of_kind(giant, CounterKind::Glyph),
        3,
        "three power, three glyph counters",
    );
    assert_eq!(
        game.counters_of_kind(stone, CounterKind::Glyph),
        0,
        "the named Wall is a restriction, not a recipient",
    );
}

// ── increment 106: sacrificing filtered permanents as an alternative cost ──────────────

/// The permanents a pending optional sacrifice is offering.
fn sacrifice_options(game: &Game) -> Vec<ObjectId> {
    match game.pending_choice() {
        Some(PendingChoice::MaySacrifice { options, .. }) => options,
        other => panic!("expected an optional sacrifice, got {other:?}"),
    }
}

fn choose_sacrifices(
    game: &mut Game,
    player: PlayerId,
    sacrifices: &[ObjectId],
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ChooseSacrifices {
        player,
        sacrifices: sacrifices.to_vec(),
    })
}

/// Cast Mold Demon over `swamps` Swamps and resolve both the creature spell and its enters
/// trigger, stopping on the trigger's choice (or on whatever it did instead).
fn mold_demon_on_swamps(game: &mut Game, swamps: usize) -> (ObjectId, Vec<ObjectId>) {
    stock_libraries(game);
    let swamps: Vec<ObjectId> = (0..swamps)
        .map(|_| game.spawn_on_battlefield(PlayerId(0), card("Swamp")))
        .collect();
    let spell = cast_spell(game, PlayerId(0), "Mold Demon", 0);
    resolve_top_of_stack(game);
    let demon = game.current_id(spell);
    resolve_top_of_stack(game);
    (demon, swamps)
}

#[test]
fn mold_demon_stays_when_two_swamps_are_given_up() {
    // "When this creature enters, sacrifice it unless you sacrifice two Swamps."
    let mut game = Game::new();
    let (demon, swamps) = mold_demon_on_swamps(&mut game, 3);

    let options = sacrifice_options(&game);
    assert_eq!(options.len(), 3, "every Swamp is payable: {options:?}");
    choose_sacrifices(&mut game, PlayerId(0), &swamps[..2]).expect("two Swamps is the price");

    assert_eq!(
        game.zone_of(demon),
        Zone::Battlefield,
        "the price was paid, the Demon stays",
    );
    assert_eq!(game.zone_of(swamps[0]), Zone::Graveyard, "first Swamp paid");
    assert_eq!(
        game.zone_of(swamps[1]),
        Zone::Graveyard,
        "second Swamp paid"
    );
    assert_eq!(
        game.zone_of(swamps[2]),
        Zone::Battlefield,
        "and no more than two",
    );
}

#[test]
fn mold_demon_eats_itself_when_the_swamps_are_declined() {
    let mut game = Game::new();
    let (demon, swamps) = mold_demon_on_swamps(&mut game, 3);

    choose_sacrifices(&mut game, PlayerId(0), &[]).expect("declining is always legal");

    assert_eq!(game.zone_of(demon), Zone::Graveyard, "unless you sacrifice");
    for &swamp in &swamps {
        assert_eq!(game.zone_of(swamp), Zone::Battlefield, "no Swamp was paid");
    }
}

#[test]
fn mold_demon_refuses_a_single_swamp_as_payment() {
    // "two Swamps" is a price, not a maximum — one is not a legal answer.
    let mut game = Game::new();
    let (_, swamps) = mold_demon_on_swamps(&mut game, 3);

    let rejected = choose_sacrifices(&mut game, PlayerId(0), &swamps[..1]);

    assert!(rejected.is_err(), "one Swamp is short of the price");
}

#[test]
fn mold_demon_with_one_swamp_is_sacrificed_without_a_prompt() {
    // Nothing to offer: one Swamp can never make two, so the trigger takes the penalty outright
    // rather than pausing on a choice its controller cannot answer.
    let mut game = Game::new();
    let (demon, swamps) = mold_demon_on_swamps(&mut game, 1);

    assert!(
        game.pending_choice().is_none(),
        "no prompt: {:?}",
        game.pending_choice(),
    );
    assert_eq!(game.zone_of(demon), Zone::Graveyard, "the Demon is gone");
    assert_eq!(
        game.zone_of(swamps[0]),
        Zone::Battlefield,
        "and the one Swamp was never taken",
    );
}

/// Roll forward until `predicate`, tossing whatever each cleanup step asks a full hand to
/// discard — two whole turns is long enough for the hand-size limit to interrupt.
fn advance_past_cleanups(game: &mut Game, predicate: impl Fn(&Game) -> bool + Copy) {
    while !predicate(game) {
        if let Some(PendingChoice::DiscardToHandSize {
            player,
            hand,
            count,
        }) = game.pending_choice()
        {
            let cards = hand[..count].to_vec();
            game.submit(Intent::Discard { player, cards }).unwrap();
            continue;
        }
        advance_until(game, |g| {
            predicate(g)
                || matches!(
                    g.pending_choice(),
                    Some(PendingChoice::DiscardToHandSize { .. })
                )
        });
    }
}

/// Roll to player 0's next upkeep with Elder Spawn (and `islands` Islands) on their battlefield.
fn elder_spawn_at_upkeep(game: &mut Game, islands: usize) -> (ObjectId, Vec<ObjectId>) {
    stock_libraries(game);
    let spawn = game.spawn_on_battlefield(PlayerId(0), card("Elder Spawn"));
    let islands: Vec<ObjectId> = (0..islands)
        .map(|_| game.spawn_on_battlefield(PlayerId(0), card("Island")))
        .collect();
    // Stop at the upkeep prompt, or — when there is nothing to offer and the trigger takes the
    // penalty outright — at the draw step just past that upkeep. Never open-ended: with no stop
    // condition the loop runs until player 0 decks out, and a lost player's objects are tombstoned
    // (CR 800.4a), which makes `zone_of` panic.
    advance_past_cleanups(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::MaySacrifice { .. }))
            || (g.active_player() == PlayerId(0) && g.current_step() == Step::Draw)
    });
    (spawn, islands)
}

#[test]
fn elder_spawn_survives_its_upkeep_on_one_island() {
    // "At the beginning of your upkeep, unless you sacrifice an Island, sacrifice this creature
    // and it deals 6 damage to you."
    let mut game = Game::new();
    let (spawn, islands) = elder_spawn_at_upkeep(&mut game, 2);
    let life = game.life(PlayerId(0));

    let options = sacrifice_options(&game);
    assert_eq!(options.len(), 2, "either Island pays: {options:?}");
    choose_sacrifices(&mut game, PlayerId(0), &islands[..1]).expect("one Island is the price");

    assert_eq!(game.zone_of(spawn), Zone::Battlefield, "it was fed");
    assert_eq!(game.zone_of(islands[0]), Zone::Graveyard, "the Island paid");
    assert_eq!(game.life(PlayerId(0)), life, "and nobody took damage");
}

#[test]
fn elder_spawn_that_is_not_fed_dies_and_burns_its_controller_for_six() {
    let mut game = Game::new();
    let (spawn, islands) = elder_spawn_at_upkeep(&mut game, 1);
    let life = game.life(PlayerId(0));

    choose_sacrifices(&mut game, PlayerId(0), &[]).expect("declining is always legal");

    assert_eq!(
        game.zone_of(spawn),
        Zone::Graveyard,
        "sacrifice this creature"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        life - 6,
        "and it deals 6 damage to you"
    );
    assert_eq!(
        game.zone_of(islands[0]),
        Zone::Battlefield,
        "the Island was never given up",
    );
}

#[test]
fn elder_spawn_with_no_island_dies_without_a_prompt() {
    let mut game = Game::new();
    let (spawn, _) = elder_spawn_at_upkeep(&mut game, 0);

    assert!(game.pending_choice().is_none(), "nothing to offer");
    assert_eq!(game.zone_of(spawn), Zone::Graveyard, "so it starves");
}
