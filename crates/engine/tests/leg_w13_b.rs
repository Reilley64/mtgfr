//! Legends (`leg`) grind, wave 13 slice B — permanent re-entry and aura shape.
//!
//! Increment 78 (`takklemaggot`): an Aura whose dies-trigger hands the new-host choice to the
//! *dead creature's controller* and, with no legal host anywhere, returns the Aura to the
//! battlefield anyway as a non-Aura enchantment that pings that player each upkeep.

mod common;

use common::*;
use engine::*;

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    give_priority(game, player);
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
    .unwrap();
    resolve_top_of_stack(game);
}

/// Stack both libraries deep enough that a handful of turns cannot deck anyone.
fn stock(game: &mut Game) {
    for p in 0..2u8 {
        game.stack_library(PlayerId(p), &vec![card("Plains"); 20]);
    }
}

/// Roll forward one whole turn and settle in its successor's first main phase — past the upkeep
/// whose triggers this card cares about.
fn next_turn_past_upkeep(game: &mut Game) {
    pass_until_next_turn(game);
    advance_until(game, |g| g.current_step() == Step::Main1);
}

// ── increment 78: Takklemaggot ────────────────────────────────────────────────────────

#[test]
fn takklemaggot_counters_the_host_on_its_own_controllers_upkeep_only() {
    // "At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on
    // that creature." The Aura's controller's upkeep is not that upkeep — only the host's
    // controller's is, and the counter is toughness-only.
    let mut game = Game::new();
    stock(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Takklemaggot"));
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(bears)));
    assert_eq!(game.attached_to(game.current_id(aura)), Some(bears));
    assert_eq!((game.power(bears), game.toughness(bears)), (2, 2));

    next_turn_past_upkeep(&mut game); // player 1's upkeep — the host's controller
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (2, 1),
        "a -0/-1 counter takes toughness only"
    );

    next_turn_past_upkeep(&mut game); // player 0's upkeep — the Aura's controller
    assert_eq!(
        (game.power(bears), game.toughness(bears)),
        (2, 1),
        "the Aura's own controller's upkeep is not the enchanted creature's controller's"
    );

    next_turn_past_upkeep(&mut game); // player 1 again
    assert_eq!(
        game.toughness(bears),
        0,
        "a second counter on the host's controller's next upkeep"
    );
    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "zero toughness is lethal (CR 704.5a)"
    );
}

#[test]
fn takklemaggot_hands_the_new_host_choice_to_the_dead_creatures_controller() {
    // "When enchanted creature dies, that creature's controller chooses a creature that this card
    // could enchant … return this card to the battlefield under your control attached to that
    // creature." The choice is the *dead creature's* controller's, not the Aura controller's, and
    // it is mandatory when any legal host exists — no matter who controls it.
    let mut game = Game::new();
    stock(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Takklemaggot"));
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(bears)));

    let terror = game.spawn_in_hand(PlayerId(0), card("Terror"));
    cast_and_resolve(&mut game, PlayerId(0), terror, Some(Target::Object(bears)));
    assert_eq!(game.zone_of(bears), Zone::Graveyard, "the host died");

    resolve_top_of_stack(&mut game); // the EnchantedCreatureDies trigger

    let Some(PendingChoice::ChooseAttachHost {
        player,
        attachment,
        candidates,
        optional,
    }) = game.pending_choice()
    else {
        panic!(
            "the dies-return pauses to choose a new host, got {:?}",
            game.pending_choice()
        );
    };
    assert_eq!(
        player,
        PlayerId(1),
        "the dead creature's controller chooses, not the Aura's"
    );
    assert_eq!(
        candidates,
        vec![mine],
        "any creature this card could enchant, whoever controls it"
    );
    assert!(
        !optional,
        "\"if the player can choose a creature, they must\""
    );

    game.submit(Intent::ChooseAttachHost {
        player,
        host: Some(mine),
    })
    .unwrap();

    assert_eq!(game.zone_of(attachment), Zone::Battlefield);
    assert_eq!(game.attached_to(attachment), Some(mine));
    assert_eq!(
        game.controller_of(attachment),
        PlayerId(0),
        "\"under your control\" — the Aura's controller does not change"
    );
    assert!(
        game.effective_subtypes(attachment).contains(&"Aura"),
        "it came back attached, so it is still an Aura"
    );

    next_turn_past_upkeep(&mut game); // player 1's upkeep — no longer any host of theirs
    assert_eq!(
        game.toughness(mine),
        3,
        "the dead host's controller's upkeep is nothing to the Aura now"
    );

    next_turn_past_upkeep(&mut game); // player 0's upkeep — the new host's controller
    assert_eq!(
        game.toughness(mine),
        2,
        "the upkeep counter follows the new host"
    );
}

#[test]
fn takklemaggot_returns_as_a_non_aura_enchantment_with_no_legal_host() {
    // "If they don't, return this card to the battlefield under your control as a non-Aura
    // enchantment. It loses \"enchant creature\" and gains \"At the beginning of that player's
    // upkeep, this enchantment deals 1 damage to that player.\"" With no creature left anywhere,
    // the choice cannot be made — so it returns hostless and starts pinging the dead creature's
    // controller instead of falling off to the graveyard (CR 704.5m does not apply: it is no
    // longer an Aura).
    let mut game = Game::new();
    stock(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Takklemaggot"));
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(bears)));

    let terror = game.spawn_in_hand(PlayerId(0), card("Terror"));
    cast_and_resolve(&mut game, PlayerId(0), terror, Some(Target::Object(bears)));
    assert_eq!(game.zone_of(bears), Zone::Graveyard, "the host died");

    resolve_top_of_stack(&mut game); // the EnchantedCreatureDies trigger
    assert_eq!(
        game.pending_choice(),
        None,
        "no creature on the battlefield — nothing to choose between"
    );

    let returned = game.current_id(aura);
    assert_eq!(
        game.zone_of(returned),
        Zone::Battlefield,
        "it returns anyway, unlike an ordinary transferrable Aura"
    );
    assert_eq!(game.attached_to(returned), None);
    assert_eq!(game.controller_of(returned), PlayerId(0));
    assert!(
        !game.effective_subtypes(returned).contains(&"Aura"),
        "a non-Aura enchantment"
    );

    next_turn_past_upkeep(&mut game); // player 1's upkeep — the dead creature's controller
    assert_eq!(
        game.life(PlayerId(1)),
        19,
        "1 damage to that player each of their upkeeps"
    );
    assert_eq!(
        game.zone_of(returned),
        Zone::Battlefield,
        "an unattached non-Aura enchantment is not swept (CR 704.5m)"
    );

    next_turn_past_upkeep(&mut game); // player 0's upkeep
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "the ping names that player, not each player"
    );

    next_turn_past_upkeep(&mut game); // player 1 again
    assert_eq!(game.life(PlayerId(1)), 18, "and again every upkeep after");
}
