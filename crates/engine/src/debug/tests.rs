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

fn apply_one(game: &mut Game, operation: Mutation) -> Result<(), EditError> {
    apply_operations(game, &[operation])
}

#[test]
fn scalar_life_accepts_full_i32_range() {
    for life in [i32::MIN, i32::MAX] {
        let mut game = Game::with_players(2, 0);
        apply_one(&mut game, Mutation::SetLife { player: P0, life }).expect("life edit");
        assert_eq!(game.players[0].life, life);
    }
}

#[test]
fn scalar_life_extrema_remain_safe_under_ordinary_changes() {
    for (life, amount) in [(i32::MAX, 1), (i32::MIN, -1)] {
        let mut game = Game::with_players(2, 0);
        apply_one(&mut game, Mutation::SetLife { player: P0, life }).expect("life edit");

        game.apply(&crate::Event::LifeChanged {
            player: P0,
            amount,
            source: None,
        });

        assert_eq!(game.life(P0), life);
    }
}

#[test]
fn wide_life_changes_clamp_at_endpoints_and_tally_each_logical_event() {
    for (start, amount, expected_life, expected_gain, expected_losses) in [
        (i32::MIN, i64::MIN, i32::MIN, 0, 1),
        (i32::MAX, i64::MIN, i32::MIN, 0, 1),
        (i32::MAX, i64::MAX, i32::MAX, u32::MAX, 0),
        (i32::MIN, i64::MAX, i32::MAX, u32::MAX, 0),
        // Sylvan-style declined-card multiplication can reach this exact loss amount.
        (i32::MIN, -i64::MAX, i32::MIN, 0, 1),
    ] {
        let mut game = Game::with_players(2, 0);
        apply_one(
            &mut game,
            Mutation::SetLife {
                player: P0,
                life: start,
            },
        )
        .expect("life edit");

        game.apply(&crate::Event::LifeChanged {
            player: P0,
            amount,
            source: None,
        });

        assert_eq!(game.life(P0), expected_life);
        assert_eq!(
            game.players[P0.0 as usize].life_gained_this_turn,
            expected_gain
        );
        assert_eq!(
            game.players[P0.0 as usize].life_losses_this_turn,
            expected_losses
        );
    }
}

#[test]
fn scalar_life_extrema_remain_safe_for_exact_setup_changes() {
    for (life, requested) in [(i32::MIN, i32::MAX), (i32::MAX, i32::MIN)] {
        let mut game = Game::with_players(2, 0);
        apply_one(&mut game, Mutation::SetLife { player: P0, life }).expect("life edit");
        let losses_before = game.players[P0.0 as usize].life_losses_this_turn;

        game.set_life(P0, requested);

        assert_eq!(game.life(P0), requested);
        assert_eq!(
            game.players[P0.0 as usize].life_losses_this_turn - losses_before,
            u32::from(requested < life),
            "one logical setup loss must record one loss occurrence"
        );
    }
}

#[test]
fn scalar_player_counters_accept_full_u8_range() {
    for counter in PlayerCounterKind::ALL {
        for value in [0, u8::MAX] {
            let mut game = Game::with_players(2, 0);
            apply_one(
                &mut game,
                Mutation::SetPlayerCounter {
                    player: P1,
                    counter,
                    value,
                },
            )
            .expect("player counter edit");
            assert_eq!(game.players[1].kind_counters[counter as usize], value);
        }
    }
    assert!(
        u8::try_from(256_u16).is_err(),
        "wire values above 255 cannot enter the domain mutation"
    );
}

#[test]
fn scalar_player_edits_reject_unknown_players() {
    for operation in [
        Mutation::SetLife {
            player: PlayerId(2),
            life: 7,
        },
        Mutation::SetPlayerCounter {
            player: PlayerId(2),
            counter: PlayerCounterKind::Poison,
            value: 1,
        },
    ] {
        let mut game = Game::with_players(2, 0);
        assert_eq!(
            apply_one(&mut game, operation),
            Err(EditError {
                operation_index: Some(0),
                reason: ErrorReason::UnknownEntity,
            })
        );
    }
}

