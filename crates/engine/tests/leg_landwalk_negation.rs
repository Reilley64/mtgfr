//! Legends (`leg`) grind — increment 4: landwalk-negation.
//!
//! "Creatures with \[type\]walk can be blocked as though they didn't have \[type\]walk" — eight
//! cards, one static per basic land type. CR 702.14b's evasion is *checked* at block declaration,
//! not stripped from the creature, so every test here also cares about what stays true of the
//! attacker.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..10 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// P0 attacks with a lone `walker`; P1 controls `land` and tries to block with a vanilla bear.
fn block_a_landwalker(
    negator: Option<&str>,
    walker: &str,
    land: &str,
) -> Result<Vec<Event>, Reject> {
    let mut game = Game::new();
    if let Some(negator) = negator {
        game.spawn_on_battlefield(PlayerId(1), card(negator));
    }
    let walker = game.spawn_on_battlefield(PlayerId(0), card(walker));
    game.spawn_on_battlefield(PlayerId(1), card(land));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![walker]);
    block_with(&mut game, vec![(bear, walker)])
}

// ── tests ─────────────────────────────────────────────────────────────────────────────

#[test]
fn crevasse_lets_a_mountainwalker_be_blocked() {
    // "Creatures with mountainwalk can be blocked as though they didn't have mountainwalk."
    assert!(
        block_a_landwalker(None, "Mountain Yeti", "Mountain").is_err(),
        "without Crevasse the defender's Mountain makes the Yeti unblockable (CR 702.14b)"
    );
    assert!(
        block_a_landwalker(Some("Crevasse"), "Mountain Yeti", "Mountain").is_ok(),
        "Crevasse waives the mountainwalk check"
    );
}

#[test]
fn landwalk_negation_reaches_the_board_whoever_controls_it() {
    // The static names "creatures with mountainwalk", not "creatures your opponents control" —
    // so the attacking player's own Crevasse frees their attacker's blockers too.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Crevasse"));
    let yeti = game.spawn_on_battlefield(PlayerId(0), card("Mountain Yeti"));
    game.spawn_on_battlefield(PlayerId(1), card("Mountain"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![yeti]);
    assert!(
        block_with(&mut game, vec![(bear, yeti)]).is_ok(),
        "a Crevasse under the attacker's own control still negates mountainwalk"
    );
}

#[test]
fn lord_magnus_negates_plainswalk_and_forestwalk_at_once() {
    // Lord Magnus prints the plainswalk static *and* the forestwalk one, so both apply — the
    // second doesn't replace the first.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Lord Magnus"));
    let avengers = game.spawn_on_battlefield(PlayerId(0), card("Righteous Avengers"));
    let cats = game.spawn_on_battlefield(PlayerId(0), card("Cat Warriors"));
    game.spawn_on_battlefield(PlayerId(1), card("Plains"));
    game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let first = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![avengers, cats]);
    assert!(
        block_with(&mut game, vec![(first, avengers), (second, cats)]).is_ok(),
        "both of Lord Magnus's statics are live, so the plainswalker and the forestwalker are \
         each blockable"
    );
}

#[test]
fn quagmire_negates_only_swampwalk() {
    // One static, one land type: a swampwalk negator says nothing about forestwalk.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Quagmire"));
    let wraith = game.spawn_on_battlefield(PlayerId(0), card("Bog Wraith"));
    let cats = game.spawn_on_battlefield(PlayerId(0), card("Cat Warriors"));
    game.spawn_on_battlefield(PlayerId(1), card("Swamp"));
    game.spawn_on_battlefield(PlayerId(1), card("Forest"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![wraith, cats]);
    assert!(
        block_with(&mut game, vec![(bear, wraith)]).is_ok(),
        "Quagmire frees the swampwalker"
    );
    assert!(
        block_with(&mut game, vec![(bear, cats)]).is_err(),
        "forestwalk is untouched by a swampwalk negator"
    );
}

#[test]
fn a_negated_landwalker_still_has_landwalk() {
    // CR 702.14b evasion is checked at block declaration, not removed: "as though they didn't
    // have mountainwalk" changes nothing about the creature's keywords.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(1), card("Crevasse"));
    let yeti = game.spawn_on_battlefield(PlayerId(0), card("Mountain Yeti"));

    assert!(
        game.has_keyword(yeti, Keyword::Landwalk(BasicLandType::Mountain)),
        "the Yeti keeps mountainwalk while Crevasse is on the battlefield"
    );
}

#[test]
fn undertow_does_not_stop_islandwalk_getting_through_island_sanctuary() {
    // The other reader of "has islandwalk": Island Sanctuary's "can't be attacked except by
    // creatures with flying and/or islandwalk" is an *attack* restriction, and Undertow only
    // waives the block check — so the islandwalker still attacks through the shield.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(1), card("Island Sanctuary"));
    game.spawn_on_battlefield(PlayerId(1), card("Undertow"));
    let leviathan = game.spawn_on_battlefield(PlayerId(0), card("Segovian Leviathan"));

    advance_until(
        &mut game,
        |g| matches!(g.pending_choice(), Some(PendingChoice::MayYesNo { player, .. }) if player == PlayerId(1)),
    );
    game.submit(Intent::AnswerMay {
        player: PlayerId(1),
        yes: true,
    })
    .unwrap();
    pass_until_next_turn(&mut game);

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    assert!(
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![(leviathan, Defender::Player(PlayerId(1)))],
        })
        .is_ok(),
        "islandwalk is still on the creature for Island Sanctuary to read"
    );
}

#[test]
fn every_landwalk_negation_card_frees_its_land_type() {
    // The whole increment-4 cycle, each against the land type it names.
    let cycle = [
        ("Crevasse", "Mountain Yeti", "Mountain"),
        ("Deadfall", "Cat Warriors", "Forest"),
        ("Gosta Dirk", "Segovian Leviathan", "Island"),
        ("Great Wall", "Righteous Avengers", "Plains"),
        ("Lord Magnus", "Righteous Avengers", "Plains"),
        ("Quagmire", "Bog Wraith", "Swamp"),
        ("Undertow", "Segovian Leviathan", "Island"),
        ("Ur-Drago", "Bog Wraith", "Swamp"),
    ];
    for (negator, walker, land) in cycle {
        assert!(
            block_a_landwalker(None, walker, land).is_err(),
            "{walker} is unblockable through a {land} without {negator}"
        );
        assert!(
            block_a_landwalker(Some(negator), walker, land).is_ok(),
            "{negator} lets {walker} be blocked"
        );
    }
}
