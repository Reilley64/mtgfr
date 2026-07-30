//! Legends (`leg`) grind — increment 40: exchange-control.
//!
//! Gauntlets of Chaos and Juxtapose, the two Legends cards that swap control of permanents
//! between two seats (CR 720, CR 701.10). Both ride the exchange path Vedalken Plotter already
//! established — two freshly-timestamped `ControlGained` events (CR 800.4a) leaving ownership
//! alone (CR 108.3) — and each adds one wrinkle on top:
//!
//! * Gauntlets' second target's legality depends on its *first* target ("target permanent an
//!   opponent controls that **shares one of those types with it**", CR 601.2c), and its Aura
//!   destruction is gated on the exchange actually happening.
//! * Juxtapose chooses its two permanents by greatest mana value rather than targeting them, and
//!   runs the same choice twice — creatures, then artifacts, the second read taken *after* the
//!   first swap has already moved permanents across the table.

mod common;

use common::*;
use engine::*;

/// Pass until the stack is empty, so a spell plus the triggers it put on top all resolve.
fn resolve_stack(game: &mut Game) {
    let mut guard = 0;
    while !game.stack().is_empty() {
        game.submit(Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
        guard += 1;
        assert!(guard < 100, "the stack did not drain within a sane bound");
    }
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .expect("the spell is castable");
    resolve_stack(game);
}

/// Activate Gauntlets of Chaos ("{5}, Sacrifice this artifact") with `first` as its "you control"
/// clause. Leaves the ability on the stack with its second clause unanswered.
fn activate_gauntlets(
    game: &mut Game,
    gauntlets: ObjectId,
    first: ObjectId,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: gauntlets,
        ability_index: 0,
        target: Some(Target::Object(first)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// Activate Gauntlets of Chaos on `first` and answer its second target clause with `second`
/// (auto-filled when the opponent has only one legal permanent). The ability is left on the stack.
fn aim_gauntlets(game: &mut Game, gauntlets: ObjectId, first: ObjectId, second: ObjectId) {
    activate_gauntlets(game, gauntlets, first).expect("{5}, Sacrifice this artifact");
    if game.pending_choice().is_some() {
        game.submit(Intent::ChooseTargets {
            player: PlayerId(0),
            targets: vec![Target::Object(second)],
        })
        .expect("the second clause takes the opponent's permanent");
    }
}

/// The second target clause's legal set, once Gauntlets' first clause is settled on `first`.
fn second_clause_legal(game: &mut Game, gauntlets: ObjectId, first: ObjectId) -> Vec<Target> {
    activate_gauntlets(game, gauntlets, first).expect("{5}, Sacrifice this artifact");
    let Some(PendingChoice::ChooseTarget { legal, clause, .. }) = game.pending_choice() else {
        panic!(
            "Gauntlets pauses on its second target clause, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(clause, 1, "the pause is the second independent clause");
    legal
}

// ── Gauntlets of Chaos ────────────────────────────────────────────────────────────────
// "{5}, Sacrifice this artifact: Exchange control of target artifact, creature, or land you
// control and target permanent an opponent controls that shares one of those types with it. If
// those permanents are exchanged this way, destroy all Auras attached to them."

#[test]
fn gauntlets_of_chaos_second_target_must_share_a_type_with_the_first() {
    // "target permanent an opponent controls that shares one of those types with it" (CR 601.2c):
    // with a land chosen first, only the opponent's lands are legal second targets — their
    // creature and their artifact share no type with a land.
    let mut game = Game::new();
    let gauntlets = game.spawn_on_battlefield(PlayerId(0), card("Gauntlets of Chaos"));
    let my_island = game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let their_forest = game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let their_mountain = game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let their_bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let their_shield = game.spawn_on_battlefield(PlayerId(1), card("Kry Shield"));

    let legal = second_clause_legal(&mut game, gauntlets, my_island);

    assert!(
        legal.contains(&Target::Object(their_forest))
            && legal.contains(&Target::Object(their_mountain)),
        "both of the opponent's lands share the land type with the chosen Island"
    );
    assert!(
        !legal.contains(&Target::Object(their_bears)),
        "a creature shares none of the chosen Island's types"
    );
    assert!(
        !legal.contains(&Target::Object(their_shield)),
        "an artifact shares none of the chosen Island's types"
    );
}

#[test]
fn gauntlets_of_chaos_second_target_may_share_either_type_of_an_artifact_creature() {
    // "shares one of those types" — an artifact creature has two of the three named types, so
    // both the opponent's creature and their artifact are legal; their land still is not.
    let mut game = Game::new();
    let gauntlets = game.spawn_on_battlefield(PlayerId(0), card("Gauntlets of Chaos"));
    let my_golem = game.spawn_on_battlefield(PlayerId(0), card("Obsianus Golem"));
    let their_bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let their_shield = game.spawn_on_battlefield(PlayerId(1), card("Kry Shield"));
    let their_forest = game.spawn_on_battlefield(PlayerId(1), card("Forest"));

    let legal = second_clause_legal(&mut game, gauntlets, my_golem);

    assert!(
        legal.contains(&Target::Object(their_bears)),
        "the Golem is a creature, so a creature shares a type with it"
    );
    assert!(
        legal.contains(&Target::Object(their_shield)),
        "the Golem is also an artifact, so an artifact shares a type with it"
    );
    assert!(
        !legal.contains(&Target::Object(their_forest)),
        "an artifact creature is not a land"
    );
}

#[test]
fn gauntlets_of_chaos_exchanges_control_of_two_lands() {
    // The swap itself: each new controller can tap the land it gained and can no longer tap the
    // one it lost (CR 602.2), with ownership untouched (CR 108.3).
    let mut game = Game::new();
    let gauntlets = game.spawn_on_battlefield(PlayerId(0), card("Gauntlets of Chaos"));
    let my_island = game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let their_forest = game.spawn_on_battlefield(PlayerId(1), card("Forest"));

    aim_gauntlets(&mut game, gauntlets, my_island, their_forest);
    assert_eq!(
        game.zone_of(gauntlets),
        Zone::Graveyard,
        "\"Sacrifice this artifact\" is paid as a cost"
    );
    resolve_stack(&mut game);

    let green_before = game.mana_in_pool(PlayerId(0), Color::Green);
    game.submit(Intent::TapForMana {
        player: PlayerId(0),
        object: their_forest,
    })
    .expect("P0 taps the Forest it gained");
    assert_eq!(
        game.mana_in_pool(PlayerId(0), Color::Green),
        green_before + 1,
        "the exchanged Forest produces into its new controller's pool"
    );
    assert_eq!(
        game.submit(Intent::TapForMana {
            player: PlayerId(0),
            object: my_island,
        }),
        Err(Reject::CannotProduceMana),
        "P0 can't tap the Island it gave away (CR 602.2)"
    );
    assert_eq!(
        game.owner_of(their_forest),
        PlayerId(1),
        "ownership is untouched by the exchange (CR 108.3)"
    );
}

#[test]
fn gauntlets_of_chaos_destroys_the_auras_on_both_exchanged_permanents() {
    // "If those permanents are exchanged this way, destroy all Auras attached to them" — both
    // sides' Auras go, and an Aura on a bystander is left alone. Equipment is not an Aura, so the
    // clause reads Auras only.
    let mut game = Game::new();
    let gauntlets = game.spawn_on_battlefield(PlayerId(0), card("Gauntlets of Chaos"));
    let my_bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bystander = game.spawn_on_battlefield(PlayerId(0), card("Craw Wurm"));
    let their_minotaur = game.spawn_on_battlefield(PlayerId(1), card("Hurloon Minotaur"));
    let _their_ogre = game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));

    let on_mine = game.spawn_in_hand(PlayerId(0), card("Flight"));
    let on_theirs = game.spawn_in_hand(PlayerId(0), card("Giant Strength"));
    let elsewhere = game.spawn_in_hand(PlayerId(0), card("Flight"));
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        on_mine,
        Some(Target::Object(my_bears)),
    );
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        on_theirs,
        Some(Target::Object(their_minotaur)),
    );
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        elsewhere,
        Some(Target::Object(bystander)),
    );

    aim_gauntlets(&mut game, gauntlets, my_bears, their_minotaur);
    resolve_stack(&mut game);

    assert_eq!(
        game.controller_of(their_minotaur),
        PlayerId(0),
        "the exchange happened"
    );
    assert_eq!(
        game.zone_of(on_mine),
        Zone::Graveyard,
        "the Aura on the permanent you gave away is destroyed"
    );
    assert_eq!(
        game.zone_of(on_theirs),
        Zone::Graveyard,
        "the Aura on the permanent you gained is destroyed"
    );
    assert_eq!(
        game.zone_of(elsewhere),
        Zone::Battlefield,
        "an Aura on a permanent that wasn't exchanged survives"
    );
    assert_eq!(
        (game.power(bystander), game.toughness(bystander)),
        (6, 4),
        "the surviving Aura is still attached and still granting flying, not P/T"
    );
}

