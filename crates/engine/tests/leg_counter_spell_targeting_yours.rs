//! Legends (`leg`) grind — increment 18: counter-spell-targeting-your-permanent.

mod common;

use common::*;
use engine::*;

/// Cast `object` for `player`, targeting `target` — the common no-frills `Intent::Cast` shape
/// every test below needs, none of the modal/kicker/strive fields.
fn cast(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));
}

fn top_spell(game: &Game) -> ObjectId {
    game.stack()
        .iter()
        .rev()
        .find_map(|entry| match *entry {
            StackEntry::Spell(id) => Some(id),
            StackEntry::Ability { .. } => None,
        })
        .expect("a spell is on the stack")
}

#[test]
fn avoid_fate_counters_instant_targeting_a_permanent_the_counterer_controls() {
    // Avoid Fate: "Counter target instant or Aura spell that targets a permanent you control."
    // Giant Growth ("target creature gets +3/+3") is cast by P0 aimed at P1's own bear — a
    // permanent Avoid Fate's controller (P1) controls, so P1's Avoid Fate is a legal counter.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));

    let their_bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(0), card("Giant Growth"));
    let avoid_fate = game.spawn_in_hand(PlayerId(1), card("Avoid Fate"));

    cast(
        &mut game,
        PlayerId(0),
        growth,
        Some(Target::Object(their_bear)),
    );
    let growth_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();

    cast(
        &mut game,
        PlayerId(1),
        avoid_fate,
        Some(Target::Object(growth_on_stack)),
    );
    resolve_top_of_stack(&mut game); // Avoid Fate resolves, countering Giant Growth.

    assert_eq!(
        game.zone_of(growth_on_stack),
        Zone::Graveyard,
        "the countered instant lands in its owner's graveyard"
    );
    assert_eq!(
        game.power(their_bear),
        2,
        "Giant Growth's +3/+3 never applied — the counter beat it to the stack"
    );
}

#[test]
fn avoid_fate_cannot_counter_a_spell_targeting_the_casters_own_permanent() {
    // The asymmetry the shape invites: "a permanent you control" reads from Avoid Fate's own
    // controller's perspective, not the target spell's. Giant Growth cast by P0 targeting P0's
    // *own* bear does not target anything P1 controls, so P1's Avoid Fate has no legal target.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));

    let own_bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(0), card("Giant Growth"));
    let avoid_fate = game.spawn_in_hand(PlayerId(1), card("Avoid Fate"));

    cast(
        &mut game,
        PlayerId(0),
        growth,
        Some(Target::Object(own_bear)),
    );
    let growth_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();

    let legal = game.legal_targets(avoid_fate, None);
    assert!(
        !legal.contains(&Target::Object(growth_on_stack)),
        "Giant Growth targets P0's own permanent, not P1's (Avoid Fate's controller) — not a \
         legal target for P1's Avoid Fate"
    );

    assert_eq!(
        game.submit(Intent::Cast {
            player: PlayerId(1),
            object: avoid_fate,
            target: Some(Target::Object(growth_on_stack)),
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
        }),
        Err(Reject::IllegalTarget),
        "casting Avoid Fate at a spell that only targets the caster's own permanent is rejected"
    );
}

#[test]
fn avoid_fate_fizzles_on_resolution_when_target_permanent_changes_control() {
    // CR 608.2b: targeting legality is checked again on resolution. Giant Growth targets P1's
    // bear (legal for P1's Avoid Fate at cast time); in response, P0 casts Ray of Command to
    // steal control of that bear until end of turn. By the time Avoid Fate tries to resolve, its
    // target no longer targets a permanent P1 controls — Avoid Fate itself has no legal target
    // and fails to resolve (the counter never happens), so Giant Growth goes on to pump the bear.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));

    let their_bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(0), card("Giant Growth"));
    let avoid_fate = game.spawn_in_hand(PlayerId(1), card("Avoid Fate"));
    let steal = game.spawn_in_hand(PlayerId(0), card("Ray of Command"));

    cast(
        &mut game,
        PlayerId(0),
        growth,
        Some(Target::Object(their_bear)),
    );
    let growth_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();

    cast(
        &mut game,
        PlayerId(1),
        avoid_fate,
        Some(Target::Object(growth_on_stack)),
    );
    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .unwrap();

    cast(
        &mut game,
        PlayerId(0),
        steal,
        Some(Target::Object(their_bear)),
    );

    resolve_top_of_stack(&mut game); // Ray of Command resolves: P0 steals the bear.
    assert_eq!(
        game.controller_of(their_bear),
        PlayerId(0),
        "P0 now controls the bear Giant Growth is aimed at"
    );

    resolve_top_of_stack(&mut game); // Avoid Fate tries to resolve: no legal target, fizzles.
    resolve_top_of_stack(&mut game); // Giant Growth, never countered, resolves normally.

    assert_eq!(
        game.power(their_bear),
        5,
        "Giant Growth's +3/+3 landed — Avoid Fate's own target went illegal on resolution, so \
         the counter never fired"
    );
}
