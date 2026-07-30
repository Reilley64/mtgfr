//! Legends (`leg`) grind — increments 105 (filter-completeness-and-disjunction), 62
//! (exact-power-toughness-filter) and 82 (time-elemental).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Cast `object` for `player` at `target` — the no-frills `Intent::Cast` every test below wants.
fn cast(game: &mut Game, player: PlayerId, object: ObjectId, target: Option<Target>) {
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
    .unwrap_or_else(|e| panic!("cast should be legal: {e:?}"));
}

/// The same cast, returning the engine's verdict instead of asserting it.
fn try_cast(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
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

fn top_spell(game: &Game) -> ObjectId {
    game.stack()
        .iter()
        .rev()
        .find_map(|entry| match *entry {
            StackEntry::Spell(id) => Some(id),
            StackEntry::Ability { .. } => None,
        })
        .expect("a spell is on the stack")
}

fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    ability_index: usize,
    target: Option<Target>,
) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

// ── #105 Flash Counter: "Counter target instant spell." ───────────────────────────────

#[test]
fn flash_counter_cannot_counter_a_sorcery_spell() {
    // The whole point of the increment: `instant_or_sorcery` would wrongly catch a sorcery.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));

    // Armageddon is a sorcery — the case `instant_or_sorcery` would wrongly admit.
    let sorcery = game.spawn_in_hand(PlayerId(0), card("Armageddon"));
    let flash_counter = game.spawn_in_hand(PlayerId(1), card("Flash Counter"));

    cast(&mut game, PlayerId(0), sorcery, None);
    let sorcery_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();

    assert!(
        !game
            .legal_targets(flash_counter, None)
            .contains(&Target::Object(sorcery_on_stack)),
        "a sorcery spell is not a legal target for \"counter target instant spell\""
    );
    assert_eq!(
        try_cast(
            &mut game,
            PlayerId(1),
            flash_counter,
            Some(Target::Object(sorcery_on_stack))
        ),
        Err(Reject::IllegalTarget),
        "Flash Counter aimed at a sorcery is rejected"
    );
}

#[test]
fn flash_counter_counters_an_instant_spell() {
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));

    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(0), card("Giant Growth"));
    let flash_counter = game.spawn_in_hand(PlayerId(1), card("Flash Counter"));

    cast(&mut game, PlayerId(0), growth, Some(Target::Object(bear)));
    let growth_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();

    cast(
        &mut game,
        PlayerId(1),
        flash_counter,
        Some(Target::Object(growth_on_stack)),
    );
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(growth_on_stack),
        Zone::Graveyard,
        "the countered instant lands in its owner's graveyard"
    );
    assert_eq!(
        game.power(bear),
        2,
        "Giant Growth's +3/+3 never applied — Flash Counter beat it"
    );
}

// ── #105 Mana Matrix: "Instant and enchantment spells you cast cost {2} less to cast." ──

/// Tap `count` fresh basics of `name` for `player`, leaving a precise pool (`fund_mana` is too
/// generous to prove a discount actually landed).
fn tap_lands(game: &mut Game, player: PlayerId, name: &str, count: usize) {
    for _ in 0..count {
        let land = game.spawn_on_battlefield(player, card(name));
        game.submit(Intent::TapForMana {
            player,
            object: land,
        })
        .unwrap();
    }
}

#[test]
fn mana_matrix_shaves_two_generic_off_an_enchantment_spell() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Mana Matrix"));
    // Glorious Anthem is {1}{W}{W}; the reducer eats its single generic, so {W}{W} pays for it.
    let anthem = game.spawn_in_hand(PlayerId(0), card("Glorious Anthem"));
    tap_lands(&mut game, PlayerId(0), "Plains", 2);

    cast(&mut game, PlayerId(0), anthem, None);
    assert_eq!(game.zone_of(anthem), Zone::Stack);
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        0,
        "both Plains paid the whole reduced cost"
    );
}

