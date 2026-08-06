//! Legends (`leg`) grind — increment 5: keyword removal.
//!
//! CR 613.1f puts "loses \[keyword\]" in the ability-adding-and-removing layer, ordered against the
//! grants it competes with by CR 613.7 timestamp: a removal beats every grant applied before it and
//! loses to every grant applied after it, whether the grant is printed, an Aura's, or an anthem's.
//!
//! Six cards: Hammerheim ("loses all landwalk abilities"), Radjan Spirit ("loses flying"), Tolaria
//! ("loses banding and all 'bands with other' abilities" — only during any upkeep step), Urborg
//! ("loses first strike or swampwalk" — CR 609.4's choice is made on resolution), Shelkin Brownie
//! ("loses all 'bands with other' abilities") and Elder Land Wurm, the one with no duration at all:
//! "When this creature blocks, it loses defender" is permanent.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 20]);
    }
}

/// Roll forward to player 0's *next* upkeep (the constructor parks at Main1).
fn advance_to_your_next_upkeep(game: &mut Game) {
    pass_until_next_turn(game);
    advance_until(game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Upkeep
    });
}

/// Hand priority to player 1 so they can activate something on player 0's turn.
fn give_priority_to_opponent(game: &mut Game) {
    advance_until(game, |g| g.priority_holder() == PlayerId(1));
}

/// `player` activates `object`'s printed ability at `ability_index` aiming at `target`.
fn tap_ability(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    ability_index: usize,
    target: ObjectId,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index,
        target: Some(Target::Object(target)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// Player 1 activates `source`'s strip ability at `target` and lets it resolve.
fn strip_with(game: &mut Game, source: ObjectId, target: ObjectId) {
    give_priority_to_opponent(game);
    tap_ability(game, PlayerId(1), source, 0, target).expect("the strip ability is activatable");
    resolve_top_of_stack(game);
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

// ── Elder Land Wurm: "Defender, trample / When this creature blocks, it loses defender." ──

#[test]
fn elder_land_wurm_cannot_attack_while_it_still_has_defender() {
    let mut game = Game::new();
    let wurm = game.spawn_on_battlefield(PlayerId(0), card("Elder Land Wurm"));

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    assert_eq!(
        game.submit(Intent::DeclareAttackers {
            player: PlayerId(0),
            attackers: vec![(wurm, Defender::Player(PlayerId(1)))],
        }),
        Err(Reject::IllegalDeclaration),
        "CR 702.3b: a creature with defender can't attack"
    );
}

#[test]
fn elder_land_wurm_loses_defender_for_good_once_it_blocks() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wurm = game.spawn_on_battlefield(PlayerId(1), card("Elder Land Wurm"));

    attack_with(&mut game, vec![bears]);
    block_with(&mut game, vec![(wurm, bears)]).expect("defender does not stop a creature blocking");
    resolve_top_of_stack(&mut game);

    // The loss has no duration, so it survives the cleanup step that ends every other one this
    // grind adds: on player 1's own turn the Wurm swings for 5.
    let before = game.life(PlayerId(0));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(wurm, Defender::Player(PlayerId(0)))],
    })
    .expect("the Wurm lost defender when it blocked, so it can attack now");
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.life(PlayerId(0)),
        before - 5,
        "the 5/5 Wurm connected — the defender loss outlived the turn it blocked on"
    );
}

// ── Radjan Spirit: "{T}: Target creature loses flying until end of turn." ──────────────

#[test]
fn radjan_spirit_strips_flying_so_a_ground_creature_can_block() {
    let mut game = Game::new();
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Air Elemental"));
    let spirit = game.spawn_on_battlefield(PlayerId(1), card("Radjan Spirit"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![elemental]);
    strip_with(&mut game, spirit, elemental);

    block_with(&mut game, vec![(bears, elemental)])
        .expect("the Air Elemental lost flying, so a ground creature may block it");
}

#[test]
fn a_grounded_flier_is_airborne_again_next_turn() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Air Elemental"));
    let spirit = game.spawn_on_battlefield(PlayerId(1), card("Radjan Spirit"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![elemental]);
    strip_with(&mut game, spirit, elemental);
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::Upkeep
    });
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });

    attack_with(&mut game, vec![elemental]);
    assert!(
        block_with(&mut game, vec![(bears, elemental)]).is_err(),
        "\"until end of turn\" — the flying is back on the Elemental's next attack"
    );
}

// ── Hammerheim: "{T}: Target creature loses all landwalk abilities until end of turn." ──

