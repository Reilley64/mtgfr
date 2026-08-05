//! Legends (`leg`) grind — wave 11, slice C: increments 38, 60, 126, 127, 128.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Keep every seat's library stocked so passing priority can't deck anybody.
fn stock_libraries(game: &mut Game) {
    for player in 0..game.player_count() as u8 {
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

// ── increment 38: damage-reduction-replacement (Forethought Amulet) ────────────────────

#[test]
fn forethought_amulet_rewrites_a_three_damage_burn_spell_down_to_two() {
    // "If an instant or sorcery source would deal 3 or more damage to you, it deals 2 damage to
    // you instead." (CR 615.9)
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Forethought Amulet"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        bolt,
        Some(Target::Player(PlayerId(0))),
    );

    assert_eq!(
        game.life(PlayerId(0)),
        before - 2,
        "a 3-damage instant deals 2 instead",
    );
}

#[test]
fn a_three_damage_burn_spell_lands_in_full_without_the_amulet() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        bolt,
        Some(Target::Player(PlayerId(0))),
    );

    assert_eq!(game.life(PlayerId(0)), before - 3, "the control case");
}

#[test]
fn forethought_amulet_rewrites_a_bigger_hit_to_the_same_two() {
    // "3 or more" — the rewrite is a flat 2, not a reduction, so a 4-damage instant lands for 2.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Forethought Amulet"));
    let blast = game.spawn_in_hand(PlayerId(1), card("Psionic Blast"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        blast,
        Some(Target::Player(PlayerId(0))),
    );

    assert_eq!(
        game.life(PlayerId(0)),
        before - 2,
        "Psionic Blast's 4 damage to the amulet's controller becomes 2",
    );
}

#[test]
fn forethought_amulet_leaves_a_sub_threshold_burn_spell_alone() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Forethought Amulet"));
    let shock = game.spawn_in_hand(PlayerId(1), card("Shock"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        shock,
        Some(Target::Player(PlayerId(0))),
    );

    assert_eq!(
        game.life(PlayerId(0)),
        before - 2,
        "under 3 damage, the spell is dealt as printed",
    );
}

#[test]
fn forethought_amulet_ignores_a_permanent_source() {
    // "an *instant or sorcery* source" — Orcish Artillery's "{T}: ... and 3 damage to you" is a
    // creature source, so its 3 lands in full even though it clears the threshold.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Forethought Amulet"));
    let artillery = game.spawn_on_battlefield(PlayerId(0), card("Orcish Artillery"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: artillery,
        ability_index: 0,
        target: Some(Target::Player(PlayerId(1))),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.life(PlayerId(0)),
        before - 3,
        "a creature source is not an instant or sorcery source",
    );
}

#[test]
fn forethought_amulet_only_protects_its_own_controller() {
    // "to *you*" — the opponent eats the whole bolt.
    let mut game = Game::new();
    stock_libraries(&mut game);
    game.spawn_on_battlefield(PlayerId(0), card("Forethought Amulet"));
    let bolt = game.spawn_in_hand(PlayerId(0), card("Lightning Bolt"));
    let before = game.life(PlayerId(1));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    cast_and_resolve(
        &mut game,
        PlayerId(0),
        bolt,
        Some(Target::Player(PlayerId(1))),
    );

    assert_eq!(
        game.life(PlayerId(1)),
        before - 3,
        "not the amulet's player"
    );
}

// ── increment 60: damage-redirection (Nova Pentacle, Shimian Night Stalker) ────────────

fn activate(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .unwrap();
    resolve_top_of_stack(game);
}

#[test]
fn nova_pentacle_moves_the_next_hit_onto_the_chosen_creature() {
    // "The next time a source of your choice would deal damage to you this turn, that damage is
    // dealt to target creature of an opponent's choice instead." (CR 615.10)
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pentacle = game.spawn_on_battlefield(PlayerId(0), card("Nova Pentacle"));
    // A 0/8 survives the whole bolt, so the marked damage is still readable afterwards.
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let bolt = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(0), pentacle, Some(Target::Object(wall)));
    cast_and_resolve(
        &mut game,
        PlayerId(1),
        bolt,
        Some(Target::Player(PlayerId(0))),
    );

    assert_eq!(game.life(PlayerId(0)), before, "none of it reached you");
    assert_eq!(
        game.marked_damage(wall),
        3,
        "all 3 landed on the chosen creature instead",
    );
}

#[test]
fn nova_pentacle_only_moves_one_hit() {
    // "The **next** time" — the shield is spent by the hit it moved (CR 615.6).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let pentacle = game.spawn_on_battlefield(PlayerId(0), card("Nova Pentacle"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let first = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let second = game.spawn_in_hand(PlayerId(1), card("Lightning Bolt"));
    let before = game.life(PlayerId(0));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    activate(&mut game, PlayerId(0), pentacle, Some(Target::Object(bear)));
    for bolt in [first, second] {
        cast_and_resolve(
            &mut game,
            PlayerId(1),
            bolt,
            Some(Target::Player(PlayerId(0))),
        );
    }

    assert_eq!(game.life(PlayerId(0)), before - 3, "the second bolt lands");
}

#[test]
fn shimian_night_stalker_takes_the_attackers_damage_for_you() {
    // "All damage that would be dealt to you this turn by target attacking creature is dealt to
    // this creature instead." (CR 615.10)
    let mut game = Game::new();
    stock_libraries(&mut game);
    let stalker = game.spawn_on_battlefield(PlayerId(0), card("Shimian Night Stalker"));
    let ogre = game.spawn_on_battlefield(PlayerId(1), card("Hill Giant"));
    let before = game.life(PlayerId(0));

    // Player 1's turn: swing the giant at player 0.
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(1) && g.current_step() == Step::DeclareAttackers
    });
    game.submit(Intent::DeclareAttackers {
        player: PlayerId(1),
        attackers: vec![(ogre, Defender::Player(PlayerId(0)))],
    })
    .unwrap();
    activate(&mut game, PlayerId(0), stalker, Some(Target::Object(ogre)));

    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.life(PlayerId(0)), before, "you took none of it");
    assert_eq!(
        game.marked_damage(stalker),
        3,
        "the Night Stalker soaked the whole swing",
    );
}