#[test]
fn mana_matrix_cannot_shave_a_colored_requirement_off_an_instant() {
    // CR 601.2f: a cost reduction only reduces the generic portion. Flash Counter is {1}{U};
    // {2} off leaves {U}, never {0} — one Island casts it, and zero Islands cannot.
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Mana Matrix"));
    let first = game.spawn_in_hand(PlayerId(0), card("Flash Counter"));
    let second = game.spawn_in_hand(PlayerId(0), card("Flash Counter"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let growth = game.spawn_in_hand(PlayerId(1), card("Giant Growth"));

    // A spell for the counters to aim at, so targeting never masks a mana rejection.
    game.fund_mana(PlayerId(1));
    cast(&mut game, PlayerId(1), growth, Some(Target::Object(bear)));
    let growth_on_stack = top_spell(&game);
    game.submit(Intent::PassPriority {
        player: PlayerId(1),
    })
    .unwrap();

    assert_eq!(
        try_cast(
            &mut game,
            PlayerId(0),
            first,
            Some(Target::Object(growth_on_stack))
        ),
        Err(Reject::CannotPayCost),
        "{{1}}{{U}} reduced by {{2}} is still {{U}} — an empty pool cannot pay it"
    );

    tap_lands(&mut game, PlayerId(0), "Island", 1);
    cast(
        &mut game,
        PlayerId(0),
        second,
        Some(Target::Object(growth_on_stack)),
    );
    assert_eq!(game.zone_of(second), Zone::Stack);
    assert_eq!(
        pool_total(&game, PlayerId(0)),
        0,
        "the lone Island paid the reduced {{U}}"
    );
}

#[test]
fn mana_matrix_leaves_a_creature_spell_alone() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Mana Matrix"));
    // Grizzly Bears is {1}{G} and is neither an instant nor an enchantment: no discount.
    let bear = game.spawn_in_hand(PlayerId(0), card("Grizzly Bears"));
    tap_lands(&mut game, PlayerId(0), "Forest", 1);

    assert_eq!(
        try_cast(&mut game, PlayerId(0), bear, None),
        Err(Reject::CannotPayCost),
        "a creature spell gets no discount from an instant-or-enchantment reducer"
    );
}

// ── #62 Pendelhaven: "{T}: Target 1/1 creature gets +1/+2 until end of turn." ──────────

#[test]
fn pendelhaven_pumps_a_one_one_creature() {
    let mut game = Game::new();
    let pendelhaven = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    let goblin = game.spawn_on_battlefield(PlayerId(0), card("Mons's Goblin Raiders"));

    activate(&mut game, PlayerId(0), pendelhaven, 0, Some(Target::Object(goblin)))
        .expect("a 1/1 is a legal Pendelhaven target");
    resolve_top_of_stack(&mut game);

    assert_eq!(game.power(goblin), 2, "+1/+2 applied to the 1/1's power");
    assert_eq!(
        game.toughness(goblin),
        3,
        "+1/+2 applied to the 1/1's toughness"
    );
}

#[test]
fn pendelhaven_cannot_target_a_creature_whose_toughness_is_wrong() {
    // Grizzly Bears is 2/2 — power and toughness both fail the "1/1" gate. Scryb Sprites is
    // 1/1, so the power axis alone would not separate the two: the toughness axis is what
    // rejects a 1/2, which the pool's power-only gate could not have.
    let mut game = Game::new();
    let pendelhaven = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert!(
        !game
            .legal_targets(pendelhaven, Some(0))
            .contains(&Target::Object(bears)),
        "a 2/2 is not a \"target 1/1 creature\""
    );
    assert_eq!(
        activate(
            &mut game,
            PlayerId(0),
            pendelhaven,
            0,
            Some(Target::Object(bears))
        ),
        Err(Reject::IllegalTarget),
        "Pendelhaven aimed at a 2/2 is rejected"
    );
}

#[test]
fn pendelhaven_rejects_a_one_two_creature_the_power_gate_alone_would_admit() {
    // The exact reason #62 needs a toughness axis: Lady Evangela is 1/2, so she passes a
    // "power 1" gate and must still fail "1/1".
    let mut game = Game::new();
    let pendelhaven = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    let evangela = game.spawn_on_battlefield(PlayerId(0), card("Lady Evangela"));

    assert_eq!(game.power(evangela), 1);
    assert_eq!(game.toughness(evangela), 2);
    assert_eq!(
        activate(
            &mut game,
            PlayerId(0),
            pendelhaven,
            0,
            Some(Target::Object(evangela))
        ),
        Err(Reject::IllegalTarget),
        "a 1/2 is not a \"target 1/1 creature\""
    );
}

#[test]
fn pendelhaven_reads_current_toughness_not_printed() {
    // "Target 1/1 creature" reads P/T through all layers (CR 613): a 1/1 already pumped to 2/3
    // is no longer a legal target.
    let mut game = Game::new();
    let first = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    let second = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    let sprite = game.spawn_on_battlefield(PlayerId(0), card("Scryb Sprites"));

    activate(&mut game, PlayerId(0), first, 0, Some(Target::Object(sprite)))
        .expect("a 1/1 is a legal target");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.toughness(sprite), 3);

    assert_eq!(
        activate(
            &mut game,
            PlayerId(0),
            second,
            0,
            Some(Target::Object(sprite))
        ),
        Err(Reject::IllegalTarget),
        "a pumped 2/3 is no longer a \"target 1/1 creature\""
    );
}

#[test]
fn pendelhaven_taps_for_green() {
    let mut game = Game::new();
    let pendelhaven = game.spawn_on_battlefield(PlayerId(0), card("Pendelhaven"));
    game.submit(Intent::TapForMana {
        player: PlayerId(0),
        object: pendelhaven,
    })
    .expect("Pendelhaven's mana ability");
    assert_eq!(game.mana_in_pool(PlayerId(0), Color::Green), 1);
}

