//! Legends (`leg`) grind, wave 12 slice E — costs, exile association and pending-choice loops.
//!
//! Increments 109 (exile the source from the graveyard), 41 (delayed chosen landwalk),
//! 86 (Voodoo Doll), 52 (exiled with this, face down), 76 (sacrifice any number as a cost)
//! and 33 (Eureka's round-robin).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
        for _ in 0..60 {
            game.spawn_in_library(PlayerId(player), card("Mountain"));
        }
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step, which is all a non-active seat needs to act at instant speed.
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

fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
    sacrifice: Vec<ObjectId>,
    x: u32,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index,
        target,
        sacrifice,
        discard_cost: vec![],
        x,
    })
}

/// Roll around to `PlayerId(0)`'s own next main phase — [`Game::new`] opens mid-Main1, so this is
/// the only way a test reaches an upkeep step or an untap step at all.
fn own_next_main(game: &mut Game) {
    for _ in 0..game.player_count() {
        pass_until_next_turn(game);
    }
    advance_until(game, |g| g.current_step() == Step::Main1);
}

// ── increment 109: exile the source from the graveyard ────────────────────────────────

#[test]
fn cyclopean_mummy_exiles_itself_out_of_the_graveyard_when_it_dies() {
    // "When this creature dies, exile it." — CR 603.6c: the dies trigger looks back at the
    // battlefield object, but "it" is the card that is now in the graveyard.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let mummy = game.spawn_on_battlefield(PlayerId(0), card("Cyclopean Mummy"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(mummy)));

    assert_eq!(
        game.zone_of(mummy),
        Zone::Graveyard,
        "the 2/1 dies to three damage and the dies trigger is on the stack",
    );

    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(mummy),
        Zone::Exile,
        "resolving the trigger exiles the card it became in the graveyard",
    );
}

// ── increment 86: Voodoo Doll ─────────────────────────────────────────────────────────

/// Activate the Doll's `{X}{X}, {T}` ability. No mana is put in the pool first — the payment
/// auto-taps whatever untapped lands the seat has, so counting what it left untapped is how these
/// tests read the price the cost actually charged.
fn stab(game: &mut Game, doll: ObjectId) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: doll,
        ability_index: 2,
        target: Some(Target::Player(PlayerId(1))),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

fn add_mountains(game: &mut Game, into: &mut Vec<ObjectId>, count: usize) {
    for _ in 0..count {
        into.push(game.spawn_on_battlefield(PlayerId(0), card("Mountain")));
    }
}

fn untapped(game: &Game, lands: &[ObjectId]) -> usize {
    lands.iter().filter(|&&id| !game.is_tapped(id)).count()
}

#[test]
fn voodoo_dolls_defined_x_charges_two_mana_per_pin_counter() {
    // "{X}{X}, {T}: … X is the number of pin counters on this artifact." (CR 107.3b — the
    // activator announces nothing, so the `x` on the intent is ignored.)
    let mut game = Game::new();
    stock_libraries(&mut game);
    let doll = game.spawn_on_battlefield(PlayerId(0), card("Voodoo Doll"));
    let mut lands = Vec::new();

    // No counters yet, so this costs {0} — and taps the Doll, which is what carries it through
    // its own end step to the upkeep that starts counting.
    stab(&mut game, doll).expect("{X}{X} with no pin counters is free");
    resolve_top_of_stack(&mut game);

    own_next_main(&mut game);
    assert_eq!(
        game.counters_of_kind(doll, CounterKind::Pin),
        1,
        "the upkeep trigger put the first pin counter on",
    );
    add_mountains(&mut game, &mut lands, 3);
    stab(&mut game, doll).expect("three lands cover the {2} one pin counter defines");
    assert_eq!(untapped(&game, &lands), 1, "and it charged exactly {{2}}");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.life(PlayerId(1)), 19, "one counter, one damage");

    own_next_main(&mut game);
    assert_eq!(
        game.counters_of_kind(doll, CounterKind::Pin),
        2,
        "a second upkeep, a second pin counter",
    );
    add_mountains(&mut game, &mut lands, 2);
    stab(&mut game, doll).expect("five lands cover the {4} two pin counters define");
    assert_eq!(
        untapped(&game, &lands),
        1,
        "the same ability now charges {{4}} — two mana per pin counter, unannounced",
    );
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        17,
        "two counters, two damage — the amount tracks the counters, not the announced X",
    );
}

