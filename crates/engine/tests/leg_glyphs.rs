//! Legends (`leg`) grind — increment 43: glyph-cycle.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..60 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step, which is all a non-active seat needs to act at instant speed.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

fn cast(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
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
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    cast(game, player, object, target).unwrap();
    resolve_top_of_stack(game);
}

/// Whether `object` is still a battlefield permanent.
fn alive(game: &Game, object: ObjectId) -> bool {
    game.zone_of(object) == Zone::Battlefield
}

fn in_graveyard(game: &Game, object: ObjectId) -> bool {
    game.zone_of(object) == Zone::Graveyard
}

// ── Glyph of Destruction ──────────────────────────────────────────────────────────────
//
// "Target blocking Wall you control gets +10/+0 until end of combat. Prevent all damage that
// would be dealt to it this turn. Destroy it at the beginning of the next end step."

/// Set up player 0 swinging a Hill Giant into player 1's `wall_name`, with the Glyph in player 1's
/// hand and blockers already declared. Returns `(giant, wall, glyph)`.
fn blocked_giant_with_glyph(game: &mut Game, wall_name: &str) -> (ObjectId, ObjectId, ObjectId) {
    stock_libraries(game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card(wall_name));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Destruction"));
    attack_with(game, vec![giant]);
    block_with(game, vec![(wall, giant)]).unwrap();
    (giant, wall, glyph)
}

#[test]
fn glyph_of_destruction_pumped_wall_kills_the_attacker_it_blocks() {
    // "Target blocking Wall you control gets +10/+0 until end of combat." An 0/3 Wall deals a 3/3
    // attacker nothing; a 10/3 kills it outright.
    let mut game = Game::new();
    let (giant, wall, glyph) = blocked_giant_with_glyph(&mut game, "Wall of Wood");

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(
        in_graveyard(&game, giant),
        "the Wall now hits for 10, which buries a 3/3",
    );
}

#[test]
fn glyph_of_destruction_prevents_the_blocked_attackers_damage_to_the_wall() {
    // "Prevent all damage that would be dealt to it this turn." Without it, a 3/3 attacker kills
    // the 0/3 Wall in the same damage step the Wall kills the attacker.
    let mut game = Game::new();
    let (_, wall, glyph) = blocked_giant_with_glyph(&mut game, "Wall of Wood");

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(alive(&game, wall), "an 0/3 walks out of a 3/3's damage",);
}

#[test]
fn glyph_of_destruction_wall_dies_at_the_beginning_of_the_next_end_step() {
    // "Destroy it at the beginning of the next end step." Destruction is not damage, so the
    // turn-long prevention shield does nothing to stop it (CR 701.7, CR 615).
    let mut game = Game::new();
    let (_, wall, glyph) = blocked_giant_with_glyph(&mut game, "Wall of Wood");

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert!(alive(&game, wall), "it survives the combat it was cast in",);

    advance_until(&mut game, |g| g.current_step() == Step::Cleanup);
    assert!(
        in_graveyard(&game, wall),
        "and is destroyed at the next end step",
    );
}

#[test]
fn glyph_of_destruction_shields_the_wall_from_noncombat_damage_for_the_rest_of_the_turn() {
    // "Prevent all damage" is not "prevent all combat damage" — a postcombat ping is prevented too,
    // and the shield is never used up (CR 615.6).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Destruction"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    give_priority(&mut game, PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: pinger,
        ability_index: 0,
        target: Some(Target::Object(wall)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.marked_damage(wall),
        0,
        "the same shield still stands after eating a whole combat's worth",
    );
}

#[test]
fn glyph_of_destruction_cannot_target_a_wall_that_is_not_blocking() {
    // "Target **blocking** Wall you control" — present tense. A Wall sitting at home is not a legal
    // target (CR 115.4).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Destruction"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let rejected = cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));

    assert!(
        rejected.is_err(),
        "a Wall that blocked nothing this combat is no target",
    );
}

// ── Glyph of Life ─────────────────────────────────────────────────────────────────────
//
// "Choose target Wall creature. Whenever that creature is dealt damage by an attacking creature
// this turn, you gain that much life."

/// Player 0 swings a Hill Giant into player 1's Wall of Stone, with a Glyph of Life in `caster`'s
/// hand and blockers already declared. Returns `(wall, glyph)`.
fn blocked_wall_with_glyph_of_life(game: &mut Game, caster: PlayerId) -> (ObjectId, ObjectId) {
    stock_libraries(game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(caster, card("Glyph of Life"));
    attack_with(game, vec![giant]);
    block_with(game, vec![(wall, giant)]).unwrap();
    (wall, glyph)
}

#[test]
fn glyph_of_life_gains_you_life_when_an_attacker_damages_the_chosen_wall() {
    // "Whenever that creature is dealt damage by an attacking creature this turn, you gain that
    // much life." A 3/3 attacker rolling into the Wall is worth three life.
    let mut game = Game::new();
    let (wall, glyph) = blocked_wall_with_glyph_of_life(&mut game, PlayerId(1));
    let before = game.life(PlayerId(1));

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before + 3,
        "the Wall soaked three points and its controller banked them",
    );
}

