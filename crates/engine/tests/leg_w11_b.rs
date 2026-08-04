//! Legends (`leg`) grind, wave 11 slice B — combat restrictions, round 2.
//!
//! Increments 111 (`activate-only-before-the-combat-damage-step`),
//! 100 (`blocks-and-becomes-blocked-by-as-separate-triggers`),
//! 14 (`becomes-blocked-color-change`), 56 (`must-block-by-filter`),
//! 88 (`cant-be-targeted-by-wall-only-effects`) and 34 (`feint`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority across several turns can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..60 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Try `object`'s ability at index 0 with mana funded from thin air, returning the verdict.
fn try_activate(game: &mut Game, player: PlayerId, object: ObjectId) -> Result<Vec<Event>, Reject> {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

// ── increment 111: Angus Mackenzie ────────────────────────────────────────────────────

#[test]
fn angus_mackenzie_fogs_while_the_window_is_open() {
    // "{G}{W}{U}, {T}: Prevent all combat damage that would be dealt this turn. Activate only
    // before the combat damage step." Declare blockers is inside the window.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    let defender_before = game.life(PlayerId(1));
    attack_with(&mut game, vec![bear]);
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    try_activate(&mut game, PlayerId(0), angus).expect("declare blockers is before combat damage");
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        defender_before,
        "the fog prevented the bear's 2 damage",
    );
}

#[test]
fn an_unfogged_bear_does_reach_the_defending_player() {
    // The control for the test above: same board, same attack, Angus never activated. Without it
    // "life is unchanged" would also pass on an attack that never happened.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let _angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    let defender_before = game.life(PlayerId(1));
    attack_with(&mut game, vec![bear]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        defender_before - 2,
        "an unprevented 2/2 deals its 2",
    );
}

#[test]
fn angus_mackenzie_is_shut_out_from_the_combat_damage_step_on() {
    // The window is "before the combat damage step" — it closes at the first one, and stays shut
    // for the rest of the turn (the second main phase is still after it).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assert_eq!(
        try_activate(&mut game, PlayerId(0), angus),
        Err(Reject::WrongTiming),
        "the combat damage step itself is already too late",
    );

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    assert_eq!(
        try_activate(&mut game, PlayerId(0), angus),
        Err(Reject::WrongTiming),
        "and the window does not reopen later in the turn",
    );
}

#[test]
fn angus_mackenzie_is_open_before_combat_ever_starts() {
    // "Before the combat damage step" is the whole turn up to it, not just combat: upkeep counts.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let angus = game.spawn_on_battlefield(PlayerId(0), card("Angus Mackenzie"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert!(
        try_activate(&mut game, PlayerId(0), angus).is_ok(),
        "the precombat main phase is before the combat damage step",
    );
}

// ── increment 56: Marble Priest ───────────────────────────────────────────────────────

/// Player 0 attacks with `attacker`; player 1's board is `blockers`. Returns the block verdict.
fn block_verdict(
    attacker_name: &str,
    blockers: &[&str],
    choose: impl Fn(&[ObjectId]) -> Vec<usize>,
) -> Result<Vec<Event>, Reject> {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card(attacker_name));
    let board: Vec<ObjectId> = blockers
        .iter()
        .map(|n| game.spawn_on_battlefield(PlayerId(1), card(n)))
        .collect();
    attack_with(&mut game, vec![attacker]);
    let blocks = choose(&board)
        .into_iter()
        .map(|i| (board[i], attacker))
        .collect();
    block_with(&mut game, blocks)
}

#[test]
fn marble_priest_forces_every_able_wall_to_block_it() {
    // "All Walls able to block this creature do so." Declaring nothing is not an option while a
    // Wall could have blocked (CR 509.1c).
    assert_eq!(
        block_verdict("Marble Priest", &["Wall of Wood"], |_| vec![]),
        Err(Reject::IllegalDeclaration),
        "the Wall was able to block and did not",
    );
    assert!(
        block_verdict("Marble Priest", &["Wall of Wood"], |_| vec![0]).is_ok(),
        "the Wall blocking satisfies the requirement",
    );
}