#[test]
fn voodoo_doll_cannot_be_activated_below_its_defined_x() {
    // An uncompletable cost makes the activation illegal (CR 602.2b), and the Doll's own counters
    // are what set the bar.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let doll = game.spawn_on_battlefield(PlayerId(0), card("Voodoo Doll"));
    let mut lands = Vec::new();

    stab(&mut game, doll).expect("free at zero pin counters");
    resolve_top_of_stack(&mut game);
    own_next_main(&mut game);

    add_mountains(&mut game, &mut lands, 1);
    assert!(
        stab(&mut game, doll).is_err(),
        "one land cannot pay the {{2}} one pin counter defines",
    );
    assert_eq!(untapped(&game, &lands), 1, "and nothing was paid");
}

#[test]
fn an_untapped_voodoo_doll_turns_on_its_controller_at_end_of_turn() {
    // "At the beginning of your end step, if this artifact is untapped, destroy this artifact and
    // it deals damage to you equal to the number of pin counters on it."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let doll = game.spawn_on_battlefield(PlayerId(0), card("Voodoo Doll"));
    stab(&mut game, doll).expect("tap it to survive the turn it arrived");
    resolve_top_of_stack(&mut game);

    own_next_main(&mut game);
    assert_eq!(game.counters_of_kind(doll, CounterKind::Pin), 1);

    // Left untapped this time.
    advance_until(&mut game, |g| g.zone_of(doll) != Zone::Battlefield);

    assert_eq!(
        game.life(PlayerId(0)),
        19,
        "an untapped Doll deals its one pin counter to its controller",
    );
    assert_eq!(game.zone_of(doll), Zone::Graveyard, "and is destroyed");
}

#[test]
fn a_tapped_voodoo_doll_is_safe_at_end_of_turn() {
    // The intervening-if (CR 603.4): tapping the Doll for its damage ability is what keeps it —
    // and you — alive through your own end step.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let doll = game.spawn_on_battlefield(PlayerId(0), card("Voodoo Doll"));
    stab(&mut game, doll).expect("{X}{X} with no pin counters is free");
    resolve_top_of_stack(&mut game);
    assert!(game.is_tapped(doll), "the ability taps the Doll");

    pass_until_next_turn(&mut game);

    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "a tapped Doll's end-step trigger never fires",
    );
    assert_eq!(game.zone_of(doll), Zone::Battlefield);
}

// ── increment 76: sacrifice any number as a cost ──────────────────────────────────────

/// Sword of the Ages enters tapped, so it can only be activated from the turn after it lands.
fn untapped_sword(game: &mut Game) -> ObjectId {
    let sword = game.spawn_on_battlefield(PlayerId(0), card("Sword of the Ages"));
    own_next_main(game);
    assert!(!game.is_tapped(sword), "the untap step freed it");
    sword
}

#[test]
fn sword_of_the_ages_throws_the_total_power_of_everything_it_ate() {
    // "{T}, Sacrifice this artifact and any number of creatures you control: This artifact deals
    // X damage to any target, where X is the total power of the creatures sacrificed this way,
    // then exile this artifact and those creature cards."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let sword = untapped_sword(&mut game);
    let grizzly = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));

    activate(
        &mut game,
        PlayerId(0),
        sword,
        0,
        Some(Target::Player(PlayerId(1))),
        vec![grizzly, giant],
        0,
    )
    .expect("the tap and the sacrifices are the whole cost");

    assert_eq!(
        (game.zone_of(sword), game.zone_of(grizzly)),
        (Zone::Graveyard, Zone::Graveyard),
        "a sacrifice cost is paid on activation, before the ability resolves",
    );

    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(1)),
        15,
        "2 (Grizzly Bears) + 3 (Hill Giant) total power",
    );
    assert_eq!(
        (
            game.zone_of(sword),
            game.zone_of(grizzly),
            game.zone_of(giant)
        ),
        (Zone::Exile, Zone::Exile, Zone::Exile),
        "the sword and the creature cards it ate leave the graveyard for exile",
    );
}

#[test]
fn sword_of_the_ages_may_be_activated_with_nothing_to_throw() {
    // "any number" includes zero (CR 601.2f) — it still sacrifices and exiles itself for 0 damage.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let sword = untapped_sword(&mut game);

    activate(
        &mut game,
        PlayerId(0),
        sword,
        0,
        Some(Target::Player(PlayerId(1))),
        vec![],
        0,
    )
    .expect("zero sacrifices is a legal payment");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.life(PlayerId(1)), 20, "no creatures, no damage");
    assert_eq!(game.zone_of(sword), Zone::Exile);
}