/// P0 attacks with `walker`; P1 controls a Hammerheim, a `basic` of the walked type and a blocker.
fn hammerheim_board(walker: &str, basic: &str) -> (Game, ObjectId, ObjectId, ObjectId) {
    let mut game = Game::new();
    let attacker = game.spawn_on_battlefield(PlayerId(0), card(walker));
    let hammerheim = game.spawn_on_battlefield(PlayerId(1), card("Hammerheim"));
    game.spawn_on_battlefield(PlayerId(1), card(basic));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    (game, attacker, hammerheim, bears)
}

#[test]
fn hammerheim_strips_swampwalk() {
    let (mut game, wraith, _hammerheim, bears) = hammerheim_board("Bog Wraith", "Swamp");

    attack_with(&mut game, vec![wraith]);
    assert!(
        block_with(&mut game, vec![(bears, wraith)]).is_err(),
        "CR 702.14b: swampwalk is unblockable while the defender controls a Swamp"
    );

    let (mut game, wraith, hammerheim, bears) = hammerheim_board("Bog Wraith", "Swamp");
    attack_with(&mut game, vec![wraith]);
    strip_with(&mut game, hammerheim, wraith);

    block_with(&mut game, vec![(bears, wraith)])
        .expect("the Wraith lost all landwalk abilities, so the Swamp no longer waves it through");
}

#[test]
fn hammerheim_strips_mountainwalk_too() {
    // "All landwalk abilities" is a family, not a list: the same activation that grounds a
    // swampwalker grounds a mountainwalker.
    let (mut game, yeti, hammerheim, bears) = hammerheim_board("Mountain Yeti", "Mountain");

    attack_with(&mut game, vec![yeti]);
    strip_with(&mut game, hammerheim, yeti);

    block_with(&mut game, vec![(bears, yeti)])
        .expect("the Yeti lost all landwalk abilities, so the Mountain no longer waves it through");
}

// ── Shelkin Brownie: "{T}: Target creature loses all 'bands with other' abilities …" ────