#[test]
fn marble_priest_leaves_non_walls_free() {
    // The requirement names Walls only — a Grizzly Bears beside the Wall is under no compulsion,
    // and a board with no Wall at all is free to decline entirely.
    assert!(
        block_verdict(
            "Marble Priest",
            &["Wall of Wood", "Grizzly Bears"],
            |_| vec![0]
        )
        .is_ok(),
        "only the Wall was required to block",
    );
    assert!(
        block_verdict("Marble Priest", &["Grizzly Bears"], |_| vec![]).is_ok(),
        "no Wall on the board is no requirement at all",
    );
}

#[test]
fn a_wall_that_cannot_block_marble_priest_is_not_forced() {
    // "Able to block" — a tapped Wall can't block, so the declaration that leaves it out is legal.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let priest = game.spawn_on_battlefield(PlayerId(0), card("Marble Priest"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    game.tap(wall);
    attack_with(&mut game, vec![priest]);
    assert!(
        block_with(&mut game, vec![]).is_ok(),
        "a tapped Wall was never able to block",
    );
}

#[test]
fn marble_priest_forces_walls_only_against_itself() {
    // The requirement is printed on Marble Priest and reads "this creature" — a Wall is free to
    // decline a different attacker even while the Priest is on the battlefield beside it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let _priest = game.spawn_on_battlefield(PlayerId(0), card("Marble Priest"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let _wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    attack_with(&mut game, vec![bear]);
    assert!(
        block_with(&mut game, vec![]).is_ok(),
        "the bear is not the creature the requirement names",
    );
}

// ── increment 88: Wall of Shadows ─────────────────────────────────────────────────────

fn cast(game: &mut Game, object: ObjectId, target: Option<Target>) -> Result<Vec<Event>, Reject> {
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

#[test]
fn a_wall_only_aura_cant_target_wall_of_shadows() {
    // Animate Wall's "Enchant Wall" can target only Walls, so the shield turns it away.
    let mut game = Game::new();
    let shadows = game.spawn_on_battlefield(PlayerId(0), card("Wall of Shadows"));
    let animate = game.spawn_in_hand(PlayerId(0), card("Animate Wall"));

    assert_eq!(
        cast(&mut game, animate, Some(Target::Object(shadows))),
        Err(Reject::IllegalTarget),
        "an Aura that can enchant only Walls can target only Walls",
    );
    assert_eq!(game.zone_of(animate), Zone::Hand, "the Aura stayed in hand");
    assert!(
        !game
            .legal_targets(animate, None)
            .contains(&Target::Object(shadows)),
        "and it isn't offered as a choice either",
    );
}

#[test]
fn a_wall_only_aura_still_targets_an_ordinary_wall() {
    let mut game = Game::new();
    let wood = game.spawn_on_battlefield(PlayerId(0), card("Wall of Wood"));
    let animate = game.spawn_in_hand(PlayerId(0), card("Animate Wall"));

    assert!(
        game.legal_targets(animate, None)
            .contains(&Target::Object(wood)),
        "the shield is Wall of Shadows', not every Wall's",
    );
}

#[test]
fn an_any_creature_aura_still_enchants_wall_of_shadows() {
    // "Enchant creature" can target plenty besides Walls, so it isn't a Wall-only effect.
    let mut game = Game::new();
    let shadows = game.spawn_on_battlefield(PlayerId(0), card("Wall of Shadows"));
    let strength = game.spawn_in_hand(PlayerId(0), card("Unholy Strength"));

    assert!(
        cast(&mut game, strength, Some(Target::Object(shadows))).is_ok(),
        "only *Wall-only* spells are turned away",
    );
}

#[test]
fn a_wall_only_ability_cant_target_wall_of_shadows() {
    // "…or of abilities that can target only Walls" — Dwarven Demolition Team's "{T}: Destroy
    // target Wall."
    let mut game = Game::new();
    let shadows = game.spawn_on_battlefield(PlayerId(1), card("Wall of Shadows"));
    let wood = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    let team = game.spawn_on_battlefield(PlayerId(0), card("Dwarven Demolition Team"));

    let destroy = |game: &mut Game, target| {
        game.fund_mana(PlayerId(0));
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: team,
            ability_index: 0,
            target: Some(Target::Object(target)),
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
    };

    assert_eq!(
        destroy(&mut game, shadows),
        Err(Reject::IllegalTarget),
        "an ability that can destroy only Walls can't be aimed at this one",
    );
    assert!(
        destroy(&mut game, wood).is_ok(),
        "an ordinary Wall is still fair game",
    );
}

// ── increment 100: Infernal Medusa ────────────────────────────────────────────────────

#[test]
fn infernal_medusa_destroys_the_attacker_it_blocked() {
    // "Whenever this creature blocks a creature, destroy that creature at end of combat."
    let mut game = Game::new();
    let minotaur = game.spawn_on_battlefield(PlayerId(0), card("Hurloon Minotaur"));
    let medusa = game.spawn_on_battlefield(PlayerId(1), card("Infernal Medusa"));

    attack_with(&mut game, vec![minotaur]);
    block_with(&mut game, vec![(medusa, minotaur)]).expect("a 2/4 can block a 2/3");
    resolve_top_of_stack(&mut game); // the "blocks" trigger schedules the destroy

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(minotaur),
        Zone::Graveyard,
        "the attacker the Medusa blocked is destroyed at end of combat"
    );
}

