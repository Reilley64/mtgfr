//! Legends (`leg`) grind — increments 9 (`block-restriction-by-filter`),
//! 10 (`attack-ban-by-filter`) and 29 (`elder-spawn`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..20 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
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

/// One whole combat: P0 attacks with `attacker`, P1 declares `blocker` against it, and the
/// engine's verdict comes back. A fresh game per blocker — a legal block ends the declaration, so
/// two blockers can't be tried against the same attack.
fn blocked_by(attacker: &str, blocker: &str) -> Result<Vec<Event>, Reject> {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card(attacker));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card(blocker));
    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)])
}

/// Advance to declare attackers and try to swing with `attacker`'s seat at the other seat,
/// returning the engine's verdict rather than unwrapping it.
fn try_attack(
    game: &mut Game,
    player: PlayerId,
    attackers: &[ObjectId],
) -> Result<Vec<Event>, Reject> {
    advance_until(game, |g| g.current_step() == Step::DeclareAttackers);
    let defender = Defender::Player(PlayerId(1 - player.0));
    game.submit(Intent::DeclareAttackers {
        player,
        attackers: attackers.iter().map(|&a| (a, defender)).collect(),
    })
}

// ── increment 9: "can't be blocked except by …" ───────────────────────────────────────

#[test]
fn evil_eye_of_orms_by_gore_cant_be_blocked_except_by_walls() {
    // "This creature can't be blocked except by Walls."
    let attacker = "Evil Eye of Orms-by-Gore";
    assert_eq!(
        blocked_by(attacker, "Grizzly Bears"),
        Err(Reject::IllegalDeclaration),
        "a Grizzly Bears is not a Wall",
    );
    assert!(
        blocked_by(attacker, "Wall of Stone").is_ok(),
        "the printed exception still gets through",
    );
}

#[test]
fn elven_riders_cant_be_blocked_except_by_walls_and_or_creatures_with_flying() {
    // "This creature can't be blocked except by Walls and/or creatures with flying." Both halves
    // of the "and/or" let a blocker through on its own (CR 509.1b).
    assert_eq!(
        blocked_by("Elven Riders", "Grizzly Bears"),
        Err(Reject::IllegalDeclaration),
        "a ground non-Wall is neither exception",
    );
    assert!(
        blocked_by("Elven Riders", "Wall of Stone").is_ok(),
        "a Wall without flying is one exception",
    );
    assert!(
        blocked_by("Elven Riders", "Scryb Sprites").is_ok(),
        "a flyer that isn't a Wall is the other",
    );
}

/// P0 casts Seeker on a Hill Giant and attacks with it; P1 blocks with `blocker`.
fn seeker_enchanted_giant_blocked_by(blocker: &str) -> Result<Vec<Event>, Reject> {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Seeker"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card(blocker));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, aura, Some(Target::Object(giant)));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(blocker, giant)])
}

#[test]
fn seeker_lets_only_artifact_and_white_creatures_block_the_enchanted_creature() {
    // "Enchanted creature can't be blocked except by artifact creatures and/or white creatures."
    assert_eq!(
        seeker_enchanted_giant_blocked_by("Grizzly Bears"),
        Err(Reject::IllegalDeclaration),
        "a green nonartifact creature is neither exception",
    );
    assert!(
        seeker_enchanted_giant_blocked_by("Obsianus Golem").is_ok(),
        "a colorless artifact creature is one exception",
    );
    assert!(
        seeker_enchanted_giant_blocked_by("Savannah Lions").is_ok(),
        "a nonartifact white creature is the other",
    );
}

// ── increment 10: "creatures … can't attack" ──────────────────────────────────────────

#[test]
fn evil_eye_of_orms_by_gore_bans_its_controllers_non_eye_creatures_from_attacking() {
    // "Non-Eye creatures you control can't attack." The Eye itself is an Eye, so it is exempt.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let eye = game.spawn_on_battlefield(PlayerId(0), card("Evil Eye of Orms-by-Gore"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[bear]),
        Err(Reject::IllegalDeclaration),
        "a Grizzly Bears is not an Eye",
    );
    assert!(
        try_attack(&mut game, PlayerId(0), &[eye]).is_ok(),
        "the Eye is exempt from its own ban",
    );
    assert_eq!(
        game.attackers(),
        vec![eye],
        "only the Eye ended up attacking",
    );
}

#[test]
fn evil_eye_of_orms_by_gore_leaves_an_opponents_creatures_alone() {
    // "Non-Eye creatures **you control** can't attack" — the ban is scoped to the Eye's own
    // controller, so an opponent's non-Eye creature swings freely.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Evil Eye of Orms-by-Gore"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    pass_until_next_turn(&mut game);
    assert!(
        try_attack(&mut game, PlayerId(1), &[bear]).is_ok(),
        "an opponent's Grizzly Bears is not covered by the Eye's ban",
    );
    assert_eq!(game.attackers(), vec![bear]);
}

#[test]
fn evil_eye_of_orms_by_gore_beats_a_goad_requirement() {
    // CR 509.1a: a restriction beats a requirement, so a goaded non-Eye creature is not "able"
    // to attack and the declaration that leaves it home is still legal.
    // Control: the same goad with no Evil Eye out does force the attack, so the assertion below
    // is about the ban rather than about goad simply not working here.
    let mut control = Game::new();
    stock_libraries(&mut control);
    let free_bear = control.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    control.goad(free_bear, PlayerId(1));
    assert_eq!(
        control.required_attacks(PlayerId(0)).len(),
        1,
        "a goaded creature with nothing stopping it must attack",
    );

    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Evil Eye of Orms-by-Gore"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.goad(bear, PlayerId(1));

    assert!(
        game.required_attacks(PlayerId(0)).is_empty(),
        "the ban makes the goaded creature unable to attack, so nothing is required",
    );
    assert!(
        try_attack(&mut game, PlayerId(0), &[]).is_ok(),
        "declaring no attackers is legal despite the goad",
    );
}

#[test]
fn akron_legionnaire_exempts_its_namesakes_and_artifact_creatures() {
    // "Except for creatures named Akron Legionnaire and artifact creatures, creatures you
    // control can't attack."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let akron = game.spawn_on_battlefield(PlayerId(0), card("Akron Legionnaire"));
    let second_akron = game.spawn_on_battlefield(PlayerId(0), card("Akron Legionnaire"));
    let golem = game.spawn_on_battlefield(PlayerId(0), card("Obsianus Golem"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[bear]),
        Err(Reject::IllegalDeclaration),
        "a Grizzly Bears is neither named Akron Legionnaire nor an artifact creature",
    );
    assert!(
        try_attack(&mut game, PlayerId(0), &[akron, second_akron, golem]).is_ok(),
        "both namesakes and the artifact creature are exempt",
    );
    assert_eq!(game.attackers(), vec![akron, second_akron, golem]);
}

// ── increment 29: Elder Spawn ─────────────────────────────────────────────────────────

#[test]
fn elder_spawn_cant_be_blocked_by_red_creatures() {
    // "This creature can't be blocked by red creatures."
    assert_eq!(
        blocked_by("Elder Spawn", "Hill Giant"),
        Err(Reject::IllegalDeclaration),
        "a red creature is turned away",
    );
    assert!(
        blocked_by("Elder Spawn", "Grizzly Bears").is_ok(),
        "a green creature blocks normally",
    );
}
