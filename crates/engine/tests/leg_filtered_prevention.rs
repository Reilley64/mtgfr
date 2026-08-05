//! Legends (`leg`) grind — increment 12: filtered-damage-prevention.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..2 {
        for _ in 0..40 {
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

/// Whether `object` is still a battlefield permanent — a shielded creature is one that survived
/// what would otherwise have killed it.
fn alive(game: &Game, object: ObjectId) -> bool {
    game.zone_of(object) == Zone::Battlefield
}

// ── "by creatures it's blocking" (Wall of Vapor, Wall of Shadows) ──────────────────────

#[test]
fn wall_of_vapor_takes_no_damage_from_the_creature_it_blocks() {
    // "Prevent all damage that would be dealt to this creature by creatures it's blocking."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Vapor"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(wall),
        0,
        "a 3/3 it blocked deals it nothing at all",
    );
    assert!(alive(&game, wall), "so the 0/1 survives the block",);
}

#[test]
fn wall_of_vapor_still_takes_damage_from_a_creature_it_isnt_blocking() {
    // The shield is gated on the block, so an unrelated source gets through: the same wall dies to
    // a Shock-sized hit from a creature it never blocked.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let wall = game.spawn_on_battlefield(PlayerId(0), card("Wall of Vapor"));
    let pinger = game.spawn_on_battlefield(PlayerId(1), card("Prodigal Sorcerer"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: pinger,
        ability_index: 0,
        target: Some(Target::Object(wall)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert!(
        !alive(&game, wall),
        "nothing shields the wall from a source it never blocked",
    );
}

#[test]
fn wall_of_shadows_shields_itself_from_the_attacker_it_blocks() {
    // "Prevent all damage that would be dealt to this creature by creatures it's blocking." Same
    // shield as Wall of Vapor's, on a black Wall.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Shadows"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(wall, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(wall),
        0,
        "the blocked attacker deals it none"
    );
    assert!(alive(&game, wall), "so the 0/1 survives");
}

// ── "by enchanted creatures" (Wall of Putrid Flesh, Enchanted Being) ───────────────────

#[test]
fn wall_of_putrid_flesh_ignores_an_enchanted_attacker_but_not_a_bare_one() {
    // "Prevent all damage that would be dealt to this creature by enchanted creatures." The gate
    // is on the source's own state, so the very same attacker matters only once an Aura is on it.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let enchanted = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let bare = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Putrid Flesh"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        aura,
        Some(Target::Object(enchanted)),
    );

    attack_with(&mut game, vec![enchanted]);
    block_with(&mut game, vec![(wall, enchanted)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.marked_damage(wall),
        0,
        "the enchanted attacker's damage is prevented",
    );

    pass_until_next_turn(&mut game);
    pass_until_next_turn(&mut game);
    attack_with(&mut game, vec![bare]);
    block_with(&mut game, vec![(wall, bare)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.marked_damage(wall),
        3,
        "an attacker carrying no Aura is not gated out",
    );
}

#[test]
fn enchanted_being_shields_only_combat_damage_from_enchanted_creatures() {
    // "Prevent all combat damage that would be dealt to this creature by enchanted creatures." The
    // word *combat* is the whole difference from Wall of Putrid Flesh's otherwise identical shield.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pinger = game.spawn_on_battlefield(PlayerId(0), card("Prodigal Sorcerer"));
    let aura = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));
    let being = game.spawn_on_battlefield(PlayerId(1), card("Enchanted Being"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(0), aura, Some(Target::Object(pinger)));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: pinger,
        ability_index: 0,
        target: Some(Target::Object(being)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.marked_damage(being),
        1,
        "an enchanted creature's *noncombat* ping still lands",
    );
}

// ── "by Walls" (Marble Priest) ─────────────────────────────────────────────────────────

#[test]
fn marble_priest_takes_no_combat_damage_from_a_blocking_wall() {
    // "Prevent all combat damage that would be dealt to this creature by Walls." A subtype gate on
    // the source, so a Wall's damage vanishes where an ordinary creature's would land.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let priest = game.spawn_on_battlefield(PlayerId(0), card("Marble Priest"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));

    attack_with(&mut game, vec![priest]);
    block_with(&mut game, vec![(wall, priest)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.marked_damage(priest),
        0,
        "a Wall of Stone's 8 power is prevented outright",
    );
    assert!(alive(&game, priest), "so the 3/3 walks away from an 0/8",);
}

// ── "by spells that target it" (Bronze Horse) ──────────────────────────────────────────

#[test]
fn bronze_horse_shrugs_off_a_targeted_burn_spell_while_you_control_another_creature() {
    // "As long as you control another creature, prevent all damage that would be dealt to this
    // creature by spells that target it."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let horse = game.spawn_on_battlefield(PlayerId(0), card("Bronze Horse"));
    game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(horse)));

    assert_eq!(
        game.marked_damage(horse),
        0,
        "a spell that targeted it deals it nothing",
    );
}

#[test]
fn bronze_horse_is_unshielded_once_it_is_your_only_creature() {
    // CR 613: the "as long as" condition is re-checked continuously, so losing the other creature
    // turns the shield off.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let horse = game.spawn_on_battlefield(PlayerId(0), card("Bronze Horse"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(horse)));

    assert_eq!(
        game.marked_damage(horse),
        3,
        "with no other creature out, the bolt lands as usual",
    );
}

// ── "by attacking creatures without flying" (Al-abara's Carpet) ────────────────────────

#[test]
fn al_abaras_carpet_stops_ground_attackers_but_not_a_flyer() {
    // "{5}, {T}: Prevent all damage that would be dealt to you this turn by attacking creatures
    // without flying."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let drake = game.spawn_on_battlefield(PlayerId(0), card("Air Elemental"));
    let carpet = game.spawn_on_battlefield(PlayerId(1), card("Al-abara's Carpet"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    give_priority(&mut game, PlayerId(1));
    game.fund_mana(PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: carpet,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![giant, drake]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before - 4,
        "the 3/3 ground attacker is prevented; only the 4/4 flyer connects",
    );
}

#[test]
fn al_abaras_carpet_shield_is_not_used_up_by_the_first_attacker() {
    // "Prevent *all* damage … this turn" (CR 615.6): the shield is never spent, so a second
    // ground attacker in the same combat is stopped too.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let carpet = game.spawn_on_battlefield(PlayerId(1), card("Al-abara's Carpet"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    give_priority(&mut game, PlayerId(1));
    game.fund_mana(PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: carpet,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    let before = game.life(PlayerId(1));
    attack_with(&mut game, vec![giant, bear]);
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "both ground attackers are prevented, not just the first",
    );
}

// ── "a black or red source of your choice" (Greater Realm of Preservation) ─────────────

#[test]
fn greater_realm_of_preservation_stops_a_red_source_but_not_a_white_one() {
    // "{1}{W}: The next time a black or red source of your choice would deal damage to you this
    // turn, prevent that damage." The union is the point: either colour is enough.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let realm = game.spawn_on_battlefield(PlayerId(1), card("Greater Realm of Preservation"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    give_priority(&mut game, PlayerId(1));
    game.fund_mana(PlayerId(1));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: realm,
        ability_index: 0,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    let before = game.life(PlayerId(1));
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        bolt,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before,
        "a red source's 3 damage is prevented",
    );
}

// ── "a spell or ability that targets that creature" (Silhouette) ───────────────────────

#[test]
fn silhouette_prevents_a_burn_spell_aimed_at_the_chosen_creature() {
    // "Choose target creature. If a spell or ability that targets that creature would cause a
    // source to deal damage to that creature this turn, prevent that damage."
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let silhouette = game.spawn_in_hand(PlayerId(0), card("Silhouette"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        silhouette,
        Some(Target::Object(bears)),
    );
    cast_and_resolve(&mut game, PlayerId(1), bolt, Some(Target::Object(bears)));

    assert!(
        alive(&game, bears),
        "the bolt that targeted the chosen creature is prevented",
    );
    assert_eq!(game.marked_damage(bears), 0, "and marks nothing on it");
}
