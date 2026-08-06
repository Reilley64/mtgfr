//! Legends (`leg`) grind, wave 12 slice B — lands and mana.
//!
//! Increments 44 (`global-characteristic-rewrite` — Gravity Sphere, Living Plane),
//! 66 (`change-land-mana-production` — Quarum Trench Gnomes) and
//! 53 (`land-etb-sacrifice-replacement` — Land Equilibrium).

mod common;

use common::*;
use engine::*;

/// Pass priority once so the state-based-action sweep runs (CR 704.3).
fn sweep(game: &mut Game) {
    game.submit(Intent::PassPriority {
        player: game.priority_holder(),
    })
    .unwrap();
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

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .unwrap();
    resolve_top_of_stack(game);
}

// ── increment 44: Living Plane ────────────────────────────────────────────────────────

#[test]
fn living_plane_animates_every_land_not_only_the_ones_with_a_basic_type() {
    // "All lands are 1/1 creatures that are still lands." Living Lands only catches Forests;
    // the Plane catches a nonbasic with no land subtype at all.
    let mut game = Game::new();
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    let temple = game.spawn_on_battlefield(PlayerId(1), card("Temple of the False God"));
    game.spawn_on_battlefield(PlayerId(0), card("Living Plane"));

    for (land, what) in [(forest, "a basic Forest"), (temple, "a typeless nonbasic")] {
        let types = game.effective_types(land);
        assert!(types.intersects(TypeSet::CREATURE), "{what} is a creature");
        assert!(types.intersects(TypeSet::LAND), "{what} is still a land");
        assert_eq!((game.power(land), game.toughness(land)), (1, 1), "{what}");
    }
}

#[test]
fn living_plane_leaves_lands_alone_while_it_is_not_on_the_battlefield() {
    // The control: without the Plane a Forest is no creature.
    let mut game = Game::new();
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));

    assert!(
        !game.effective_types(forest).intersects(TypeSet::CREATURE),
        "an unanimated Forest is not a creature",
    );
}

#[test]
fn living_plane_leaves_nonland_permanents_alone() {
    // "All lands" — an enchantment on the same battlefield is untouched.
    let mut game = Game::new();
    let moon = game.spawn_on_battlefield(PlayerId(0), card("Bad Moon"));
    game.spawn_on_battlefield(PlayerId(0), card("Living Plane"));

    assert!(!game.effective_types(moon).intersects(TypeSet::CREATURE));
}

#[test]
fn an_animated_land_takes_counters_and_anthems_on_top_of_its_one_one_base() {
    // The 0/0-plus-counters interaction: the animation sets base P/T in layer 7b, so a +1/+1
    // counter and an anthem both sum on top of 1/1 rather than being wiped by it.
    let mut game = Game::new();
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    game.spawn_on_battlefield(PlayerId(0), card("Living Plane"));
    game.spawn_on_battlefield(PlayerId(0), card("Glorious Anthem"));
    game.add_plus_counter(forest);
    game.add_plus_counter(forest);

    assert_eq!(
        (game.power(forest), game.toughness(forest)),
        (4, 4),
        "1/1 base + two +1/+1 counters + a +1/+1 anthem",
    );
}

#[test]
fn an_animated_land_dies_to_lethal_damage() {
    // A 1/1 land is a creature for state-based actions too (CR 704.5g).
    let mut game = Game::new();
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    game.spawn_on_battlefield(PlayerId(0), card("Living Plane"));

    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(forest)));

    assert_eq!(
        game.zone_of(forest),
        Zone::Graveyard,
        "3 damage is lethal to a 1/1",
    );
}

