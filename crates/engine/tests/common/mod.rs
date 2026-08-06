//! Helpers shared by the per-set integration test files (`leg_c*.rs`).
//!
//! `game.rs` predates this module and keeps its own private copies of the four generic
//! helpers below; folding it in is a mechanical consolidation, not a behavior change.

#![allow(dead_code)]

use engine::*;

/// Load a real card from the `cards` crate's TOML pool (a dev-dependency).
pub fn card(name: &str) -> CardDef {
    cards::get_by_name(name).unwrap_or_else(|| panic!("unknown card {name:?}"))
}

/// The battlefield permanent id minted by a `LandPlayed` event (the played land's card object
/// is left behind as a redirect, so `is_tapped` and friends must read the new permanent).
pub fn land_permanent(events: &[Event]) -> ObjectId {
    events
        .iter()
        .find_map(|e| match e {
            Event::LandPlayed { permanent, .. } => Some(*permanent),
            _ => None,
        })
        .expect("playing a land emits LandPlayed")
}

pub fn deal_opening(game: &mut Game, deck_size: usize) {
    let plains = card("Plains");
    let deck = vec![plains; deck_size];
    for p in 0..game.player_count() as u8 {
        let player = PlayerId(p);
        game.stack_library(player, &deck);
        game.shuffle(player);
    }
    for _ in 0..7 {
        for p in 0..game.player_count() as u8 {
            game.draw_card(PlayerId(p));
        }
    }
    game.begin_mulligans();
}

/// Pass priority for whichever player currently holds it until `predicate` holds,
/// rolling the game forward through steps. On declare attackers, submits
/// [`Game::required_attacks`] (empty when nothing is forced) so goad cannot wedge
/// an all-pass loop. (CR 701.38, CR 117)
pub fn advance_until(game: &mut Game, predicate: impl Fn(&Game) -> bool) {
    let mut guard = 0;
    while !predicate(game) {
        if let Some(PendingChoice::DeclineUntap {
            player,
            at_most_one,
            ..
        }) = game.pending_choice()
        {
            // Neutral default: untap everything (Rubinia Soulsinger's optional-untap pause), minus
            // whatever a Smoke/Winter Orb cap forbids — the first of each capped group comes up and
            // the rest stay tapped. A test that wants a different answer stops on this choice via
            // its predicate and answers it itself before advancing further.
            let keep_tapped = at_most_one
                .iter()
                .flat_map(|group| group.iter().skip(1).copied())
                .collect();
            game.submit(Intent::DeclineUntap {
                player,
                keep_tapped,
            })
            .unwrap();
        } else if game.current_step() == Step::DeclareAttackers && !game.attackers_declared() {
            // Whoever declares this turn — the active player, unless a live Master Warcraft moved
            // the choice; the attackers on offer are the active player's either way.
            let player = game.attack_declarer();
            let attackers = game.required_attacks(game.active_player());
            game.submit(Intent::DeclareAttackers { player, attackers })
                .expect("required_attacks must be a legal declaration");
        } else {
            let p = game.priority_holder();
            game.submit(Intent::PassPriority { player: p }).unwrap();
        }
        guard += 1;
        assert!(guard < 1000, "did not reach the target within a sane bound");
    }
}

/// Everyone passes priority until the active player changes (one whole turn elapses). (CR 117, CR 500)
pub fn pass_until_next_turn(game: &mut Game) {
    let start = game.active_player();
    advance_until(game, |g| g.active_player() != start);
}

pub fn pool_total(game: &Game, player: PlayerId) -> u32 {
    let colored: u32 = [
        Color::White,
        Color::Blue,
        Color::Black,
        Color::Red,
        Color::Green,
    ]
    .into_iter()
    .map(|c| game.mana_in_pool(player, c) as u32)
    .sum();
    colored + game.colorless_in_pool(player) as u32
}

/// Resolve the top of the stack by having every seat pass in succession (CR 117.4).
pub fn resolve_top_of_stack(game: &mut Game) {
    for _ in 0..game.player_count() {
        game.submit(Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
    }
}

/// Advance to declare attackers and swing with player 0's `attackers` at player 1.
pub fn attack_with(game: &mut Game, attackers: Vec<ObjectId>) {
    advance_until(game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: attackers
            .into_iter()
            .map(|a| (a, Defender::Player(PlayerId(1))))
            .collect(),
    })
    .unwrap();
}

/// Advance to declare blockers and declare `blocks` for player 1.
pub fn block_with(
    game: &mut Game,
    blocks: Vec<(ObjectId, ObjectId)>,
) -> Result<Vec<Event>, Reject> {
    advance_until(game, |g| g.current_step() == Step::DeclareBlockers);
    game.submit(Intent::DeclareBlockers {
        player: PlayerId(1),
        blocks,
    })
}

/// Tap `count` freshly-spawned basic lands of `name` for player 0, leaving `count` mana in the pool.
pub fn tap_basics(game: &mut Game, name: &str, count: usize) {
    for _ in 0..count {
        let land = game.spawn_on_battlefield(PlayerId(0), card(name));
        game.submit(Intent::TapForMana {
            player: PlayerId(0),
            object: land,
        })
        .unwrap();
    }
}
