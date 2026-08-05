//! Legends (`leg`) grind, wave 12 slice D — granted abilities and control duration.
//!
//! Increments 28 (`becomes-color-of-your-choice`, Dream Coat),
//! 32 (`granted-land-ability-with-conditional-counter`, Equinox),
//! 81 (`granted-upkeep-tax-to-all-creatures`, The Tabernacle at Pendrell Vale) and
//! 104 (`blocking-this-creature-and-indefinite-gain-control`, The Wretched).

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

/// Cast `name` from `player`'s hand at `target` and let it resolve onto the battlefield.
fn cast_permanent(
    game: &mut Game,
    player: PlayerId,
    name: &str,
    target: Option<Target>,
) -> ObjectId {
    let card_id = game.spawn_in_hand(player, card(name));
    cast(game, player, card_id, target).unwrap_or_else(|e| panic!("{name} is castable: {e:?}"));
    resolve_top_of_stack(game);
    game.live_object_ids()
        .into_iter()
        .find(|&id| game.zone_of(id) == Zone::Battlefield && game.def_of(id).name == name)
        .unwrap_or_else(|| panic!("{name} resolved onto the battlefield"))
}

fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// Roll forward to `player`'s next upkeep, stopping the instant anything pauses on a choice.
/// The spell on top of the stack — casting mints a fresh stack object, so a hand card's id is not
/// the id a counterspell targets.
fn top_spell(game: &Game) -> ObjectId {
    match game.stack().last().expect("a spell is on the stack") {
        StackEntry::Spell(id) => *id,
        other => panic!("expected a spell on top of the stack, got {other:?}"),
    }
}

fn to_upkeep_of(game: &mut Game, player: PlayerId) {
    advance_until(game, |g| {
        g.pending_choice().is_some()
            || (g.active_player() == player && g.current_step() == Step::Upkeep)
    });
}

// ── increment 28: Dream Coat ───────────────────────────────────────────────────────────

/// "{0}: Enchanted creature becomes the color or colors of your choice." The colour lands on the
/// *host*, not on the Aura, and it prints no duration (CR 400.7) so it outlives the turn.
#[test]
fn dream_coat_recolors_the_enchanted_creature_indefinitely() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    assert!(
        game.colors_of(bear)[Color::Green.index()],
        "Grizzly Bears starts green",
    );

    let coat = cast_permanent(
        &mut game,
        PlayerId(0),
        "Dream Coat",
        Some(Target::Object(bear)),
    );
    activate(&mut game, PlayerId(0), coat, 0, None).expect("{0} costs nothing");
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
    .expect("blue is one of the five colours");

    assert!(
        game.colors_of(bear)[Color::Blue.index()],
        "the bear is blue"
    );
    assert!(
        !game.colors_of(bear)[Color::Green.index()],
        "a layer-5 SET replaces the printed colour rather than adding to it",
    );

    pass_until_next_turn(&mut game);
    assert!(
        game.colors_of(bear)[Color::Blue.index()],
        "no printed duration, so cleanup never takes it back",
    );
}

/// "Activate only once each turn." — the second activation in the same turn is rejected.
#[test]
fn dream_coat_activates_only_once_each_turn() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let coat = cast_permanent(
        &mut game,
        PlayerId(0),
        "Dream Coat",
        Some(Target::Object(bear)),
    );

    activate(&mut game, PlayerId(0), coat, 0, None).expect("the first activation this turn");
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseColor {
        player: PlayerId(0),
        color: Color::Blue,
    })
    .unwrap();

    assert!(
        activate(&mut game, PlayerId(0), coat, 0, None).is_err(),
        "\"Activate only once each turn\"",
    );
}

// ── increment 32: Equinox ──────────────────────────────────────────────────────────────

/// A land P0 controls, enchanted by Equinox: "Enchanted land has '{T}: Counter target spell if it
/// would destroy a land you control.'" The granted ability is activated off the *land*, past its
/// own abilities (index 1 — a Plains has exactly one mana ability).
fn equinox_setup(game: &mut Game) -> ObjectId {
    stock_libraries(game);
    let plains = game.spawn_on_battlefield(PlayerId(0), card("Plains"));
    cast_permanent(game, PlayerId(0), "Equinox", Some(Target::Object(plains)));
    plains
}

#[test]
fn equinox_counters_a_spell_that_would_destroy_a_land_you_control() {
    let mut game = Game::new();
    let plains = equinox_setup(&mut game);
    // Stone Rain: "Destroy target land."
    let stone_rain = game.spawn_in_hand(PlayerId(1), card("Stone Rain"));
    // A sorcery, so it waits for its caster's own main phase.
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast(
        &mut game,
        PlayerId(1),
        stone_rain,
        Some(Target::Object(plains)),
    )
    .expect("Stone Rain targets the enchanted land");
    let stone_rain = top_spell(&game);

    let ability_index = game.def_of(plains).abilities.len();
    activate(
        &mut game,
        PlayerId(0),
        plains,
        ability_index,
        Some(Target::Object(stone_rain)),
    )
    .expect("the granted ability sees a land-destroying spell");
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(stone_rain),
        Zone::Graveyard,
        "Stone Rain was countered",
    );
    assert_eq!(
        game.zone_of(plains),
        Zone::Battlefield,
        "and the land it would have destroyed survives",
    );
}