#[test]
fn a_second_world_enchantment_sends_living_plane_to_the_graveyard_and_the_lands_back() {
    // CR 704.5k — the World rule. The animation is the Plane's, so it stops when the Plane does.
    let mut game = Game::new();
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    let plane = game.spawn_on_battlefield(PlayerId(0), card("Living Plane"));
    assert!(game.effective_types(forest).intersects(TypeSet::CREATURE));

    game.spawn_on_battlefield(PlayerId(1), card("Gravity Sphere"));
    sweep(&mut game);

    assert_eq!(
        game.zone_of(plane),
        Zone::Graveyard,
        "the older World enchantment is put into its owner's graveyard",
    );
    assert!(
        !game.effective_types(forest).intersects(TypeSet::CREATURE),
        "with the Plane gone the Forest is a land again",
    );
}

// ── increment 44: Gravity Sphere ──────────────────────────────────────────────────────

#[test]
fn gravity_sphere_grounds_every_creature_on_the_battlefield() {
    // "All creatures lose flying." Board-wide and both-sided, not "creatures you control".
    let mut game = Game::new();
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Serra Angel"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Serra Angel"));
    assert!(game.has_keyword(mine, Keyword::Flying), "printed flying");

    game.spawn_on_battlefield(PlayerId(1), card("Gravity Sphere"));

    assert!(!game.has_keyword(mine, Keyword::Flying), "grounded");
    assert!(!game.has_keyword(theirs, Keyword::Flying), "both sides");
}

#[test]
fn gravity_sphere_leaves_the_creatures_other_keywords_alone() {
    // Only flying goes: Serra Angel keeps vigilance.
    let mut game = Game::new();
    let angel = game.spawn_on_battlefield(PlayerId(0), card("Serra Angel"));
    game.spawn_on_battlefield(PlayerId(0), card("Gravity Sphere"));

    assert!(
        game.has_keyword(angel, Keyword::Vigilance),
        "vigilance stays"
    );
}

#[test]
fn a_grounded_creature_can_be_blocked_by_a_ground_creature() {
    // The point of the card: flying's evasion is gone, so a 2/2 Bears can block a Serra Angel.
    let mut game = Game::new();
    let angel = game.spawn_on_battlefield(PlayerId(0), card("Serra Angel"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(0), card("Gravity Sphere"));

    attack_with(&mut game, vec![angel]);
    block_with(&mut game, vec![(bears, angel)]).expect("a grounded Angel can be blocked");
}

#[test]
fn without_gravity_sphere_a_ground_creature_cannot_block_a_flier() {
    // The control for the test above.
    let mut game = Game::new();
    let angel = game.spawn_on_battlefield(PlayerId(0), card("Serra Angel"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![angel]);
    block_with(&mut game, vec![(bears, angel)]).expect_err("flying still evades");
}

// ── increment 66: Quarum Trench Gnomes ────────────────────────────────────────────────

/// The Gnomes plus a Plains for each seat. Returns `(game, gnomes, their_plains)`.
fn gnome_board() -> (Game, ObjectId, ObjectId) {
    let mut game = Game::new();
    let gnomes = game.spawn_on_battlefield(PlayerId(0), card("Quarum Trench Gnomes"));
    // `spawn_on_battlefield` puts a permanent down as if it had been there since before the turn,
    // so the Gnomes' `{T}` cost is payable.
    let plains = game.spawn_on_battlefield(PlayerId(1), card("Plains"));
    (game, gnomes, plains)
}

fn point_gnomes_at(
    game: &mut Game,
    gnomes: ObjectId,
    land: ObjectId,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: gnomes,
        ability_index: 0,
        target: Some(Target::Object(land)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

#[test]
fn a_gnomed_plains_taps_for_colorless_instead_of_white() {
    let (mut game, gnomes, plains) = gnome_board();
    point_gnomes_at(&mut game, gnomes, plains).expect("a Plains is a legal target");
    resolve_top_of_stack(&mut game);

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: plains,
    })
    .unwrap();

    assert_eq!(
        game.mana_in_pool(PlayerId(1), Color::White),
        0,
        "no white mana",
    );
    assert_eq!(game.colorless_in_pool(PlayerId(1)), 1, "{{C}} instead");
}

#[test]
fn an_untouched_plains_still_taps_for_white() {
    // The control: the rewrite is per-land, not per-controller.
    let (mut game, gnomes, plains) = gnome_board();
    let other = game.spawn_on_battlefield(PlayerId(1), card("Plains"));
    point_gnomes_at(&mut game, gnomes, plains).expect("a Plains is a legal target");
    resolve_top_of_stack(&mut game);

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: other,
    })
    .unwrap();

    assert_eq!(game.mana_in_pool(PlayerId(1), Color::White), 1);
}

