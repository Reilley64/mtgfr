//! Legends (`leg`) grind, wave 11 slice A — Auras: moving them, counting them, gating them.
//!
//! Increments 30 (`move-attached-aura`), 67 (`pump-per-attached-aura`),
//! 117 (`grant-to-attached-under-a-condition`), 20 (`damage-triggered-aura-payback`),
//! 71 (`remove-enchantments`) and 65 (`puppet-master`).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

fn stock_libraries(game: &mut Game) {
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &vec![card("Grizzly Bears"); 40]);
    }
}

/// Hand priority to `player`: with an empty stack a single pass moves it along without advancing
/// the step.
fn give_priority(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

fn cast(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
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
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    cast(game, player, object, target).unwrap();
    resolve_top_of_stack(game);
}

/// Cast `aura` from `player`'s hand onto `host` and let it resolve, returning the Aura permanent.
fn enchant(game: &mut Game, player: PlayerId, aura: &str, host: ObjectId) -> ObjectId {
    let spell = game.spawn_in_hand(player, card(aura));
    cast_and_resolve(game, player, spell, Some(Target::Object(host)));
    game.current_id(spell)
}

/// Hand the turn to the next seat and stop in its precombat main phase, where an Aura (a sorcery
/// -speed cast, CR 307.1) is legal.
fn next_turn_main(game: &mut Game) {
    pass_until_next_turn(game);
    advance_until(game, |g| g.current_step() == Step::Main1);
}

/// The legal targets currently offered by a pending `ChooseTarget`, with its clause index.
fn pending_targets(game: &Game) -> (u8, Vec<Target>) {
    match game.pending_choice() {
        Some(PendingChoice::ChooseTarget { clause, legal, .. }) => (clause, legal),
        other => panic!("expected a target choice, got {other:?}"),
    }
}

fn choose(game: &mut Game, player: PlayerId, targets: Vec<Target>) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ChooseTargets { player, targets })
}

// ── increment 30: Enchantment Alteration ──────────────────────────────────────────────

#[test]
fn enchantment_alteration_moves_an_aura_to_another_creature() {
    // "Attach target Aura attached to a creature or land to another permanent of that type."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let first = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let third = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    let creature_aura = enchant(&mut game, PlayerId(0), "Burrowing", first);
    let land_aura = enchant(&mut game, PlayerId(0), "Fertile Ground", forest);
    assert_eq!(
        game.attached_to(creature_aura),
        Some(first),
        "Burrowing starts here",
    );

    let spell = game.spawn_in_hand(PlayerId(0), card("Enchantment Alteration"));
    cast(&mut game, PlayerId(0), spell, None).expect("both clauses are chosen on the stack");

    let (clause, legal) = pending_targets(&game);
    assert_eq!(clause, 0, "clause 0 is the Aura");
    assert_eq!(legal.len(), 2, "both attached Auras qualify: {legal:?}");
    assert!(legal.contains(&Target::Object(creature_aura)));
    assert!(legal.contains(&Target::Object(land_aura)));
    choose(&mut game, PlayerId(0), vec![Target::Object(creature_aura)]).expect("the Aura is legal");

    let (clause, legal) = pending_targets(&game);
    assert_eq!(clause, 1, "clause 1 is the new host");
    assert_eq!(
        legal,
        vec![Target::Object(second), Target::Object(third)],
        "'another permanent of that type' — the other creatures, never the host, never the land",
    );
    choose(&mut game, PlayerId(0), vec![Target::Object(second)]).expect("the other bear is legal");

    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.attached_to(creature_aura),
        Some(second),
        "the Aura moved to the second creature",
    );
}

