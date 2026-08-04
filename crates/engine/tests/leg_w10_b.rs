//! Legends (`leg`) grind, wave 10 slice B — attack and block legality restrictions.
//!
//! Increments 107 (`board-wide-attack-ban`), 89 (`attack-as-though-no-defender`),
//! 42 (`attacked-last-turn-restriction`), 23 (`attacker-blocker-count-cap`),
//! 16 (`arboria-attack-restriction`) and 37 (`attacks-unblocked-trigger`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody across the several
/// turns these restrictions need to become observable.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..60 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Advance to declare attackers and try to swing with `player`'s creatures at the other seat,
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

/// Activate `object`'s only activated ability with mana funded from thin air.
fn activate(game: &mut Game, player: PlayerId, object: ObjectId) {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(game);
}

// ── increment 107: Moat ───────────────────────────────────────────────────────────────

#[test]
fn moat_bans_ground_creatures_from_attacking_on_both_sides() {
    // "Creatures without flying can't attack." Board-wide, so Moat's own controller is caught too.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Moat"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let sprites = game.spawn_on_battlefield(PlayerId(0), card("Scryb Sprites"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[bear]),
        Err(Reject::IllegalDeclaration),
        "a Grizzly Bears has no flying",
    );
    assert!(
        try_attack(&mut game, PlayerId(0), &[sprites]).is_ok(),
        "a flyer is untouched by Moat",
    );
    assert_eq!(game.attackers(), vec![sprites]);
}

#[test]
fn moat_reaches_across_the_table() {
    // Board-wide rather than "creatures you control": the opponent's ground creature is banned by
    // a Moat its controller never played.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Moat"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    pass_until_next_turn(&mut game);
    assert_eq!(
        try_attack(&mut game, PlayerId(1), &[bear]),
        Err(Reject::IllegalDeclaration),
        "the ban is not scoped to Moat's controller",
    );
}

// ── increment 42: Giant Turtle, Wall of Dust ──────────────────────────────────────────

#[test]
fn giant_turtle_must_rest_the_turn_after_it_attacks() {
    // "This creature can't attack if it attacked during your last turn." Not every other turn of
    // the *table* — every other turn of its controller's.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let turtle = game.spawn_on_battlefield(PlayerId(0), card("Giant Turtle"));

    try_attack(&mut game, PlayerId(0), &[turtle]).expect("nothing happened on a turn before this");
    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[turtle]),
        Err(Reject::IllegalDeclaration),
        "it attacked during its controller's last turn",
    );

    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    assert!(
        try_attack(&mut game, PlayerId(0), &[turtle]).is_ok(),
        "it sat out the last turn, so the restriction has lapsed",
    );
}

#[test]
fn wall_of_dust_keeps_what_it_blocked_home_next_turn() {
    // "Whenever this creature blocks a creature, that creature can't attack during its controller's
    // next turn."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Dust"));

    try_attack(&mut game, PlayerId(0), &[bear]).expect("nothing stops the first attack");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(1),
        blocks: vec![(wall, bear)],
    })
    .expect("a 1/4 Wall can block a 2/2");

    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[bear]),
        Err(Reject::IllegalDeclaration),
        "the blocked creature sits out its controller's next turn",
    );

    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    assert!(
        try_attack(&mut game, PlayerId(0), &[bear]).is_ok(),
        "the ban covered one turn, not every turn after",
    );
}

// ── increment 16: Arboria ─────────────────────────────────────────────────────────────

#[test]
fn arboria_shields_a_player_who_did_nothing_last_turn() {
    // "Creatures can't attack a player unless that player cast a spell or put a nontoken permanent
    // onto the battlefield during their last turn." A player who has not yet taken a turn has done
    // nothing during it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Arboria"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[bear]),
        Err(Reject::IllegalDeclaration),
        "the defending player has done nothing to open themselves up",
    );
}

#[test]
fn arboria_opens_a_player_who_played_a_land() {
    // A land is a nontoken permanent, so playing one during your turn opens you to attack on the
    // next one.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Arboria"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    pass_until_next_turn(&mut game);
    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let mountain = game.spawn_in_hand(PlayerId(1), card("Mountain"));
    game.submit(Intent::PlayLand {
        player: PlayerId(1),
        object: mountain,
    })
    .expect("a land drop on your own main phase");

    pass_until_next_turn(&mut game);
    assert!(
        try_attack(&mut game, PlayerId(0), &[bear]).is_ok(),
        "they put a nontoken permanent onto the battlefield during their last turn",
    );
}

// ── increment 89: Wall of Wonder ──────────────────────────────────────────────────────

#[test]
fn wall_of_wonder_can_attack_the_turn_it_is_pumped() {
    // "{2}{U}{U}: This creature gets +4/-4 until end of turn and can attack this turn as though it
    // didn't have defender."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Wonder"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        (game.power(wall), game.toughness(wall)),
        (2, 5),
        "printed 2/5 before the ability",
    );
    activate(&mut game, PlayerId(0), wall);
    assert_eq!(
        (game.power(wall), game.toughness(wall)),
        (6, 1),
        "+4/-4 until end of turn",
    );

    assert!(
        try_attack(&mut game, PlayerId(0), &[wall]).is_ok(),
        "the pumped Wall attacks despite its printed defender",
    );
    assert_eq!(game.attackers(), vec![wall]);
}

