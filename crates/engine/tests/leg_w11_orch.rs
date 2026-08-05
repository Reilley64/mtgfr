//! Legends (`leg`) grind, wave 11, orchestrator slice — the increments held back from the
//! per-slice fan-out because they touch shared engine machinery rather than one card family.
//!
//! Increments 134 (`until-end-of-combat-pump-duration`), 7 (`legendary-landwalk`),
//! 31 (`tap-creature-for-mana-by-mana-value`), 70 (`dies-this-turn-delayed-trigger`),
//! 83 (`triassic-egg`), 125 (`describe-a-filtered-anthems-pt-delta`) and
//! 54 (`lesser-werewolf`, the combat-partner target relation).

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Pass priority around until `player` holds it — the active player gets it first after blockers
/// are declared, so a defending player's instant has to wait its turn in the round (CR 117.3c).
fn pass_priority_to(game: &mut Game, player: PlayerId) {
    while game.priority_holder() != player {
        let holder = game.priority_holder();
        game.submit(Intent::PassPriority { player: holder })
            .unwrap();
    }
}

/// Cast an instant from `player`'s hand, auto-funded, at `target`.
fn cast_at(game: &mut Game, player: PlayerId, name: &str, target: ObjectId) -> ObjectId {
    pass_priority_to(game, player);
    game.fund_mana(player);
    let spell = game.spawn_in_hand(player, card(name));
    game.submit(Intent::Cast {
        player,
        object: spell,
        target: Some(Target::Object(target)),
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
    .unwrap_or_else(|e| panic!("{name} is castable: {e:?}"));
    spell
}

// ── increment 31: tap a creature for mana by its mana value ───────────────────────────

/// "Tap target untapped creature you control. If you do, add an amount of {C} equal to that
/// creature's mana value." Grizzly Bears is {1}{G}, so the ritual pays two colorless.
#[test]
fn energy_tap_pays_its_creatures_mana_value_in_colorless() {
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    cast_at(&mut game, PlayerId(0), "Energy Tap", bear);
    // `cast_at` funds the {U} from thin air, so read the ritual's payout as a delta.
    let before = game.colorless_in_pool(PlayerId(0));
    resolve_top_of_stack(&mut game);

    assert!(game.is_tapped(bear), "the creature taps");
    assert_eq!(
        game.colorless_in_pool(PlayerId(0)) - before,
        2,
        "Grizzly Bears' mana value is 2",
    );
}

/// The target clause is "untapped creature **you control**" on both axes — an already-tapped
/// creature is not a legal target at all, so the ritual can't be run twice off one body.
#[test]
fn energy_tap_cannot_target_an_already_tapped_creature() {
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    cast_at(&mut game, PlayerId(0), "Energy Tap", bear);
    resolve_top_of_stack(&mut game);

    game.fund_mana(PlayerId(0));
    let second = game.spawn_in_hand(PlayerId(0), card("Energy Tap"));
    let rejected = game.submit(Intent::Cast {
        player: PlayerId(0),
        object: second,
        target: Some(Target::Object(bear)),
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
    });
    assert_eq!(rejected, Err(Reject::IllegalTarget));
}

// ── increment 83: an activation gated on a counter threshold ──────────────────────────

/// Activate `object`'s ability `index` for `player`, funding whatever mana it asks for.
fn activate(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    index: usize,
    target: Option<Target>,
) -> Result<(), Reject> {
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: index,
        target,
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
    .map(|_| ())
}

/// Give every seat a library. Banking a second hatchling counter costs a full turn cycle, and a
/// seat that has to draw from an empty one loses the game before the Egg is ever ready
/// (CR 104.3c) — taking its permanents, and their counters, with it.
fn stack_libraries(game: &mut Game) {
    let deck = vec![card("Plains"); 40];
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &deck);
    }
}

/// Run Triassic Egg's `{3}, {T}` counter ability `times` times, waiting out the untap step in
/// between — the Egg can only bank one hatchling counter per turn cycle.
fn bank_hatchlings(game: &mut Game, egg: ObjectId, times: u8) {
    for _ in 0..times {
        advance_until(game, |g| {
            !g.is_tapped(egg) && g.priority_holder() == PlayerId(0)
        });
        activate(game, PlayerId(0), egg, 0, None).expect("the counter ability is unconditional");
        resolve_top_of_stack(game);
    }
}