#[test]
fn glyph_of_life_pays_its_own_caster_not_the_walls_controller() {
    // "Choose target **Wall creature**" — any Wall, no controller restriction — and "**you** gain
    // that much life". The attacking player can Glyph the Wall standing in their way and profit.
    let mut game = Game::new();
    let (wall, glyph) = blocked_wall_with_glyph_of_life(&mut game, PlayerId(0));
    let caster_before = game.life(PlayerId(0));
    let defender_before = game.life(PlayerId(1));

    cast_and_resolve(&mut game, PlayerId(0), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(0)),
        caster_before + 3,
        "the Glyph's controller gains the life",
    );
    assert_eq!(
        game.life(PlayerId(1)),
        defender_before,
        "the Wall's controller gains nothing",
    );
}

#[test]
fn glyph_of_life_ignores_damage_from_a_creature_that_is_not_attacking() {
    // "dealt damage by an **attacking** creature" — a pinger sitting at home is not one.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Life"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    let before = game.life(PlayerId(1));

    give_priority(&mut game, PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: pinger,
        ability_index: 0,
        target: Some(Target::Object(wall)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.marked_damage(wall),
        1,
        "the ping lands on the Wall as usual",
    );
    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "but a non-attacking source pays nothing",
    );
}

#[test]
fn glyph_of_life_stops_watching_after_the_turn_it_was_cast() {
    // "…this turn." The same Wall blocking the same attacker next turn is worth nothing.
    let mut game = Game::new();
    let (wall, glyph) = blocked_wall_with_glyph_of_life(&mut game, PlayerId(1));

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    let after_first_combat = game.life(PlayerId(1));

    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        after_first_combat,
        "the watch expired with the turn that armed it",
    );
}

#[test]
fn glyph_of_life_cannot_choose_a_creature_that_is_not_a_wall() {
    // "Choose target **Wall** creature."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Life"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let rejected = cast(&mut game, PlayerId(1), glyph, Some(Target::Object(bears)));

    assert!(rejected.is_err(), "a Bear is no Wall");
}

#[test]
fn glyph_of_destruction_cannot_target_an_opponents_blocking_wall() {
    // "Target blocking Wall **you control**."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Wood"));
    let glyph = game.spawn_in_hand(PlayerId(0), card("Glyph of Destruction"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    let rejected = cast(&mut game, PlayerId(0), glyph, Some(Target::Object(wall)));

    assert!(
        rejected.is_err(),
        "the attacking player cannot pump the Wall standing in front of them",
    );
}

// ── the blocked-this-turn ledger ──────────────────────────────────────────────────────
//
// Glyph of Doom, Glyph of Delusion and Glyph of Reincarnation all read "creatures that <this
// Wall> blocked **this turn**" — a memory that outlives the combat the block happened in, and
// outlives the blocker itself. CR 509.1h keeps an attacker *blocked* for the rest of combat;
// these three want the same fact for the rest of the **turn**, keyed the other way round.

/// Player 0 swings `attackers` into player 1's `wall_name`, every attacker blocked by that one
/// Wall, with `glyph` in player 1's hand. Returns `(wall, glyph)`.
fn wall_blocks_all(
    game: &mut Game,
    wall_name: &str,
    glyph_name: &str,
    attackers: &[ObjectId],
) -> (ObjectId, ObjectId) {
    stock_libraries(game);
    let wall = game.spawn_on_battlefield(PlayerId(1), card(wall_name));
    let glyph = game.spawn_in_hand(PlayerId(1), card(glyph_name));
    attack_with(game, attackers.to_vec());
    block_with(
        game,
        attackers.iter().map(|&a| (wall, a)).collect::<Vec<_>>(),
    )
    .unwrap();
    (wall, glyph)
}

// ── Glyph of Doom ─────────────────────────────────────────────────────────────────────
//
// "Choose target Wall creature. At this turn's next end of combat, destroy all creatures that
// were blocked by that creature this turn."

#[test]
fn glyph_of_doom_destroys_the_creature_its_wall_blocked_at_end_of_combat() {
    // The sweep waits for end of combat: the blocked attacker is still alive through the combat
    // damage step and dies only once the delayed ability fires (CR 603.7).
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (wall, glyph) = wall_blocks_all(&mut game, "Wall of Stone", "Glyph of Doom", &[giant]);

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assert!(alive(&game, giant), "nothing happens before end of combat");

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    assert!(
        in_graveyard(&game, giant),
        "end of combat buries what the Wall blocked",
    );
}

#[test]
fn glyph_of_doom_spares_a_creature_a_different_blocker_stopped() {
    // "creatures that were blocked by **that creature**" — one Wall's ledger, not the table's.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let doomed = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let spared = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let other = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Doom"));

    attack_with(&mut game, vec![doomed, spared]);
    block_with(&mut game, vec![(wall, doomed), (other, spared)]).unwrap();
    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert!(in_graveyard(&game, doomed), "the targeted Wall's own block");
    assert!(
        alive(&game, spared),
        "the other Wall's block is not its own"
    );
}

#[test]
fn glyph_of_doom_sweeps_a_block_declared_after_it_resolved() {
    // "At this turn's next end of combat, destroy all creatures that were blocked by that
    // creature this turn" — the sweep reads the ledger when it *fires*, so a block declared after
    // the Glyph resolved is still in it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Doom"));

    attack_with(&mut game, vec![giant]);
    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert!(
        in_graveyard(&game, giant),
        "a block declared after the Glyph resolved still counts",
    );
}

#[test]
fn glyph_of_doom_still_sweeps_after_its_wall_has_died() {
    // The ledger is a historic fact about the *turn*, not a live read of the battlefield: an 0/3
    // Wall that traded itself away still names what it blocked.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (wall, glyph) = wall_blocks_all(&mut game, "Wall of Wood", "Glyph of Doom", &[giant]);

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)));
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert!(in_graveyard(&game, wall), "the 0/3 died to the 3/3");
    assert!(
        in_graveyard(&game, giant),
        "and took the attacker with it at end of combat",
    );
}

