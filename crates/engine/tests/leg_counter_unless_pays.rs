//! Legends (`leg`) grind — increments 108 (`counter-the-triggering-spell`) and 48
//! (`counter-unless-pays-x`).

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

/// Decline the offered "unless that player pays" cost.
fn decline(game: &mut Game, player: PlayerId) {
    game.submit(Intent::PayOptionalCost {
        player,
        pay: false,
        discard_cost: vec![],
    })
    .unwrap_or_else(|e| panic!("declining the tax should be legal: {e:?}"));
}

/// Pay the offered "unless that player pays" cost.
fn pay(game: &mut Game, player: PlayerId) {
    game.submit(Intent::PayOptionalCost {
        player,
        pay: true,
        discard_cost: vec![],
    })
    .unwrap_or_else(|e| panic!("paying the tax should be legal: {e:?}"));
}

/// Assert the pause is a pay-or-counter offer aimed at `player`.
fn assert_taxed(game: &Game, player: PlayerId) {
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::PayOrCounter { player: p, .. }) if p == player
        ),
        "expected a pay-or-counter offer to {player:?}, got {:?}",
        game.pending_choice()
    );
}

// ── Presence of the Master (#108): "counter it" names the triggering spell, no target ─────────

#[test]
fn presence_of_the_master_counters_an_opponents_enchantment_spell() {
    // "Whenever a player casts an enchantment spell, counter it." The spell that fired the watch
    // is countered with no chosen target (CR 115.1: "it" never says "target").
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Presence of the Master"));

    let study = game.spawn_in_hand(PlayerId(0), card("Rhystic Study"));
    cast(&mut game, PlayerId(0), study, None);

    resolve_top_of_stack(&mut game); // Presence of the Master's trigger resolves: counter it.

    assert_eq!(
        game.zone_of(study),
        Zone::Graveyard,
        "the countered enchantment spell lands in its owner's graveyard instead of resolving"
    );
}

#[test]
fn presence_of_the_master_counters_its_own_controllers_enchantment_spell() {
    // "a player" is symmetrical — Presence of the Master counters its own controller's
    // enchantment spells too.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(0), card("Presence of the Master"));

    let study = game.spawn_in_hand(PlayerId(0), card("Rhystic Study"));
    cast(&mut game, PlayerId(0), study, None);

    resolve_top_of_stack(&mut game); // The trigger resolves: counter it.

    assert_eq!(
        game.zone_of(study),
        Zone::Graveyard,
        "its own controller's enchantment spell is countered just the same"
    );
}

#[test]
fn presence_of_the_master_ignores_a_creature_spell() {
    // The watch is filtered to enchantment spells — a creature spell resolves untouched.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Presence of the Master"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);

    resolve_top_of_stack(&mut game); // No trigger was queued; the bear itself resolves.

    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "a creature spell is not an enchantment spell — it resolves"
    );
}

// ── Nether Void (#48): a flat {3} tax on every spell, its own controller's included ────────────

/// Cast Grizzly Bears for `caster` into a Nether Void `voider` controls, resolving the trigger
/// down to its pay-or-counter pause. Returns the bear's object id.
fn cast_bear_into_nether_void(game: &mut Game, voider: PlayerId, caster: PlayerId) -> ObjectId {
    game.fund_mana(caster);
    game.spawn_on_battlefield(voider, card("Nether Void"));

    let bear = game.spawn_in_hand(caster, card("Grizzly Bears"));
    cast(game, caster, bear, None);
    resolve_top_of_stack(game); // Nether Void's trigger resolves: pauses on pay-or-counter.
    bear
}

#[test]
fn nether_void_declined_counters_the_triggering_spell() {
    // "Whenever a player casts a spell, counter it unless that player pays {3}." The caster
    // declines, so the spell is countered.
    let mut game = Game::new();
    let bear = cast_bear_into_nether_void(&mut game, PlayerId(1), PlayerId(0));

    assert_taxed(&game, PlayerId(0));
    decline(&mut game, PlayerId(0));

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "declining the {{3}} counters the spell — it never reached the battlefield"
    );
}

