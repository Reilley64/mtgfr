use super::*;
use crate::{Effect, Game, Object, PendingChoice, PlayerId, StackItem, Step, TargetCount};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn card(name: &str) -> crate::CardDef {
    cards::get_by_name(name).unwrap_or_else(|| panic!("missing test card {name}"))
}

fn codes(game: &Game) -> Vec<&'static str> {
    validate_structural(game)
        .expect_err("corruption must be rejected")
        .into_iter()
        .map(|violation| violation.code)
        .collect()
}

#[test]
fn inspect_returns_raw_arena_zones_stack_and_orchestration_facts() {
    let mut game = Game::with_players(2, 7);
    let library = game.stack_library(P0, &[card("Forest"), card("Lightning Bolt")]);
    let hidden_hand = game.spawn_in_hand(P0, card("Swords to Plowshares"));
    let other_hand = game.spawn_in_hand(P0, card("Grizzly Bears"));
    let permanent = game.spawn_on_battlefield(P1, card("Grizzly Bears"));
    let bolt = game.spawn_in_hand(P0, card("Lightning Bolt"));
    game.fund_mana(P0);
    game.cast(
        P0,
        bolt,
        Some(crate::Target::Player(P1)),
        0,
        vec![],
        vec![],
        vec![],
        vec![],
        false,
        false,
        false,
        0,
        0,
        0,
        false,
    )
    .expect("test spell casts");
    let moved = game.spawn_in_hand(P1, card("Forest"));
    let removed = game.spawn_in_hand(P1, card("Lightning Bolt"));

    let hidden_id = game.def_of(hidden_hand).id;
    if let Object::Card(card) = &mut game.objects[hidden_hand as usize] {
        card.face_down = true;
    }
    game.objects[moved as usize] = Object::Moved { to: permanent };
    let removed_def = match game.objects[removed as usize] {
        Object::Card(ref card) => card.def,
        _ => unreachable!(),
    };
    game.objects[removed as usize] = Object::Removed {
        def: removed_def,
        owner: P1,
    };
    game.stack.push(StackItem::Ability {
        controller: P1,
        source: permanent,
        effect: Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count: crate::Amount::Fixed(1),
        }),
        activated: false,
        target: None,
        targets_second: Default::default(),
        x: 0,
        spent_mana: [0; 6],
    });
    game.active_player = P1;
    game.step = Step::End;
    game.priority = P0;
    game.consecutive_passes = 1;
    game.pending_choice = Some(PendingChoice::ChooseTarget {
        player: P0,
        controller: P0,
        source: permanent,
        effect: None,
        legal: vec![],
        count: TargetCount::default(),
        clause: 0,
        target: None,
        x: 0,
        spent_mana: [0; 6],
        activated: false,
    });
    game.resume.spell_finish = Some(permanent);

    let inspection = inspect(&game);

    assert_eq!(inspection.active_player, P1);
    assert_eq!(inspection.step, Step::End);
    assert_eq!(inspection.priority_player, P0);
    assert_eq!(inspection.consecutive_passes, 1);
    assert!(inspection.has_pending_choice);
    assert!(inspection.has_deferred_resume);
    assert_eq!(inspection.players[0].library, library);
    assert_eq!(inspection.players[0].hand, vec![hidden_hand, other_hand]);
    assert_eq!(inspection.objects.len(), game.objects.len());
    assert!(inspection.objects.iter().any(|object| matches!(
        object,
        ObjectInspection::Card { object_id, card_id, face_down: true, .. }
            if *object_id == hidden_hand && card_id == hidden_id
    )));
    assert!(inspection.objects.iter().any(|object| matches!(
        object,
        ObjectInspection::Permanent { object_id, owner: P1, controller: P1, .. }
            if *object_id == permanent
    )));
    assert!(inspection.objects.iter().any(|object| matches!(
        object,
        ObjectInspection::Moved { object_id, to_object_id }
            if *object_id == moved && *to_object_id == permanent
    )));
    assert!(inspection.objects.iter().any(|object| matches!(
        object,
        ObjectInspection::Removed { object_id, owner: P1, .. } if *object_id == removed
    )));
    assert_eq!(inspection.stack.len(), 2);
    assert_eq!(inspection.stack[0].position_from_bottom, 0);
    assert_eq!(inspection.stack[0].kind, "spell");
    assert_eq!(inspection.stack[0].controller, Some(P0));
    assert_eq!(inspection.stack[0].label, "Lightning Bolt");
    assert_eq!(inspection.stack[1].position_from_bottom, 1);
    assert_eq!(inspection.stack[1].kind, "ability");
    assert_eq!(inspection.stack[1].source_object_id, Some(permanent));
    assert_eq!(inspection.stack[1].controller, Some(P1));
    assert!(!inspection.stack[1].label.is_empty());
}

