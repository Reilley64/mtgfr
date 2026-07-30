//! Legends (`leg`) grind — increment 94: source-keyed-prevention-shield, and increment 95:
//! grant-prevention-to-attached.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
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

/// Resolve the top of the stack, handing back every event the resolution minted.
fn resolve_capturing(game: &mut Game) -> Vec<Event> {
    let mut all = Vec::new();
    for _ in 0..game.player_count() {
        let player = game.priority_holder();
        all.extend(game.submit(Intent::PassPriority { player }).unwrap());
    }
    all
}

fn cast_and_resolve(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    cast_capturing(game, player, object, target);
}

fn cast_capturing(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Vec<Event> {
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
    resolve_capturing(game)
}

/// Activate `object`'s first ability for `player`, funding whatever it costs, and resolve it.
fn activate(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
    try_activate(game, player, object, target).unwrap();
    resolve_capturing(game);
}

/// The engine's verdict on an activation, without resolving it — for the targeting-legality tests.
fn try_activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    give_priority(game, player);
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// Whether `object` is still a battlefield permanent — a shielded creature is one that survived
/// what would otherwise have killed it.
fn alive(game: &Game, object: ObjectId) -> bool {
    game.zone_of(object) == Zone::Battlefield
}

/// Roll forward to `player`'s own main phase, where a sorcery-speed Aura can be cast.
fn reach_main_phase_of(game: &mut Game, player: PlayerId) {
    advance_until(game, |g| {
        g.active_player() == player && g.current_step() == Step::Main1
    });
}

// ── #94 "dealt **by** target creature this turn" (Lady Evangela, Horn of Deafening) ────

#[test]
fn lady_evangela_prevents_her_targets_combat_damage_to_a_player() {
    // "{W}{B}, {T}: Prevent all combat damage that would be dealt by target creature this turn."
    // The shield is keyed to the damage's *source*, so it reaches an unblocked attacker's hit on
    // the defending player.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let evangela = game.spawn_on_battlefield(PlayerId(1), card("Lady Evangela"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(
        &mut game,
        PlayerId(1),
        evangela,
        Some(Target::Object(giant)),
    );

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![giant]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "the named creature's combat damage is prevented however it connects",
    );
}

#[test]
fn lady_evangela_shield_covers_a_blocking_creature_too() {
    // A source-keyed shield stands in front of *every* recipient at once (CR 615): the same
    // activation that saves the player also saves whatever blocks the named attacker.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let evangela = game.spawn_on_battlefield(PlayerId(1), card("Lady Evangela"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(
        &mut game,
        PlayerId(1),
        evangela,
        Some(Target::Object(giant)),
    );

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(bears, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(bears),
        0,
        "the blocker takes nothing from the shielded attacker",
    );
    assert!(alive(&game, bears), "so the 2/2 survives a 3/3");
    assert_eq!(
        game.marked_damage(giant),
        2,
        "the shield is one-way — the blocker's own damage still lands",
    );
}

#[test]
fn lady_evangela_leaves_her_targets_noncombat_damage_alone() {
    // "Prevent all **combat** damage": the word is the whole gate, so a ping from the very same
    // creature still lands.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let evangela = game.spawn_on_battlefield(PlayerId(1), card("Lady Evangela"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(
        &mut game,
        PlayerId(1),
        evangela,
        Some(Target::Object(pinger)),
    );

    let before = game.life(PlayerId(1));
    activate(
        &mut game,
        PlayerId(0),
        pinger,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before - 1,
        "a noncombat ping from the named creature is not prevented",
    );
}

#[test]
fn horn_of_deafening_prevents_its_targets_combat_damage() {
    // "{2}, {T}: Prevent all combat damage that would be dealt by target creature this turn."
    // Lady Evangela's shield on an artifact.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let horn = game.spawn_on_battlefield(PlayerId(1), card("Horn of Deafening"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(1), horn, Some(Target::Object(giant)));

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![giant]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "the named attacker connects for nothing",
    );
}