/// "Sacrifice this artifact: … Activate only if there are two or more hatchling counters on this
/// artifact." (CR 602.2b) The gate is board state, not timing — one counter is not enough, and the
/// Egg is still on the battlefield afterwards because the illegal activation paid no cost.
#[test]
fn triassic_egg_wont_hatch_below_two_hatchling_counters() {
    let mut game = Game::new();
    stack_libraries(&mut game);
    let egg = game.spawn_on_battlefield(PlayerId(0), card("Triassic Egg"));
    game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(
        activate(&mut game, PlayerId(0), egg, 2, None),
        Err(Reject::CannotActivate),
        "a fresh Egg has no hatchling counters at all",
    );

    bank_hatchlings(&mut game, egg, 1);
    assert_eq!(
        activate(&mut game, PlayerId(0), egg, 2, None),
        Err(Reject::CannotActivate),
        "one counter is short of \"two or more\"",
    );
    assert_eq!(game.counters_of_kind(egg, CounterKind::Hatchling), 1);
    assert!(
        game.zone_of(egg) == Zone::Battlefield,
        "a rejected activation costs nothing"
    );
}

/// The same gate the other way: at two hatchling counters the reanimate mode activates, the Egg
/// pays its own sacrifice cost, and the creature card comes back.
#[test]
fn triassic_egg_hatches_a_graveyard_creature_at_two_counters() {
    let mut game = Game::new();
    stack_libraries(&mut game);
    let egg = game.spawn_on_battlefield(PlayerId(0), card("Triassic Egg"));
    let bear = game.spawn_in_graveyard(PlayerId(0), card("Grizzly Bears"));

    bank_hatchlings(&mut game, egg, 2);
    activate(&mut game, PlayerId(0), egg, 2, Some(Target::Object(bear)))
        .expect("two hatchling counters open the gate");
    assert_ne!(
        game.zone_of(egg),
        Zone::Battlefield,
        "\"Sacrifice this artifact\" is the cost",
    );

    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.zone_of(bear),
        Zone::Battlefield,
        "the Bears are reanimated"
    );
}

// ── increment 70: a delayed trigger keyed to one permanent dying ───────────────────────

/// Answer a [`PendingChoice::MayReturnFromGraveyard`] with `cards` (empty declines).
fn choose_returns(game: &mut Game, player: PlayerId, cards: Vec<ObjectId>) -> Result<(), Reject> {
    game.submit(Intent::ChooseSacrifices {
        player,
        sacrifices: cards,
    })
    .map(|_| ())
}

/// "Choose target creature. When that creature dies this turn, return a creature card from its
/// owner's graveyard to the battlefield under that creature's owner's control." (CR 603.7) The
/// delayed trigger is keyed to one permanent's *identity* rather than to a step, and the graveyard
/// it reaches into is the dead creature's owner's — not the caster's.
#[test]
fn reincarnation_returns_from_the_dead_creatures_owners_graveyard() {
    let mut game = Game::new();
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let theirs = game.spawn_in_graveyard(PlayerId(1), card("Hill Giant"));
    let mine = game.spawn_in_graveyard(PlayerId(0), card("Hill Giant"));

    cast_at(&mut game, PlayerId(0), "Reincarnation", victim);
    resolve_top_of_stack(&mut game);
    assert!(
        game.pending_choice().is_none(),
        "nothing happens until the chosen creature actually dies",
    );

    cast_at(&mut game, PlayerId(0), "Terror", victim);
    resolve_top_of_stack(&mut game);
    resolve_top_of_stack(&mut game);

    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::MayReturnFromGraveyard { .. })
        ),
        "the death fired the delayed trigger",
    );
    assert_eq!(
        choose_returns(&mut game, PlayerId(0), vec![mine]),
        Err(Reject::IllegalChoice),
        "the caster's own graveyard is not \"its owner's graveyard\"",
    );

    choose_returns(&mut game, PlayerId(0), vec![theirs]).expect("the dead creature's owner's is");
    assert_eq!(game.zone_of(theirs), Zone::Battlefield);
    assert_eq!(
        game.controller_of(game.current_id(theirs)),
        PlayerId(1),
        "\"under that creature's owner's control\" — the caster gets nothing",
    );
}