#[test]
fn validator_reports_player_range_for_turn_players() {
    let mut game = Game::with_players(2, 0);
    game.active_player = PlayerId(9);
    game.priority = PlayerId(8);
    assert_eq!(codes(&game), vec!["player_range", "player_range"]);
}

#[test]
fn validator_reports_duplicate_and_missing_library_membership() {
    let mut game = Game::with_players(2, 0);
    let ids = game.stack_library(P0, &[card("Forest"), card("Lightning Bolt")]);
    game.players[0].library = vec![ids[0], ids[0]];
    assert!(
        codes(&game)
            .iter()
            .filter(|&&code| code == "library_membership")
            .count()
            >= 2
    );
}

#[test]
fn validator_reports_zone_disagreement_for_nonlibrary_entry() {
    let mut game = Game::with_players(2, 0);
    let hand = game.spawn_in_hand(P0, card("Forest"));
    game.players[0].library.push(hand);
    assert!(codes(&game).contains(&"zone_disagreement"));
}

#[test]
fn validator_reports_live_object_zone_kind_disagreement() {
    let mut game = Game::with_players(2, 0);
    let card = game.spawn_in_hand(P0, card("Forest"));
    if let Object::Card(card) = &mut game.objects[card as usize] {
        card.zone = crate::Zone::Battlefield;
    }
    assert!(codes(&game).contains(&"zone_disagreement"));
}

#[test]
fn validator_reports_owner_range_without_leaking_card_names() {
    let mut game = Game::with_players(2, 0);
    let hand = game.spawn_in_hand(P0, card("Lightning Bolt"));
    if let Object::Card(card) = &mut game.objects[hand as usize] {
        card.owner = PlayerId(4);
    }
    let violations = validate_structural(&game).expect_err("bad owner");
    assert!(violations.iter().any(|v| v.code == "owner_range"));
    assert!(
        violations
            .iter()
            .all(|v| !v.message.contains("Lightning Bolt"))
    );
}

#[test]
fn validator_reports_negative_damage_and_counter() {
    let mut game = Game::with_players(2, 0);
    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    if let Object::Permanent(permanent) = &mut game.objects[permanent as usize] {
        permanent.marked_damage = -1;
        permanent.plus_counters = -2;
    }
    assert_eq!(codes(&game), vec!["negative_damage", "negative_counter"]);
}

#[test]
fn validator_reports_missing_attachment_target() {
    let mut game = Game::with_players(2, 0);
    let aura = game.spawn_on_battlefield(P0, card("Wild Growth"));
    if let Object::Permanent(permanent) = &mut game.objects[aura as usize] {
        permanent.attached_to = Some(999);
    }
    assert!(codes(&game).contains(&"attachment_target"));
}

#[test]
fn validator_reports_attachment_cycle() {
    let mut game = Game::with_players(2, 0);
    let first = game.spawn_on_battlefield(P0, card("Wild Growth"));
    let second = game.spawn_on_battlefield(P0, card("Wild Growth"));
    if let Object::Permanent(permanent) = &mut game.objects[first as usize] {
        permanent.attached_to = Some(second);
    }
    if let Object::Permanent(permanent) = &mut game.objects[second as usize] {
        permanent.attached_to = Some(first);
    }
    assert!(codes(&game).contains(&"attachment_cycle"));
}