#[test]
fn enchantment_alteration_keeps_a_land_aura_on_lands() {
    // "…to another permanent of that type": an Aura on a land may only move to another land,
    // even with creatures on the battlefield.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let forest = game.spawn_on_battlefield(PlayerId(0), card("Forest"));
    let island = game.spawn_on_battlefield(PlayerId(0), card("Island"));
    let swamp = game.spawn_on_battlefield(PlayerId(0), card("Swamp"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let land_aura = enchant(&mut game, PlayerId(0), "Fertile Ground", forest);
    enchant(&mut game, PlayerId(0), "Burrowing", bear);

    let spell = game.spawn_in_hand(PlayerId(0), card("Enchantment Alteration"));
    cast(&mut game, PlayerId(0), spell, None).unwrap();
    choose(&mut game, PlayerId(0), vec![Target::Object(land_aura)]).expect("the Aura is legal");

    let (_, legal) = pending_targets(&game);
    assert_eq!(
        legal,
        vec![Target::Object(island), Target::Object(swamp)],
        "only the other lands — not the bear, and not the enchanted land itself",
    );
    assert!(
        !legal.contains(&Target::Object(bear)),
        "a creature is not 'a permanent of that type' for a land Aura",
    );
    choose(&mut game, PlayerId(0), vec![Target::Object(island)]).unwrap();

    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.attached_to(land_aura),
        Some(island),
        "moved land to land",
    );
}

#[test]
fn enchantment_alteration_needs_an_aura_on_a_creature_or_land() {
    // The clause-0 target is "target Aura attached to a creature or land" — an Aura enchanting an
    // artifact (Relic Bind) does not qualify, so the spell has nothing to target (CR 601.2c).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let spell = game.spawn_in_hand(PlayerId(0), card("Enchantment Alteration"));

    assert_eq!(
        cast(&mut game, PlayerId(0), spell, None),
        Err(Reject::IllegalTarget),
        "no attached Aura on the battlefield at all",
    );
}

// ── increment 67: Rabid Wombat ────────────────────────────────────────────────────────

#[test]
fn rabid_wombat_grows_by_two_for_each_attached_aura() {
    // "Vigilance / This creature gets +2/+2 for each Aura attached to it."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wombat = game.spawn_on_battlefield(PlayerId(0), card("Rabid Wombat"));
    assert_eq!(
        (game.power(wombat), game.toughness(wombat)),
        (0, 1),
        "printed 0/1 with nothing attached",
    );
    assert!(
        game.has_keyword(wombat, Keyword::Vigilance),
        "printed vigilance",
    );

    enchant(&mut game, PlayerId(0), "Burrowing", wombat);
    assert_eq!(
        (game.power(wombat), game.toughness(wombat)),
        (2, 3),
        "one Aura: +2/+2",
    );

    enchant(&mut game, PlayerId(0), "Flight", wombat);
    assert_eq!(
        (game.power(wombat), game.toughness(wombat)),
        (4, 5),
        "a second Aura stacks — the count is live, not a snapshot",
    );
}

// ── increment 117: Spectral Cloak ─────────────────────────────────────────────────────

#[test]
fn spectral_cloak_grants_shroud_only_while_the_creature_is_untapped() {
    // "Enchanted creature has shroud as long as it's untapped." The grant is a live read, not a
    // latch taken at attach time — tapping the creature turns it off.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    enchant(&mut game, PlayerId(0), "Spectral Cloak", bear);
    assert!(
        game.has_keyword(bear, Keyword::Shroud),
        "untapped and enchanted: shroud",
    );

    // Shroud is real, not just a keyword flag: a targeted spell can't name the creature.
    let tap_spell = game.spawn_in_hand(PlayerId(0), card("Energy Tap"));
    assert_eq!(
        cast(
            &mut game,
            PlayerId(0),
            tap_spell,
            Some(Target::Object(bear)),
        ),
        Err(Reject::IllegalTarget),
        "a creature with shroud can't be the target of a spell",
    );

    // Attacking taps it (Grizzly Bears has no vigilance) — the condition stops holding.
    attack_with(&mut game, vec![bear]);
    assert!(game.is_tapped(bear), "declared attacker taps");
    assert!(
        !game.has_keyword(bear, Keyword::Shroud),
        "tapped: the 'as long as it's untapped' grant falls away",
    );
}

// ── increment 20: Backfire and Relic Bind ─────────────────────────────────────────────

/// Swing `attackers`, controlled by `attacker`, at `defender`, and run combat out to end of combat
/// so damage (and anything it triggers) has resolved. [`attack_with`] hardcodes P0-at-P1.
fn attack_seat_with(
    game: &mut Game,
    attacker: PlayerId,
    defender: PlayerId,
    attackers: Vec<ObjectId>,
) {
    advance_until(game, |g| g.current_step() == Step::DeclareAttackers);
    game.submit(Intent::DeclareAttackers {
        player: attacker,
        attackers: attackers
            .into_iter()
            .map(|a| (a, Defender::Player(defender)))
            .collect(),
    })
    .unwrap();
    advance_until(game, |g| g.current_step() == Step::EndCombat);
}

#[test]
fn backfire_reflects_damage_dealt_to_you_onto_the_creatures_controller() {
    // "Whenever enchanted creature deals damage to you, this Aura deals that much damage to that
    // creature's controller." P0 hangs it on P1's bear; P1 swings at P0 and takes it back.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let host = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    enchant(&mut game, PlayerId(0), "Backfire", host);

    pass_until_next_turn(&mut game); // P1, the host's controller, is active
    attack_seat_with(&mut game, PlayerId(1), PlayerId(0), vec![host]);

    assert_eq!(game.life(PlayerId(0)), 18, "P0 took the 2 combat damage");
    assert_eq!(
        game.life(PlayerId(1)),
        18,
        "'that much damage to that creature's controller' — the attacker eats its own 2",
    );
}

#[test]
fn backfire_ignores_damage_dealt_to_anyone_but_its_controller() {
    // "…deals damage to **you**": the Aura's own controller, not whoever the host happens to hit.
    let mut game = Game::with_players(4, 0);
    for seat in 0..4 {
        game.stack_library(PlayerId(seat), &vec![card("Grizzly Bears"); 40]);
    }
    let host = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Backfire"));
    cast(&mut game, PlayerId(0), aura, Some(Target::Object(host))).unwrap();
    for _ in 0..4 {
        game.submit(Intent::PassPriority {
            player: game.priority_holder(),
        })
        .unwrap();
    }

    pass_until_next_turn(&mut game); // P1 is active
    attack_seat_with(&mut game, PlayerId(1), PlayerId(2), vec![host]);

    assert_eq!(game.life(PlayerId(2)), 18, "P2 took the combat damage");
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "the host's controller pays nothing — the damage wasn't dealt to the Aura's controller",
    );
    assert_eq!(
        game.life(PlayerId(0)),
        20,
        "and the Aura's controller is untouched"
    );
}

