//! Legends (`leg`) grind — increment 15: cant-be-targeted-by.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

fn cast(game: &mut Game, object: ObjectId, target: Option<Target>) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::Cast {
        player: PlayerId(0),
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

fn cast_and_resolve(game: &mut Game, object: ObjectId, target: Option<Target>) -> ObjectId {
    cast(game, object, target).expect("a legal cast");
    resolve_top_of_stack(game);
    game.current_id(object)
}

/// Activate Tetsuo's "{U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature" (mana
/// prefunded) at `target` — ability index 1, behind the static shield in slot 0.
fn tetsuo_destroys(
    game: &mut Game,
    tetsuo: ObjectId,
    target: ObjectId,
) -> Result<Vec<Event>, Reject> {
    game.fund_mana(PlayerId(0));
    game.submit(Intent::ActivateAbility {
        player: PlayerId(0),
        object: tetsuo,
        ability_index: 1,
        target: Some(Target::Object(target)),
        sacrifice: vec![],
        discard_cost: vec![],
        x: 0,
    })
}

// ── Bartel Runeaxe: "Vigilance / Bartel Runeaxe can't be the target of Aura spells." ──

#[test]
fn an_aura_spell_cant_target_bartel_runeaxe() {
    let mut game = Game::new();
    let bartel = game.spawn_on_battlefield(PlayerId(0), card("Bartel Runeaxe"));
    let strength = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));

    assert_eq!(
        cast(&mut game, strength, Some(Target::Object(bartel))),
        Err(Reject::IllegalTarget),
        "\"can't be the target of Aura spells\" — even its own controller's"
    );
    assert_eq!(
        game.zone_of(strength),
        Zone::Hand,
        "the rejected Aura never left the hand"
    );
    assert!(
        !game
            .legal_targets(strength, None)
            .contains(&Target::Object(bartel)),
        "and it isn't offered as a choice either"
    );
}

#[test]
fn bartel_runeaxe_leaves_other_creatures_open_to_aura_spells() {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Bartel Runeaxe"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let strength = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));

    let strength = cast_and_resolve(&mut game, strength, Some(Target::Object(bears)));

    assert_eq!(
        game.attached_to(strength),
        Some(bears),
        "the shield is Bartel's own, not a board-wide Aura ban"
    );
}

#[test]
fn bartel_runeaxe_is_still_a_legal_target_for_a_non_aura_spell() {
    // "can't be the target of *Aura* spells" — a filtered restriction, not shroud (CR 702.18).
    let mut game = Game::new();
    let bartel = game.spawn_on_battlefield(PlayerId(0), card("Bartel Runeaxe"));
    let unsummon = game.spawn_in_hand(PlayerId(0), card("Unsummon"));

    cast_and_resolve(&mut game, unsummon, Some(Target::Object(bartel)));

    assert_eq!(
        game.zone_of(bartel),
        Zone::Hand,
        "an instant still bounces the Giant"
    );
}

#[test]
fn bartel_runeaxe_has_vigilance() {
    let mut game = Game::new();
    let bartel = game.spawn_on_battlefield(PlayerId(0), card("Bartel Runeaxe"));

    attack_with(&mut game, vec![bartel]);

    assert!(
        !game.is_tapped(bartel),
        "\"Vigilance\" — attacking doesn't tap it"
    );
}

// ── Tetsuo Umezawa: "Tetsuo Umezawa can't be the target of Aura spells. /
// {U}{B}{B}{R}, {T}: Destroy target tapped or blocking creature." ─────────────────────

#[test]
fn an_aura_spell_cant_target_tetsuo_umezawa() {
    let mut game = Game::new();
    let tetsuo = game.spawn_on_battlefield(PlayerId(0), card("Tetsuo Umezawa"));
    let strength = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));

    assert_eq!(
        cast(&mut game, strength, Some(Target::Object(tetsuo))),
        Err(Reject::IllegalTarget),
        "the same shield Bartel Runeaxe prints"
    );
    assert_eq!(game.zone_of(strength), Zone::Hand, "the Aura stays in hand");
}

#[test]
fn tetsuo_umezawa_destroys_a_tapped_creature() {
    let mut game = Game::new();
    let tetsuo = game.spawn_on_battlefield(PlayerId(0), card("Tetsuo Umezawa"));
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.tap(victim);

    tetsuo_destroys(&mut game, tetsuo, victim).expect("a tapped creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(victim),
        Zone::Graveyard,
        "\"target tapped or blocking creature\" — the tapped half"
    );
    assert!(game.is_tapped(tetsuo), "the {{T}} cost tapped Tetsuo");
}

#[test]
fn tetsuo_umezawa_destroys_a_blocking_creature() {
    let mut game = Game::new();
    let tetsuo = game.spawn_on_battlefield(PlayerId(0), card("Tetsuo Umezawa"));
    let attacker = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let blocker = game.spawn_on_battlefield(PlayerId(1), card("Gray Ogre"));
    attack_with(&mut game, vec![attacker]);
    block_with(&mut game, vec![(blocker, attacker)]).expect("a legal block");

    tetsuo_destroys(&mut game, tetsuo, blocker).expect("a blocking creature is a legal target");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(blocker),
        Zone::Graveyard,
        "the blocking half of the union axis — the blocker is untapped"
    );
    assert!(
        !game.is_tapped(blocker),
        "it was never tapped; blocking alone made it legal"
    );
}