#[test]
fn validator_reports_stack_spell_without_matching_spell_object() {
    let mut game = Game::with_players(2, 0);
    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.stack.push(StackItem::Spell(permanent));
    assert!(codes(&game).contains(&"stack_pairing"));
}

#[test]
fn validator_bounds_safe_ascii_violations() {
    let mut game = Game::with_players(2, 0);
    for _ in 0..32 {
        let card = game.spawn_in_hand(P0, card("Lightning Bolt"));
        if let Object::Card(card) = &mut game.objects[card as usize] {
            card.owner = PlayerId(9);
        }
    }
    let violations = validate_structural(&game).expect_err("many bad owners");
    assert_eq!(violations.len(), 16);
    assert!(
        violations
            .iter()
            .all(|v| v.message.len() <= 256 && v.message.is_ascii())
    );
}

#[test]
fn apply_operations_skeleton_rejects_an_empty_batch() {
    let mut game = Game::with_players(2, 0);
    assert_eq!(
        apply_operations(&mut game, &[]),
        Err(EditError {
            operation_index: None,
            reason: ErrorReason::EmptyBatch
        })
    );
}

fn has_reference(game: &Game, object: ObjectId, disposition: ReferenceDisposition) -> bool {
    references_to(game, object)
        .iter()
        .any(|site| site.disposition == disposition)
}

#[test]
fn reference_walker_visits_draw_after_sources() {
    use crate::resolution::{DrawAfter, DrawBatch};
    for after in [
        DrawAfter::TradeSecretsCaster {
            caster: P0,
            opponent: P1,
            source: 41,
            max: 2,
        },
        DrawAfter::TradeSecretsRepeat {
            caster: P0,
            opponent: P1,
            source: 42,
            max: 2,
        },
    ] {
        let expected = match after {
            DrawAfter::TradeSecretsCaster { source, .. }
            | DrawAfter::TradeSecretsRepeat { source, .. } => source,
            DrawAfter::Nothing | DrawAfter::DrawStep => unreachable!(),
        };
        let mut game = Game::with_players(2, 0);
        game.resume.draw_batch = Some(DrawBatch {
            seats: vec![],
            after,
            paused: true,
        });
        assert!(has_reference(
            &game,
            expected,
            ReferenceDisposition::Blocking
        ));
    }
}

#[test]
fn reference_walker_visits_may_draw_resume_source() {
    let mut game = Game::with_players(2, 0);
    game.pending_choice = Some(PendingChoice::MayDrawUpTo {
        player: P0,
        max: 2,
        effect: Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count: crate::Amount::Fixed(1),
        }),
        resume: crate::MayDrawUpToResume::TradeSecretsRepeat {
            opponent: P1,
            source: 77,
        },
    });
    assert!(has_reference(&game, 77, ReferenceDisposition::Blocking));
}

#[test]
fn reference_walker_visits_control_condition_source() {
    let mut game = Game::with_players(2, 0);
    let controlled = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions.conditioned_control_overrides.push((
        controlled,
        P1,
        crate::ControlCondition {
            source: 88,
            needs_tapped: true,
        },
        1,
    ));
    assert!(has_reference(&game, 88, ReferenceDisposition::Blocking));
}

