//! Legends (`leg`) grind — wave 12 slice C: combat mutation.
//!
//! - increment 123: `token-named-band-quality` — Master of the Hunt's "bands with other creatures
//!   named Wolves of the Hunt" (CR 702.22b, where the \[quality\] is a card *name* rather than a
//!   supertype).
//! - increment 87: `conditional-banding-grant` — Wall of Caltrops' co-blocker intervening-if.
//! - increment 47: `imprison` — removal from combat (CR 506.4).

mod common;

use common::*;
use engine::*;

/// The "bands with other creatures named Wolves of the Hunt" keyword the Master's token prints.
fn wolf_band() -> Keyword {
    Keyword::BandsWith(BandsWithQuality::Named("Wolves of the Hunt"))
}

/// A Wolves of the Hunt token on player 0's battlefield, spawned from the token profile directly
/// rather than through the Master's activated ability.
fn spawn_wolf(game: &mut Game) -> ObjectId {
    game.spawn_on_battlefield(
        PlayerId(0),
        cards::get_token("leg-token-wolves-of-the-hunt")
            .expect("Wolves of the Hunt token profile is loaded"),
    )
}

/// P0 declares `attackers` at P1 with `bands` as its declared attacking bands (CR 702.22c).
fn attack_in_bands(
    game: &mut Game,
    attackers: &[ObjectId],
    bands: Vec<Vec<ObjectId>>,
) -> Result<Vec<Event>, Reject> {
    advance_until(game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackersInBands {
        player: PlayerId(0),
        attackers: attackers
            .iter()
            .map(|&a| (a, Defender::Player(PlayerId(1))))
            .collect(),
        bands,
    })
}

// ── increment 123: Master of the Hunt ────────────────────────────────────────────────

#[test]
fn the_master_of_the_hunt_token_carries_the_name_keyed_band_keyword() {
    // "Create a 1/1 green Wolf creature token named Wolves of the Hunt. It has 'bands with other
    // creatures named Wolves of the Hunt.'" — the token has it; the Master itself never does.
    let mut game = Game::new();
    let master = game.spawn_on_battlefield(PlayerId(0), card("Master of the Hunt"));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: master,
        ability_index: 0, // {2}{G}{G}: create a Wolves of the Hunt token.
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    let mut events = Vec::new();
    for _ in 0..game.player_count() {
        events = game
            .submit(Intent::PassPriority {
                player: game.priority_holder(),
            })
            .unwrap();
    }
    let wolf = events
        .iter()
        .find_map(|e| match e {
            Event::TokenCreated { token, .. } => Some(*token),
            _ => None,
        })
        .expect("activating the ability creates a Wolves of the Hunt token");

    assert_eq!(game.def_of(wolf).name, "Wolves of the Hunt");
    assert_eq!(game.power(wolf), 1);
    assert_eq!(game.toughness(wolf), 1);
    assert!(game.colors_of(wolf)[Color::Green.index()], "a green Wolf");
    assert!(game.def_of(wolf).subtypes.contains(&"Wolf"));
    assert!(
        game.has_keyword(wolf, wolf_band()),
        "the token has 'bands with other creatures named Wolves of the Hunt'"
    );
    assert!(
        !game.has_keyword(master, wolf_band()),
        "the Master itself has no band keyword — only the tokens it makes do"
    );
}

#[test]
fn a_pack_of_wolves_attacks_as_a_band() {
    // CR 702.22c's second sentence with a card name as the \[quality\]: both members are named
    // Wolves of the Hunt and both carry the keyword, so the band is legal.
    let mut game = Game::new();
    let first = spawn_wolf(&mut game);
    let second = spawn_wolf(&mut game);
    attack_in_bands(&mut game, &[first, second], vec![vec![first, second]])
        .expect("two Wolves of the Hunt are a legal band");

    assert_eq!(
        game.attacking_bands(),
        [vec![first, second]],
        "the pack is recorded as a band"
    );
}

