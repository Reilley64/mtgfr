//! Legends (`leg`) grind — increment 8: attacking-or-blocking-filter.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Activate `source`'s single `{T}` ability (mana cost prefunded) at `target`.
fn ping(game: &mut Game, source: ObjectId, target: ObjectId) -> Result<Vec<Event>, Reject> {
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

/// Player 0 swings with a fresh `attacker`; player 1 blocks with a fresh `blocker`. Returns the
/// two creatures, with the game parked in the declare-blockers step where the archers can fire.
fn combat(game: &mut Game, attacker: &str, blocker: &str) -> (ObjectId, ObjectId) {
    let attacker = game.spawn_on_battlefield(PlayerId(0), card(attacker));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card(blocker));
    attack_with(game, vec![attacker]);
    block_with(game, vec![(blocker, attacker)]).expect("a legal block");
    (attacker, blocker)
}

// ── Tor Wauki: "{T}: Tor Wauki deals 2 damage to target attacking or blocking creature." ──

#[test]
fn tor_wauki_kills_an_attacking_creature() {
    let mut game = Game::new();
    let wauki = game.spawn_on_battlefield(PlayerId(0), card("Tor Wauki"));
    let (attacker, _) = combat(&mut game, "Grizzly Bears", "Prodigal Sorcerer");

    ping(&mut game, wauki, attacker).expect("an attacking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(attacker),
        Zone::Graveyard,
        "2 damage is lethal to the attacking 2/2"
    );
}

#[test]
fn tor_wauki_kills_a_blocking_creature() {
    // The half the union axis adds: the same ability reaches the *blocker*, not just the attacker.
    let mut game = Game::new();
    let wauki = game.spawn_on_battlefield(PlayerId(0), card("Tor Wauki"));
    let (_, blocker) = combat(&mut game, "Grizzly Bears", "Prodigal Sorcerer");

    ping(&mut game, wauki, blocker).expect("a blocking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(blocker),
        Zone::Graveyard,
        "2 damage is lethal to the blocking 1/1"
    );
}

#[test]
fn tor_wauki_cannot_target_a_creature_out_of_combat() {
    // Neither attacking nor blocking, so it is not a legal target and cannot be chosen as the
    // ability is announced (CR 602.2b → CR 601.2c).
    let mut game = Game::new();
    let wauki = game.spawn_on_battlefield(PlayerId(0), card("Tor Wauki"));
    let bystander = game.spawn_on_battlefield(PlayerId(1), card("Prodigal Sorcerer"));
    combat(&mut game, "Grizzly Bears", "Mons's Goblin Raiders");

    assert_eq!(
        ping(&mut game, wauki, bystander),
        Err(Reject::IllegalTarget),
        "a creature sitting at home is neither attacking nor blocking"
    );
    assert_eq!(
        game.zone_of(bystander),
        Zone::Battlefield,
        "a creature sitting at home takes nothing"
    );
}

// ── Lady Caleria: "{T}: Lady Caleria deals 3 damage to target attacking or blocking creature." ──

#[test]
fn lady_caleria_kills_a_blocking_creature() {
    let mut game = Game::new();
    let caleria = game.spawn_on_battlefield(PlayerId(0), card("Lady Caleria"));
    let (_, blocker) = combat(&mut game, "Mons's Goblin Raiders", "Gray Ogre");

    ping(&mut game, caleria, blocker).expect("a blocking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(blocker),
        Zone::Graveyard,
        "3 damage is lethal to the blocking 2/2"
    );
}

// ── D'Avenant Archer: "{T}: This creature deals 1 damage to target attacking or blocking creature." ──

#[test]
fn davenant_archer_kills_an_attacking_creature() {
    let mut game = Game::new();
    let archer = game.spawn_on_battlefield(PlayerId(0), card("D'Avenant Archer"));
    let (attacker, _) = combat(&mut game, "Mons's Goblin Raiders", "Gray Ogre");

    ping(&mut game, archer, attacker).expect("an attacking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(attacker),
        Zone::Graveyard,
        "1 damage is lethal to the attacking 1/1"
    );
}

// ── Crimson Manticore: "{R}, {T}: This creature deals 1 damage to target attacking or blocking creature." ──

#[test]
fn crimson_manticore_kills_a_blocking_creature() {
    let mut game = Game::new();
    let manticore = game.spawn_on_battlefield(PlayerId(0), card("Crimson Manticore"));
    let (_, blocker) = combat(&mut game, "Grizzly Bears", "Mons's Goblin Raiders");

    assert!(
        game.submit(Intent::ActivateAbility {
            player: PlayerId(0),
            object: manticore,
            ability_index: 0,
            target: Some(Target::Object(blocker)),
            sacrifice: vec![],
            discard_cost: vec![],
            x: 0,
        })
        .is_err(),
        "the ability costs {{R}} on top of the tap, so an empty pool can't pay for it"
    );

    ping(&mut game, manticore, blocker).expect("a blocking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(blocker),
        Zone::Graveyard,
        "1 damage is lethal to the blocking 1/1"
    );
    assert!(
        game.has_keyword(manticore, Keyword::Flying),
        "the Manticore's printed Flying"
    );
}