#[test]
fn tetsuo_umezawa_cannot_target_an_untapped_creature_out_of_combat() {
    // Neither tapped nor blocking, so it is not a legal target and cannot be chosen as the
    // ability is announced (CR 602.2b → CR 601.2c).
    let mut game = Game::new();
    let tetsuo = game.spawn_on_battlefield(PlayerId(0), card("Tetsuo Umezawa"));
    let bystander = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    assert_eq!(
        tetsuo_destroys(&mut game, tetsuo, bystander),
        Err(Reject::IllegalTarget),
        "\"target tapped or blocking creature\" — this one is neither"
    );

    assert_eq!(
        game.zone_of(bystander),
        Zone::Battlefield,
        "an untapped creature at home is neither tapped nor blocking"
    );
}

// ── Anti-Magic Aura: "Enchant creature / Enchanted creature can't be the target of spells and
// can't be enchanted by other Auras." ─────────────────────────────────────────────────

#[test]
fn anti_magic_aura_stops_a_spell_targeting_the_enchanted_creature() {
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let anti = game.spawn_in_hand(PlayerId(0), card("Anti-Magic Aura"));
    cast_and_resolve(&mut game, anti, Some(Target::Object(bears)));

    let unsummon = game.spawn_in_hand(PlayerId(0), card("Unsummon"));
    assert_eq!(
        cast(&mut game, unsummon, Some(Target::Object(bears))),
        Err(Reject::IllegalTarget),
        "\"can't be the target of spells\""
    );
    assert_eq!(
        game.zone_of(bears),
        Zone::Battlefield,
        "the bounce never happened"
    );
}

#[test]
fn an_aura_spell_cant_target_a_creature_wearing_anti_magic_aura() {
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let anti = game.spawn_in_hand(PlayerId(0), card("Anti-Magic Aura"));
    cast_and_resolve(&mut game, anti, Some(Target::Object(bears)));

    let strength = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));
    assert_eq!(
        cast(&mut game, strength, Some(Target::Object(bears))),
        Err(Reject::IllegalTarget),
        "an Aura spell is a spell"
    );
    assert_eq!(game.zone_of(strength), Zone::Hand, "it stays in hand");
}

#[test]
fn an_ability_can_still_target_a_creature_wearing_anti_magic_aura() {
    // The clause names *spells*; a permanent's activated ability is not one (CR 111.1), so the
    // shield is narrower than shroud (CR 702.18).
    let mut game = Game::new();
    let tetsuo = game.spawn_on_battlefield(PlayerId(0), card("Tetsuo Umezawa"));
    let victim = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    game.tap(victim);
    let anti = game.spawn_in_hand(PlayerId(0), card("Anti-Magic Aura"));
    cast_and_resolve(&mut game, anti, Some(Target::Object(victim)));

    tetsuo_destroys(&mut game, tetsuo, victim).expect("an ability is not a spell");
    resolve_top_of_stack(&mut game);

    assert_eq!(
        game.zone_of(victim),
        Zone::Graveyard,
        "Tetsuo's activated ability still destroys it"
    );
}

#[test]
fn anti_magic_aura_puts_an_aura_already_on_the_creature_into_the_graveyard() {
    // CR 303.4 / 704.5n: "can't be enchanted by other Auras" is an *attach* restriction checked
    // continuously, so an Aura that was legally there first becomes illegally attached and falls
    // off — not merely a targeting rejection.
    let mut game = Game::new();
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let strength = game.spawn_in_hand(PlayerId(0), card("Holy Strength"));
    let strength = cast_and_resolve(&mut game, strength, Some(Target::Object(bears)));
    assert_eq!(game.attached_to(strength), Some(bears), "it went on first");

    let anti = game.spawn_in_hand(PlayerId(0), card("Anti-Magic Aura"));
    let anti = cast_and_resolve(&mut game, anti, Some(Target::Object(bears)));

    assert_eq!(
        game.zone_of(strength),
        Zone::Graveyard,
        "the earlier Aura is now illegally attached (CR 704.5n)"
    );
    assert_eq!(
        game.attached_to(anti),
        Some(bears),
        "the Aura doing the closing isn't an *other* Aura"
    );
}

#[test]
fn anti_magic_aura_leaves_other_creatures_targetable() {
    let mut game = Game::new();
    let warded = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let open = game.spawn_on_battlefield(PlayerId(0), card("Gray Ogre"));
    let anti = game.spawn_in_hand(PlayerId(0), card("Anti-Magic Aura"));
    cast_and_resolve(&mut game, anti, Some(Target::Object(warded)));

    let unsummon = game.spawn_in_hand(PlayerId(0), card("Unsummon"));
    cast_and_resolve(&mut game, unsummon, Some(Target::Object(open)));

    assert_eq!(
        game.zone_of(open),
        Zone::Hand,
        "only the enchanted creature is shielded"
    );
}