#[test]
fn reference_walker_visits_pile_continuations() {
    let cases = vec![
        (
            PendingChoice::SplitBlockersIntoPiles {
                player: P0,
                source: 1,
                options: vec![],
                left: vec![(P1, vec![91])],
                defenders: vec![],
                attackers: vec![],
            },
            vec![91],
        ),
        (
            PendingChoice::ChoosePileForAttacker {
                player: P0,
                source: 1,
                attacker: 2,
                left: vec![(P1, vec![92])],
                remaining: vec![],
            },
            vec![92],
        ),
        (
            PendingChoice::ChooseSplittingOpponent {
                player: P0,
                source: 1,
                legal: vec![P1],
                then: crate::SplittingContinuation::ExilePiles {
                    pile_a: vec![93],
                    pile_b: vec![94],
                },
            },
            vec![93, 94],
        ),
        (
            PendingChoice::ChooseSplittingOpponent {
                player: P0,
                source: 1,
                legal: vec![P1],
                then: crate::SplittingContinuation::Partition { revealed: vec![95] },
            },
            vec![95],
        ),
        (
            PendingChoice::ChooseSplittingOpponent {
                player: P0,
                source: 1,
                legal: vec![P1],
                then: crate::SplittingContinuation::PickOneToGraveyard { revealed: vec![96] },
            },
            vec![96],
        ),
        (
            PendingChoice::ChooseSplittingOpponent {
                player: P0,
                source: 1,
                legal: vec![P1],
                then: crate::SplittingContinuation::GainControlOf { objects: vec![97] },
            },
            vec![97],
        ),
    ];
    for (choice, expected_ids) in cases {
        let mut game = Game::with_players(2, 0);
        game.pending_choice = Some(choice);
        for expected in expected_ids {
            assert!(
                has_reference(&game, expected, ReferenceDisposition::Blocking),
                "missing {expected}"
            );
        }
    }
}

#[test]
fn reference_dispositions_allow_lineage_membership_and_recomputed_caches() {
    let mut game = Game::with_players(2, 0);
    let first = game.spawn_in_hand(P0, card("Forest"));
    let second = game.spawn_in_hand(P0, card("Forest"));
    let current = game.spawn_in_hand(P0, card("Forest"));
    game.objects[first as usize] = Object::Moved { to: second };
    game.objects[second as usize] = Object::Moved { to: current };
    game.refresh_actions();

    assert!(has_reference(&game, current, ReferenceDisposition::Lineage));
    assert!(has_reference(
        &game,
        current,
        ReferenceDisposition::Recomputed
    ));
    assert!(blocking_references_to(&game, current).is_empty());

    let library = game.stack_library(P0, &[card("Forest")])[0];
    assert!(has_reference(
        &game,
        library,
        ReferenceDisposition::Membership
    ));
    assert!(blocking_references_to(&game, library).is_empty());

    game.drawn_this_turn.push(current);
    assert!(has_reference(
        &game,
        current,
        ReferenceDisposition::Historical
    ));
    assert!(blocking_references_to(&game, current).is_empty());
    assert!(validate_structural(&game).is_ok());
}

#[test]
fn reference_dispositions_keep_attachment_combat_and_pending_blocking() {
    let mut game = Game::with_players(2, 0);
    let host = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let aura = game.spawn_on_battlefield(P0, card("Wild Growth"));
    if let Object::Permanent(permanent) = &mut game.objects[aura as usize] {
        permanent.attached_to = Some(host);
    }
    game.combat.attackers.push(host);
    game.pending_choice = Some(PendingChoice::MayRevealLandFromHand {
        player: P0,
        land: host,
        subtypes: &[],
    });
    assert!(blocking_references_to(&game, host).len() >= 3);
}

#[test]
fn validator_rejects_non_attachment_and_accepts_legal_aura_and_equipment() {
    let mut game = Game::with_players(2, 0);
    let host = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let creature = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    if let Object::Permanent(permanent) = &mut game.objects[creature as usize] {
        permanent.attached_to = Some(host);
    }
    assert!(codes(&game).contains(&"attachment_type"));

    if let Object::Permanent(permanent) = &mut game.objects[creature as usize] {
        permanent.attached_to = None;
    }
    for name in ["Wild Growth", "Bonesplitter"] {
        let attachment = game.spawn_on_battlefield(P0, card(name));
        if let Object::Permanent(permanent) = &mut game.objects[attachment as usize] {
            permanent.attached_to = Some(host);
        }
        assert!(
            !codes(&game).contains(&"attachment_type"),
            "{name} must be a legal attachment kind"
        );
    }
}