#[test]
fn scalar_turn_state_accepts_every_step_and_living_seats() {
    let steps = [
        Step::Untap,
        Step::Upkeep,
        Step::Draw,
        Step::Main1,
        Step::BeginCombat,
        Step::DeclareAttackers,
        Step::DeclareBlockers,
        Step::FirstStrikeCombatDamage,
        Step::CombatDamage,
        Step::EndCombat,
        Step::Main2,
        Step::End,
        Step::Cleanup,
    ];
    for step in steps {
        let mut game = Game::with_players(2, 0);
        apply_one(
            &mut game,
            Mutation::SetTurnState {
                active_player: P1,
                step,
                priority_player: P0,
                consecutive_passes: 1,
            },
        )
        .expect("turn state edit");
        assert_eq!(game.active_player, P1);
        assert_eq!(game.step, step);
        assert_eq!(game.priority, P0);
        assert_eq!(game.consecutive_passes, 1);
    }
}

#[test]
fn scalar_turn_state_rejects_unknown_lost_players_and_completed_pass_rounds() {
    let cases = [
        Mutation::SetTurnState {
            active_player: PlayerId(2),
            step: Step::Main1,
            priority_player: P0,
            consecutive_passes: 0,
        },
        Mutation::SetTurnState {
            active_player: P0,
            step: Step::Main1,
            priority_player: PlayerId(2),
            consecutive_passes: 0,
        },
    ];
    for operation in cases {
        let mut game = Game::with_players(2, 0);
        assert_eq!(
            apply_one(&mut game, operation),
            Err(EditError {
                operation_index: Some(0),
                reason: ErrorReason::UnknownEntity,
            })
        );
    }

    for (lost, active_player, priority_player) in [(P0, P0, P1), (P1, P0, P1)] {
        let mut game = Game::with_players(2, 0);
        game.players[lost.0 as usize].lost = true;
        assert_eq!(
            apply_one(
                &mut game,
                Mutation::SetTurnState {
                    active_player,
                    step: Step::Main1,
                    priority_player,
                    consecutive_passes: 0,
                },
            ),
            Err(EditError {
                operation_index: Some(0),
                reason: ErrorReason::InvalidValue,
            })
        );
    }

    let mut game = Game::with_players(4, 0);
    game.players[2].lost = true;
    game.players[3].lost = true;
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetTurnState {
                active_player: P0,
                step: Step::Main1,
                priority_player: P1,
                consecutive_passes: 2,
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::InvalidValue,
        })
    );
}

#[test]
fn scalar_permanent_state_sets_requested_raw_fields_and_preserves_omitted_fields() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id,
            tapped: Some(true),
            marked_damage: Some(i32::MAX),
            plus_one_counters: Some(i32::MAX),
        },
    )
    .expect("permanent edit");
    let Object::Permanent(permanent) = &game.objects[object_id as usize] else {
        panic!("fixture permanent disappeared");
    };
    assert!(permanent.tapped);
    assert_eq!(permanent.marked_damage, i32::MAX);
    assert_eq!(permanent.plus_counters, i32::MAX);

    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id,
            tapped: Some(false),
            marked_damage: None,
            plus_one_counters: None,
        },
    )
    .expect("partial permanent edit");
    let Object::Permanent(permanent) = &game.objects[object_id as usize] else {
        panic!("fixture permanent disappeared");
    };
    assert!(!permanent.tapped);
    assert_eq!(permanent.marked_damage, i32::MAX);
    assert_eq!(permanent.plus_counters, i32::MAX);
}

#[test]
fn scalar_permanent_extrema_remain_safe_for_characteristics_and_damage() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id,
            tapped: None,
            marked_damage: Some(i32::MAX),
            plus_one_counters: Some(i32::MAX),
        },
    )
    .expect("permanent edit");

    assert_eq!(game.power(object_id), i32::MAX);
    assert_eq!(game.toughness(object_id), i32::MAX);
    game.apply(&crate::Event::DamageMarked {
        object: object_id,
        amount: 1,
        cant_be_regenerated: false,
        exile_instead_of_dying: false,
        source: None,
    });
    assert_eq!(game.marked_damage(object_id), i32::MAX);
}