/// "When that creature dies **this turn**" (CR 603.7): the delayed trigger is a one-turn window.
/// A creature that survives to the next turn and dies there returns nothing.
#[test]
fn reincarnation_expires_at_the_end_of_the_turn_it_was_cast() {
    let mut game = Game::new();
    stack_libraries(&mut game);
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let theirs = game.spawn_in_graveyard(PlayerId(1), card("Hill Giant"));

    cast_at(&mut game, PlayerId(0), "Reincarnation", victim);
    resolve_top_of_stack(&mut game);

    pass_until_next_turn(&mut game);
    cast_at(&mut game, PlayerId(0), "Terror", victim);
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(victim),
        Zone::Graveyard,
        "the Bears still died"
    );
    assert!(
        game.pending_choice().is_none(),
        "the watch expired with the turn it was cast in",
    );
    assert_eq!(game.zone_of(theirs), Zone::Graveyard);
}

// ── increment 134: until-end-of-combat pump duration ──────────────────────────────────

/// "Target blocking Wall you control gets +10/+0 until end of combat." (CR 511.3) The boost is
/// gone the moment combat ends — the postcombat main phase is already too late to still see it.
#[test]
fn glyph_of_destruction_shrinks_its_wall_when_combat_ends() {
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Living Wall"));

    attack_with(&mut game, vec![bear]);
    block_with(&mut game, vec![(wall, bear)]).expect("a Wall may block");
    cast_at(&mut game, PlayerId(1), "Glyph of Destruction", wall);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.power(wall),
        10,
        "the Wall is 0/6 and the Glyph is +10/+0"
    );

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    assert_eq!(
        game.power(wall),
        0,
        "\"until end of combat\" ended at the end of combat step, not at cleanup",
    );
}

/// The bug the end-of-combat sweep's `ponytail:` predicted: it took *every* duration-scoped
/// modifier off a permanent that carried an until-end-of-combat one, so an ordinary until-end-of-
/// turn pump cast on the same creature ended early too (CR 514.2 — it should survive to cleanup).
#[test]
fn an_until_end_of_turn_pump_outlives_the_combat_it_was_cast_in() {
    let mut game = Game::new();
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Living Wall"));

    attack_with(&mut game, vec![bear]);
    block_with(&mut game, vec![(wall, bear)]).expect("a Wall may block");
    // Both durations land on the same Wall: the Glyph's until-end-of-combat +10/+0 and Giant
    // Growth's until-end-of-turn +3/+3.
    cast_at(&mut game, PlayerId(1), "Glyph of Destruction", wall);
    resolve_top_of_stack(&mut game);
    cast_at(&mut game, PlayerId(1), "Giant Growth", wall);
    resolve_top_of_stack(&mut game);
    assert_eq!(
        game.power(wall),
        13,
        "+10/+0 and +3/+3 are both live in combat"
    );

    advance_until(&mut game, |g| g.current_step() == Step::Main2);
    assert_eq!(
        game.power(wall),
        3,
        "only the Glyph's boost ended — Giant Growth lasts until cleanup",
    );
}

// ── increment 125: describing a filtered anthem's P/T delta ───────────────────────────

/// The Alt-inspect ledger's "+0/+2 from …" attribution has to name a *filtered* anthem
/// ([`StaticEffect::FilteredAnthem`]) the same way it names a plain one — Arcades Sabboth's "Each
/// untapped creature you control gets +0/+2 as long as it's not attacking" moves the board, so a
/// creature showing the boosted toughness with no source for it is a lie by omission.
#[test]
fn a_filtered_anthems_pt_delta_is_attributed_to_its_source() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));

    assert_eq!(game.toughness(bear), 4, "the anthem is live on the board");

    let sabboth = game
        .modifier_sources(bear)
        .into_iter()
        .find(|g| g.source_name == "Arcades Sabboth")
        .expect("the boost the Bears is showing has a named source");
    assert!(
        sabboth
            .contributions
            .contains(&ModifierContribution::PowerToughness {
                power: 0,
                toughness: 2,
            }),
        "the ledger names the delta it granted, not just the card: {:?}",
        sabboth.contributions,
    );
}

/// The filter is re-read live, so the attribution has to disappear with the boost: a tapped
/// creature is no longer "each untapped creature you control".
#[test]
fn a_filtered_anthems_attribution_drops_when_its_filter_stops_matching() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Arcades Sabboth"));
    let bear = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    game.tap(bear);

    assert_eq!(game.toughness(bear), 2, "a tapped creature loses the boost");
    assert!(
        !game
            .modifier_sources(bear)
            .iter()
            .any(|g| g.source_name == "Arcades Sabboth"),
        "and loses the attribution with it",
    );
}

