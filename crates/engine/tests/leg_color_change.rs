//! Legends (`leg`) grind — increment 96: target-becomes-color.

mod common;

use common::*;
use engine::*;

/// Cast a one-mana instant from P0's hand, auto-funded, leaving it on the stack with its
/// multi-target choice pending.
fn cast_instant(game: &mut Game, name: &str) -> ObjectId {
    game.fund_mana(PlayerId(0));
    let spell = game.spawn_in_hand(PlayerId(0), card(name));
    game.submit(Intent::Cast {
        player: PlayerId(0),
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
    .unwrap_or_else(|e| panic!("{name} is castable: {e:?}"));
    spell
}

fn choose_targets(game: &mut Game, targets: &[ObjectId]) {
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: targets.iter().map(|&id| Target::Object(id)).collect(),
    })
    .expect("a legal set of targets");
}

fn activate(
    game: &mut Game,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object,
        ability_index: 0,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// "{T}: Destroy target blue creature" — a colour-reading probe that says whether the layer-5
/// SET actually took, rather than trusting a getter. A victim that isn't blue can't be named as
/// the target at all (CR 602.2b → CR 601.2c), so the probe reports that refusal instead of a zone.
fn spinal_villain_destroys(
    game: &mut Game,
    villain: ObjectId,
    victim: ObjectId,
) -> Result<Zone, Reject> {
    activate(game, villain, Some(Target::Object(victim)))?;
    resolve_top_of_stack(game);
    Ok(game.zone_of(victim))
}

/// "One or more target creatures become blue until end of turn" — CR 601.2c lets the caster
/// choose more than one, and every chosen creature really is blue afterwards (Spinal Villain's
/// "target blue creature" destroys each of them).
#[test]
fn sea_kings_blessing_recolors_every_chosen_target() {
    let mut game = Game::new();
    let first = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let villain_a = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    let villain_b = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));

    cast_instant(&mut game, "Sea Kings' Blessing");
    choose_targets(&mut game, &[first, second]);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        spinal_villain_destroys(&mut game, villain_a, first),
        Ok(Zone::Graveyard),
        "the first chosen creature is blue now",
    );
    assert_eq!(
        spinal_villain_destroys(&mut game, villain_b, second),
        Ok(Zone::Graveyard),
        "so is the second — the count is a real multi-target clause, not a single target",
    );
}

/// Give every seat something to draw so passing a turn does not deck anybody (CR 704.5b).
fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 10]);
    }
}

/// A green creature Spinal Villain could never touch is only destroyable while the colour wash
/// lasts: at the next cleanup the layer-5 SET is swept (CR 514.2) and the creature is green again.
#[test]
fn sylvan_paradise_wears_off_at_cleanup() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let drake = game.spawn_on_battlefield(PlayerId(1), card("Azure Drake"));
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    // A second, still-untapped Villain for after the turn rolls — the first one paid {T}.
    let later_villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));

    // Sylvan Paradise makes the blue Drake green, so Spinal Villain's "target blue creature"
    // no longer describes it and it can't be named as the target (CR 602.2b → CR 601.2c).
    cast_instant(&mut game, "Sylvan Paradise");
    choose_targets(&mut game, &[drake]);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        spinal_villain_destroys(&mut game, villain, drake),
        Err(Reject::IllegalTarget),
        "a green Drake is not a blue creature",
    );
    assert_eq!(
        game.zone_of(drake),
        Zone::Battlefield,
        "so the Drake is untouched",
    );

    pass_until_next_turn(&mut game);

    assert_eq!(
        spinal_villain_destroys(&mut game, later_villain, drake),
        Ok(Zone::Graveyard),
        "the colour wash ended at cleanup, so the Drake is blue again",
    );
}

/// CR 702.16c — a creature with protection from white can't be blocked by a white creature.
/// Heaven's Gate turns the would-be blocker white, which is what makes the block illegal.
#[test]
fn heavens_gate_makes_a_blocker_white_so_protection_from_white_stops_it() {
    let mut game = Game::new();
    let yeti = game.spawn_on_battlefield(PlayerId(0), card("Mountain Yeti"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    cast_instant(&mut game, "Heaven's Gate");
    choose_targets(&mut game, &[bears]);
    resolve_top_of_stack(&mut game);

    attack_with(&mut game, vec![yeti]);
    assert_eq!(
        block_with(&mut game, vec![(bears, yeti)]),
        Err(Reject::IllegalDeclaration),
        "a white creature can't block a creature with protection from white",
    );
}

/// "{2}, {T}: Target permanent you control becomes the color of your choice." The colour is
/// picked by the ability's controller on resolution (CR 609.3), through the shared colour picker.
#[test]
fn alchors_tomb_sets_the_color_its_controller_chooses() {
    let mut game = Game::new();
    let tomb = game.spawn_on_battlefield(PlayerId(0), card("Alchor's Tomb"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    game.fund_mana(PlayerId(0));

    activate(&mut game, tomb, Some(Target::Object(giant))).expect("a permanent P0 controls");
    resolve_top_of_stack(&mut game);
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::ChooseColor { .. })
        ),
        "resolution pauses on the colour picker",
    );
    game.submit(Intent::ChooseColor {
        player: PlayerId(0),
        color: Color::Blue,
    })
    .expect("blue is one of the five colors");

    assert_eq!(
        spinal_villain_destroys(&mut game, villain, giant),
        Ok(Zone::Graveyard),
        "the red Hill Giant became blue, so Spinal Villain can destroy it",
    );
}

/// "(This effect lasts indefinitely.)" — no printed duration, so cleanup must not take it back
/// (CR 400.7 / 514.2).
#[test]
fn alchors_tomb_color_change_survives_cleanup() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let tomb = game.spawn_on_battlefield(PlayerId(0), card("Alchor's Tomb"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    game.fund_mana(PlayerId(0));

    activate(&mut game, tomb, Some(Target::Object(giant))).expect("a permanent P0 controls");
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseColor {
        player: PlayerId(0),
        color: Color::Blue,
    })
    .expect("blue is one of the five colors");

    pass_until_next_turn(&mut game);

    assert_eq!(
        spinal_villain_destroys(&mut game, villain, giant),
        Ok(Zone::Graveyard),
        "the Giant is still blue a cleanup later",
    );
}

/// "Target permanent **you control**" — an opponent's permanent never becomes the chosen color:
/// it is not a legal target, so it can't be named as the ability is announced
/// (CR 602.2b → CR 601.2c) and the controller is never asked to pick a color.
#[test]
fn alchors_tomb_cannot_recolor_a_permanent_an_opponent_controls() {
    let mut game = Game::new();
    let tomb = game.spawn_on_battlefield(PlayerId(0), card("Alchor's Tomb"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let villain = game.spawn_on_battlefield(PlayerId(0), card("Spinal Villain"));
    game.fund_mana(PlayerId(0));

    assert_eq!(
        activate(&mut game, tomb, Some(Target::Object(theirs))),
        Err(Reject::IllegalTarget),
        "\"target permanent you control\" — an opponent's Hill Giant is not one",
    );
    assert!(
        game.pending_choice().is_none(),
        "no color is picked for a permanent the Tomb's controller doesn't control",
    );
    assert_eq!(
        spinal_villain_destroys(&mut game, villain, theirs),
        Err(Reject::IllegalTarget),
        "the opponent's Hill Giant is still red, not any color the Tomb could have named",
    );
    assert_eq!(
        game.zone_of(theirs),
        Zone::Battlefield,
        "so Spinal Villain's \"target blue creature\" can't reach it",
    );
}
