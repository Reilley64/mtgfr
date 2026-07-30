//! Legends (`leg`) grind — increment 3: bands-with-other, slices 1 (band formation), 2 (blocked as
//! a group) and 3 (the damage assignment transfer).
//!
//! Slice 1 is CR 702.22c: "As a player declares attackers, they may declare that one or more
//! attacking creatures with banding and up to one attacking creature without banding … are all in a
//! 'band.' They may also declare that one or more attacking \[quality\] creatures with 'bands with
//! other \[quality\]' and any number of other attacking \[quality\] creatures are all in a band."
//!
//! Slice 2 is CR 702.22h: "If an attacking creature becomes blocked by a creature, each other
//! creature in the same band as the attacking creature becomes blocked by that same blocking
//! creature."
//!
//! Slice 3 is CR 702.22j — "if an attacking creature is being blocked by a creature with banding,
//! or by both a \[quality\] creature with 'bands with other \[quality\]' and another \[quality\]
//! creature, the defending player (rather than the active player) chooses how the attacking
//! creature's damage is assigned" — and its mirror CR 702.22k, "if a blocking creature is blocking
//! a creature with banding, or both a \[quality\] creature with 'bands with other \[quality\]' and
//! another \[quality\] creature, the active player (rather than the defending player) chooses how
//! the blocking creature's damage is assigned." They are exceptions to CR 510.1c and CR 510.1d
//! respectively, so the plain blocker-side division (CR 510.1d) is tested first, with no banding
//! in it at all.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Player 0 with `land` on the battlefield, plus `creatures` spawned for them.
fn banding_land_board(land: &str, creatures: &[&str]) -> (Game, Vec<ObjectId>) {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card(land));
    let ids = creatures
        .iter()
        .map(|name| game.spawn_on_battlefield(PlayerId(0), card(name)))
        .collect();
    (game, ids)
}

/// Player 0 with a Cathedral of Serra on the battlefield, plus `creatures` spawned for them.
fn cathedral_board(creatures: &[&str]) -> (Game, Vec<ObjectId>) {
    banding_land_board("Cathedral of Serra", creatures)
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

/// The seat the pending combat-damage division belongs to, panicking if none is pending.
fn pending_assigner(game: &Game) -> PlayerId {
    let Some(PendingChoice::AssignCombatDamage { player, .. }) = game.pending_choice() else {
        panic!(
            "a combat damage division should be pending; got {:?}",
            game.pending_choice()
        );
    };
    player
}

/// Answer the pending division on behalf of the seat it belongs to.
fn assign(game: &mut Game, assignment: Vec<(ObjectId, i32)>) -> Result<Vec<Event>, Reject> {
    game.submit(Intent::AssignDamage {
        player: pending_assigner(game),
        assignment,
    })
}

// ── tests ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cathedral_of_serra_grants_bands_with_other_legendary_creatures() {
    // "White legendary creatures you control have 'bands with other legendary creatures.'"
    let (game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard", "Grizzly Bears"]);
    let [jasmine, barktooth, bears] = ids[..] else {
        unreachable!("three creatures spawned")
    };
    let bands_with = Keyword::BandsWith(BandsWithQuality::Legendary);
    assert!(
        game.has_keyword(jasmine, bands_with),
        "Jasmine Boreal is a white legendary creature, so the Cathedral grants it"
    );
    assert!(
        !game.has_keyword(barktooth, bands_with),
        "Barktooth Warbeard is legendary but not white — no grant"
    );
    assert!(
        !game.has_keyword(bears, bands_with),
        "Grizzly Bears is neither white nor legendary"
    );
}

#[test]
fn a_legendary_band_forms_and_still_deals_its_combat_damage() {
    // CR 702.22c: one member has "bands with other legendary creatures" (Jasmine, via the
    // Cathedral) and the other is a legendary creature, so the band is legal — Barktooth needs no
    // grant of its own. Slice 1 changes no damage, so the 4/5 and the 6/5 still connect for 10.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band is a legal declaration");

    assert_eq!(
        game.attacking_bands(),
        [vec![jasmine, barktooth]],
        "the declared band is recorded as a group"
    );
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        10,
        "recording the band must not cost the band its combat damage"
    );
}