#[test]
fn sword_of_the_ages_cant_eat_a_creature_someone_else_controls() {
    // "creatures **you control**" — CR 701.17, a sacrifice cost only reaches your own permanents.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let sword = untapped_sword(&mut game);
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    let rejected = activate(
        &mut game,
        PlayerId(0),
        sword,
        0,
        Some(Target::Player(PlayerId(1))),
        vec![theirs],
        0,
    );

    assert!(
        rejected.is_err(),
        "an opponent's creature can't pay the cost",
    );
    assert_eq!(
        game.zone_of(sword),
        Zone::Battlefield,
        "and nothing is paid"
    );
}

// ── increment 41: delayed chosen landwalk ─────────────────────────────────────────────

#[test]
fn giant_slug_picks_its_landwalk_type_at_the_upkeep_the_delayed_trigger_fires() {
    // "{5}: At the beginning of your next upkeep, choose a basic land type. This creature gains
    // landwalk of the chosen type until the end of that turn." — CR 603.7: nothing happens on
    // activation beyond scheduling; the pick and the grant both wait for that upkeep.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let slug = game.spawn_on_battlefield(PlayerId(0), card("Giant Slug"));

    activate(&mut game, PlayerId(0), slug, 0, None, vec![], 0).unwrap();
    resolve_top_of_stack(&mut game);

    assert!(
        !game.has_keyword(slug, Keyword::Landwalk(BasicLandType::Swamp)),
        "the activation only schedules — no landwalk yet, and nothing to choose",
    );
    assert!(
        game.pending_choice().is_none(),
        "the basic land type is chosen as the delayed trigger resolves, not on activation",
    );

    // Roll to player 0's own next upkeep: the delayed trigger fires there, goes on the stack and
    // pauses on the basic-land-type pick as it resolves.
    advance_until(&mut game, |g| g.pending_choice().is_some());
    game.submit(Intent::ChooseCreatureType {
        player: PlayerId(0),
        subtype: "Swamp".to_string(),
    })
    .unwrap();

    assert!(
        game.has_keyword(slug, Keyword::Landwalk(BasicLandType::Swamp)),
        "the rest of the trigger grants landwalk of the type just chosen",
    );
    assert!(
        !game.has_keyword(slug, Keyword::Landwalk(BasicLandType::Forest)),
        "and only that type",
    );

    pass_until_next_turn(&mut game);

    assert!(
        !game.has_keyword(slug, Keyword::Landwalk(BasicLandType::Swamp)),
        "'until the end of that turn' — the grant is gone the following turn",
    );
}

// ── increment 52: exiled with this, face down ─────────────────────────────────────────

/// Feed the Vault the top card of player 0's library and hand back that card's id. One helping per
/// turn — the ability taps the Vault.
fn vault_swallows_top(game: &mut Game, vault: ObjectId) -> ObjectId {
    let top = game.library_top(PlayerId(0)).expect("a stocked library");
    activate(game, PlayerId(0), vault, 0, None, vec![], 0).unwrap();
    resolve_top_of_stack(game);
    assert_eq!(
        game.zone_of(top),
        Zone::Exile,
        "'{{2}}, {{T}}: Exile the top card of your library face down'",
    );
    assert!(
        game.is_card_face_down(top),
        "face down — nobody but its owner may read it in the pile",
    );
    top
}

#[test]
fn knowledge_vault_trades_your_hand_for_everything_it_swallowed() {
    // "{0}: Sacrifice this artifact. If you do, discard your hand, then put all cards exiled with
    // this artifact into their owner's hand." — CR 400.10a's "exiled with" association.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vault = game.spawn_on_battlefield(PlayerId(0), card("Knowledge Vault"));
    let held = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    let swallowed = vault_swallows_top(&mut game, vault);

    activate(&mut game, PlayerId(0), vault, 1, None, vec![], 0).unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(vault),
        Zone::Graveyard,
        "the Vault sacrifices itself to cash out",
    );
    assert_eq!(
        game.zone_of(held),
        Zone::Graveyard,
        "the hand you were holding is discarded first",
    );
    assert_eq!(
        game.zone_of(swallowed),
        Zone::Hand,
        "and only what the Vault swallowed is left in hand",
    );
    assert_eq!(game.hand(PlayerId(0)).len(), 1);
}

