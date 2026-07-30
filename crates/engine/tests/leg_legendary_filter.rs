//! Legends (`leg`) grind — increment 6: legendary-filter-axis.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so rolling to a later untap step can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

fn cast_and_resolve(game: &mut Game, object: ObjectId) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
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
    .unwrap();
    resolve_top_of_stack(game);
}

/// Activate the source's single `{T}` ability at `target`.
fn tap_ability_at(
    game: &mut Game,
    source: ObjectId,
    target: ObjectId,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: source,
        ability_index: 0,
        target: Some(Target::Object(target)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

// ── Karakas: "{T}: Return target legendary creature to its owner's hand." ─────────────

#[test]
fn karakas_returns_a_legendary_creature_to_its_owners_hand() {
    let mut game = Game::new();
    let karakas = game.spawn_on_battlefield(PlayerId(0), card("Karakas"));
    let legend = game.spawn_on_battlefield(PlayerId(1), card("Barktooth Warbeard"));

    tap_ability_at(&mut game, karakas, legend).expect("a legendary creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(legend),
        Zone::Hand,
        "the legend went to its owner's hand"
    );
    assert!(game.is_tapped(karakas), "the {{T}} cost tapped Karakas");
}

#[test]
fn karakas_cannot_target_a_nonlegendary_creature() {
    // A nonlegendary creature is not a legal target, and the targets are chosen as the ability is
    // announced (CR 602.2b → CR 601.2c), so the activation itself is refused.
    let mut game = Game::new();
    let karakas = game.spawn_on_battlefield(PlayerId(0), card("Karakas"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    assert_eq!(
        tap_ability_at(&mut game, karakas, bears),
        Err(Reject::IllegalTarget),
        "\"target legendary creature\" — Grizzly Bears is not one"
    );
    assert!(
        !game.is_tapped(karakas),
        "a refused activation never paid the {{T}} cost"
    );

    assert_eq!(
        game.zone_of(bears),
        Zone::Battlefield,
        "\"target legendary creature\" — a nonlegendary one stays put"
    );
}

// ── Arena of the Ancients: "Legendary creatures don't untap during their controllers' untap
// steps. / When this artifact enters, tap all legendary creatures." ───────────────────

#[test]
fn arena_of_the_ancients_taps_all_legendary_creatures_as_it_enters() {
    let mut game = Game::new();
    let arena = game.spawn_in_hand(PlayerId(0), card("Arena of the Ancients"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Jasmine Boreal"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    cast_and_resolve(&mut game, arena);
    resolve_top_of_stack(&mut game); // the enters trigger

    assert!(game.is_tapped(mine), "\"all legendary creatures\" — yours");
    assert!(
        game.is_tapped(theirs),
        "\"all legendary creatures\" — across the table too"
    );
    assert!(
        !game.is_tapped(bears),
        "a nonlegendary creature is left alone"
    );
}

#[test]
fn arena_of_the_ancients_holds_legendary_creatures_down_through_the_untap_step() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Arena of the Ancients"));
    let legend = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    attack_with(&mut game, vec![legend, bears]);
    assert!(
        game.is_tapped(legend) && game.is_tapped(bears),
        "both attacked"
    );

    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
    assert!(game.is_tapped(legend), "the legend is held down");
    assert!(!game.is_tapped(bears), "a nonlegendary creature untaps");
}

// ── Willow Satyr: "You may choose not to untap this creature during your untap step. / {T}: Gain
// control of target legendary creature for as long as you control this creature and this creature
// remains tapped." ───────────────────────────────────────────────────────────────────

#[test]
fn willow_satyr_gains_control_of_a_legendary_creature_while_it_stays_tapped() {
    let mut game = Game::new();
    let satyr = game.spawn_on_battlefield(PlayerId(0), card("Willow Satyr"));
    let legend = game.spawn_on_battlefield(PlayerId(1), card("Barktooth Warbeard"));

    tap_ability_at(&mut game, satyr, legend).expect("a legendary creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.controller_of(legend),
        PlayerId(0),
        "the legend changed hands"
    );
    assert!(game.is_tapped(satyr), "the {{T}} cost tapped the Satyr");
}

#[test]
fn willow_satyr_keeps_the_legend_only_while_it_stays_tapped() {
    // "for as long as you control this creature and this creature remains tapped" (CR 611.2b):
    // declining the optional untap holds the steal, letting it untap hands the legend back.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let satyr = game.spawn_on_battlefield(PlayerId(0), card("Willow Satyr"));
    let legend = game.spawn_on_battlefield(PlayerId(1), card("Barktooth Warbeard"));

    tap_ability_at(&mut game, satyr, legend).expect("a legendary creature is a legal target");
    resolve_top_of_stack(&mut game);

    // P0's next untap step: keep the Satyr tapped and the legend stays stolen.
    advance_until(&mut game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::DeclineUntap { .. }))
    });
    game.submit(Intent::DeclineUntap {
        player: PlayerId(0),
        keep_tapped: vec![satyr],
    })
    .expect("declining the untap is legal");
    assert!(game.is_tapped(satyr), "the Satyr stayed down by choice");
    assert_eq!(
        game.controller_of(legend),
        PlayerId(0),
        "still tapped, so the steal holds"
    );

    // The untap step after that: let it untap, and the duration ends.
    advance_until(&mut game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::DeclineUntap { .. }))
    });
    game.submit(Intent::DeclineUntap {
        player: PlayerId(0),
        keep_tapped: vec![],
    })
    .expect("untapping is the ordinary answer");
    assert!(!game.is_tapped(satyr), "the Satyr untapped");
    assert_eq!(
        game.controller_of(legend),
        PlayerId(1),
        "no longer tapped, so the legend goes home"
    );
}

#[test]
fn willow_satyr_cannot_target_a_nonlegendary_creature() {
    // Same refusal as Karakas above: the steal never gets announced for want of a legal target.
    let mut game = Game::new();
    let satyr = game.spawn_on_battlefield(PlayerId(0), card("Willow Satyr"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    assert_eq!(
        tap_ability_at(&mut game, satyr, bears),
        Err(Reject::IllegalTarget),
        "\"target legendary creature\" — Grizzly Bears is not one"
    );

    assert_eq!(
        game.controller_of(bears),
        PlayerId(1),
        "\"target legendary creature\" — a nonlegendary one does not change hands"
    );
}