#[test]
fn a_band_needs_a_member_with_bands_with_other() {
    // CR 702.22c: "at least one has 'bands with other legendary creatures.'" Two legends with no
    // Cathedral in play is two legends, not a band.
    let mut game = Game::new();
    let jasmine = game.spawn_on_battlefield(PlayerId(0), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    assert!(
        attack_in_bands(
            &mut game,
            &[jasmine, barktooth],
            vec![vec![jasmine, barktooth]]
        )
        .is_err(),
        "no member has bands with other, so the band is illegal"
    );
    assert!(
        game.attacking_bands().is_empty(),
        "a rejected declaration records nothing"
    );
    assert_eq!(game.life(PlayerId(1)), 20, "and nobody attacked");
}

#[test]
fn a_nonlegendary_creature_cant_join_a_legendary_band() {
    // CR 702.22c: every other member must be a creature of the band's own \[quality\] — the Bears
    // is not legendary, so it can't ride along on Jasmine's grant.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Grizzly Bears"]);
    let [jasmine, bears] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    assert!(
        attack_in_bands(&mut game, &ids, vec![vec![jasmine, bears]]).is_err(),
        "Grizzly Bears fails the band's quality (CR 702.22c)"
    );
    assert!(game.attacking_bands().is_empty());
}

#[test]
fn a_band_must_attack_the_same_defender() {
    // CR 702.22d: "All creatures in an attacking band must attack the same player, planeswalker,
    // or battle."
    let mut game = Game::with_players(4, 0);
    game.spawn_on_battlefield(PlayerId(0), card("Cathedral of Serra"));
    let jasmine = game.spawn_on_battlefield(PlayerId(0), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(0), card("Barktooth Warbeard"));
    advance_until(&mut game, |g| g.current_step() == Step::DeclareAttackers);
    let split = game.submit(Intent::DeclareAttackersInBands {
        player: PlayerId(0),
        attackers: vec![
            (jasmine, Defender::Player(PlayerId(1))),
            (barktooth, Defender::Player(PlayerId(2))),
        ],
        bands: vec![vec![jasmine, barktooth]],
    });
    assert!(
        split.is_err(),
        "a band split across two defenders is illegal"
    );
}

#[test]
fn a_band_member_must_be_an_attacker() {
    // CR 702.22c declares bands out of the *attacking* creatures.
    let (mut game, ids) =
        cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard", "Jedit Ojanen"]);
    let [jasmine, barktooth, jedit] = ids[..] else {
        unreachable!("three creatures spawned")
    };
    assert!(
        attack_in_bands(&mut game, &[jasmine, barktooth], vec![vec![jasmine, jedit]]).is_err(),
        "Jedit was never declared as an attacker"
    );
}

#[test]
fn a_creature_can_be_in_only_one_band() {
    // CR 702.22c: "A player may declare as many attacking bands as they want, but each creature
    // may be a member of only one of them."
    let (mut game, ids) =
        cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard", "Jedit Ojanen"]);
    let [jasmine, barktooth, jedit] = ids[..] else {
        unreachable!("three creatures spawned")
    };
    assert!(
        attack_in_bands(
            &mut game,
            &ids,
            vec![vec![jasmine, barktooth], vec![jasmine, jedit]],
        )
        .is_err(),
        "Jasmine cannot be a member of two bands"
    );
}

#[test]
fn two_bands_can_be_declared_in_one_attack() {
    // CR 702.22c: "A player may declare as many attacking bands as they want." Jedit is white and
    // legendary, so the Cathedral grants it too — each band carries its own granter.
    let (mut game, ids) = cathedral_board(&[
        "Jasmine Boreal",
        "Barktooth Warbeard",
        "Jedit Ojanen",
        "Hunding Gjornersen",
    ]);
    let [jasmine, barktooth, jedit, hunding] = ids[..] else {
        unreachable!("four creatures spawned")
    };
    attack_in_bands(
        &mut game,
        &ids,
        vec![vec![jasmine, barktooth], vec![jedit, hunding]],
    )
    .expect("two legendary bands in one declaration");
    assert_eq!(
        game.attacking_bands(),
        [vec![jasmine, barktooth], vec![jedit, hunding]]
    );
}

#[test]
fn plain_banding_forms_a_band_with_up_to_one_creature_without_it() {
    // CR 702.22c's first sentence, which "bands with other" is a special form of (CR 702.22b):
    // "one or more attacking creatures with banding and up to one attacking creature without
    // banding … are all in a band."
    let mut game = Game::new();
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Timber Wolves"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    assert!(
        attack_in_bands(&mut game, &[wolves, bears], vec![vec![wolves, bears]]).is_ok(),
        "one banding creature plus one without it is a legal band"
    );

    let mut game = Game::new();
    let wolves = game.spawn_on_battlefield(PlayerId(0), card("Timber Wolves"));
    let bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let more_bears = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    assert!(
        attack_in_bands(
            &mut game,
            &[wolves, bears, more_bears],
            vec![vec![wolves, bears, more_bears]],
        )
        .is_err(),
        "only one member may lack banding (CR 702.22c)"
    );
}