#[test]
fn relic_bind_offers_both_modes_when_the_artifact_becomes_tapped() {
    // "Whenever enchanted artifact becomes tapped, choose one — • This Aura deals 1 damage to
    // target player or planeswalker. • Target player gains 1 life."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let ring = game.spawn_on_battlefield(PlayerId(1), card("Sol Ring"));
    enchant(&mut game, PlayerId(0), "Relic Bind", ring);

    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: ring,
    })
    .unwrap();
    game.submit(Intent::ChooseMode {
        player: PlayerId(0),
        mode: 0,
    })
    .expect("the Aura's controller picks the mode");
    choose(&mut game, PlayerId(0), vec![Target::Player(PlayerId(1))]).unwrap();
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(1)),
        19,
        "mode 0: the Aura deals 1 damage to the chosen player",
    );

    // A second tap is a second trigger — this time take the life mode.
    game.untap(ring);
    game.submit(Intent::TapForMana {
        player: PlayerId(1),
        object: ring,
    })
    .unwrap();
    game.submit(Intent::ChooseMode {
        player: PlayerId(0),
        mode: 1,
    })
    .unwrap();
    choose(&mut game, PlayerId(0), vec![Target::Player(PlayerId(0))]).unwrap();
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.life(PlayerId(0)),
        21,
        "mode 1: target player gains 1 life"
    );
}