#[test]
fn shattering_knowledge_vault_buries_everything_it_swallowed() {
    // "When this artifact leaves the battlefield, put all cards exiled with it into their owner's
    // graveyard." — the punishment for losing the Vault rather than cashing it in.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let vault = game.spawn_on_battlefield(PlayerId(0), card("Knowledge Vault"));
    let shatter = game.spawn_in_hand(PlayerId(1), card("Shatter"));

    let swallowed = vault_swallows_top(&mut game, vault);

    cast_and_resolve(&mut game, PlayerId(1), shatter, Some(Target::Object(vault)));

    assert_eq!(
        game.zone_of(vault),
        Zone::Graveyard,
        "Shatter destroys the Vault and its leaves-the-battlefield trigger goes on the stack",
    );

    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(swallowed),
        Zone::Graveyard,
        "the exiled card is buried, not returned",
    );
    assert!(game.hand(PlayerId(0)).is_empty());
}

// ── increment 33: Eureka's round-robin ────────────────────────────────────────────────

/// Whoever the live "put a permanent from your hand" offer belongs to, or `None` when the
/// round-robin has closed.
fn offered_to(game: &Game) -> Option<PlayerId> {
    match game.pending_choice() {
        Some(PendingChoice::PutCreatureFromHand { player, .. }) => Some(player),
        _ => None,
    }
}

fn answer_offer(game: &mut Game, player: PlayerId, choice: Option<ObjectId>) {
    game.submit(Intent::PutCreatureFromHand { player, choice })
        .unwrap();
}

#[test]
fn eureka_keeps_going_around_until_a_whole_lap_passes_with_nobody_acting() {
    // "Starting with you, each player may put a permanent card from their hand onto the
    // battlefield. Repeat this process until no one puts a card onto the battlefield."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let eureka = game.spawn_in_hand(PlayerId(0), card("Eureka"));
    let lotus = game.spawn_in_hand(PlayerId(0), card("Black Lotus"));
    let ring = game.spawn_in_hand(PlayerId(0), card("Sol Ring"));
    let giant = game.spawn_in_hand(PlayerId(1), card("Hill Giant"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    cast_and_resolve(&mut game, PlayerId(0), eureka, None);

    // Lap one: the caster first (CR 101.4), then the next seat with something to offer.
    assert_eq!(offered_to(&game), Some(PlayerId(0)), "starting with you");
    answer_offer(&mut game, PlayerId(0), Some(lotus));

    let Some(PendingChoice::PutCreatureFromHand {
        player, candidates, ..
    }) = game.pending_choice()
    else {
        panic!("the next seat is offered the same choice");
    };
    assert_eq!(player, PlayerId(1));
    assert_eq!(
        candidates,
        vec![giant],
        "a permanent card — the Bolt in the same hand is not one",
    );
    answer_offer(&mut game, PlayerId(1), Some(giant));

    // Seats 2 and 3 hold nothing, so the lap runs straight back to the caster, who still has one
    // permanent left — the process repeated because somebody acted.
    assert_eq!(
        offered_to(&game),
        Some(PlayerId(0)),
        "somebody acted, so the process repeats",
    );
    answer_offer(&mut game, PlayerId(0), None);

    assert_eq!(
        offered_to(&game),
        None,
        "a whole lap with nobody acting ends it",
    );
    assert_eq!(game.zone_of(lotus), Zone::Battlefield);
    assert_eq!(game.zone_of(giant), Zone::Battlefield);
    assert_eq!(game.zone_of(ring), Zone::Hand, "declined, so it stays");
    assert_eq!(game.zone_of(bolt), Zone::Hand, "never on offer");
}

#[test]
fn eureka_with_nothing_worth_putting_out_resolves_and_does_nothing() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let eureka = game.spawn_in_hand(PlayerId(0), card("Eureka"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    cast_and_resolve(&mut game, PlayerId(0), eureka, None);

    assert_eq!(
        offered_to(&game),
        None,
        "no seat holds a permanent card, so nothing pauses",
    );
    assert_eq!(game.zone_of(bolt), Zone::Hand);
    assert_eq!(game.zone_of(eureka), Zone::Graveyard);
}