#[test]
fn a_band_of_one_is_not_a_band() {
    // A single creature is behaviorally never a band — nothing is blocked as a group with it and
    // no damage division moves — so the declaration is rejected rather than recorded as noise.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal"]);
    assert!(
        attack_in_bands(&mut game, &ids, vec![vec![ids[0]]]).is_err(),
        "a one-member band is rejected"
    );
}

#[test]
fn an_ordinary_attack_declares_no_bands() {
    // The additive half of the slice: nothing about a bandless attack changed.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal"]);
    attack_with(&mut game, ids.clone());
    assert!(
        game.attacking_bands().is_empty(),
        "an ordinary declaration records no band"
    );
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(game.life(PlayerId(1)), 16, "the 4/5 connected as usual");
}

// ── slice 2: blocked as a group (CR 702.22h) ──────────────────────────────────────────

#[test]
fn blocking_one_member_of_a_band_blocks_the_whole_band() {
    // CR 702.22h: "if an attacking creature becomes blocked by a creature, each other creature in
    // the same band as the attacking creature becomes blocked by that same blocking creature." The
    // defending player declares one block and gets two blocked attackers — so nothing gets through.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band is a legal declaration");
    block_with(&mut game, vec![(bears, jasmine)]).expect("blocking one member is legal");

    assert!(
        game.blocks().contains(&(bears, barktooth)),
        "the undeclared band member is blocked by the same creature"
    );
    // Blocking two attackers, the Bears divides its 2 — and the band it blocked moves that division
    // to the active player (CR 702.22k, covered in its own test below).
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assign(&mut game, vec![(jasmine, 2), (barktooth, 0)]).expect("the Bears' 2 goes somewhere");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "both band members were blocked, so no combat damage reached the defending player"
    );
    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "the 2/2 blocked a 4/5 and a 6/5 at once and died to both"
    );
    assert_eq!(game.zone_of(jasmine), Zone::Battlefield);
    assert_eq!(game.zone_of(barktooth), Zone::Battlefield);
}

#[test]
fn a_swampwalking_band_member_becomes_blocked_with_its_band() {
    // CR 702.22h's own example, in the shape the pool can build it: "a player attacks with a band
    // consisting of a creature with flying and a creature with swampwalk. The defending player, who
    // controls a Swamp, can block the flying creature if able. If they do, then the creature with
    // swampwalk will also become blocked." Being blocked as a group is a consequence, not a
    // legality test — a member the blocker could never have blocked itself is blocked anyway.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Sol'kanar the Swamp King"]);
    let [jasmine, solkanar] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    game.spawn_on_battlefield(PlayerId(1), card("Swamp"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, solkanar]])
        .expect("a legendary band is a legal declaration");

    assert!(
        block_with(&mut game, vec![(bears, solkanar)]).is_err(),
        "swampwalk still can't be blocked by a defender with a Swamp (CR 702.14b)"
    );
    block_with(&mut game, vec![(bears, jasmine)]).expect("blocking the other member is legal");
    assert!(
        game.blocked_attackers().contains(&solkanar),
        "the swampwalker becomes blocked along with its band"
    );
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assign(&mut game, vec![(jasmine, 2), (solkanar, 0)]).expect("the Bears' 2 goes somewhere");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "the swampwalker was blocked, so it dealt no damage to the defending player"
    );
    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "the 2/2 blocked 4 + 5"
    );
}