#[test]
fn nether_void_paid_lets_the_triggering_spell_resolve() {
    // Paying the flat {3} — not the spell's own mana value — saves it.
    let mut game = Game::new();
    let bear = cast_bear_into_nether_void(&mut game, PlayerId(1), PlayerId(0));

    let before = pool_total(&game, PlayerId(0));
    pay(&mut game, PlayerId(0));
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        before - 3,
        "exactly {{3}} was paid — a flat tax, not Grizzly Bears' mana value of 2"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Stack,
        "paying leaves the spell on the stack"
    );

    resolve_top_of_stack(&mut game); // The saved bear resolves normally.
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the paid-for creature spell resolves onto the battlefield"
    );
}

#[test]
fn nether_void_taxes_its_own_controllers_spells() {
    // "a player" includes Nether Void's own controller — the scope difference from Invoke
    // Prejudice's "an opponent".
    let mut game = Game::new();
    let bear = cast_bear_into_nether_void(&mut game, PlayerId(0), PlayerId(0));

    assert_taxed(&game, PlayerId(0));
    decline(&mut game, PlayerId(0));

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "its own controller's spell is countered on a decline just the same"
    );
}

// ── In the Eye of Chaos (#48): instants only, taxed by the spell's own mana value ──────────────

/// Cast Vision Skeins (mana value 2) for `caster` into an In the Eye of Chaos `voider` controls,
/// resolving the trigger down to its pay-or-counter pause. Returns the spell's object id.
fn cast_skeins_into_eye_of_chaos(game: &mut Game, voider: PlayerId, caster: PlayerId) -> ObjectId {
    game.fund_mana(caster);
    game.spawn_on_battlefield(voider, card("In the Eye of Chaos"));
    for seat in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(seat), &[card("Plains"), card("Plains")]);
    }

    let skeins = game.spawn_in_hand(caster, card("Vision Skeins"));
    cast(game, caster, skeins, None);
    resolve_top_of_stack(game); // The trigger resolves: pauses on pay-or-counter.
    skeins
}

#[test]
fn in_the_eye_of_chaos_declined_counters_the_instant() {
    // "Whenever a player casts an instant spell, counter it unless that player pays {X}, where X
    // is its mana value." Declining counters Vision Skeins, so nobody draws.
    let mut game = Game::new();
    let skeins = cast_skeins_into_eye_of_chaos(&mut game, PlayerId(1), PlayerId(0));

    assert_taxed(&game, PlayerId(0));
    let hand_before = game.hand(PlayerId(0)).len();
    decline(&mut game, PlayerId(0));

    assert_eq!(
        game.zone_of(skeins),
        Zone::Graveyard,
        "declining the tax counters the instant"
    );
    assert_eq!(
        game.hand(PlayerId(0)).len(),
        hand_before,
        "the countered \"each player draws two cards\" never happened"
    );
}

#[test]
fn in_the_eye_of_chaos_taxes_the_instants_own_mana_value() {
    // X is the *triggering spell's* mana value: Vision Skeins costs {1}{U}, so the tax is exactly
    // {2} — not a flat number.
    let mut game = Game::new();
    let skeins = cast_skeins_into_eye_of_chaos(&mut game, PlayerId(1), PlayerId(0));

    let before = pool_total(&game, PlayerId(0));
    pay(&mut game, PlayerId(0));
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        before - 2,
        "exactly {{2}} was paid — Vision Skeins' mana value"
    );

    resolve_top_of_stack(&mut game); // The saved Vision Skeins resolves.
    assert_eq!(
        game.zone_of(skeins),
        Zone::Graveyard,
        "the paid-for instant resolved and went to its owner's graveyard"
    );
    assert_eq!(
        game.hand(PlayerId(0)).len(),
        2,
        "\"each player draws two cards\" happened for the caster"
    );
}

#[test]
fn in_the_eye_of_chaos_ignores_a_creature_spell() {
    // Instants only — a creature spell fires no watch at all.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("In the Eye of Chaos"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);

    resolve_top_of_stack(&mut game); // No trigger was queued; the bear itself resolves.

    assert!(
        game.pending_choice().is_none(),
        "a creature spell raises no payment prompt"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the untaxed creature spell resolves"
    );
}

// ── Invoke Prejudice (#48): opponents only, gated on colors of creatures *you* control ────────