#[test]
fn debug_set_plus_counters_keeps_all_counter_readers_and_provenance_coherent() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_on_battlefield(P0, card("Steelbane Hydra"));
    game.fund_mana(P0);
    game.apply(&crate::Event::CountersPlaced {
        object: object_id,
        count: 2,
        source_name: "ordinary source",
    });

    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id,
            tapped: None,
            marked_damage: None,
            plus_one_counters: Some(i32::MAX),
        },
    )
    .expect("counter edit");

    let Object::Permanent(permanent) = &game.objects[object_id as usize] else {
        panic!("fixture permanent disappeared");
    };
    assert_eq!(permanent.plus_counters, i32::MAX);
    assert_eq!(game.plus_counters(object_id), i32::MAX);
    assert_eq!(
        game.resolve_amount(crate::Amount::PerCounterOnSource, P0, object_id, None, 0),
        i32::MAX
    );
    let filter = crate::PermanentFilter {
        with_counter: Some(crate::CounterAxis::PlusOnePlusOne),
        ..Default::default()
    };
    assert_eq!(
        game.count_matching(&filter, crate::AmountZone::Battlefield, P0, Some(object_id)),
        1
    );
    assert!(game.ability_activation_gate(P0, object_id, 1).is_ok());
    assert!(
        game.modifier_provenance
            .counter_batches
            .iter()
            .any(|&(host, count, source)| host == object_id
                && count == 2
                && source == "ordinary source")
    );
}

#[test]
fn endpoint_plus_counter_placement_and_removal_keep_ledger_and_raw_total_coherent() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id,
            tapped: None,
            marked_damage: None,
            plus_one_counters: Some(i32::MAX),
        },
    )
    .expect("counter edit");

    game.apply(&crate::Event::CountersPlaced {
        object: object_id,
        count: 1,
        source_name: "clamped placement",
    });
    assert_eq!(game.plus_counters(object_id), i32::MAX);
    assert_eq!(game.permanent(object_id).plus_counters, i32::MAX);

    game.apply(&crate::Event::CountersPlaced {
        object: object_id,
        count: -1,
        source_name: "remove one",
    });
    assert_eq!(game.plus_counters(object_id), i32::MAX - 1);
    assert_eq!(game.permanent(object_id).plus_counters, i32::MAX - 1);

    game.apply(&crate::Event::KindCountersPlaced {
        object: object_id,
        kind: crate::CounterKind::Charge,
        count: 1,
    });
    let (mut events, removed) = game.remove_counters_events(object_id, true, 0);
    assert_eq!(removed, i32::MAX);
    game.apply_all(&mut events);
    assert_eq!(game.plus_counters(object_id), 0);
    assert_eq!(game.permanent(object_id).plus_counters, 0);
    assert_eq!(
        game.counters_of_kind(object_id, crate::CounterKind::Charge),
        0
    );
    assert!(
        game.modifier_provenance
            .counter_batches
            .iter()
            .all(|&(host, ..)| host != object_id)
    );
}

#[test]
fn scalar_permanent_state_rejects_negative_totals_wrong_kinds_and_missing_objects() {
    let mut game = Game::with_players(2, 0);
    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let hand = game.spawn_in_hand(P0, card("Forest"));
    for operation in [
        Mutation::SetPermanentState {
            object_id: permanent,
            tapped: None,
            marked_damage: Some(-1),
            plus_one_counters: None,
        },
        Mutation::SetPermanentState {
            object_id: permanent,
            tapped: None,
            marked_damage: None,
            plus_one_counters: Some(-1),
        },
    ] {
        assert_eq!(
            apply_one(&mut game, operation),
            Err(EditError {
                operation_index: Some(0),
                reason: ErrorReason::InvalidValue,
            })
        );
    }
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetPermanentState {
                object_id: hand,
                tapped: Some(true),
                marked_damage: None,
                plus_one_counters: None,
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::WrongObjectKind,
        })
    );
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetPermanentState {
                object_id: 999,
                tapped: Some(true),
                marked_damage: None,
                plus_one_counters: None,
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::UnknownEntity,
        })
    );
}