// ── increment 71: Remove Enchantments ─────────────────────────────────────────────────

#[test]
fn remove_enchantments_returns_yours_and_destroys_the_rest() {
    // "Return to your hand all enchantments you both own and control, all Auras you own attached
    // to permanents you control, and all Auras you own attached to attacking creatures your
    // opponents control. Then destroy all other enchantments you control, all other Auras attached
    // to permanents you control, and all other Auras attached to attacking creatures your
    // opponents control."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let mine = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    // P0 owns and controls both of these: one on their own creature, one on P1's.
    let ours_here = enchant(&mut game, PlayerId(0), "Burrowing", mine);
    let ours_there = enchant(&mut game, PlayerId(0), "Flight", theirs);
    // P1 owns this one, but it's attached to a permanent P0 controls — destroyed, not returned.
    let untouched = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    next_turn_main(&mut game);
    let theirs_on_mine = enchant(&mut game, PlayerId(1), "Flight", mine);
    // And this one is P1's, on P1's own creature, not attacking — out of scope entirely.
    let theirs_on_theirs = enchant(&mut game, PlayerId(1), "Burrowing", untouched);
    next_turn_main(&mut game);

    let spell = game.spawn_in_hand(PlayerId(0), card("Remove Enchantments"));
    cast_and_resolve(&mut game, PlayerId(0), spell, None);

    assert!(
        game.hand(PlayerId(0)).contains(&game.current_id(ours_here)),
        "an enchantment P0 both owns and controls goes back to hand",
    );
    assert!(
        game.hand(PlayerId(0))
            .contains(&game.current_id(ours_there)),
        "so does P0's Aura sitting on an opponent's creature",
    );
    assert_eq!(
        game.zone_of(game.current_id(theirs_on_mine)),
        Zone::Graveyard,
        "P1's Aura on a permanent P0 controls is 'all other' — destroyed, not returned",
    );
    assert_eq!(
        game.attached_to(game.current_id(theirs_on_theirs)),
        Some(untouched),
        "P1's Aura on P1's own non-attacking creature is out of scope",
    );
}

// ── increment 65: Puppet Master ───────────────────────────────────────────────────────

#[test]
fn puppet_master_returns_the_dead_creature_and_may_buy_itself_back() {
    // "When enchanted creature dies, return that card to its owner's hand. If that card is
    // returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to
    // its owner's hand."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let aura = enchant(&mut game, PlayerId(0), "Puppet Master", bear);

    // Kill it. Both halves of the death trigger are on the stack by the time the dust settles.
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    cast_and_resolve(&mut game, PlayerId(0), bolt, Some(Target::Object(bear)));
    game.fund_mana(PlayerId(0));
    while game.pending_choice().is_some() || !game.stack().is_empty() {
        let intent = match game.pending_choice() {
            // Two triggers off one death, same controller — CR 603.3b makes P0 order them.
            None => {
                resolve_top_of_stack(&mut game);
                continue;
            }
            Some(PendingChoice::OrderTriggers { effects, .. }) => Intent::ChooseOrder {
                player: PlayerId(0),
                order: (0..effects.len()).collect(),
            },
            // "you may pay {U}{U}{U}" — pay it.
            _ => Intent::PayOptionalCost {
                player: PlayerId(0),
                pay: true,
                discard_cost: vec![],
            },
        };
        game.submit(intent).unwrap();
    }

    assert!(
        game.hand(PlayerId(0)).contains(&game.current_id(bear)),
        "the dead creature card went back to its owner's hand",
    );
    assert!(
        game.hand(PlayerId(0)).contains(&game.current_id(aura)),
        "and paying {{U}}{{U}}{{U}} bought the Aura itself back out of the graveyard",
    );
}