// ── #82 Time Elemental ────────────────────────────────────────────────────────────────

#[test]
fn time_elemental_returns_an_unenchanted_permanent_to_its_owners_hand() {
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Time Elemental"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    activate(
        &mut game,
        PlayerId(0),
        elemental,
        1,
        Some(Target::Object(bears)),
    )
    .expect("an unenchanted permanent is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(bears),
        Zone::Hand,
        "the bounced permanent is in its owner's hand"
    );
}

#[test]
fn time_elemental_cannot_target_an_enchanted_permanent() {
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Time Elemental"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(1), card("Changing Loyalty"));

    cast(&mut game, PlayerId(1), aura, Some(Target::Object(bears)));
    resolve_top_of_stack(&mut game);
    assert_eq!(game.zone_of(aura), Zone::Battlefield);

    assert!(
        !game
            .legal_targets(elemental, Some(1))
            .contains(&Target::Object(bears)),
        "an enchanted permanent is not a \"target permanent that isn't enchanted\""
    );
    assert_eq!(
        activate(
            &mut game,
            PlayerId(0),
            elemental,
            1,
            Some(Target::Object(bears))
        ),
        Err(Reject::IllegalTarget),
        "Time Elemental aimed at an enchanted permanent is rejected"
    );
}

#[test]
fn time_elemental_fizzles_when_its_target_becomes_enchanted_in_response() {
    // CR 608.2b: targeting legality is re-checked on resolution. Changing Loyalty has flash, so
    // the target can pick up an Aura while the ability is on the stack.
    let mut game = Game::new();
    game.fund_mana(PlayerId(0));
    game.fund_mana(PlayerId(1));
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Time Elemental"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let aura = game.spawn_in_hand(PlayerId(1), card("Changing Loyalty"));

    activate(
        &mut game,
        PlayerId(0),
        elemental,
        1,
        Some(Target::Object(bears)),
    )
    .expect("an unenchanted permanent is a legal target at activation");
    game.submit(Intent::PassPriority {
        player: PlayerId(0),
    })
    .unwrap();
    cast(&mut game, PlayerId(1), aura, Some(Target::Object(bears)));
    resolve_top_of_stack(&mut game); // the Aura attaches
    assert_eq!(game.attached_to(aura), Some(bears));

    resolve_top_of_stack(&mut game); // Time Elemental's ability tries to resolve

    assert_eq!(
        game.zone_of(bears),
        Zone::Battlefield,
        "the target became enchanted in response, so the bounce fizzled (CR 608.2b)"
    );
}

#[test]
fn time_elemental_sacrifices_itself_and_deals_five_to_you_after_attacking() {
    let mut game = Game::new();
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Time Elemental"));
    let before = game.life(PlayerId(0));

    attack_with(&mut game, vec![elemental]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.zone_of(elemental),
        Zone::Graveyard,
        "the end-of-combat delayed trigger sacrificed the attacker"
    );
    assert_eq!(
        game.life(PlayerId(0)),
        before - 5,
        "and it dealt 5 damage to its own controller"
    );
}

#[test]
fn time_elemental_sacrifices_itself_and_deals_five_to_you_after_blocking() {
    let mut game = Game::new();
    let goblin = game.spawn_on_battlefield(PlayerId(0), card("Mons's Goblin Raiders"));
    let elemental = game.spawn_on_battlefield(PlayerId(1), card("Time Elemental"));
    let before = game.life(PlayerId(1));

    attack_with(&mut game, vec![goblin]);
    block_with(&mut game, vec![(elemental, goblin)]).expect("a legal block");
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(
        game.zone_of(elemental),
        Zone::Graveyard,
        "the end-of-combat delayed trigger sacrificed the blocker"
    );
    assert_eq!(
        game.life(PlayerId(1)),
        before - 5,
        "and it dealt 5 damage to its own controller"
    );
}

#[test]
fn time_elemental_stays_put_when_it_neither_attacks_nor_blocks() {
    // The trigger is "when this creature attacks or blocks", not "at end of combat" — a
    // bystander survives a combat it sat out.
    let mut game = Game::new();
    let goblin = game.spawn_on_battlefield(PlayerId(0), card("Mons's Goblin Raiders"));
    let elemental = game.spawn_on_battlefield(PlayerId(0), card("Time Elemental"));
    let before = game.life(PlayerId(0));

    attack_with(&mut game, vec![goblin]);
    advance_until(&mut game, |g| g.current_step() == Step::Main2);

    assert_eq!(game.zone_of(elemental), Zone::Battlefield);
    assert_eq!(game.life(PlayerId(0)), before);
}