#[test]
fn gauntlets_of_chaos_leaves_auras_alone_when_the_exchange_does_not_happen() {
    // "If those permanents are exchanged this way" gates the Aura destruction: a second target
    // that has left the battlefield cancels the swap (CR 608.2b), so the Aura on the first target
    // survives and that permanent stays put.
    let mut game = Game::new();
    let gauntlets = game.spawn_on_battlefield(PlayerId(0), card("Gauntlets of Chaos"));
    let my_bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let their_minotaur = game.spawn_on_battlefield(PlayerId(1), card("Hurloon Minotaur"));
    let _their_ogre = game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Flight"));
    let terminate = game.spawn_in_hand(PlayerId(1), card("Terminate"));
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(my_bears)));

    aim_gauntlets(&mut game, gauntlets, my_bears, their_minotaur);

    // P0 passes; P1 kills the second target under the ability, then everything resolves.
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();
    game.fund_mana(PlayerId(1));
    game.submit(Intent::Cast {
        player: PlayerId(1),
        object: terminate,
        target: Some(Target::Object(their_minotaur)),
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
    .expect("Terminate destroys target creature");
    resolve_stack(&mut game);

    assert_eq!(
        game.zone_of(their_minotaur),
        Zone::Graveyard,
        "the second target died before the ability resolved"
    );
    assert_eq!(
        game.controller_of(my_bears),
        PlayerId(0),
        "with only one of the two permanents left there is no exchange"
    );
    assert_eq!(
        game.zone_of(aura),
        Zone::Battlefield,
        "no exchange means no Aura destruction"
    );
    assert!(
        game.has_keyword(my_bears, Keyword::Flying),
        "the surviving Aura is still attached to its host"
    );
}

// ── Juxtapose ─────────────────────────────────────────────────────────────────────────
// "You and target player exchange control of the creature you each control with the greatest mana
// value. Then exchange control of artifacts the same way. If two or more permanents a player
// controls are tied for greatest, their controller chooses one of them."

/// Cast Juxtapose from P0's hand at `victim`.
fn cast_juxtapose(game: &mut Game, victim: PlayerId) {
    let juxtapose = game.spawn_in_hand(PlayerId(0), card("Juxtapose"));
    cast_and_resolve(game, PlayerId(0), juxtapose, Some(Target::Player(victim)));
}

#[test]
fn juxtapose_swaps_each_players_greatest_mana_value_creature() {
    // Only the single greatest-mana-value creature on each side moves: P0's Grizzly Bears (2) for
    // P1's Craw Wurm (6), with P1's Hill Giant (4) staying home.
    let mut game = Game::new();
    let my_bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let their_wurm = game.spawn_on_battlefield(PlayerId(1), card("Craw Wurm"));
    let their_giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    // This test crosses a turn boundary, so every seat needs a library: a player who draws from
    // an empty one loses, and losing a seat drops every control override that hands *them* a
    // permanent (CR 800.4a), silently undoing the exchange under test.
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 10]);
    }

    cast_juxtapose(&mut game, PlayerId(1));

    assert_eq!(
        game.controller_of(their_giant),
        PlayerId(1),
        "only the greatest-mana-value creature is exchanged"
    );

    // Outcome, on P1's turn: P1 attacks with the Bears it now controls and P0 blocks with the
    // Wurm it now controls — each seat can only declare with permanents it controls (CR 508/509).
    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| {
        g.current_step() == Step::DeclareAttackers && !g.attackers_declared()
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(my_bears, Defender::Player(PlayerId(0)))],
    })
    .expect("P1 attacks with the Grizzly Bears it gained");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(0),
        blocks: vec![(their_wurm, my_bears)],
    })
    .expect("P0 blocks with the Craw Wurm it gained");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "the gained Craw Wurm blocked the gained Bears, so nothing got through"
    );
    assert_eq!(
        game.zone_of(my_bears),
        Zone::Graveyard,
        "6 power killed the 2/2 in combat"
    );
}