// ── increments 126/127: CR 510 damage assignment ──────────────────────────────────────

/// Answer the pending combat damage division on behalf of the seat it belongs to.
fn assign(game: &mut Game, assignment: Vec<(ObjectId, i32)>) -> Result<Vec<Event>, Reject> {
    let Some(PendingChoice::AssignCombatDamage { player, .. }) = game.pending_choice() else {
        panic!(
            "a division should be pending; got {:?}",
            game.pending_choice()
        );
    };
    game.submit(Intent::AssignDamage { player, assignment })
}

fn advance_to_division(game: &mut Game) {
    advance_until(game, |g| {
        matches!(
            g.pending_choice(),
            Some(PendingChoice::AssignCombatDamage { .. })
        )
    });
}

#[test]
fn a_division_may_not_leave_two_blockers_short_of_lethal() {
    // CR 510.1c: no damage may be assigned to a blocker later in the damage assignment order until
    // every blocker ahead of it has lethal. The order is the attacker's to pick (CR 509.2), so a
    // *single* short blocker is always realizable — two of them never are.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let first = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![bears]);
    block_with(&mut game, vec![(first, bears), (second, bears)]).unwrap();
    advance_to_division(&mut game);

    assert!(
        matches!(
            assign(&mut game, vec![(first, 1), (second, 1)]),
            Err(Reject::IllegalChoice)
        ),
        "1 apiece half-kills both 2/2s — no order makes that legal",
    );
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::AssignCombatDamage { .. })
        ),
        "a rejected division stays pending",
    );

    assign(&mut game, vec![(first, 0), (second, 2)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(game.zone_of(second), Zone::Graveyard, "lethal, in order");
    assert_eq!(
        game.marked_damage(first),
        0,
        "nothing spilled onto the other"
    );
}

#[test]
fn a_double_striker_divides_again_in_the_normal_batch() {
    // CR 510.4: the second combat damage step's assignment is chosen fresh — the first-strike
    // split is not carried over.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    // "All creatures have double strike" — the cheapest way to put a double striker in combat.
    game.spawn_on_battlefield(PlayerId(1), card("Avatar of Slaughter"));
    // 0/8s: they deal nothing back and survive both batches, so both are still recipients.
    let first = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    let second = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(first, giant), (second, giant)]).unwrap();

    advance_to_division(&mut game);
    assert_eq!(game.current_step(), Step::FirstStrikeCombatDamage);
    assign(&mut game, vec![(first, 3), (second, 0)]).unwrap();

    advance_to_division(&mut game);
    assert_eq!(
        game.current_step(),
        Step::CombatDamage,
        "the normal batch asks for its own division",
    );
    assign(&mut game, vec![(first, 3), (second, 0)]).unwrap();

    assert_eq!(game.marked_damage(first), 6, "both batches landed");
}