#[test]
fn scalar_controller_replaces_all_overrides_stamps_and_removes_from_combat() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let other = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions
        .control_overrides
        .push((object_id, P0, "old", 3));
    game.play_permissions
        .permanent_control_overrides
        .push((object_id, P0, 4));
    game.play_permissions.conditioned_control_overrides.push((
        object_id,
        P0,
        crate::ControlCondition {
            source: other,
            needs_tapped: false,
        },
        5,
    ));
    game.next_control_timestamp = 17;
    game.combat.attackers.push(object_id);
    game.combat
        .attack_targets
        .push((object_id, crate::Defender::Player(P1)));
    game.combat.blocks.push((object_id, other));
    game.combat.blocks.push((other, object_id));
    game.combat.blocked_ever.push((object_id, other));
    game.combat.blocked_ever.push((other, object_id));

    apply_one(
        &mut game,
        Mutation::SetController {
            object_id,
            controller: P1,
        },
    )
    .expect("controller edit");

    assert_eq!(game.controller_of(object_id), P1);
    assert!(
        game.play_permissions
            .control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
    assert_eq!(
        game.play_permissions.permanent_control_overrides,
        vec![(object_id, P1, 17)]
    );
    assert_eq!(game.next_control_timestamp, 18);
    assert!(!game.combat.attackers.contains(&object_id));
    assert!(
        game.combat
            .attack_targets
            .iter()
            .all(|&(attacker, _)| attacker != object_id)
    );
    assert!(
        game.combat
            .blocks
            .iter()
            .all(|&(blocker, attacker)| blocker != object_id && attacker != object_id)
    );
    assert!(
        game.combat
            .blocked_ever
            .iter()
            .all(|&(_, attacker)| attacker != object_id)
    );
}

#[test]
fn scalar_controller_rejects_nonpermanents_and_invalid_players() {
    let mut game = Game::with_players(2, 0);
    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let hand = game.spawn_in_hand(P0, card("Forest"));
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetController {
                object_id: hand,
                controller: P1
            }
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::WrongObjectKind
        })
    );
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetController {
                object_id: permanent,
                controller: PlayerId(2)
            }
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::UnknownEntity
        })
    );
}

#[test]
fn attachment_set_and_detach_require_compatible_live_permanents() {
    let mut game = Game::with_players(2, 0);
    let host = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let aura = game.spawn_on_battlefield(P0, card("Prison Term"));
    let equipment = game.spawn_on_battlefield(P0, card("Bonesplitter"));
    for attachment in [aura, equipment] {
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: attachment,
                attached_to: Some(host),
            },
        )
        .expect("compatible attachment");
        assert_eq!(game.attached_to(attachment), Some(host));
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: attachment,
                attached_to: None,
            },
        )
        .expect("detach");
        assert_eq!(game.attached_to(attachment), None);
    }

    let hand = game.spawn_in_hand(P0, card("Forest"));
    for operation in [
        Mutation::SetAttachment {
            object_id: hand,
            attached_to: Some(host),
        },
        Mutation::SetAttachment {
            object_id: aura,
            attached_to: Some(hand),
        },
    ] {
        assert_eq!(
            apply_one(&mut game, operation),
            Err(EditError {
                operation_index: Some(0),
                reason: ErrorReason::WrongObjectKind
            })
        );
    }
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: host,
                attached_to: Some(aura)
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::InvalidValue
        })
    );
}

#[test]
fn attachment_validates_effective_type_in_the_requested_result_state() {
    let mut game = Game::with_players(2, 0);
    let host = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let bestowed = game.spawn_on_battlefield(P0, card("Eidolon of Countless Battles"));
    let type_lost_aura = game.spawn_on_battlefield(P0, card("Prison Term"));
    game.permanent_mut(bestowed).bestowed = true;
    game.permanent_mut(type_lost_aura).face_down = true;

    apply_one(
        &mut game,
        Mutation::SetAttachment {
            object_id: bestowed,
            attached_to: Some(host),
        },
    )
    .expect("a prospective attached bestow permanent is an Aura");
    assert_eq!(game.attached_to(bestowed), Some(host));
    assert!(game.effective_subtypes(bestowed).contains(&"Aura"));

    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: type_lost_aura,
                attached_to: Some(host),
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::InvalidValue,
        })
    );
    assert_eq!(game.attached_to(type_lost_aura), None);
}

