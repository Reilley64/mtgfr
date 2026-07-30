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