#[test]
fn shelkin_brownie_strips_a_bands_with_other_the_cathedral_granted() {
    // The interesting layer case: the keyword is not printed on Jasmine Boreal at all — it comes
    // from Cathedral of Serra's anthem, applied earlier by timestamp, so the strip still beats it.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Cathedral of Serra"));
    let jasmine = game.spawn_on_battlefield(PlayerId(0), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let brownie = game.spawn_on_battlefield(PlayerId(1), card("Shelkin Brownie"));

    strip_with(&mut game, brownie, jasmine);

    assert!(
        attack_in_bands(
            &mut game,
            &[jasmine, barktooth],
            vec![vec![jasmine, barktooth]]
        )
        .is_err(),
        "CR 702.22c: with the granted \"bands with other legendary creatures\" gone, no member of \
         the band has it, so the band is not a legal declaration"
    );
}

#[test]
fn an_anthem_arriving_after_the_strip_grants_the_band_anyway() {
    // CR 613.7 the other way round: a grant with a *later* timestamp than the removal wins. A
    // "lose … and can't have" reading of the same strip would keep the band illegal.
    let mut game = Game::new();
    let jasmine = game.spawn_on_battlefield(PlayerId(0), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let brownie = game.spawn_on_battlefield(PlayerId(1), card("Shelkin Brownie"));

    strip_with(&mut game, brownie, jasmine);
    game.spawn_on_battlefield(PlayerId(0), card("Cathedral of Serra"));

    attack_in_bands(
        &mut game,
        &[jasmine, barktooth],
        vec![vec![jasmine, barktooth]],
    )
    .expect("the Cathedral's grant is newer than the strip, so Jasmine bands again");
}

// ── Tolaria: "{T}: Target creature loses banding and all 'bands with other' abilities …" ─

#[test]
fn tolaria_strips_printed_banding() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Timber Wolves"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let tolaria = game.spawn_on_battlefield(PlayerId(1), card("Tolaria"));

    advance_to_your_next_upkeep(&mut game);
    strip_with(&mut game, tolaria, wolves);

    assert!(
        attack_in_bands(&mut game, &[wolves, bears], vec![vec![wolves, bears]]).is_err(),
        "CR 702.22c: without banding on the Wolves there is nothing to band the Bears to"
    );
}

#[test]
fn tolaria_strips_bands_with_other_in_the_same_activation() {
    // "Banding **and** all 'bands with other' abilities" — one activation, two different things.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Cathedral of Serra"));
    let jasmine = game.spawn_on_battlefield(PlayerId(0), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    let tolaria = game.spawn_on_battlefield(PlayerId(1), card("Tolaria"));

    advance_to_your_next_upkeep(&mut game);
    strip_with(&mut game, tolaria, jasmine);

    assert!(
        attack_in_bands(
            &mut game,
            &[jasmine, barktooth],
            vec![vec![jasmine, barktooth]]
        )
        .is_err(),
        "the granted \"bands with other legendary creatures\" went with the banding"
    );
}

#[test]
fn tolaria_cannot_be_activated_outside_an_upkeep_step() {
    let mut game = Game::new();
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Timber Wolves"));
    let tolaria = game.spawn_on_battlefield(PlayerId(1), card("Tolaria"));

    // The constructor parks at Main1 — an ordinary activation window for anything else.
    give_priority_to_opponent(&mut game);
    assert_eq!(
        tap_ability(&mut game, PlayerId(1), tolaria, 0, wolves),
        Err(Reject::CannotActivate),
        "\"Activate only during any upkeep step\""
    );
}

#[test]
fn tolaria_may_be_activated_during_an_opponents_upkeep() {
    // "Any upkeep step", not "your upkeep": Tolaria's controller is the non-active player here.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Timber Wolves"));
    let tolaria = game.spawn_on_battlefield(PlayerId(1), card("Tolaria"));

    advance_to_your_next_upkeep(&mut game);
    give_priority_to_opponent(&mut game);

    tap_ability(&mut game, PlayerId(1), tolaria, 0, wolves)
        .expect("player 0's upkeep is an upkeep step");
}

// ── Urborg: "{T}: Target creature loses first strike or swampwalk until end of turn." ───

/// The pending mode choice's owner and how many modes it offers.
fn pending_modes(game: &Game) -> (PlayerId, usize) {
    let Some(PendingChoice::ChooseMode { player, modes, .. }) = game.pending_choice() else {
        panic!(
            "Urborg's \"first strike or swampwalk\" should pause on a mode choice; got {:?}",
            game.pending_choice()
        );
    };
    (player, modes.len())
}

#[test]
fn urborg_asks_its_controller_which_keyword_on_resolution() {
    let mut game = Game::new();
    let wraith = game.spawn_on_battlefield(PlayerId(0), card("Bog Wraith"));
    let urborg = game.spawn_on_battlefield(PlayerId(1), card("Urborg"));

    give_priority_to_opponent(&mut game);
    tap_ability(&mut game, PlayerId(1), urborg, 0, wraith)
        .expect("Urborg's ability is activatable");
    // CR 609.4: the choice belongs to resolution, not activation — the ability sits on the stack
    // with its target already locked and nothing chosen yet.
    assert!(
        game.pending_choice().is_none(),
        "nothing is chosen while the ability is still on the stack"
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        pending_modes(&game),
        (PlayerId(1), 2),
        "the ability's controller picks one of the two keywords"
    );
}

#[test]
fn urborg_can_take_the_swampwalk() {
    let mut game = Game::new();
    let wraith = game.spawn_on_battlefield(PlayerId(0), card("Bog Wraith"));
    let urborg = game.spawn_on_battlefield(PlayerId(1), card("Urborg"));
    game.spawn_on_battlefield(PlayerId(1), card("Swamp"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![wraith]);
    give_priority_to_opponent(&mut game);
    tap_ability(&mut game, PlayerId(1), urborg, 0, wraith)
        .expect("Urborg's ability is activatable");
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseMode {
        player: PlayerId(1),
        mode: 1,
    })
    .expect("swampwalk is the second of the two modes");

    block_with(&mut game, vec![(bears, wraith)])
        .expect("the Wraith lost swampwalk, so the Swamp no longer waves it through");
}

#[test]
fn urborg_can_take_the_first_strike() {
    let mut game = Game::new();
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Tundra Wolves"));
    let urborg = game.spawn_on_battlefield(PlayerId(1), card("Urborg"));
    let mystic = game.spawn_on_battlefield(PlayerId(1), card("Elvish Mystic"));

    attack_with(&mut game, vec![wolves]);
    give_priority_to_opponent(&mut game);
    tap_ability(&mut game, PlayerId(1), urborg, 0, wolves)
        .expect("Urborg's ability is activatable");
    resolve_top_of_stack(&mut game);
    game.submit(Intent::ChooseMode {
        player: PlayerId(1),
        mode: 0,
    })
    .expect("first strike is the first of the two modes");
    block_with(&mut game, vec![(mystic, wolves)]).expect("a 1/1 may block a 1/1");
    advance_until(&mut game, |g| g.current_step() == Step::End);

    assert_eq!(
        game.zone_of(wolves),
        Zone::Graveyard,
        "without first strike the two 1/1s trade instead of the attacker walking away"
    );
    assert_eq!(
        game.zone_of(mystic),
        Zone::Graveyard,
        "the blocker trades too"
    );
}