#[test]
fn infernal_medusa_destroys_a_wall_it_blocked() {
    // The half that separates the two: "blocks a creature" has no Wall exception, only "becomes
    // blocked by a **non-Wall** creature" does. Animate Wall is what lets a Wall attack at all.
    let mut game = Game::new();
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Wood"));
    let animate = game.spawn_in_hand(PlayerId(0), card("Animate Wall"));
    cast(&mut game, animate, Some(Target::Object(wall))).expect("a legal Aura cast");
    resolve_top_of_stack(&mut game);
    let medusa = game.spawn_on_battlefield(PlayerId(1), card("Infernal Medusa"));

    attack_with(&mut game, vec![wall]);
    block_with(&mut game, vec![(medusa, wall)]).expect("a 2/4 can block a 0/3");
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(wall),
        Zone::Graveyard,
        "a Wall the Medusa blocks is destroyed all the same"
    );
}

#[test]
fn infernal_medusa_destroys_the_non_wall_that_blocked_it() {
    // "Whenever this creature becomes blocked by a non-Wall creature, destroy that creature at
    // end of combat."
    let mut game = Game::new();
    let medusa = game.spawn_on_battlefield(PlayerId(0), card("Infernal Medusa"));
    let minotaur = game.spawn_on_battlefield(PlayerId(1), card("Hurloon Minotaur"));

    attack_with(&mut game, vec![medusa]);
    block_with(&mut game, vec![(minotaur, medusa)]).expect("a 2/3 can block a 2/4");
    resolve_top_of_stack(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(minotaur),
        Zone::Graveyard,
        "the non-Wall blocker is destroyed at end of combat"
    );
}

#[test]
fn infernal_medusa_leaves_the_wall_that_blocked_it_alone() {
    let mut game = Game::new();
    let medusa = game.spawn_on_battlefield(PlayerId(0), card("Infernal Medusa"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));

    attack_with(&mut game, vec![medusa]);
    block_with(&mut game, vec![(wall, medusa)]).expect("a 0/3 Wall can block");

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(wall),
        Zone::Battlefield,
        "\"becomes blocked by a **non-Wall** creature\" spares the Wall"
    );
}

