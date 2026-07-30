//! Legends (`leg`) grind — increment 3: bands-with-other, slices 1 (band formation) and 2 (blocked
//! as a group).
//!
//! Slice 1 is CR 702.22c: "As a player declares attackers, they may declare that one or more
//! attacking creatures with banding and up to one attacking creature without banding … are all in a
//! 'band.' They may also declare that one or more attacking \[quality\] creatures with 'bands with
//! other \[quality\]' and any number of other attacking \[quality\] creatures are all in a band."
//!
//! Slice 2 is CR 702.22h: "If an attacking creature becomes blocked by a creature, each other
//! creature in the same band as the attacking creature becomes blocked by that same blocking
//! creature." The damage-assignment transfer (CR 702.22j/k) is slice 3 and is not modeled: a
//! blocker still deals its full power to every band member it is blocking.

mod common;

use common::*;
use engine::*;

// ── local drivers ─────────────────────────────────────────────────────────────────────

/// Player 0 with a Cathedral of Serra on the battlefield, plus `creatures` spawned for them.
fn cathedral_board(creatures: &[&str]) -> (Game, Vec<ObjectId>) {
    let mut game = Game::new();
    game.spawn_on_battlefield(PlayerId(0), card("Cathedral of Serra"));
    let ids = creatures
        .iter()
        .map(|name| game.spawn_on_battlefield(PlayerId(0), card(name)))
        .collect();
    (game, ids)
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
    advance_until(&mut game, |g| g.current_step() == Step::EndCombat);
    assert!(
        game.pending_choice().is_none(),
        "neither attacker has two blockers, so no damage division is asked for"
    );
    assert_eq!(game.life(PlayerId(1)), 20, "both attackers were blocked");
    assert_eq!(
        game.zone_of(giant),
        Zone::Graveyard,
        "the 4/4 took 4 from Jasmine Boreal and 6 from Barktooth Warbeard"
    );
}