#[test]
fn only_the_blocked_creatures_own_band_becomes_blocked() {
    // The extension is per band (CR 702.22h says "in the same band"), so an attacker that joined no
    // band is unaffected and connects for its own power.
    let (mut game, ids) =
        cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard", "Jedit Ojanen"]);
    let [jasmine, barktooth, jedit] = ids[..] else {
        unreachable!("three creatures spawned")
    };
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band plus a lone attacker is a legal declaration");
    block_with(&mut game, vec![(wall, jasmine)]).expect("blocking a band member is legal");

    assert!(
        !game.blocked_attackers().contains(&jedit),
        "Jedit Ojanen is in no band, so blocking the band leaves it unblocked"
    );
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        15,
        "only the unbanded 5/5 got through"
    );
}

#[test]
fn an_unbanded_attack_is_blocked_creature_by_creature() {
    // The additive half of the slice: with no band declared, one block blocks one attacker.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let wall = game.spawn_on_battlefield(PlayerId(1), card("Wall of Stone"));
    attack_with(&mut game, ids.clone());
    block_with(&mut game, vec![(wall, jasmine)]).expect("an ordinary block");

    assert_eq!(
        game.blocks(),
        [(wall, jasmine)],
        "no band, no extension — Barktooth Warbeard stays unblocked"
    );
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(game.life(PlayerId(1)), 14, "the unblocked 6/5 connected");
    assert_eq!(game.zone_of(barktooth), Zone::Battlefield);
}

#[test]
fn a_blocker_blocking_two_band_members_records_each_block_once() {
    // A creature that can block an additional creature may declare both pairs itself, and the band
    // extension must not write the second one down twice — a doubled block would double the damage
    // and re-fire the blocker's triggers.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Two-Headed Giant of Foriys"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band is a legal declaration");
    block_with(&mut game, vec![(giant, jasmine), (giant, barktooth)])
        .expect("Two-Headed Giant of Foriys can block an additional creature");

    assert_eq!(
        game.blocks(),
        [(giant, jasmine), (giant, barktooth)],
        "each pair is recorded exactly once"
    );
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assert_eq!(
        pending_assigner(&game),
        PlayerId(0),
        "neither attacker has two blockers, but the Giant blocks two attackers and so divides its \
         own damage (CR 510.1d) — moved to the active player by the band it blocked (CR 702.22k)"
    );
    assign(&mut game, vec![(jasmine, 4), (barktooth, 0)])
        .expect("the Giant's whole 4 may go to one member of the band");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(game.life(PlayerId(1)), 20, "both attackers were blocked");
    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "the 4/4 took 4 from Jasmine Boreal and 6 from Barktooth Warbeard"
    );
}

// ── slice 4: the remaining granting cards ─────────────────────────────────────────────

#[test]
fn each_legends_banding_land_grants_bands_with_other_to_its_own_color() {
    // "<Color> legendary creatures you control have 'bands with other legendary creatures.'" — the
    // `cycle-leg-banding-land` cycle is one card per color, and each grant is color-gated.
    let bands_with = Keyword::BandsWith(BandsWithQuality::Legendary);
    let cycle: &[(&str, &str, &str)] = &[
        // land, a legendary creature of its color, a legendary creature of some other color
        (
            "Adventurers' Guildhouse",
            "Jerrard of the Closed Fist",
            "Hunding Gjornersen",
        ),
        (
            "Mountain Stronghold",
            "Barktooth Warbeard",
            "Jasmine Boreal",
        ),
        ("Seafarer's Quay", "Jedit Ojanen", "Barktooth Warbeard"),
        ("Unholy Citadel", "Sivitri Scarzam", "Jedit Ojanen"),
    ];
    for &(land, on_color, off_color) in cycle {
        let (game, ids) = banding_land_board(land, &[on_color, off_color]);
        let [granted, ungranted] = ids[..] else {
            unreachable!("two creatures spawned")
        };
        assert!(
            game.has_keyword(granted, bands_with),
            "{land} grants {on_color}, a legendary creature of its color"
        );
        assert!(
            !game.has_keyword(ungranted, bands_with),
            "{land} does not reach {off_color}, a legendary creature of another color"
        );
    }
}