#[test]
fn attachment_rejects_self_links_and_cycles() {
    let mut game = Game::with_players(2, 0);
    let first = game.spawn_on_battlefield(P0, card("Faith's Fetters"));
    let second = game.spawn_on_battlefield(P0, card("Faith's Fetters"));
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: first,
                attached_to: Some(first)
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::AttachmentCycle
        })
    );
    apply_one(
        &mut game,
        Mutation::SetAttachment {
            object_id: first,
            attached_to: Some(second),
        },
    )
    .expect("first link");
    assert_eq!(
        apply_one(
            &mut game,
            Mutation::SetAttachment {
                object_id: second,
                attached_to: Some(first)
            },
        ),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::AttachmentCycle
        })
    );
}

#[test]
fn reports_late_operation_index_after_applying_earlier_operations() {
    let mut game = Game::with_players(2, 0);
    assert_eq!(
        apply_operations(
            &mut game,
            &[
                Mutation::SetLife {
                    player: P0,
                    life: 7
                },
                Mutation::SetPermanentState {
                    object_id: 999,
                    tapped: Some(true),
                    marked_damage: None,
                    plus_one_counters: None,
                },
            ],
        ),
        Err(EditError {
            operation_index: Some(1),
            reason: ErrorReason::UnknownEntity
        })
    );
    assert_eq!(game.players[0].life, 7);
}

#[test]
fn scalar_unimplemented_object_operations_remain_out_of_scope() {
    let mut game = Game::with_players(2, 0);
    assert_eq!(
        apply_one(&mut game, Mutation::RemoveCard { object_id: 0 },),
        Err(EditError {
            operation_index: Some(0),
            reason: ErrorReason::InvalidValue
        })
    );
}

#[test]
fn bounded_counter_events_report_only_the_accepted_state_delta() {
    let mut game = Game::with_players(2, 0);
    let object = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id: object,
            tapped: None,
            marked_damage: None,
            plus_one_counters: Some(i32::MAX),
        },
    )
    .expect("counter edit");
    game.permanent_mut(object).kind_counters[crate::CounterKind::Charge as usize] = u8::MAX;
    game.players[P0.0 as usize].kind_counters[crate::PlayerCounterKind::Poison as usize] = u8::MAX;

    let mut events = Vec::new();
    game.push_apply(
        &mut events,
        crate::Event::CountersPlaced {
            object,
            count: 1,
            source_name: "bounded",
        },
    );
    game.push_apply(
        &mut events,
        crate::Event::KindCountersPlaced {
            object,
            kind: crate::CounterKind::Charge,
            count: 1,
        },
    );
    game.push_apply(
        &mut events,
        crate::Event::PlayerCountersPlaced {
            player: P0,
            kind: crate::PlayerCounterKind::Poison,
            count: 1,
        },
    );

    assert!(events.is_empty(), "zero accepted delta publishes no event");
}

#[test]
fn bounded_counter_events_publish_a_partial_accepted_delta() {
    let mut game = Game::with_players(2, 0);
    let object = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_one(
        &mut game,
        Mutation::SetPermanentState {
            object_id: object,
            tapped: None,
            marked_damage: None,
            plus_one_counters: Some(i32::MAX - 1),
        },
    )
    .expect("counter edit");

    let mut events = Vec::new();
    game.push_apply(
        &mut events,
        crate::Event::CountersPlaced {
            object,
            count: 2,
            source_name: "bounded",
        },
    );

    assert_eq!(game.plus_counters(object), i32::MAX);
    assert!(matches!(
        events.as_slice(),
        [crate::Event::CountersPlaced { count: 1, .. }]
    ));
}