#[test]
fn a_source_keyed_shield_is_not_used_up_by_the_first_creature_it_saves() {
    // CR 615.6: "prevent **all** combat damage … this turn" is never consumed, so a shielded
    // attacker that is double-blocked deals nothing to either blocker.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let horn = game.spawn_on_battlefield(PlayerId(1), card("Horn of Deafening"));
    let first = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(1), horn, Some(Target::Object(giant)));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(first, giant), (second, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(alive(&game, first), "the first blocker takes nothing");
    assert!(alive(&game, second), "and so does the second");
}

// ── #94 Subdue: the same shield plus "+0/+X, where X is its mana value" ────────────────

#[test]
fn subdue_prevents_its_targets_combat_damage_and_grows_its_toughness() {
    // "Prevent all combat damage that would be dealt by target creature this turn. That creature
    // gets +0/+X until end of turn, where X is its mana value." Both halves land on the one
    // target: the 2/2 it names deals nothing and survives a blocker that would have killed it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let subdue = game.spawn_in_hand(PlayerId(0), card("Subdue"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), subdue, Some(Target::Object(bears)));

    attack_with(&mut game, vec![bears]);
    block_with(&mut game, vec![(giant, bears)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(giant),
        0,
        "the subdued creature deals its blocker nothing",
    );
    assert!(
        alive(&game, bears),
        "and the +0/+2 (mana value 2) carries the 2/2 through a 3/3's damage",
    );
}

// ── #94 Kry Shield: all damage, not just combat, from a creature you control ───────────

#[test]
fn kry_shield_prevents_the_noncombat_damage_its_target_deals() {
    // "{2}, {T}: Prevent all damage that would be dealt this turn by target creature you control."
    // No "combat" in the wording, so a ping from the named creature is prevented too — the whole
    // difference from Lady Evangela's shield.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let shield = game.spawn_on_battlefield(PlayerId(0), card("Kry Shield"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(0), shield, Some(Target::Object(pinger)));

    let before = game.life(PlayerId(1));
    activate(
        &mut game,
        PlayerId(0),
        pinger,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "the named creature's noncombat damage is prevented as well",
    );
}

#[test]
fn kry_shield_grows_its_target_by_its_mana_value() {
    // "That creature gets +0/+X until end of turn, where X is its mana value." Prodigal Sorcerer
    // costs {2}{U}, so the 1/1 becomes a 1/4 and shrugs off a Lightning Bolt.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let shield = game.spawn_on_battlefield(PlayerId(0), card("Kry Shield"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(0), shield, Some(Target::Object(pinger)));
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(pinger)));

    assert!(
        alive(&game, pinger),
        "a 1/1 grown to 1/4 lives through 3 damage",
    );
}

#[test]
fn kry_shield_cannot_name_a_creature_you_dont_control() {
    // "target creature **you control**" is checked when the ability is activated (CR 115.4), so
    // the engine rejects the declaration outright.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let shield = game.spawn_on_battlefield(PlayerId(0), card("Kry Shield"));
    let theirs = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    let verdict = try_activate(&mut game, PlayerId(0), shield, Some(Target::Object(theirs)));

    assert!(
        verdict.is_err(),
        "an opponent's creature is not a legal target for Kry Shield",
    );
}

// ── #94 Indestructible Aura: the recipient-keyed corner (all damage, turn-long) ────────

#[test]
fn indestructible_aura_prevents_every_hit_the_creature_takes_this_turn() {
    // "Prevent all damage that would be dealt to target creature this turn." CR 615.6 — the
    // shield is not used up, so a second burn spell is stopped as flatly as the first.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Indestructible Aura"));
    let first = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let second = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(bears)));
    cast_and_resolve(&mut game, PlayerId(1), first, Some(Target::Object(bears)));
    cast_and_resolve(&mut game, PlayerId(1), second, Some(Target::Object(bears)));

    assert!(alive(&game, bears), "both bolts are prevented outright");
    assert_eq!(game.marked_damage(bears), 0, "and neither marks anything");
}