#[test]
fn a_green_band_under_adventurers_guildhouse_is_blocked_as_a_group() {
    // The Guildhouse's grant carries the whole banding surface, not just the keyword: two green
    // legendary creatures band (CR 702.22c) and one block catches both (CR 702.22h), so nothing
    // reaches the defending player.
    let (mut game, ids) = banding_land_board(
        "Adventurers' Guildhouse",
        &["Jerrard of the Closed Fist", "Marhault Elsdragon"],
    );
    let [jerrard, marhault] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_in_bands(&mut game, &ids, vec![vec![jerrard, marhault]])
        .expect("two green legendary creatures under the Guildhouse are a legal band");
    block_with(&mut game, vec![(bears, jerrard)]).expect("blocking one member is legal");

    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);
    assign(&mut game, vec![(jerrard, 2), (marhault, 0)]).expect("the Bears' 2 goes somewhere");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert_eq!(
        game.life(PlayerId(1)),
        20,
        "both band members were blocked, so no combat damage reached the defending player"
    );
    assert_eq!(
        game.zone_of(bears),
        Zone::Graveyard,
        "the 2/2 blocked a 6/5 and a 4/6 at once and died to both"
    );
    assert_eq!(game.zone_of(jerrard), Zone::Battlefield);
    assert_eq!(game.zone_of(marhault), Zone::Battlefield);
}

// ── slice 3: the damage assignment transfer (CR 510.1d, then CR 702.22j/k) ────────────

#[test]
fn a_blocker_blocking_two_attackers_divides_its_combat_damage() {
    // CR 510.1d, with no banding anywhere: "each blocking creature assigns combat damage, divided
    // as its controller chooses, among the attacking creatures it's blocking." Two-Headed Giant of
    // Foriys blocks an additional creature on its own printed text, so the plain rule is reachable
    // without a band — and its whole 4 power belongs to one of the two Bears if its controller
    // says so, rather than being dealt twice over.
    let mut game = Game::new();
    let first = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let second = game.spawn_on_battlefield(PlayerId(0), card("Grizzly Bears"));
    let giant = game.spawn_on_battlefield(PlayerId(1), card("Two-Headed Giant of Foriys"));

    attack_with(&mut game, vec![first, second]);
    block_with(&mut game, vec![(giant, first), (giant, second)])
        .expect("Two-Headed Giant of Foriys can block an additional creature");
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);

    assert_eq!(
        pending_assigner(&game),
        PlayerId(1),
        "CR 510.1d leaves a blocker's division with its own controller"
    );
    assert!(
        assign(&mut game, vec![(first, 2), (second, 1)]).is_err(),
        "CR 702.19c: the Giant's trample is no licence to hold damage back while blocking — the \
         division must total its power"
    );
    assign(&mut game, vec![(first, 4), (second, 0)])
        .expect("the whole 4 may go to one of the two attackers");

    assert_eq!(
        (game.zone_of(first), game.zone_of(second)),
        (Zone::Graveyard, Zone::Battlefield),
        "the division decided which 2/2 died; the Giant did not deal 4 to each"
    );
    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "the 4/4 still took 2 from each attacker"
    );
}

#[test]
fn a_blocker_blocking_one_attacker_is_asked_for_no_division() {
    // CR 510.1d only divides among "the attacking creatures it's blocking" — with one of them
    // there is nothing to divide, so no choice is raised and the blocker deals its whole power.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal"]);
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));
    attack_with(&mut game, ids.clone());
    block_with(&mut game, vec![(bears, ids[0])]).expect("an ordinary block");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert!(
        game.pending_choice().is_none(),
        "one attacker blocked is no division; got {:?}",
        game.pending_choice()
    );
    assert_eq!(game.marked_damage(ids[0]), 2, "the Bears dealt its whole 2");
}

