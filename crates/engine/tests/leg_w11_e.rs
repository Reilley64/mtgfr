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