#[test]
fn the_gnomes_cannot_point_at_a_land_that_is_not_a_plains() {
    // "target Plains" — a Mountain is not a legal target.
    let (mut game, gnomes, _) = gnome_board();
    let mountain = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));

    point_gnomes_at(&mut game, gnomes, mountain).expect_err("a Mountain is not a Plains");
}

#[test]
fn the_rewrite_outlives_the_gnomes() {
    // "(This effect lasts indefinitely.)" — nothing sweeps it, and it isn't tied to the source.
    let (mut game, gnomes, plains) = gnome_board();
    point_gnomes_at(&mut game, gnomes, plains).expect("a Plains is a legal target");
    resolve_top_of_stack(&mut game);

    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(gnomes)));
    assert_eq!(game.zone_of(gnomes), Zone::Graveyard);

    // The Bolt was funded, so read the *delta* the tap adds rather than the whole pool.
    let before = (
        game.colorless_in_pool(PlayerId(1)),
        game.mana_in_pool(PlayerId(1), Color::White),
    );
    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: plains,
    })
    .unwrap();
    assert_eq!(
        game.colorless_in_pool(PlayerId(1)),
        before.0 + 1,
        "still {{C}}"
    );
    assert_eq!(
        game.mana_in_pool(PlayerId(1), Color::White),
        before.1,
        "and no {{W}}"
    );
}

// ── increment 53: Land Equilibrium ────────────────────────────────────────────────────

#[test]
fn land_equilibrium_makes_a_land_flush_opponent_sacrifice_a_land() {
    // "If an opponent who controls at least as many lands as you do would put a land onto the
    // battlefield, that player instead puts that land onto the battlefield then sacrifices a land
    // of their choice."
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Land Equilibrium"));
    game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let played = game.spawn_in_hand(PlayerId(1), card("Forest"));

    // Reaching the opponent's turn means everyone draws on the way, so nobody may be decking.
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 40]);
    }
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });
    game.submit(Intent::PlayLand {
        player: PlayerId(1),
        object: played,
    })
    .unwrap();

    let Some(PendingChoice::SacrificeEdict { player, count, .. }) = game.pending_choice() else {
        panic!("the Equilibrium asks the opponent for a land");
    };
    assert_eq!(player, PlayerId(1), "the land's controller chooses");
    assert_eq!(count, 1, "a land, singular");
}

#[test]
fn land_equilibrium_leaves_an_opponent_who_is_behind_on_lands_alone() {
    // The control: "at least as many lands as you do" — an opponent still behind after the land
    // is compared as they stood *before* it entered, so they give up nothing.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Land Equilibrium"));
    for _ in 0..3 {
        game.spawn_on_battlefield(PlayerId(0), card("Island"));
    }
    let played = game.spawn_in_hand(PlayerId(1), card("Forest"));

    // Reaching the opponent's turn means everyone draws on the way, so nobody may be decking.
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 40]);
    }
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Main1
    });
    game.submit(Intent::PlayLand {
        player: PlayerId(1),
        object: played,
    })
    .unwrap();

    assert!(
        game.pending_choice().is_none(),
        "0 lands before the drop is fewer than 3",
    );
}

#[test]
fn land_equilibrium_does_not_tax_its_own_controller() {
    // "an opponent" — the Equilibrium's controller plays lands freely however far ahead they are.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Land Equilibrium"));
    game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let played = game.spawn_in_hand(PlayerId(0), card("Island"));

    game.submit(Intent::PlayLand {
        player: PlayerId(0),
        object: played,
    })
    .unwrap();

    assert!(game.pending_choice().is_none());
}