#[test]
fn glyph_of_doom_cannot_choose_a_creature_that_is_not_a_wall() {
    // "Choose target **Wall** creature."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Doom"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let rejected = cast(&mut game, PlayerId(1), glyph, Some(Target::Object(bears)));

    assert!(rejected.is_err(), "a Bear is no Wall");
}

// ── Glyph of Delusion ─────────────────────────────────────────────────────────────────
//
// "Put X glyph counters on target creature that target Wall blocked this turn, where X is the
// power of that blocked creature. The creature gains 'This creature doesn't untap during your
// untap step if it has a glyph counter on it' and 'At the beginning of your upkeep, remove a
// glyph counter from this creature.'"

/// Roll forward to player 0's next Main1, answering nothing along the way.
fn to_active_players_next_main(game: &mut Game) {
    pass_until_next_turn(game);
    pass_until_next_turn(game);
    advance_until(game, |g| g.current_step() == Step::Main1);
}

#[test]
fn glyph_of_delusion_lands_counters_equal_to_the_blocked_creatures_power() {
    // "where X is the power of that blocked creature" — a Hill Giant is worth three.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (_, glyph) = wall_blocks_all(&mut game, "Wall of Stone", "Glyph of Delusion", &[giant]);

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(giant)));

    assert_eq!(
        game.counters_of_kind(giant, CounterKind::Glyph),
        3,
        "three power, three glyph counters",
    );
}

#[test]
fn glyph_of_delusion_holds_the_creature_down_for_one_untap_step_per_counter() {
    // The two granted abilities together: a glyph counter stops the untap, and one counter comes
    // off at each of its controller's upkeeps — so a 3/3 misses exactly three untap steps.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (_, glyph) = wall_blocks_all(&mut game, "Wall of Stone", "Glyph of Delusion", &[giant]);

    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(giant)));
    assert!(game.is_tapped(giant), "it is tapped from attacking");

    for expected in [2, 1, 0] {
        to_active_players_next_main(&mut game);
        assert!(
            game.is_tapped(giant),
            "a creature with a glyph counter does not untap",
        );
        assert_eq!(
            game.counters_of_kind(giant, CounterKind::Glyph),
            expected,
            "one counter comes off at each of its controller's upkeeps",
        );
    }

    to_active_players_next_main(&mut game);
    assert!(
        !game.is_tapped(giant),
        "the first untap step with no counter left frees it",
    );
}

#[test]
fn glyph_of_delusion_can_be_cast_after_the_combat_the_block_happened_in() {
    // "…that target Wall blocked **this turn**" — the ledger outlives the combat phase, so a
    // postcombat main-phase Glyph still finds the creature.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (_, glyph) = wall_blocks_all(&mut game, "Wall of Stone", "Glyph of Delusion", &[giant]);

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast_and_resolve(&mut game, PlayerId(1), glyph, Some(Target::Object(giant)));

    assert_eq!(
        game.counters_of_kind(giant, CounterKind::Glyph),
        3,
        "combat is over, the memory of the block is not",
    );
}

#[test]
fn glyph_of_delusion_cannot_choose_a_creature_no_wall_blocked_this_turn() {
    // The target restriction is the whole point — an unblocked attacker is no target.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Delusion"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![]).unwrap();
    let rejected = cast(&mut game, PlayerId(1), glyph, Some(Target::Object(giant)));

    assert!(rejected.is_err(), "nothing blocked it");
}