#[test]
fn invoke_prejudice_taxes_an_opponents_creature_spell_sharing_no_color() {
    // "Whenever an opponent casts a creature spell that doesn't share a color with a creature you
    // control, counter that spell unless that player pays {X}, where X is its mana value."
    // P1 controls Invoke Prejudice and a blue Azure Drake; P0 casts a green bear — no shared color.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));
    game.spawn_on_battlefield(PlayerId(1), card("Azure Drake"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);
    resolve_top_of_stack(&mut game); // The trigger resolves: pauses on pay-or-counter.

    assert_taxed(&game, PlayerId(0));
    decline(&mut game, PlayerId(0));

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "declining counters the off-color creature spell"
    );
}

#[test]
fn invoke_prejudice_paid_is_the_creature_spells_mana_value() {
    // Same board, but P0 pays: the tax is Grizzly Bears' mana value of {2}, and the bear resolves.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));
    game.spawn_on_battlefield(PlayerId(1), card("Azure Drake"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);
    resolve_top_of_stack(&mut game); // The trigger resolves: pauses on pay-or-counter.

    let before = pool_total(&game, PlayerId(0));
    pay(&mut game, PlayerId(0));
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        before - 2,
        "exactly {{2}} was paid — Grizzly Bears' mana value"
    );

    resolve_top_of_stack(&mut game); // The saved bear resolves.
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the paid-for creature spell resolves onto the battlefield"
    );
}

#[test]
fn invoke_prejudice_spares_a_creature_spell_sharing_a_color() {
    // The gate reads *your* creatures' colors: P1 controls a green bear, so P0's green bear shares
    // a color and the watch never fires.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));
    game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);

    resolve_top_of_stack(&mut game); // No trigger was queued; the bear itself resolves.

    assert!(
        game.pending_choice().is_none(),
        "a creature spell sharing a color with a creature you control is not taxed"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the shared-color creature spell resolves"
    );
}

#[test]
fn invoke_prejudice_shares_a_color_through_a_multicolored_creature_you_control() {
    // CR 105.1/202.2: a multicolored creature you control shares a color with any spell holding one
    // of its colors. Jerrard of the Closed Fist is red-green, so P0's green bear shares green with
    // it even though nothing on P1's board is mono-green.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));
    game.spawn_on_battlefield(PlayerId(1), card("Jerrard of the Closed Fist"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);

    resolve_top_of_stack(&mut game); // No trigger was queued; the bear itself resolves.

    assert!(
        game.pending_choice().is_none(),
        "green is one of the red-green creature's colors — the bear shares a color"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the shared-color creature spell resolves"
    );
}

#[test]
fn invoke_prejudice_ignores_its_own_controllers_creature_spells() {
    // "an opponent casts" — Invoke Prejudice never taxes its own controller, the scope difference
    // from Nether Void's "a player".
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(0), card("Invoke Prejudice"));
    game.spawn_on_battlefield(PlayerId(0), card("Azure Drake"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);

    resolve_top_of_stack(&mut game); // No trigger was queued; the bear itself resolves.

    assert!(
        game.pending_choice().is_none(),
        "your own off-color creature spell raises no payment prompt"
    );
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "your own creature spell resolves untaxed"
    );
}

#[test]
fn invoke_prejudice_taxes_when_you_control_no_creatures_at_all() {
    // An empty board shares no color with anything, so every opponent creature spell is taxed.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));

    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    cast(&mut game, PlayerId(0), bear, None);
    resolve_top_of_stack(&mut game); // The trigger resolves: pauses on pay-or-counter.

    assert_taxed(&game, PlayerId(0));
    decline(&mut game, PlayerId(0));

    assert_eq!(
        game.zone_of(bear),
        Zone::Graveyard,
        "with no creatures of your own, nothing shares a color — the spell is countered"
    );
}

#[test]
fn invoke_prejudice_ignores_a_noncreature_spell() {
    // Creature spells only — an off-color instant is untouched.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.spawn_on_battlefield(PlayerId(1), card("Invoke Prejudice"));

    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    cast(
        &mut game,
        PlayerId(0),
        bolt,
        Some(Target::Player(PlayerId(1))),
    );

    resolve_top_of_stack(&mut game); // No trigger was queued; the bolt itself resolves.

    assert!(
        game.pending_choice().is_none(),
        "a noncreature spell raises no payment prompt"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        17,
        "the untaxed bolt resolved and dealt its 3 damage"
    );
}