#[test]
fn juxtapose_exchanges_artifacts_after_the_creature_swap_has_moved_them() {
    // "Then exchange control of artifacts the same way" reads the board *after* the creature
    // exchange: P0's Obsianus Golem (an artifact creature, mana value 6) goes to P1 on the
    // creature step, and is then P1's greatest-mana-value artifact — so it comes straight back and
    // P0's Sol Ring goes over in its place, leaving P1's Basalt Monolith untouched.
    let mut game = Game::new();
    let my_golem = game.spawn_on_battlefield(PlayerId(0), card("Obsianus Golem"));
    let my_sol_ring = game.spawn_on_battlefield(PlayerId(0), card("Sol Ring"));
    let their_bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let their_monolith = game.spawn_on_battlefield(PlayerId(1), card("Basalt Monolith"));

    cast_juxtapose(&mut game, PlayerId(1));

    assert_eq!(
        game.controller_of(my_golem),
        PlayerId(0),
        "the Golem went over as a creature and came back as an artifact"
    );
    assert_eq!(
        game.controller_of(their_bears),
        PlayerId(0),
        "the creature step swapped the Bears in"
    );
    assert_eq!(
        game.controller_of(their_monolith),
        PlayerId(1),
        "the Monolith is not P1's greatest-mana-value artifact once the Golem arrives"
    );

    // Outcome: the Sol Ring only produces for its new controller (CR 602.2 / 302.6 — an artifact
    // has no summoning sickness, so the swap is usable the turn it happens).
    assert!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: my_sol_ring,
            ability_index: 0,
            target: None,
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
        .is_err(),
        "P0 can't tap the Sol Ring it gave away"
    );
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();
    let before = game.colorless_in_pool(PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: my_sol_ring,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .expect("P1 taps the Sol Ring it gained");
    assert_eq!(
        game.colorless_in_pool(PlayerId(1)),
        before + 2,
        "the exchanged Sol Ring adds two colorless to its new controller's pool"
    );
}