#[test]
fn glyph_of_delusion_stops_looking_once_the_turn_that_armed_it_is_over() {
    // The ledger is turn-scoped: last turn's block is no licence for this turn's Glyph.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (_, glyph) = wall_blocks_all(&mut game, "Wall of Stone", "Glyph of Delusion", &[giant]);

    pass_until_next_turn(&mut game);
    let rejected = cast(&mut game, PlayerId(1), glyph, Some(Target::Object(giant)));

    assert!(rejected.is_err(), "the ledger expired with its turn");
}

// ── Glyph of Reincarnation ────────────────────────────────────────────────────────────
//
// "Destroy all creatures that were blocked by target Wall this turn. They can't be regenerated.
// For each creature that died this way, put a creature card from the graveyard of the player who
// controlled that creature the last time it became blocked by that Wall onto the battlefield
// under its owner's control."

#[test]
fn glyph_of_reincarnation_cant_be_cast_until_combat_is_over() {
    // "Cast this spell only after combat" (CR 601.3e): closed from untap through end of combat,
    // open from the postcombat main phase on. The Wall it targets is on the battlefield the whole
    // time, so a reject here is the timing window and nothing else.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Reincarnation"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert!(
        cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).is_err(),
        "the precombat main phase is before combat, not after it",
    );

    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    assert!(
        cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).is_err(),
        "mid-combat is not after combat either",
    );

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall)))
        .expect("the postcombat main phase is after combat");
}

#[test]
fn glyph_of_reincarnation_destroys_what_its_wall_blocked_earlier_this_turn() {
    // Cast after combat, over a block that happened before it: the ledger is what makes the card
    // work at all.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (wall, glyph) = wall_blocks_all(
        &mut game,
        "Wall of Stone",
        "Glyph of Reincarnation",
        &[giant],
    );
    let corpse = game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).unwrap();
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseSacrifices {
        player: PlayerId(1),
        sacrifices: vec![corpse],
    })
    .expect("the Glyph's controller picks the creature card that comes back");

    assert!(in_graveyard(&game, giant), "the blocked attacker died");
    assert_eq!(
        game.zone_of(corpse),
        Zone::Battlefield,
        "and a creature card came back out of its controller's graveyard",
    );
    assert_eq!(
        game.controller_of(corpse),
        PlayerId(0),
        "under its owner's control, not the Glyph controller's",
    );
}

#[test]
fn glyph_of_reincarnation_reads_the_graveyard_of_the_blocked_creatures_controller() {
    // "…from the graveyard of the player who controlled that creature the last time it became
    // blocked by that Wall" — the attacker's controller, not the Glyph's.
    let mut game = Game::new();
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let (wall, glyph) = wall_blocks_all(
        &mut game,
        "Wall of Stone",
        "Glyph of Reincarnation",
        &[giant],
    );
    let theirs = game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));
    let mine = game.spawn_in_graveyard(PlayerId(1), card("Savannah Lions"));

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).unwrap();
    resolve_top_of_stack(&mut game);
    let rejected = game.submit(Intent::ChooseSacrifices {
        player: PlayerId(1),
        sacrifices: vec![mine],
    });

    assert!(
        rejected.is_err(),
        "the Glyph controller's own graveyard is not on offer",
    );
    game.submit(Intent::ChooseSacrifices {
        player: PlayerId(1),
        sacrifices: vec![theirs],
    })
    .unwrap();
    assert_eq!(game.zone_of(theirs), Zone::Battlefield);
}

#[test]
fn glyph_of_reincarnation_leaves_a_creature_the_wall_never_blocked_alone() {
    // "creatures that were blocked by target Wall this turn" — an unblocked attacker walks.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let blocked = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let through = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Reincarnation"));
    game.spawn_in_graveyard(PlayerId(0), card("Savannah Lions"));

    attack_with(&mut game, vec![blocked, through]);
    block_with(&mut game, vec![(wall, blocked)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).unwrap();
    resolve_top_of_stack(&mut game);

    assert!(in_graveyard(&game, blocked), "the Wall's own block dies");
    assert!(
        alive(&game, through),
        "the creature that got through does not"
    );
}

#[test]
fn glyph_of_reincarnation_does_nothing_when_the_wall_blocked_nothing() {
    // No death, no reanimation, and no pause to answer.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let glyph = game.spawn_in_hand(PlayerId(1), card("Glyph of Reincarnation"));
    game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    cast(&mut game, PlayerId(1), glyph, Some(Target::Object(wall))).unwrap();
    resolve_top_of_stack(&mut game);

    assert!(
        game.pending_choice().is_none(),
        "nothing died, so nobody is asked to reanimate anything",
    );
}