#[test]
fn the_active_player_divides_a_blockers_damage_among_the_band_it_blocks() {
    // CR 702.22k: the blocking Craw Wurm is blocking Jasmine Boreal — a legendary creature with
    // "bands with other legendary creatures" (via the Cathedral) — and Barktooth Warbeard, another
    // legendary creature, so the *active* player divides the Wurm's 6 rather than its controller.
    // Spread 3 and 3 it kills neither 5-toughness band member; dealt in full to each it would kill
    // both, which is what the unmodeled transfer used to do.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let wurm = game.spawn_on_battlefield(PlayerId(1), card("Craw Wurm"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band is a legal declaration");
    block_with(&mut game, vec![(wurm, jasmine)]).expect("blocking one member blocks the band");
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);

    assert_eq!(
        pending_assigner(&game),
        PlayerId(0),
        "the attacking player divides the blocker's damage (CR 702.22k)"
    );
    assert_eq!(
        game.submit(Intent::AssignDamage {
            player: PlayerId(1),
            assignment: vec![(jasmine, 6), (barktooth, 0)],
        }),
        Err(Reject::ChoicePending),
        "the Wurm's own controller no longer owns this division"
    );
    assign(&mut game, vec![(jasmine, 3), (barktooth, 3)])
        .expect("the active player spreads the Wurm's 6 across the band");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        (game.zone_of(jasmine), game.zone_of(barktooth)),
        (Zone::Battlefield, Zone::Battlefield),
        "3 apiece is lethal to neither — the band survives its own blocker"
    );
    assert_eq!(
        game.zone_of(wurm),
        Zone::Graveyard,
        "the 6/4 still took 4 and 6 from the band it blocked"
    );
    assert_eq!(game.life(PlayerId(1)), 20, "both band members were blocked");
}

#[test]
fn a_blocking_band_moves_the_attackers_division_to_the_defending_player() {
    // CR 702.22j's second clause: the lone attacker is blocked by both a legendary creature with
    // "bands with other legendary creatures" (Jasmine, via the defending player's own Cathedral)
    // and another legendary creature (Barktooth), so the *defending* player divides the attacker's
    // damage. Spread 3 and 3 it kills neither blocker.
    let mut game = Game::new();
    let wurm = game.spawn_on_battlefield(PlayerId(0), card("Craw Wurm"));
    game.spawn_on_battlefield(PlayerId(1), card("Cathedral of Serra"));
    let jasmine = game.spawn_on_battlefield(PlayerId(1), card("Jasmine Boreal"));
    let barktooth = game.spawn_on_battlefield(PlayerId(1), card("Barktooth Warbeard"));

    attack_with(&mut game, vec![wurm]);
    block_with(&mut game, vec![(jasmine, wurm), (barktooth, wurm)]).expect("two legal blocks");
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);

    assert_eq!(
        pending_assigner(&game),
        PlayerId(1),
        "the defending player divides the attacker's damage (CR 702.22j)"
    );
    assign(&mut game, vec![(jasmine, 3), (barktooth, 3)])
        .expect("the defending player spreads the Wurm's 6 across its blockers");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        (game.zone_of(jasmine), game.zone_of(barktooth)),
        (Zone::Battlefield, Zone::Battlefield),
        "3 apiece is lethal to neither 5-toughness blocker"
    );
    assert_eq!(
        game.zone_of(wurm),
        Zone::Graveyard,
        "the 6/4 took 4 and 6 back from the two creatures blocking it"
    );
}

#[test]
fn a_lone_bands_with_other_blocker_leaves_the_division_with_the_attacker() {
    // CR 702.22j's second clause needs "a \[quality\] creature with 'bands with other \[quality\]'
    // *and another* \[quality\] creature" — one of the two blockers is a plain Grizzly Bears, so
    // Jasmine's grant moves nothing and CR 510.1c stands.
    let mut game = Game::new();
    let wurm = game.spawn_on_battlefield(PlayerId(0), card("Craw Wurm"));
    game.spawn_on_battlefield(PlayerId(1), card("Cathedral of Serra"));
    let jasmine = game.spawn_on_battlefield(PlayerId(1), card("Jasmine Boreal"));
    let bears = game.spawn_on_battlefield(PlayerId(1), card("Grizzly Bears"));

    attack_with(&mut game, vec![wurm]);
    block_with(&mut game, vec![(jasmine, wurm), (bears, wurm)]).expect("two legal blocks");
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);

    assert_eq!(
        pending_assigner(&game),
        PlayerId(0),
        "the Bears is not a legendary creature, so no band is blocking"
    );
    assign(&mut game, vec![(jasmine, 0), (bears, 6)]).expect("the attacker picks its own target");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        (game.zone_of(jasmine), game.zone_of(bears)),
        (Zone::Battlefield, Zone::Graveyard),
        "the attacking player spent all 6 on the 2/2"
    );
}