#[test]
fn a_shield_does_not_follow_a_creature_that_left_and_came_back() {
    // CR 400.7: a permanent that leaves the battlefield and returns is a new object, so the
    // shield armed on the old one no longer stands in front of it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Indestructible Aura"));
    let blink = game.spawn_in_hand(PlayerId(0), card("Momentary Blink"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(bears)));
    let events = cast_capturing(&mut game, PlayerId(0), blink, Some(Target::Object(bears)));
    let returned = events
        .iter()
        .find_map(|e| match e {
            Event::FlickeredToBattlefield { permanent, .. } => Some(*permanent),
            _ => None,
        })
        .expect("the blink returns the creature to the battlefield");

    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(returned)));

    assert!(
        !alive(&game, returned),
        "the returned creature is a new object and carries no shield",
    );
}

// ── #95 Gaseous Form: "dealt to and dealt by enchanted creature" ───────────────────────

#[test]
fn gaseous_form_prevents_combat_damage_in_both_directions() {
    // "Prevent all combat damage that would be dealt to and dealt by enchanted creature." Two
    // 3/3s trade into nothing: neither marks the other.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let form = game.spawn_in_hand(PlayerId(0), card("Gaseous Form"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), form, Some(Target::Object(attacker)));

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(blocker),
        0,
        "the enchanted creature deals its blocker nothing",
    );
    assert_eq!(
        game.marked_damage(attacker),
        0,
        "and takes nothing back from it",
    );
    assert!(alive(&game, attacker) && alive(&game, blocker), "both live");
}

#[test]
fn two_gaseous_formed_creatures_prevent_all_four_halves() {
    // Each creature carries both directions, so the block is covered twice over in each
    // direction — the case that catches a shield that consumes itself.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let mine = game.spawn_in_hand(PlayerId(0), card("Gaseous Form"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let theirs = game.spawn_in_hand(PlayerId(1), card("Gaseous Form"));

    reach_main_phase_of(&mut game, PlayerId(0));
    cast_and_resolve(&mut game, PlayerId(0), mine, Some(Target::Object(attacker)));
    reach_main_phase_of(&mut game, PlayerId(1));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        theirs,
        Some(Target::Object(blocker)),
    );
    reach_main_phase_of(&mut game, PlayerId(0));

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.marked_damage(blocker), 0, "the blocker takes nothing");
    assert_eq!(game.marked_damage(attacker), 0, "nor does the attacker");
    assert!(alive(&game, attacker) && alive(&game, blocker), "both live");
}

#[test]
fn gaseous_form_leaves_noncombat_damage_alone() {
    // "Prevent all **combat** damage": a burn spell still kills the enchanted creature.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let form = game.spawn_in_hand(PlayerId(0), card("Gaseous Form"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), form, Some(Target::Object(bears)));
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(bears)));

    assert!(
        !alive(&game, bears),
        "a noncombat 3 damage is untouched by a combat-only shield",
    );
}

// ── #95 Demonic Torment: "dealt **by** enchanted creature" only ────────────────────────

#[test]
fn demonic_torment_prevents_only_the_damage_the_enchanted_creature_deals() {
    // "Prevent all combat damage that would be dealt by enchanted creature." One direction, not
    // two — the single word that separates it from Gaseous Form.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let torment = game.spawn_in_hand(PlayerId(1), card("Demonic Torment"));

    reach_main_phase_of(&mut game, PlayerId(1));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        torment,
        Some(Target::Object(blocker)),
    );
    reach_main_phase_of(&mut game, PlayerId(0));

    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(
        alive(&game, attacker),
        "the tormented blocker deals nothing, so its attacker survives",
    );
    assert_eq!(
        game.marked_damage(attacker),
        0,
        "the tormented creature deals nothing",
    );
    assert_eq!(
        game.marked_damage(blocker),
        2,
        "but takes its blocked attacker's damage in full",
    );
}

#[test]
fn demonic_torment_still_keeps_the_enchanted_creature_from_attacking() {
    // "Enchanted creature can't attack" (CR 506.4) — the clause that already worked, kept honest.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let torment = game.spawn_in_hand(PlayerId(0), card("Demonic Torment"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), torment, Some(Target::Object(giant)));

    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    let verdict = game.submit(Intent::DeclareAttackers {
        player: PlayerId(0),
        attackers: vec![(giant, Defender::Player(PlayerId(1)))],
    });

    assert!(verdict.is_err(), "the tormented creature can't be declared");
}