#[test]
fn a_gang_block_past_the_blocker_ceiling_still_finishes_combat() {
    // `MAX_BLOCKERS` is 8, and a 9-recipient division has no answer the engine can accept — so
    // it is never raised, and the default lethal-in-order split runs instead (increment 127).
    let mut game = Game::new();
    stock_libraries(&mut game);
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));
    let walls: Vec<ObjectId> = (0..9)
        .map(|_| game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone")))
        .collect();

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, walls.iter().map(|&wall| (wall, giant)).collect()).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(
        game.pending_choice().is_none(),
        "the combat damage step completed rather than softlocking",
    );
    assert_eq!(
        game.marked_damage(walls[0]),
        3,
        "the default split walks the declaration order",
    );
}

// ── increment 128: costless-permanent-regeneration-shield (Clergy of the Holy Nimbus) ──

/// A destroy spell with no "can't be regenerated" rider, aimed at `victim`.
fn destroy(game: &mut Game, victim: ObjectId) {
    let grasp = game.spawn_in_hand(PlayerId(1), card("Infernal Grasp"));
    cast_and_resolve(game, PlayerId(1), grasp, Some(Target::Object(victim)));
}

#[test]
fn clergy_regenerates_in_place_of_being_destroyed() {
    // "If this creature would be destroyed, regenerate it." (CR 701.15)
    let mut game = Game::new();
    stock_libraries(&mut game);
    let clergy = game.spawn_on_battlefield(PlayerId(0), card("Clergy of the Holy Nimbus"));

    destroy(&mut game, clergy);

    assert_eq!(
        game.zone_of(clergy),
        Zone::Battlefield,
        "the standing shield replaced the destruction",
    );
    assert!(
        game.is_tapped(clergy),
        "CR 701.15a taps the regenerated creature"
    );
}

#[test]
fn clergy_regenerates_every_time_rather_than_once() {
    // The shield is a standing replacement, not a one-shot the first destruction uses up.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let clergy = game.spawn_on_battlefield(PlayerId(0), card("Clergy of the Holy Nimbus"));

    destroy(&mut game, clergy);
    destroy(&mut game, clergy);

    assert_eq!(
        game.zone_of(clergy),
        Zone::Battlefield,
        "the second destruction was replaced too",
    );
}

#[test]
fn an_ordinary_creature_still_dies_to_the_same_spell() {
    let mut game = Game::new();
    stock_libraries(&mut game);
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    destroy(&mut game, bear);

    assert_eq!(game.zone_of(bear), Zone::Graveyard);
}

#[test]
fn clergys_own_second_ability_turns_its_shield_off() {
    // "{1}: This creature can't be regenerated this turn. Only your opponents may activate this
    // ability." (CR 701.15d) — the standing shield honors the same suppression flag.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let clergy = game.spawn_on_battlefield(PlayerId(0), card("Clergy of the Holy Nimbus"));

    give_priority(&mut game, PlayerId(1));
    game.fund_mana(PlayerId(1));
    // Index 1: the static regeneration replacement is ability 0.
    game.submit(Intent::ActivateAbility {
        player: PlayerId(1),
        object: clergy,
        ability_index: 1,
        target: None,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .unwrap();
    resolve_top_of_stack(&mut game);

    destroy(&mut game, clergy);

    assert_eq!(
        game.zone_of(clergy),
        Zone::Graveyard,
        "\"can't be regenerated this turn\" beats the standing shield",
    );
}

#[test]
fn lethal_combat_damage_is_replaced_by_the_standing_shield() {
    // CR 704.5g's destroy is a destroy too, so the state-based action reads the same shield.
    let mut game = Game::new();
    stock_libraries(&mut game);
    let clergy = game.spawn_on_battlefield(PlayerId(1), card("Clergy of the Holy Nimbus"));
    let giant = game.spawn_on_battlefield(PlayerId(0), card("Hill Giant"));

    attack_with(&mut game, vec![giant]);
    block_with(&mut game, vec![(clergy, giant)]).unwrap();
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(clergy),
        Zone::Battlefield,
        "the lethal-damage state-based action was replaced",
    );
}