#[test]
fn juxtapose_exchanges_exactly_one_of_a_players_tied_greatest_creatures() {
    // "If two or more permanents a player controls are tied for greatest, their controller chooses
    // one of them": P0's Gray Ogre and Hurloon Minotaur are both mana value 3, and exactly one of
    // them crosses the table — never both, never neither.
    let mut game = Game::new();
    let my_ogre = game.spawn_on_battlefield(PlayerId(0), card("Gray Ogre"));
    let my_minotaur = game.spawn_on_battlefield(PlayerId(0), card("Hurloon Minotaur"));
    let their_bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    cast_juxtapose(&mut game, PlayerId(1));

    let given_away = [my_ogre, my_minotaur]
        .into_iter()
        .filter(|&id| game.controller_of(id) == PlayerId(1))
        .count();
    assert_eq!(
        given_away, 1,
        "exactly one of the two tied mana-value-3 creatures is exchanged"
    );
    assert_eq!(
        game.controller_of(their_bears),
        PlayerId(0),
        "the other side of the exchange still comes across"
    );
}

#[test]
fn juxtapose_exchanges_nothing_when_only_one_player_has_a_creature() {
    // An exchange needs two objects (CR 701.10c): with no creature on P1's side the creature step
    // moves nothing, and the artifact step is likewise a no-op with no artifacts anywhere.
    let mut game = Game::new();
    let my_bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    cast_juxtapose(&mut game, PlayerId(1));

    assert_eq!(
        game.controller_of(my_bears),
        PlayerId(0),
        "one-sided exchanges don't happen"
    );
}