#[test]
fn banding_moves_each_side_of_the_division_the_opposite_way() {
    // The asymmetry, in one combat: CR 702.22j hands the *attackers'* divisions to the defending
    // player (Timber Wolves has banding), while CR 702.22k hands the *blockers'* divisions to the
    // active player (both are blocking a "bands with other legendary" creature and another
    // legendary creature). Each seat owns exactly the divisions the rules gave it and is refused
    // the others.
    let (mut game, ids) = cathedral_board(&["Jasmine Boreal", "Barktooth Warbeard"]);
    let [jasmine, barktooth] = ids[..] else {
        unreachable!("two creatures spawned")
    };
    let wolves = game.spawn_on_battlefield(PlayerId(1), card("Timber Wolves"));
    let wurm = game.spawn_on_battlefield(PlayerId(1), card("Craw Wurm"));
    attack_in_bands(&mut game, &ids, vec![vec![jasmine, barktooth]])
        .expect("a legendary band is a legal declaration");
    block_with(&mut game, vec![(wolves, jasmine), (wurm, jasmine)])
        .expect("both blocks extend across the band");
    advance_until(&mut game, |g| g.current_step() == Step::CombatDamage);

    // The two attackers' divisions: CR 702.22j moved them to the defending player, who pours
    // everything onto the 1/1 to keep the Wurm alive.
    assert_eq!(
        pending_assigner(&game),
        PlayerId(1),
        "a banding blocker takes the attacker's division (CR 702.22j)"
    );
    assert_eq!(
        game.submit(Intent::AssignDamage {
            player: PlayerId(0),
            assignment: vec![(wolves, 0), (wurm, 4)],
        }),
        Err(Reject::ChoicePending),
        "the active player cannot answer a division CR 702.22j took away"
    );
    assign(&mut game, vec![(wolves, 4), (wurm, 0)]).expect("Jasmine Boreal's 4");
    assert_eq!(pending_assigner(&game), PlayerId(1));
    assign(&mut game, vec![(wolves, 6), (wurm, 0)]).expect("Barktooth Warbeard's 6");

    // The two blockers' divisions: CR 702.22k moved them to the active player, who spreads them so
    // that neither band member dies.
    assert_eq!(
        pending_assigner(&game),
        PlayerId(0),
        "a blocked band takes the blocker's division (CR 702.22k)"
    );
    assert_eq!(
        game.submit(Intent::AssignDamage {
            player: PlayerId(1),
            assignment: vec![(jasmine, 1), (barktooth, 0)],
        }),
        Err(Reject::ChoicePending),
        "the defending player cannot answer a division CR 702.22k took away"
    );
    assign(&mut game, vec![(jasmine, 1), (barktooth, 0)]).expect("Timber Wolves' 1");
    assert_eq!(pending_assigner(&game), PlayerId(0));
    assign(&mut game, vec![(jasmine, 3), (barktooth, 3)]).expect("Craw Wurm's 6");
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);

    assert_eq!(
        game.zone_of(wolves),
        Zone::Graveyard,
        "the defending player took all 10 on the 1/1"
    );
    assert_eq!(
        game.zone_of(wurm),
        Zone::Battlefield,
        "and so kept the 6/4 alive — the division was really theirs to make"
    );
    assert_eq!(
        (game.zone_of(jasmine), game.zone_of(barktooth)),
        (Zone::Battlefield, Zone::Battlefield),
        "the active player spread 7 across two 5-toughness band members and lost neither"
    );
    assert_eq!(game.life(PlayerId(1)), 20, "the whole band was blocked");
}