#[test]
fn a_differently_named_creature_cannot_join_the_wolf_pack() {
    // CR 702.22c: every other member must match the band's \[quality\] — here, be *named* Wolves
    // of the Hunt. A Grizzly Bears is a creature, but not that creature.
    let mut game = Game::new();
    let wolf = spawn_wolf(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    assert!(
        attack_in_bands(&mut game, &[wolf, bears], vec![vec![wolf, bears]]).is_err(),
        "Grizzly Bears fails the band's name quality"
    );
    assert!(
        game.attacking_bands().is_empty(),
        "a rejected declaration records nothing"
    );
}

#[test]
fn blocking_one_wolf_of_a_pack_blocks_the_whole_pack() {
    // CR 702.22h: "if an attacking creature becomes blocked by a creature, each other creature in
    // the same band as the attacking creature becomes blocked by that same blocking creature."
    let mut game = Game::new();
    let first = spawn_wolf(&mut game);
    let second = spawn_wolf(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_in_bands(&mut game, &[first, second], vec![vec![first, second]])
        .expect("two Wolves of the Hunt are a legal band");
    block_with(&mut game, vec![(bears, first)]).expect("the Bears blocks the first wolf");

    assert!(
        game.blocks().contains(&(bears, second)),
        "the second wolf becomes blocked by the same blocker (CR 702.22h)"
    );
    assert!(
        game.blocked_attackers().contains(&second),
        "and so is blocked, even though nobody declared a block on it"
    );
}

// ── increment 87: Wall of Caltrops ───────────────────────────────────────────────────

/// Player 0 attacks with a Grizzly Bears; player 1's `blockers` are spawned and all block it.
/// Returns `(attacker, blockers)`.
fn wall_block(game: &mut Game, blockers: &[&str]) -> (ObjectId, Vec<ObjectId>) {
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blockers: Vec<ObjectId> = blockers
        .iter()
        .map(|name| game.spawn_on_battlefield(PlayerId(1), card(name)))
        .collect();
    attack_with(game, vec![bears]);
    block_with(game, blockers.iter().map(|&b| (b, bears)).collect()).expect("the blocks are legal");
    (bears, blockers)
}

#[test]
fn wall_of_caltrops_gains_banding_beside_another_wall() {
    // "Whenever this creature blocks a creature, if at least one other Wall creature is blocking
    // that creature and no non-Wall creatures are blocking that creature, this creature gains
    // banding until end of turn."
    let mut game = Game::new();
    let (_, blockers) = wall_block(&mut game, &["Wall of Caltrops", "Wall of Caltrops"]);
    // One trigger per Wall, and each is its own group.
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);

    for wall in blockers {
        assert!(
            game.has_keyword(wall, Keyword::Banding),
            "each Wall's intervening-if held, so each gained banding"
        );
    }
}

#[test]
fn one_wall_of_caltrops_blocking_alone_gains_nothing() {
    // "at least one **other** Wall creature is blocking that creature" — a lone Wall has no
    // co-blocker, so the intervening-if fails at placement and the trigger never goes on the
    // stack (CR 603.4).
    let mut game = Game::new();
    let (_, blockers) = wall_block(&mut game, &["Wall of Caltrops"]);

    assert!(!game.has_keyword(blockers[0], Keyword::Banding));
}

#[test]
fn a_non_wall_co_blocker_stops_the_caltrops_banding() {
    // "and **no non-Wall creatures** are blocking that creature" — the Bears blocking alongside
    // fails the second half of the intervening-if for both Walls.
    let mut game = Game::new();
    let (_, blockers) = wall_block(
        &mut game,
        &["Wall of Caltrops", "Wall of Caltrops", "Grizzly Bears"],
    );

    for blocker in blockers {
        assert!(
            !game.has_keyword(blocker, Keyword::Banding),
            "a non-Wall is in the block, so nobody gains banding"
        );
    }
}

#[test]
fn caltrops_banding_moves_the_damage_division_to_the_defending_player() {
    // CR 702.22j: with banding among the blockers, the *defending* player divides the attacker's
    // combat damage instead of the active player. The gained banding is what shifts it.
    let mut game = Game::new();
    wall_block(&mut game, &["Wall of Caltrops", "Wall of Caltrops"]);
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);
    advance_until(&mut game, |g| g.pending_choice().is_some());

    let Some(PendingChoice::AssignCombatDamage { player, .. }) = game.pending_choice() else {
        panic!("the doubly-blocked Bears owes a division");
    };
    assert_eq!(
        player,
        PlayerId(1),
        "the Walls have banding, so the defending player divides the Bears' damage"
    );
}

// ── increment 47: Imprison ───────────────────────────────────────────────────────────

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step, which is all a non-active seat needs to act at instant speed.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

/// Player 0 casts Imprison from hand onto `host` at sorcery speed and lets it resolve. Returns the
/// Aura's battlefield id.
fn imprison(game: &mut Game, host: ObjectId) -> ObjectId {
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
    let aura = game.spawn_in_hand(PlayerId(0), card("Imprison"));
    give_priority(game, PlayerId(0));
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
        object: aura,
        target: Some(Target::Object(host)),
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
    game.current_id(aura)
}