// ── increment 23: Caverns of Despair ──────────────────────────────────────────────────

#[test]
fn caverns_of_despair_caps_the_attack_at_two_creatures() {
    // "No more than two creatures can attack each combat." A whole-declaration ceiling: no single
    // creature is banned, the third one just has no room.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Caverns of Despair"));
    let a = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let b = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let c = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[a, b, c]),
        Err(Reject::IllegalDeclaration),
        "three attackers exceed the ceiling",
    );
    assert!(
        try_attack(&mut game, PlayerId(0), &[a, b]).is_ok(),
        "two attackers sit exactly at it",
    );
    assert_eq!(game.attackers(), vec![a, b]);
}

#[test]
fn caverns_of_despair_caps_the_block_at_two_creatures() {
    // "No more than two creatures can block each combat" — counted in blocking creatures across the
    // whole combat, so a double block plus a single block is one blocker too many.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Caverns of Despair"));
    let a = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let b = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let x = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let y = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let z = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    try_attack(&mut game, PlayerId(0), &[a, b]).expect("two attackers are legal");
    advance_until(&mut game, |g| g.current_step() == Step::DeclareBlockers);
    assert_eq!(
        game.submit(Intent::DeclareBlockers {
            player: PlayerId(1),
            blocks: vec![(x, a), (y, a), (z, b)],
        }),
        Err(Reject::IllegalDeclaration),
        "three blocking creatures exceed the ceiling",
    );
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(1),
        blocks: vec![(x, a), (y, a)],
    })
    .expect("a two-creature double block sits at the ceiling");
    assert_eq!(game.blocks(), vec![(x, a), (y, a)]);
}

#[test]
fn caverns_of_despair_beats_goads_requirement_to_attack() {
    // CR 509.1a — a restriction beats a requirement: with all three goaded, the two the ceiling
    // allows is a legal declaration, rather than a combat nobody can declare.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Caverns of Despair"));
    let a = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let b = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let c = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    for id in [a, b, c] {
        game.goad(id, PlayerId(1));
    }

    assert!(
        try_attack(&mut game, PlayerId(0), &[a, b]).is_ok(),
        "a full declaration discharges the third goad",
    );
    assert_eq!(game.attackers(), vec![a, b]);
}

#[test]
fn wall_of_wonder_cant_attack_without_its_ability() {
    // Defender is only waived for the turn the ability resolved on: an unpumped Wall stays home.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Wonder"));

    assert_eq!(
        try_attack(&mut game, PlayerId(0), &[wall]),
        Err(Reject::IllegalDeclaration),
        "an unpumped Wall of Wonder has defender",
    );
}

// ── increment 37: Floral Spuzzem ──────────────────────────────────────────────────────

/// Walk the unblocked attack to Floral Spuzzem's trigger and answer its "you may".
fn spuzzem_attacks_unblocked(game: &mut Game, spuzzem: ObjectId, take_it: bool) {
    try_attack(game, PlayerId(0), &[spuzzem]).expect("nothing stops the attack");
    advance_until(game, |g| g.pending_choice().is_some());
    assert!(
        matches!(game.pending_choice(), Some(PendingChoice::MayYesNo { .. })),
        "the unblocked attack raises Spuzzem's may; got {:?}",
        game.pending_choice(),
    );
    game.submit(Intent::AnswerMay {
        player: PlayerId(0),
        yes: take_it,
    })
    .unwrap();
}

#[test]
fn floral_spuzzem_destroys_an_artifact_and_then_deals_no_damage() {
    // "Whenever this creature attacks and isn't blocked, you may destroy target artifact defending
    // player controls. If you do, this creature assigns no combat damage this turn."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let spuzzem = game.spawn_on_battlefield(PlayerId(0), card("Floral Spuzzem"));
    let ring = game.spawn_on_battlefield(PlayerId(1), card("Sol Ring"));
    let life = game.life(PlayerId(1));

    spuzzem_attacks_unblocked(&mut game, spuzzem, true);
    game.submit(Intent::ChooseTargets {
        player: PlayerId(0),
        targets: vec![Target::Object(ring)],
    })
    .expect("the artifact its defending player controls is the one legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.zone_of(ring), Zone::Graveyard, "the artifact is destroyed");
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        life,
        "having destroyed the artifact, Spuzzem assigned no combat damage",
    );
}

#[test]
fn floral_spuzzem_that_declines_still_connects() {
    // "If you do" — declining the may leaves an ordinary 4/4 attack.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let spuzzem = game.spawn_on_battlefield(PlayerId(0), card("Floral Spuzzem"));
    game.spawn_on_battlefield(PlayerId(1), card("Sol Ring"));
    let life = game.life(PlayerId(1));

    spuzzem_attacks_unblocked(&mut game, spuzzem, false);
    advance_until(&mut game, |g| g.current_step() == Step::End);
    assert_eq!(
        game.life(PlayerId(1)),
        life - 2,
        "a declined trigger takes nothing off its damage",
    );
}