#[test]
fn equinox_cannot_counter_a_spell_that_destroys_no_land_of_yours() {
    let mut game = Game::new();
    let plains = equinox_setup(&mut game);
    // "A land **you** control" is the counterer's land: Stone Rain aimed at the caster's own
    // Mountain would destroy a land, just not one of P0's.
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let stone_rain = game.spawn_in_hand(PlayerId(1), card("Stone Rain"));
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast(
        &mut game,
        PlayerId(1),
        stone_rain,
        Some(Target::Object(theirs)),
    )
    .expect("Stone Rain targets its caster's own land");
    let stone_rain = top_spell(&game);

    let ability_index = game.def_of(plains).abilities.len();
    assert!(
        activate(
            &mut game,
            PlayerId(0),
            plains,
            ability_index,
            Some(Target::Object(stone_rain)),
        )
        .is_err(),
        "no land P0 controls would be destroyed, so the spell is not a legal target",
    );
}

// ── increment 81: The Tabernacle at Pendrell Vale ──────────────────────────────────────

/// "All creatures have 'At the beginning of your upkeep, destroy this creature unless you pay
/// {1}.'" — "your" is each affected creature's *own* controller, and "this creature" is the
/// grantee, not the Tabernacle.
#[test]
fn tabernacle_taxes_every_creature_at_its_own_controllers_upkeep() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("The Tabernacle at Pendrell Vale"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    // P1's upkeep comes first: only *their* creature is taxed, and the tax is offered to them.
    to_upkeep_of(&mut game, PlayerId(1));
    advance_until(&mut game, |g| g.pending_choice().is_some());
    let Some(PendingChoice::PayOrElse { player, .. }) = game.pending_choice() else {
        panic!(
            "P1's upkeep offers the tax, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(player, PlayerId(1), "\"you\" is the creature's controller");
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(1),
        pay: false,
        discard_cost: Vec::new(),
    })
    .expect("declining is legal");

    assert_eq!(
        game.zone_of(theirs),
        Zone::Graveyard,
        "\"this creature\" is the grantee — P1's bear died",
    );
    assert_eq!(
        game.zone_of(mine),
        Zone::Battlefield,
        "P0's creature is untouched at P1's upkeep",
    );
}

#[test]
fn tabernacle_paying_the_tax_keeps_the_creature() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("The Tabernacle at Pendrell Vale"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Mountain"));

    to_upkeep_of(&mut game, PlayerId(1));
    advance_until(&mut game, |g| g.pending_choice().is_some());
    game.fund_mana(PlayerId(1));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(1),
        pay: true,
        discard_cost: Vec::new(),
    })
    .expect("{1} is payable");

    assert_eq!(
        game.zone_of(theirs),
        Zone::Battlefield,
        "the tax was paid, so nothing is destroyed",
    );
}

// ── increment 104: The Wretched ────────────────────────────────────────────────────────

/// "At end of combat, gain control of all creatures blocking this creature for as long as you
/// control this creature."
fn wretched_combat(game: &mut Game) -> (ObjectId, ObjectId, ObjectId) {
    stock_libraries(game);
    let wretched = game.spawn_on_battlefield(PlayerId(0), card("The Wretched"));
    // A 3/3 blocker: it has to survive the 2/5's combat damage to still be there at end of combat.
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    // A creature blocking nobody: only the creatures blocking *The Wretched* are stolen.
    let bystander = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_with(game, vec![wretched]);
    block_with(game, vec![(blocker, wretched)]).expect("a legal block");
    advance_until(game, |g| g.current_step() == Step::EndCombat);
    resolve_top_of_stack(game);
    (wretched, blocker, bystander)
}

#[test]
fn the_wretched_steals_only_its_own_blockers() {
    let mut game = Game::new();
    let (_wretched, blocker, bystander) = wretched_combat(&mut game);

    assert_eq!(
        game.controller_of(blocker),
        PlayerId(0),
        "the creature blocking The Wretched changed hands",
    );
    assert_eq!(
        game.controller_of(bystander),
        PlayerId(1),
        "a creature blocking nothing is not \"blocking this creature\"",
    );
}

#[test]
fn the_wretcheds_steal_ends_when_it_leaves_the_battlefield() {
    let mut game = Game::new();
    let (wretched, blocker, _) = wretched_combat(&mut game);
    assert_eq!(game.controller_of(blocker), PlayerId(0));

    // Kill The Wretched: the CR 611.2b condition ("for as long as you control this creature")
    // stops holding, so control reverts as a state-based check — no trigger involved.
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    cast(&mut game, PlayerId(1), bolt, Some(Target::Object(wretched))).expect("castable");
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(wretched),
        Zone::Graveyard,
        "3 combat damage plus 3 kills the 2/5",
    );

    assert_eq!(
        game.controller_of(blocker),
        PlayerId(1),
        "the steal ended with the source (CR 611.2b)",
    );
}