/// Roll to Imprison's "you may pay {1}" and answer it for the Aura's controller.
fn answer_imprison(game: &mut Game, pay: bool) {
    advance_until(game, |g| {
        matches!(g.pending_choice(), Some(PendingChoice::PayOrElse { .. }))
    });
    game.fund_mana(PlayerId(0));
    game.submit(Intent::PayOptionalCost {
        player: PlayerId(0),
        pay,
        discard_cost: Vec::new(),
    })
    .unwrap();
}

/// Player 1 activates their Prodigal Sorcerer's `{T}` ping at player 0.
fn ping(game: &mut Game, sorcerer: ObjectId) {
    give_priority(game, PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: sorcerer,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(0))),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
}

#[test]
fn paying_for_imprison_counters_the_enchanted_creatures_tap_ability() {
    // "Whenever a player activates an ability of enchanted creature with {T} in its activation cost
    // that isn't a mana ability, you may pay {1}. If you do, counter that ability."
    let mut game = Game::new();
    let sorcerer = game.spawn_on_battlefield(PlayerId(1), card("Prodigal Sorcerer"));
    let aura = imprison(&mut game, sorcerer);
    let before = game.life(PlayerId(0));

    ping(&mut game, sorcerer);
    answer_imprison(&mut game, true);
    advance_until(&mut game, |g| g.stack_is_empty());

    assert_eq!(
        game.life(PlayerId(0)),
        before,
        "the ping was countered, so it never dealt its damage"
    );
    assert_eq!(
        game.zone_of(aura),
        Zone::Battlefield,
        "paying keeps the Aura around"
    );
}

#[test]
fn declining_imprisons_cost_destroys_the_aura_and_lets_the_ability_resolve() {
    // "If you don't, destroy this Aura." — and the ability it was watching is untouched.
    let mut game = Game::new();
    let sorcerer = game.spawn_on_battlefield(PlayerId(1), card("Prodigal Sorcerer"));
    let aura = imprison(&mut game, sorcerer);
    let before = game.life(PlayerId(0));

    ping(&mut game, sorcerer);
    answer_imprison(&mut game, false);
    advance_until(&mut game, |g| g.stack_is_empty());

    assert_eq!(
        game.zone_of(aura),
        Zone::Graveyard,
        "declining destroys the Aura"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        before - 1,
        "and the uncountered ping still resolves"
    );
}

#[test]
fn imprison_ignores_a_mana_ability() {
    // "…that isn't a mana ability" (CR 605.3a): the Elves' `{T}: Add {G}` never uses the stack, so
    // Imprison never triggers and never asks for {1}.
    let mut game = Game::new();
    let elves = game.spawn_on_battlefield(PlayerId(1), card("Llanowar Elves"));
    let aura = imprison(&mut game, elves);

    give_priority(&mut game, PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: elves,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();

    assert!(
        game.pending_choice().is_none(),
        "a mana ability doesn't trip Imprison's watch"
    );
    assert!(game.stack_is_empty(), "nor does anything go on the stack");
    assert_eq!(game.zone_of(aura), Zone::Battlefield);
}

#[test]
fn paying_for_imprison_pulls_the_enchanted_attacker_out_of_combat() {
    // "Whenever enchanted creature attacks or blocks, you may pay {1}. If you do, tap the creature,
    // remove it from combat…" (CR 506.4).
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let aura = imprison(&mut game, bears);
    let before = game.life(PlayerId(1));

    attack_with(&mut game, vec![bears]);
    answer_imprison(&mut game, true);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert!(game.is_tapped(bears), "paying taps the creature");
    assert!(
        !game.attackers().contains(&bears),
        "and removes it from combat"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "a creature removed from combat deals no combat damage"
    );
    assert_eq!(game.zone_of(aura), Zone::Battlefield);
}

#[test]
fn imprisoning_a_blocker_releases_the_creature_it_was_solely_blocking() {
    // "…and creatures it was blocking that had become blocked by only that creature this combat
    // become unblocked." The blocks half of the trigger is what puts this on the stack at all.
    let mut game = Game::new();
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Prodigal Sorcerer"));
    imprison(&mut game, blocker);
    let before = game.life(PlayerId(1));

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).expect("the Sorcerer blocks");
    answer_imprison(&mut game, true);
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert!(
        !game.blocked_attackers().contains(&attacker),
        "its only blocker left combat, so the Bears is unblocked again"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        before - 2,
        "and the now-unblocked Bears connects"
    );
    assert_eq!(
        game.marked_damage(blocker),
        0,
        "the blocker that left combat takes none"
    );
}