// ── increment 7: legendary landwalk ───────────────────────────────────────────────────

/// "Legendary landwalk (This creature can't be blocked as long as defending player controls a
/// legendary land.)" — the same CR 702.14b evasion the basic-type walks get, keyed on a
/// *supertype* rather than a land subtype.
#[test]
fn livonya_silone_is_unblockable_across_a_legendary_land() {
    let mut game = Game::new();
    let livonya = game.spawn_on_battlefield(PlayerId(0), card("Livonya Silone"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(1), card("Karakas"));

    attack_with(&mut game, vec![livonya]);
    assert_eq!(
        block_with(&mut game, vec![(blocker, livonya)]),
        Err(Reject::IllegalDeclaration),
        "the defender's Karakas is a legendary land",
    );
}

/// The evasion is scoped to the *defending* player's board: a legendary land on the attacker's own
/// side waves nothing through, and neither does an ordinary Forest.
#[test]
fn livonya_silone_is_blockable_when_the_defender_runs_no_legendary_land() {
    let mut game = Game::new();
    let livonya = game.spawn_on_battlefield(PlayerId(0), card("Livonya Silone"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.spawn_on_battlefield(PlayerId(0), card("Karakas"));
    game.spawn_on_battlefield(PlayerId(1), card("Forest"));

    attack_with(&mut game, vec![livonya]);
    block_with(&mut game, vec![(blocker, livonya)])
        .expect("only the defending player's lands are read");
}

// ── increment 54: the combat-partner target relation ──────────────────────────────────

/// Activate `player`'s ability on `object` at `target`, with blocks already declared.
fn activate_at(
    game: &mut Game,
    player: PlayerId,
    object: ObjectId,
    target: Option<ObjectId>,
) -> Result<Vec<Event>, Reject> {
    pass_priority_to(game, player);
    game.fund_mana(player);
    game.submit(Intent::ActivateAbility {
        player,
        object,
        ability_index: 0,
        target: target.map(Target::Object),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

/// "{B}: If this creature's power is 1 or more, it gets -1/-0 until end of turn and put a -0/-1
/// counter on target creature blocking or blocked by this creature." The Werewolf pays its own
/// power to shrink whoever it is in combat with.
#[test]
fn lesser_werewolf_trades_its_power_for_its_blockers_toughness() {
    let mut game = Game::new();
    let werewolf = game.spawn_on_battlefield(PlayerId(0), card("Lesser Werewolf"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![werewolf]);
    block_with(&mut game, vec![(bear, werewolf)]).expect("the Bears can block");
    activate_at(&mut game, PlayerId(0), werewolf, Some(bear))
        .expect("its blocker is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        (game.power(werewolf), game.toughness(werewolf)),
        (1, 4),
        "-1/-0 on itself"
    );
    assert_eq!(
        (game.power(bear), game.toughness(bear)),
        (2, 1),
        "a -0/-1 counter on the blocker"
    );
}

/// "target creature blocking or blocked by *this creature*" is a pairing, not a board-wide axis:
/// a creature blocking someone else is blocking, and still not a legal target.
#[test]
fn lesser_werewolf_cant_reach_a_creature_it_is_not_in_combat_with() {
    let mut game = Game::new();
    let werewolf = game.spawn_on_battlefield(PlayerId(0), card("Lesser Werewolf"));
    let other = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let bystander = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![werewolf, other]);
    block_with(&mut game, vec![(bear, other)]).expect("the Bears block the other attacker");

    assert_eq!(
        activate_at(&mut game, PlayerId(0), werewolf, Some(bear)),
        Err(Reject::IllegalTarget),
        "blocking someone else is not blocking the Werewolf",
    );
    assert_eq!(
        activate_at(&mut game, PlayerId(0), werewolf, Some(bystander)),
        Err(Reject::IllegalTarget),
        "and a creature at home is in no combat at all",
    );
}

/// "Activate only during the declare blockers step" (CR 602.5b) — the window is the step, so the
/// precombat main phase is shut.
#[test]
fn lesser_werewolf_only_activates_during_declare_blockers() {
    let mut game = Game::new();
    let werewolf = game.spawn_on_battlefield(PlayerId(0), card("Lesser Werewolf"));
    let bear = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    advance_until(&mut game, |g| g.current_step() == Step::Main1);
    assert_eq!(
        activate_at(&mut game, PlayerId(0), werewolf, Some(bear)),
        Err(Reject::WrongTiming),
        "no blocks are declared yet, so there is nothing to shrink",
    );
}

/// Sentinel's "{0}: Change this creature's base toughness to 1 plus the power of target creature
/// blocking or blocked by this creature" reads the same pairing, and drops the `approximates` it
/// carried while the filter had no combat-partner axis.
#[test]
fn sentinel_reads_only_the_creature_it_is_in_combat_with() {
    let mut game = Game::new();
    let sentinel = game.spawn_on_battlefield(PlayerId(0), card("Sentinel"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    let bystander = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![sentinel]);
    block_with(&mut game, vec![(blocker, sentinel)]).expect("the Bears can block");

    assert_eq!(
        activate_at(&mut game, PlayerId(0), sentinel, Some(bystander)),
        Err(Reject::IllegalTarget),
        "the creature that stayed home is out of reach",
    );
    activate_at(&mut game, PlayerId(0), sentinel, Some(blocker))
        .expect("the creature blocking it is in reach");
    resolve_top_of_stack(&mut game);
    assert_eq!(game.toughness(sentinel), 3, "1 plus the Bears' power");
}

// ── increment 36: dying into a revealed, unplayable hand card ─────────────────────────

/// Cast `object` untargeted from `player`'s hand, auto-funded.
fn cast(game: &mut Game, player: PlayerId, object: ObjectId) -> Result<Vec<Event>, Reject> {
    pass_priority_to(game, player);
    game.fund_mana(player);
    game.submit(Intent::Cast {
        player,
        object,
        target: None,
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

/// Kill `victim` with a Lightning Bolt from `player`.
fn bolt(game: &mut Game, player: PlayerId, victim: ObjectId) {
    cast_at(game, player, "Lightning Bolt", victim);
    resolve_top_of_stack(game);
}

/// "If this creature would die, return it to its owner's hand instead" (CR 614.1b) — a replacement,
/// so the Phoenix never touches the graveyard and nothing sees it die.
#[test]
fn firestorm_phoenix_returns_to_hand_instead_of_dying() {
    let mut game = Game::new();
    let phoenix = game.spawn_on_battlefield(PlayerId(0), card("Firestorm Phoenix"));

    bolt(&mut game, PlayerId(1), phoenix);

    let hand = game.hand(PlayerId(0));
    assert_eq!(hand.len(), 1, "the Phoenix came back, exactly once");
    assert_eq!(
        game.zone_of(hand[0]),
        Zone::Hand,
        "the only object it left behind is that hand card, not a corpse",
    );
}

/// "Until that player's next turn, that player … can't play it." The window shuts at the owner's
/// next turn, not at the end of the turn it died in.
#[test]
fn the_returned_phoenix_cant_be_played_until_its_owners_next_turn() {
    let mut game = Game::new();
    let deck = vec![card("Mountain"); 40];
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &deck);
    }
    let phoenix = game.spawn_on_battlefield(PlayerId(0), card("Firestorm Phoenix"));
    bolt(&mut game, PlayerId(1), phoenix);
    let returned = game.hand(PlayerId(0))[0];

    assert_eq!(
        cast(&mut game, PlayerId(0), returned),
        Err(Reject::NotCastable),
        "this is still the turn it died in",
    );

    advance_until(&mut game, |g| g.active_player() != PlayerId(0));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
    cast(&mut game, PlayerId(0), returned).expect("the owner's next turn opens the window");
}

/// "…that player plays with that card revealed in their hand": every other seat can see it while
/// the window is open, and stops being able to when it shuts.
#[test]
fn the_returned_phoenix_is_revealed_while_it_is_unplayable() {
    let mut game = Game::new();
    let deck = vec![card("Mountain"); 40];
    for p in 0..game.player_count() as u8 {
        game.stack_library(PlayerId(p), &deck);
    }
    let phoenix = game.spawn_on_battlefield(PlayerId(0), card("Firestorm Phoenix"));
    bolt(&mut game, PlayerId(1), phoenix);
    let returned = game.hand(PlayerId(0))[0];

    assert!(
        game.has_seen_hand_card(PlayerId(1), returned),
        "an opponent can read the card while the window is open",
    );

    advance_until(&mut game, |g| g.active_player() != PlayerId(0));
    advance_until(&mut game, |g| {
        g.active_player() == PlayerId(0) && g.current_step() == Step::Main1
    });
    assert!(
        !game.has_seen_hand_card(PlayerId(1), returned),
        "and the hand closes again with the window",
    );
}