#[test]
fn validator_reports_nested_player_references_and_pass_count() {
    let mut game = Game::with_players(2, 0);
    let source = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.stack.push(StackItem::Ability {
        controller: P0,
        source,
        effect: Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count: crate::Amount::Fixed(1),
        }),
        activated: false,
        target: Some(crate::Target::Player(PlayerId(8))),
        targets_second: Default::default(),
        x: 0,
        spent_mana: [0; 6],
    });
    game.pending_choice = Some(PendingChoice::ChooseTarget {
        player: P0,
        controller: PlayerId(7),
        source,
        effect: None,
        legal: vec![crate::Target::Player(PlayerId(6))],
        count: TargetCount::default(),
        clause: 0,
        target: None,
        x: 0,
        spent_mana: [0; 6],
        activated: false,
    });
    game.combat
        .attack_targets
        .push((source, crate::Defender::Player(PlayerId(5))));
    game.combat.blocked_by.push(PlayerId(4));
    game.resume.clash_scry = Some(PlayerId(3));
    game.consecutive_passes = 3;

    let found = codes(&game);
    assert!(
        found.iter().filter(|&&code| code == "player_range").count() >= 6,
        "{found:?}"
    );
    assert!(found.contains(&"consecutive_passes"));
}

#[test]
fn validator_reports_resume_sequence_player_references() {
    use std::sync::Arc;
    game_with_bad_resume_player();

    fn game_with_bad_resume_player() {
        let mut game = Game::with_players(2, 0);
        game.resume.sequence = Some(crate::resolution::SequenceCont {
            steps: Arc::from([]),
            ctx: crate::resolution::ResolveCtx {
                controller: PlayerId(9),
                source: 0,
                target: Some(crate::Target::Player(PlayerId(8))),
                targets_second: Default::default(),
                x: 0,
                spent_mana: [0; 6],
            },
        });
        assert!(
            codes(&game)
                .iter()
                .filter(|&&code| code == "player_range")
                .count()
                >= 2
        );
    }
}

#[test]
fn validator_requires_moved_lineage_to_terminate_forward() {
    for objects in [
        vec![Object::Moved { to: 0 }],
        vec![Object::Moved { to: 1 }, Object::Moved { to: 0 }],
        vec![
            Object::Removed {
                def: crate::CardId(0),
                owner: P0,
            },
            Object::Moved { to: 0 },
        ],
    ] {
        let mut game = Game::with_players(2, 0);
        game.objects = objects;
        assert!(codes(&game).contains(&"moved_lineage"));
    }
}

#[test]
fn validator_returns_exact_deterministic_first_sixteen_library_violations() {
    let mut game = Game::with_players(2, 0);
    let defs = vec![card("Forest"); 20];
    let ids = game.stack_library(P0, &defs);
    game.players[0].library.extend(ids.iter().copied());
    let violations = validate_structural(&game).expect_err("duplicate memberships");
    assert_eq!(violations.len(), 16);
    let messages: Vec<_> = violations.into_iter().map(|v| v.message).collect();
    let expected: Vec<_> = (0..16)
        .map(|id| format!("library object {id} occurs 2 times"))
        .collect();
    assert_eq!(messages, expected);
}

#[test]
fn corrupt_stack_spell_inspection_reports_unknown_controller() {
    let mut game = Game::with_players(2, 0);
    game.stack.push(StackItem::Spell(99));
    assert_eq!(inspect(&game).stack[0].controller, None);
}

#[test]
fn validator_rejects_out_of_range_commander_damage_owner() {
    let mut game = Game::with_players(2, 0);
    game.players[0].commander_damage.push((PlayerId(8), 1));

    let violations = validate_structural(&game).expect_err("commander owner is out of range");
    assert!(violations.iter().any(|violation| {
        violation.code == "player_range"
            && violation.message.contains("commander damage")
            && violation.message.contains("player 8")
    }));
}

#[test]
fn validator_bounds_passes_by_living_players_and_handles_terminal_state() {
    let mut game = Game::with_players(4, 0);
    game.players[2].lost = true;
    game.players[3].lost = true;
    game.consecutive_passes = 2;
    assert!(codes(&game).contains(&"consecutive_passes"));

    game.players[0].lost = true;
    game.players[1].lost = true;
    game.consecutive_passes = 0;
    assert!(validate_structural(&game).is_ok());

    game.consecutive_passes = 1;
    assert!(codes(&game).contains(&"consecutive_passes"));
}