// ── increment 34: Feint ───────────────────────────────────────────────────────────────

#[test]
fn feint_taps_the_blockers_and_fogs_the_group() {
    // "Tap all creatures blocking target attacking creature. Prevent all combat damage that would
    // be dealt this turn by that creature and each creature blocking it."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let feint = game.spawn_in_hand(PlayerId(0), card("Feint"));

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).expect("a 2/2 can block a 2/2");
    cast(&mut game, feint, Some(Target::Object(attacker))).expect("a legal Feint");
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(blocker), "the blocker is tapped");

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(attacker),
        Zone::Battlefield,
        "the attacker took no damage from the creature blocking it"
    );
    assert_eq!(
        game.zone_of(blocker),
        Zone::Battlefield,
        "and dealt none back"
    );
}

#[test]
fn feint_leaves_a_different_attackers_combat_alone() {
    // "…blocking **target** attacking creature": the other pair still trades.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let feinted = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let other = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let other_blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let feint = game.spawn_in_hand(PlayerId(0), card("Feint"));

    attack_with(&mut game, vec![feinted, other]);
    block_with(&mut game, vec![(blocker, feinted), (other_blocker, other)])
        .expect("both blocks are legal");
    cast(&mut game, feint, Some(Target::Object(feinted))).expect("a legal Feint");
    resolve_top_of_stack(&mut game);

    assert!(
        game.is_tapped(blocker),
        "the targeted attacker's blocker taps"
    );
    assert!(
        !game.is_tapped(other_blocker),
        "the other attacker's blocker is untouched"
    );

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(feinted),
        Zone::Battlefield,
        "the feinted pair survives"
    );
    assert_eq!(game.zone_of(blocker), Zone::Battlefield, "both of it");
    assert_eq!(
        game.zone_of(other),
        Zone::Graveyard,
        "while the untouched pair still trades"
    );
    assert_eq!(game.zone_of(other_blocker), Zone::Graveyard, "both of it");
}

// ── increment 14: Aisling Leprechaun ──────────────────────────────────────────────────

#[test]
fn aisling_leprechaun_paints_the_attacker_it_blocked_green() {
    // "Whenever this creature blocks or becomes blocked by a creature, that creature becomes
    // green. (This effect lasts indefinitely.)"
    let mut game = Game::new();
    stock_libraries(&mut game);
    let minotaur = game.spawn_on_battlefield(PlayerId(0), card("Hurloon Minotaur"));
    let aisling = game.spawn_on_battlefield(PlayerId(1), card("Aisling Leprechaun"));

    attack_with(&mut game, vec![minotaur]);
    block_with(&mut game, vec![(aisling, minotaur)]).expect("a 1/1 can block");
    resolve_top_of_stack(&mut game);

    assert!(
        game.colors_of(minotaur)[Color::Green.index()],
        "the creature it blocked is green now"
    );
    assert!(
        !game.colors_of(minotaur)[Color::Red.index()],
        "and the set replaces its printed red, it doesn't union with it"
    );
}

#[test]
fn aisling_leprechauns_paint_job_outlives_the_turn() {
    // "(This effect lasts indefinitely.)" — no duration to sweep at cleanup.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let aisling = game.spawn_on_battlefield(PlayerId(0), card("Aisling Leprechaun"));
    let minotaur = game.spawn_on_battlefield(PlayerId(1), card("Hurloon Minotaur"));

    attack_with(&mut game, vec![aisling]);
    block_with(&mut game, vec![(minotaur, aisling)]).expect("a 2/3 can block a 1/1");
    resolve_top_of_stack(&mut game);
    assert!(game.colors_of(minotaur)[Color::Green.index()]);

    pass_until_next_turn(&mut game);

    assert!(
        game.colors_of(minotaur)[Color::Green.index()],
        "still green a turn later"
    );
}
