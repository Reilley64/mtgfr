use super::*;
use crate::{Effect, Game, Object, PendingChoice, PlayerId, StackRenderSource, Step, TargetCount};

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
    game.push_stack_item(
        StackRenderSource::Object(permanent),
        StackPayload::Ability {
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
        },
    )
    .unwrap();
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
    assert_eq!(
        inspection.next_object_id,
        u32::try_from(game.objects.len()).ok()
    );
    assert_eq!(
        inspection.next_stack_entry_id,
        game.next_stack_entry_id.map(|id| StackEntryId(id.get()))
    );
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
    assert_eq!(inspection.stack[0].entry_id, game.stack[0].entry_id);
    assert_eq!(inspection.stack[1].entry_id, game.stack[1].entry_id);
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
fn inspect_preserves_exhausted_stack_identity_frontier() {
    let mut game = Game::with_players(2, 7);
    game.next_stack_entry_id = None;

    let inspection = inspect(&game);

    assert_eq!(inspection.next_stack_entry_id, None);
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
    game.push_stack_item(
        StackRenderSource::Object(permanent),
        StackPayload::Spell(permanent),
    )
    .unwrap();
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
    game.push_stack_item(
        StackRenderSource::Object(source),
        StackPayload::Ability {
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
        },
    )
    .unwrap();
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
    game.push_stack_item(StackRenderSource::Object(99), StackPayload::Spell(99))
        .unwrap();
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

        game.apply_recorded(&crate::Event::LifeChanged {
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

        game.apply_recorded(&crate::Event::LifeChanged {
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
    game.apply_recorded(&crate::Event::DamageMarked {
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
    game.apply_recorded(&crate::Event::CountersPlaced {
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

    game.apply_recorded(&crate::Event::CountersPlaced {
        object: object_id,
        count: 1,
        source_name: "clamped placement",
    });
    assert_eq!(game.plus_counters(object_id), i32::MAX);
    assert_eq!(game.permanent(object_id).plus_counters, i32::MAX);

    game.apply_recorded(&crate::Event::CountersPlaced {
        object: object_id,
        count: -1,
        source_name: "remove one",
    });
    assert_eq!(game.plus_counters(object_id), i32::MAX - 1);
    assert_eq!(game.permanent(object_id).plus_counters, i32::MAX - 1);

    game.apply_recorded(&crate::Event::KindCountersPlaced {
        object: object_id,
        kind: crate::CounterKind::Charge,
        count: 1,
    });
    let (mut events, removed) = game.remove_counters_events(object_id, true, 0);
    assert_eq!(removed, i32::MAX);
    game.apply_all_recorded(&mut events);
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

fn card_id(name: &str) -> String {
    card(name).id.to_owned()
}

fn assert_operation_error(game: &mut Game, operation: Mutation, reason: ErrorReason) {
    assert_eq!(
        apply_one(game, operation),
        Err(EditError {
            operation_index: Some(0),
            reason,
        })
    );
}

#[test]
fn create_places_known_cards_in_every_zone_with_exact_arena_ids() {
    for destination in [
        DebugZone::Library,
        DebugZone::Hand,
        DebugZone::Battlefield,
        DebugZone::Graveyard,
        DebugZone::Exile,
        DebugZone::Command,
    ] {
        let mut game = Game::with_players(2, 0);
        let object_id = game.objects.len() as ObjectId;
        apply_one(
            &mut game,
            Mutation::CreateCard {
                object_id,
                card_id: card_id("Grizzly Bears"),
                owner: P0,
                controller: if destination == DebugZone::Battlefield {
                    P1
                } else {
                    P0
                },
                destination,
                commander: true,
                face_down: false,
            },
        )
        .expect("valid card creation");

        match (&game.objects[object_id as usize], destination) {
            (Object::Permanent(permanent), DebugZone::Battlefield) => {
                assert_eq!(permanent.owner, P0);
                assert_eq!(game.controller_of(object_id), P1);
                assert!(permanent.commander);
                assert!(permanent.summoning_sick);
                assert!(permanent.entered_this_turn);
                assert_eq!(permanent.marked_damage, 0);
                assert_eq!(permanent.plus_counters, 0);
            }
            (Object::Card(created), _) => {
                assert_eq!(created.owner, P0);
                assert_eq!(
                    created.zone,
                    match destination {
                        DebugZone::Library => Zone::Library,
                        DebugZone::Hand => Zone::Hand,
                        DebugZone::Battlefield => unreachable!(),
                        DebugZone::Graveyard => Zone::Graveyard,
                        DebugZone::Exile => Zone::Exile,
                        DebugZone::Command => Zone::Command,
                    }
                );
                assert!(created.commander);
            }
            other => panic!("wrong created object: {other:?}"),
        }
        if destination == DebugZone::Library {
            assert_eq!(game.players[0].library, vec![object_id]);
        }
    }
}

#[test]
fn create_rejects_unknown_cards_bad_ids_players_and_public_face_down_cards() {
    let base = || Game::with_players(2, 0);
    let create = |object_id, card_id: String, owner, controller, destination, face_down| {
        Mutation::CreateCard {
            object_id,
            card_id,
            owner,
            controller,
            destination,
            commander: false,
            face_down,
        }
    };

    let mut game = base();
    assert_operation_error(
        &mut game,
        create(0, "not-a-known-card".into(), P0, P0, DebugZone::Hand, false),
        ErrorReason::UnknownEntity,
    );
    let mut game = base();
    assert_operation_error(
        &mut game,
        create(1, card_id("Forest"), P0, P0, DebugZone::Hand, false),
        ErrorReason::InvalidValue,
    );
    let existing = game.spawn_in_hand(P0, card("Forest"));
    assert_operation_error(
        &mut game,
        create(existing, card_id("Forest"), P0, P0, DebugZone::Hand, false),
        ErrorReason::DuplicateId,
    );
    for (owner, controller, destination, face_down, reason) in [
        (
            PlayerId(8),
            P0,
            DebugZone::Hand,
            false,
            ErrorReason::UnknownEntity,
        ),
        (
            P0,
            PlayerId(8),
            DebugZone::Battlefield,
            false,
            ErrorReason::UnknownEntity,
        ),
        (P0, P1, DebugZone::Hand, false, ErrorReason::InvalidValue),
        (
            P0,
            P0,
            DebugZone::Graveyard,
            true,
            ErrorReason::InvalidValue,
        ),
        (P0, P0, DebugZone::Command, true, ErrorReason::InvalidValue),
    ] {
        let mut game = base();
        assert_operation_error(
            &mut game,
            create(
                0,
                card_id("Forest"),
                owner,
                controller,
                destination,
                face_down,
            ),
            reason,
        );
    }
}

#[test]
fn move_mints_a_new_object_tombstones_the_old_and_preserves_identity() {
    let mut game = Game::with_players(2, 0);
    let old = game.spawn_in_library(P0, card("Grizzly Bears"));
    if let Object::Card(card) = &mut game.objects[old as usize] {
        card.commander = true;
    }
    let new = game.next_object_id();
    apply_one(
        &mut game,
        Mutation::MoveCard {
            object_id: old,
            new_object_id: new,
            destination: DebugZone::Battlefield,
            controller: P1,
            face_down: true,
        },
    )
    .expect("valid move");

    assert!(matches!(game.objects[old as usize], Object::Moved { to } if to == new));
    let Object::Permanent(permanent) = &game.objects[new as usize] else {
        panic!("destination is not a permanent");
    };
    assert_eq!(permanent.owner, P0);
    assert_eq!(game.controller_of(new), P1);
    assert!(permanent.commander);
    assert!(permanent.face_down);
    assert!(!game.players[0].library.contains(&old));

    let next = game.next_object_id();
    apply_one(
        &mut game,
        Mutation::MoveCard {
            object_id: new,
            new_object_id: next,
            destination: DebugZone::Library,
            controller: P0,
            face_down: false,
        },
    )
    .expect("the newly controlled permanent can move again");
    assert_eq!(game.players[0].library.last(), Some(&next));
    assert!(
        matches!(&game.objects[next as usize], Object::Card(card) if card.commander && card.owner == P0)
    );
}

#[test]
fn move_card_visits_every_destination_and_inserts_library_at_bottom() {
    for destination in [
        DebugZone::Library,
        DebugZone::Hand,
        DebugZone::Battlefield,
        DebugZone::Graveyard,
        DebugZone::Exile,
        DebugZone::Command,
    ] {
        let mut game = Game::with_players(2, 0);
        let prior_bottom = game.spawn_in_library(P0, card("Forest"));
        let old = game.spawn_in_hand(P0, card("Grizzly Bears"));
        let new = game.next_object_id();
        apply_one(
            &mut game,
            Mutation::MoveCard {
                object_id: old,
                new_object_id: new,
                destination,
                controller: P0,
                face_down: false,
            },
        )
        .expect("valid move destination");
        if destination == DebugZone::Library {
            assert_eq!(game.players[0].library, vec![prior_bottom, new]);
        }
    }
}

#[test]
fn move_dirty_permanents_to_every_destination_gets_fresh_destination_state() {
    for destination in [
        DebugZone::Library,
        DebugZone::Hand,
        DebugZone::Battlefield,
        DebugZone::Graveyard,
        DebugZone::Exile,
        DebugZone::Command,
    ] {
        let mut game = Game::with_players(2, 0);
        let old = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
        game.permanent_mut(old).commander = true;
        game.permanent_mut(old).tapped = true;
        game.permanent_mut(old).marked_damage = 7;
        game.set_plus_counter_aggregate(old, 5);
        let new = game.next_object_id();

        apply_one(
            &mut game,
            Mutation::MoveCard {
                object_id: old,
                new_object_id: new,
                destination,
                controller: P0,
                face_down: false,
            },
        )
        .expect("unreferenced permanent moves to any debug destination");

        if destination == DebugZone::Battlefield {
            let Object::Permanent(permanent) = &game.objects[new as usize] else {
                panic!("battlefield destination must be a permanent");
            };
            assert!(permanent.commander);
            assert!(!permanent.tapped);
            assert_eq!(permanent.marked_damage, 0);
            assert_eq!(permanent.plus_counters, 0);
        } else {
            assert!(matches!(&game.objects[new as usize], Object::Card(card) if card.commander));
        }
        assert!(
            game.modifier_provenance
                .counter_batches
                .iter()
                .all(|&(object, ..)| object != old),
            "battlefield provenance is discarded with old object state"
        );
    }
}

#[test]
fn move_clears_ordinary_modifier_provenance_from_departing_permanent() {
    let mut game = Game::with_players(2, 0);
    let old = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.apply_recorded(&crate::Event::TempBoost {
        object: old,
        power: 2,
        toughness: 2,
        keywords: &[],
        source_name: "ordinary boost",
        ends_at_end_of_combat: false,
    });
    assert!(
        game.modifier_provenance
            .modifiers
            .iter()
            .any(|modifier| modifier.host == old)
    );

    let new = game.next_object_id();
    apply_one(
        &mut game,
        Mutation::MoveCard {
            object_id: old,
            new_object_id: new,
            destination: DebugZone::Hand,
            controller: P0,
            face_down: false,
        },
    )
    .expect("an ordinary boost lapses when its permanent changes zones");

    assert!(
        game.modifier_provenance
            .modifiers
            .iter()
            .all(|modifier| modifier.host != old)
    );
}

#[test]
fn detach_then_move_control_aura_clears_its_control_timestamp() {
    let mut game = Game::with_players(2, 0);
    let aura = game.spawn_on_battlefield(P0, card("Control Magic"));
    let host = game.spawn_on_battlefield(P1, card("Grizzly Bears"));
    game.apply_recorded(&crate::Event::AttachedTo {
        object: aura,
        host: Some(host),
    });
    assert!(
        game.play_permissions
            .aura_control_timestamps
            .iter()
            .any(|&(object, _)| object == aura)
    );

    let new = game.next_object_id();
    apply_operations(
        &mut game,
        &[
            Mutation::SetAttachment {
                object_id: aura,
                attached_to: None,
            },
            Mutation::MoveCard {
                object_id: aura,
                new_object_id: new,
                destination: DebugZone::Graveyard,
                controller: P0,
                face_down: false,
            },
        ],
    )
    .expect("detached control Aura can change zones in the same ordered batch");

    assert!(
        game.play_permissions
            .aura_control_timestamps
            .iter()
            .all(|&(object, _)| object != aura)
    );
}

#[test]
fn move_clears_owned_conditioned_control_override_with_departing_source() {
    let mut game = Game::with_players(2, 0);
    let old = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions.conditioned_control_overrides.push((
        old,
        P1,
        crate::ControlCondition {
            source: old,
            needs_tapped: false,
        },
        1,
    ));

    let new = game.next_object_id();
    apply_one(
        &mut game,
        Mutation::MoveCard {
            object_id: old,
            new_object_id: new,
            destination: DebugZone::Hand,
            controller: P0,
            face_down: false,
        },
    )
    .expect("a departing object's owned control metadata is cleared before blocker checks");

    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .all(|entry| entry.0 != old)
    );
}

#[test]
fn move_rejects_external_condition_dependency_without_clearing_its_owner() {
    let mut game = Game::with_players(2, 0);
    let dependency = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let controlled = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions.conditioned_control_overrides.push((
        controlled,
        P1,
        crate::ControlCondition {
            source: dependency,
            needs_tapped: false,
        },
        1,
    ));

    let new = game.next_object_id();
    assert_operation_error(
        &mut game,
        Mutation::MoveCard {
            object_id: dependency,
            new_object_id: new,
            destination: DebugZone::Hand,
            controller: P0,
            face_down: false,
        },
        ErrorReason::ReferencedObject,
    );
    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .any(|entry| entry.0 == controlled && entry.2.source == dependency)
    );
}

#[test]
fn move_allows_recomputed_action_references_and_refreshes_actions() {
    let mut game = Game::with_players(2, 0);
    let old = game.spawn_in_hand(P0, card("Forest"));
    game.refresh_actions();
    assert!(
        references_to(&game, old)
            .iter()
            .any(|site| site.disposition == ReferenceDisposition::Recomputed)
    );

    let new = game.next_object_id();
    apply_one(
        &mut game,
        Mutation::MoveCard {
            object_id: old,
            new_object_id: new,
            destination: DebugZone::Exile,
            controller: P0,
            face_down: false,
        },
    )
    .expect("derived actions do not block moves");

    assert!(
        references_to(&game, old)
            .iter()
            .all(|site| site.disposition != ReferenceDisposition::Recomputed)
    );
}

#[test]
fn move_rejects_bad_ids_kinds_constraints_tokens_and_blocking_references() {
    let mutation =
        |object_id, new_object_id, destination, controller, face_down| Mutation::MoveCard {
            object_id,
            new_object_id,
            destination,
            controller,
            face_down,
        };

    let mut game = Game::with_players(2, 0);
    let source_card = game.spawn_in_hand(P0, card("Forest"));
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(999, next, DebugZone::Hand, P0, false),
        ErrorReason::UnknownEntity,
    );
    assert_operation_error(
        &mut game,
        mutation(source_card, source_card, DebugZone::Hand, P0, false),
        ErrorReason::DuplicateId,
    );
    assert_operation_error(
        &mut game,
        mutation(source_card, next + 1, DebugZone::Hand, P0, false),
        ErrorReason::InvalidValue,
    );
    assert_operation_error(
        &mut game,
        mutation(source_card, next, DebugZone::Hand, P1, false),
        ErrorReason::InvalidValue,
    );
    assert_operation_error(
        &mut game,
        mutation(source_card, next, DebugZone::Graveyard, P0, true),
        ErrorReason::InvalidValue,
    );

    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.combat.attackers.push(permanent);
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(permanent, next, DebugZone::Hand, P0, false),
        ErrorReason::ReferencedObject,
    );

    let aura = game.spawn_on_battlefield(P0, card("Wild Growth"));
    let host = game.spawn_on_battlefield(P0, card("Forest"));
    game.permanent_mut(aura).attached_to = Some(host);
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(host, next, DebugZone::Hand, P0, false),
        ErrorReason::ReferencedObject,
    );
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(aura, next, DebugZone::Hand, P0, false),
        ErrorReason::ReferencedObject,
    );

    let pending = game.spawn_in_hand(P0, card("Lightning Bolt"));
    game.resume.spell_finish = Some(pending);
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(pending, next, DebugZone::Exile, P0, false),
        ErrorReason::ReferencedObject,
    );

    let token = game.spawn_token_on_battlefield(P0, card("Grizzly Bears"));
    let next = game.next_object_id();
    assert_operation_error(
        &mut game,
        mutation(token, next, DebugZone::Graveyard, P0, false),
        ErrorReason::InvalidValue,
    );
    apply_one(
        &mut game,
        mutation(token, next, DebugZone::Battlefield, P0, false),
    )
    .expect("a structurally valid battlefield reset keeps token identity");
    assert!(
        matches!(&game.objects[next as usize], Object::Permanent(permanent) if permanent.token)
    );
}

#[test]
fn move_rejects_spell_moved_and_removed_sources() {
    let mut game = Game::with_players(2, 0);
    let moved = game.spawn_in_hand(P0, card("Forest"));
    let target = game.spawn_in_hand(P0, card("Forest"));
    game.objects[moved as usize] = Object::Moved { to: target };
    let removed = game.spawn_in_hand(P0, card("Forest"));
    game.mark_removed(removed);
    let spell = game.spawn_in_hand(P0, card("Lightning Bolt"));
    game.fund_mana(P0);
    game.cast(
        P0,
        spell,
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
    .unwrap();
    let StackPayload::Spell(live_spell) = game.stack[0].payload else {
        panic!("cast did not create a spell");
    };
    for source in [moved, removed, live_spell] {
        let next = game.next_object_id();
        assert_operation_error(
            &mut game,
            Mutation::MoveCard {
                object_id: source,
                new_object_id: next,
                destination: DebugZone::Hand,
                controller: P0,
                face_down: false,
            },
            ErrorReason::WrongObjectKind,
        );
    }
}

#[test]
fn library_order_requires_the_exact_current_player_library_set() {
    let mut game = Game::with_players(2, 0);
    let ids = game.stack_library(P0, &[card("Forest"), card("Island"), card("Mountain")]);
    let foreign = game.spawn_in_library(P1, card("Swamp"));
    let hand = game.spawn_in_hand(P0, card("Plains"));
    apply_one(
        &mut game,
        Mutation::SetLibraryOrder {
            player: P0,
            object_ids: vec![ids[2], ids[0], ids[1]],
        },
    )
    .expect("exact reorder");
    assert_eq!(game.players[0].library, vec![ids[2], ids[0], ids[1]]);

    assert_operation_error(
        &mut game,
        Mutation::SetLibraryOrder {
            player: P0,
            object_ids: vec![ids[0], ids[0], ids[1]],
        },
        ErrorReason::DuplicateId,
    );
    for order in [
        vec![ids[0], ids[1]],
        vec![ids[0], ids[1], foreign],
        vec![ids[0], ids[1], hand],
    ] {
        assert_operation_error(
            &mut game,
            Mutation::SetLibraryOrder {
                player: P0,
                object_ids: order,
            },
            ErrorReason::ZoneDisagreement,
        );
    }
    assert_operation_error(
        &mut game,
        Mutation::SetLibraryOrder {
            player: PlayerId(9),
            object_ids: vec![],
        },
        ErrorReason::UnknownEntity,
    );
}

#[test]
fn remove_card_tombstones_only_unreferenced_live_nonbattlefield_cards() {
    let mut game = Game::with_players(2, 0);
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard { object_id: 999 },
        ErrorReason::UnknownEntity,
    );
    let removable = game.spawn_in_library(P0, card("Lightning Bolt"));
    let expected_id = game.def_id_of(removable);
    apply_one(
        &mut game,
        Mutation::RemoveCard {
            object_id: removable,
        },
    )
    .expect("unreferenced card removal");
    assert!(
        matches!(game.objects[removable as usize], Object::Removed { def, owner: P0 } if def == expected_id)
    );

    let permanent = game.spawn_on_battlefield(P0, card("Forest"));
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard {
            object_id: permanent,
        },
        ErrorReason::WrongObjectKind,
    );
    let token = game.spawn_token_on_battlefield(P0, card("Grizzly Bears"));
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard { object_id: token },
        ErrorReason::WrongObjectKind,
    );
    let moved = game.spawn_in_hand(P0, card("Forest"));
    game.objects[moved as usize] = Object::Moved { to: permanent };
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard { object_id: moved },
        ErrorReason::WrongObjectKind,
    );
    let removed = game.spawn_in_hand(P0, card("Forest"));
    game.mark_removed(removed);
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard { object_id: removed },
        ErrorReason::WrongObjectKind,
    );
    let spell_card = game.spawn_in_hand(P0, card("Lightning Bolt"));
    game.fund_mana(P0);
    game.cast(
        P0,
        spell_card,
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
    .unwrap();
    let StackPayload::Spell(spell) = game.stack[0].payload else {
        panic!("cast did not create a spell");
    };
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard { object_id: spell },
        ErrorReason::WrongObjectKind,
    );

    let referenced = game.spawn_in_library(P0, card("Island"));
    game.resume.spell_finish = Some(referenced);
    assert_operation_error(
        &mut game,
        Mutation::RemoveCard {
            object_id: referenced,
        },
        ErrorReason::ReferencedObject,
    );
}

#[test]
fn remove_card_clears_owned_control_override_memberships() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_in_hand(P0, card("Forest"));
    let condition_source = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions
        .control_overrides
        .push((object_id, P1, "debug fixture", 1));
    game.play_permissions
        .permanent_control_overrides
        .push((object_id, P1, 2));
    game.play_permissions.conditioned_control_overrides.push((
        object_id,
        P1,
        crate::ControlCondition {
            source: condition_source,
            needs_tapped: false,
        },
        3,
    ));
    validate_structural(&game)
        .expect("owned metadata on a live card remains structurally reachable");

    apply_one(&mut game, Mutation::RemoveCard { object_id })
        .expect("owned control metadata is cleared before removal");

    assert!(
        game.play_permissions
            .control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
    assert!(
        game.play_permissions
            .permanent_control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
}

#[test]
fn remove_card_clears_owned_aura_modifier_and_counter_memberships() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_in_hand(P0, card("Forest"));
    game.play_permissions
        .aura_control_timestamps
        .push((object_id, 1));
    game.register_modifier(
        object_id,
        "debug fixture",
        crate::ModifierDuration::Indefinite,
        crate::ModifierKind::Boost {
            power: 1,
            toughness: 1,
            keywords: &[],
        },
    );
    game.modifier_provenance
        .counter_batches
        .push((object_id, 1, "debug fixture"));
    validate_structural(&game)
        .expect("owned provenance on a live card remains structurally reachable");

    apply_one(&mut game, Mutation::RemoveCard { object_id })
        .expect("owned Aura and modifier provenance is cleared before removal");

    assert!(
        game.play_permissions
            .aura_control_timestamps
            .iter()
            .all(|entry| entry.0 != object_id)
    );
    assert!(
        game.modifier_provenance
            .modifiers
            .iter()
            .all(|modifier| modifier.host != object_id)
    );
    assert!(
        game.modifier_provenance
            .counter_batches
            .iter()
            .all(|entry| entry.0 != object_id)
    );
}

#[test]
fn remove_card_explicitly_removes_library_membership() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_in_library(P0, card("Forest"));
    assert!(game.players[0].library.contains(&object_id));

    apply_one(&mut game, Mutation::RemoveCard { object_id })
        .expect("library membership is explicitly removed");

    assert!(!game.players[0].library.contains(&object_id));
}

#[test]
fn remove_card_allows_recomputed_action_reference_and_refreshes_actions() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_in_hand(P0, card("Forest"));
    game.refresh_actions();
    assert!(
        references_to(&game, object_id)
            .iter()
            .any(|site| site.kind == "legal_action")
    );

    apply_one(&mut game, Mutation::RemoveCard { object_id })
        .expect("legal actions are recomputed after removal");

    assert!(
        references_to(&game, object_id)
            .iter()
            .all(|site| site.kind != "legal_action")
    );
}

#[test]
fn remove_card_clears_owned_conditioned_control_override_with_departing_source() {
    let mut game = Game::with_players(2, 0);
    let object_id = game.spawn_in_hand(P0, card("Forest"));
    game.play_permissions.conditioned_control_overrides.push((
        object_id,
        P1,
        crate::ControlCondition {
            source: object_id,
            needs_tapped: false,
        },
        1,
    ));

    apply_one(&mut game, Mutation::RemoveCard { object_id })
        .expect("a departing object's owned control metadata is cleared before blocker checks");

    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .all(|entry| entry.0 != object_id)
    );
}

#[test]
fn remove_card_rejects_external_condition_dependency_instead_of_clearing_its_owner() {
    let mut game = Game::with_players(2, 0);
    let dependency = game.spawn_in_hand(P0, card("Forest"));
    let controlled = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.play_permissions.conditioned_control_overrides.push((
        controlled,
        P1,
        crate::ControlCondition {
            source: dependency,
            needs_tapped: false,
        },
        1,
    ));

    assert_operation_error(
        &mut game,
        Mutation::RemoveCard {
            object_id: dependency,
        },
        ErrorReason::ReferencedObject,
    );
    assert!(
        game.play_permissions
            .conditioned_control_overrides
            .iter()
            .any(|entry| entry.0 == controlled && entry.2.source == dependency)
    );
}

#[test]
fn structurally_valid_rule_illegal_debug_state_is_accepted() {
    let mut game = Game::with_players(2, 0);
    let ids = game.stack_library(P0, &[card("Forest"), card("Island"), card("Mountain")]);
    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    apply_operations(
        &mut game,
        &[
            Mutation::SetLife {
                player: P0,
                life: 0,
            },
            Mutation::SetTurnState {
                active_player: P0,
                step: Step::Cleanup,
                priority_player: P1,
                consecutive_passes: 0,
            },
            Mutation::SetController {
                object_id: permanent,
                controller: P1,
            },
            Mutation::SetLibraryOrder {
                player: P0,
                object_ids: vec![ids[1], ids[2], ids[0]],
            },
        ],
    )
    .expect("rule-illegal state is structurally coherent");
    assert_eq!(game.players[0].life, 0);
    assert_eq!(game.step, Step::Cleanup);
    assert_eq!(game.controller_of(permanent), P1);
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

fn game_with_every_pending_rider() -> Game {
    use std::sync::Arc;

    use crate::resolution::{DrawAfter, DrawBatch, ResolveCtx, SearchFanout, SequenceCont};

    let mut game = Game::with_players(2, 0);
    let trigger_source = game.spawn_on_battlefield(P0, card("Phyrexian Arena"));
    let scratch_object = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let spell = game.spawn_in_hand(P0, card("Lightning Bolt"));
    game.fund_mana(P0);
    game.cast(
        P0,
        spell,
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
    .expect("fixture spell casts");
    game.spawn_in_hand(P1, card("Lightning Bolt"));
    game.fund_mana(P1);

    game.pending_choice = Some(PendingChoice::ChooseTarget {
        player: P0,
        controller: P0,
        source: spell,
        effect: None,
        legal: vec![crate::Target::Player(P1)],
        count: TargetCount::default(),
        clause: 0,
        target: None,
        x: 0,
        spent_mana: [0; 6],
        activated: false,
    });
    game.resume.clash_scry = Some(P1);
    game.resume.sequence = Some(SequenceCont {
        steps: Arc::from([Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count: crate::Amount::Fixed(1),
        })]),
        ctx: ResolveCtx {
            controller: P0,
            source: spell,
            target: Some(crate::Target::Player(P1)),
            targets_second: Default::default(),
            x: 0,
            spent_mana: [0; 6],
        },
    });
    game.resume.demonstrate_opponent_copy = Some((P1, spell));
    game.resume.spell_finish = Some(spell);
    game.resume.draw_batch = Some(DrawBatch {
        seats: vec![(P0, 1)],
        after: DrawAfter::DrawStep,
        paused: true,
    });

    let scratch_def = match &game.objects[scratch_object as usize] {
        Object::Permanent(permanent) => permanent.def,
        _ => unreachable!("fixture object is a permanent"),
    };
    game.resolution_frame.destroyed_this_way = vec![crate::state::DestroyedThisWay {
        def: scratch_def,
        controller: P0,
        token: false,
    }];
    game.resolution_frame.nonland_cards_exiled_this_way = 1;
    game.resolution_frame.cards_discarded_this_way = 1;
    game.resolution_frame.creatures_sacrificed_this_way = 1;
    game.resolution_frame.cards_exiled_by_search_this_way = 1;
    game.resolution_frame.join_forces_mana = 1;
    game.resolution_frame.council_past_votes = 1;
    game.resolution_frame.council_present_votes = 1;
    game.resolution_frame.milled_mana_value_this_way = 1;
    game.resolution_frame.counters_removed_this_way = 1;
    game.resolution_frame.damage_dealt_this_way = 1;
    game.resolution_frame.resolving_targets = vec![crate::Target::Player(P1)];
    game.resolution_frame.surge_exiled_card = Some((scratch_object, 1));
    game.resolution_frame.returned_nonland_card_mana_value = Some(1);
    game.resolution_frame.power_exiled_this_way = vec![crate::state::PowerExiledThisWay {
        controller: P0,
        power: 2,
    }];
    game.resolution_frame.sacrificed_by_edict_controller = true;
    game.resolution_frame.vanished_permanent_owner = Some((scratch_object, P0));
    game.resolution_frame.chosen_damage_source = Some(scratch_object);
    game.resolution_frame.search_fanout = Some(SearchFanout {
        remaining: vec![P1],
        filter: crate::CardFilter::AnyCard,
        to_zone: crate::SearchDest::Hand,
        tapped: false,
        count: 1,
        overflow: None,
    });
    game.resolution_frame.discard_cause = Some(P0);

    game.resolution_finish = Some(crate::FinishPolicy::Exile);
    game.pending_enter_bonus_counters.push((spell, 1));
    game.clash_won = true;
    game.pending_trigger_groups.push(crate::TriggerGroup {
        controller: P0,
        source: trigger_source,
        abilities: vec![card("Phyrexian Arena").abilities[0].clone()],
        expanded: false,
    });
    game.pending_obligations.push(crate::Obligation::Echo {
        permanent: scratch_object,
    });
    game.refresh_actions();
    game
}

#[test]
fn pending_orchestration_inspection_reports_every_rider_category() {
    let game = game_with_every_pending_rider();

    assert_eq!(
        inspect_pending_orchestration(&game),
        PendingOrchestrationInspection {
            has_pending_choice: true,
            has_resume: true,
            has_resolution_frame: true,
            has_resolution_finish: true,
            pending_enter_bonus_counters: 1,
            pending_trigger_groups: 1,
            pending_obligations: 1,
        }
    );
    assert!(game.clash_won, "the fixture includes clash-local scratch");
    assert_eq!(object_slot_count(&game), game.objects.len());
}

#[test]
fn clear_pending_orchestration_drops_pause_resume_scratch_and_refreshes_actions() {
    let mut game = game_with_every_pending_rider();
    assert!(inspect_pending_orchestration(&game).has_pending_choice);

    apply_operations(
        &mut game,
        &[Mutation::ClearPendingOrchestration {
            clear_queued_triggers: true,
        }],
    )
    .expect("the coherent clear is structurally safe");

    assert_eq!(
        inspect_pending_orchestration(&game),
        PendingOrchestrationInspection::default()
    );
    assert!(!game.clash_won);
    assert!(
        !game.legal_actions().is_empty(),
        "the batch tail refreshes actions"
    );
    assert!(
        validate_structural(&game).is_ok(),
        "the paused spell remains paired with its stack item"
    );
    game.submit(crate::Intent::PassPriority { player: P0 })
        .expect("the active player can pass priority");
    game.submit(crate::Intent::PassPriority { player: P1 })
        .expect("the opponent's pass resolves the retained spell");
    assert!(game.stack.is_empty(), "the formerly paused spell resolves");
    assert!(validate_structural(&game).is_ok());
}

#[test]
fn cleared_pausing_spell_restarts_from_its_first_effect_on_the_next_resolution() {
    let mut game = Game::with_players(2, 0);
    let library = game.stack_library(
        P0,
        &[
            card("Forest"),
            card("Grizzly Bears"),
            card("Lightning Bolt"),
        ],
    );
    let spell = game.spawn_in_hand(P0, card("Prismari Charm"));
    game.fund_mana(P0);
    game.cast(
        P0,
        spell,
        None,
        0,
        vec![(0, None)],
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
    .expect("Prismari Charm's surveil-then-draw mode casts");

    for player in [P0, P1] {
        game.submit(crate::Intent::PassPriority { player })
            .expect("ordinary priority passes resolve the spell");
    }
    assert!(
        matches!(
            game.pending_choice(),
            Some(PendingChoice::ArrangeTop { .. })
        ),
        "the first surveil pauses the spell"
    );

    apply_operations(
        &mut game,
        &[Mutation::ClearPendingOrchestration {
            clear_queued_triggers: true,
        }],
    )
    .expect("clearing the first pause retains a structurally valid stack item");

    for player in [P0, P1] {
        game.submit(crate::Intent::PassPriority { player })
            .expect("ordinary priority passes safely restart the retained spell");
    }
    let shown = match game.pending_choice() {
        Some(PendingChoice::ArrangeTop { cards, .. }) => cards.clone(),
        other => panic!("restarted spell should repeat its surveil, got {other:?}"),
    };
    assert_eq!(
        shown,
        library[..2],
        "clearing a paused continuation documents restart-from-the-beginning semantics"
    );

    game.submit(crate::Intent::ArrangeTop {
        player: P0,
        top: shown,
        bottom: vec![],
    })
    .expect("answering the repeated surveil completes the restarted spell");
    assert!(game.stack.is_empty());
    assert!(game.pending_choice().is_none());
    assert!(validate_structural(&game).is_ok());
}

#[test]
fn clear_pending_orchestration_can_preserve_queued_triggers_and_obligations() {
    let mut game = game_with_every_pending_rider();

    apply_operations(
        &mut game,
        &[Mutation::ClearPendingOrchestration {
            clear_queued_triggers: false,
        }],
    )
    .unwrap();

    let pending = inspect_pending_orchestration(&game);
    assert!(!pending.has_pending_choice);
    assert!(!pending.has_resume);
    assert!(!pending.has_resolution_frame);
    assert!(!pending.has_resolution_finish);
    assert_eq!(pending.pending_enter_bonus_counters, 0);
    assert_eq!(pending.pending_trigger_groups, 1);
    assert_eq!(pending.pending_obligations, 1);
    assert!(!game.clash_won);
}

#[test]
fn object_slot_count_includes_moved_and_removed_tombstones() {
    let mut game = Game::with_players(2, 0);
    let moved = game.spawn_in_hand(P0, card("Forest"));
    let destination = game.spawn_in_hand(P0, card("Forest"));
    let removed = game.spawn_in_hand(P1, card("Forest"));
    let (removed_def, removed_owner) = match game.objects[removed as usize] {
        Object::Card(ref card) => (card.def, card.owner),
        _ => unreachable!("fixture object is a card"),
    };
    game.objects[moved as usize] = Object::Moved { to: destination };
    game.objects[removed as usize] = Object::Removed {
        def: removed_def,
        owner: removed_owner,
    };

    assert_eq!(object_slot_count(&game), 3);
}

#[test]
fn later_invalid_operation_does_not_commit_an_earlier_pending_clear() {
    let live = game_with_every_pending_rider();
    let before = inspect_pending_orchestration(&live);
    let mut candidate = live.clone();
    let error = apply_operations(
        &mut candidate,
        &[
            Mutation::ClearPendingOrchestration {
                clear_queued_triggers: true,
            },
            Mutation::SetLife {
                player: PlayerId(9),
                life: 20,
            },
        ],
    )
    .unwrap_err();

    assert_eq!(error.operation_index, Some(1));
    assert_eq!(inspect_pending_orchestration(&live), before);
    assert_eq!(
        inspect_pending_orchestration(&candidate),
        PendingOrchestrationInspection::default(),
        "the failed candidate was partially edited and must be discarded"
    );
}

#[test]
fn stack_render_source_public_ghost_is_source_less_targetless_and_resolves_without_events() {
    let mut game = Game::with_players(2, 7);
    let public = PublicStackGhost {
        name: "Public fixture".to_string(),
        label: "No-op ability".to_string(),
        printing_id: "11111111-1111-1111-1111-111111111111".to_string(),
        card_id: Some(card("Lightning Bolt").id.to_string()),
        printed_sentences: vec!["This is explicit public text.".to_string()],
    };

    let query_source = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let entry_id = push_public_stack_ghost(&mut game, P1, public.clone())
        .expect("valid public metadata inserts one ghost");

    assert_eq!(entry_id, crate::StackEntryId(1));
    assert_eq!(
        game.stack(),
        vec![crate::StackEntry {
            entry_id,
            kind: crate::StackEntryKind::DebugNoOp {
                controller: P1,
                public: public.clone(),
            },
        }]
    );
    assert_eq!(game.stack()[0].kind.object_source(), None);
    assert!(matches!(
        &game.stack[0].render_source,
        StackRenderSource::InlinePublic(_)
    ));
    assert!(matches!(&game.stack[0].payload, StackPayload::DebugNoOp));
    assert!(
        game.legal_targets_for(
            crate::TargetSpec::InstantOrSorcerySpellOnStack,
            query_source,
            P0,
            [false; crate::Color::COUNT],
            0,
        )
        .is_empty(),
        "a source-less ghost is not copy/spell-target eligible"
    );
    assert!(
        game.legal_targets_for(
            crate::TargetSpec::ActivatedAbilityOnStack {
                artifact_source: false,
            },
            query_source,
            P0,
            [false; crate::Color::COUNT],
            0,
        )
        .is_empty(),
        "a ghost is not an activated-ability target"
    );

    let mut resolution_events = Vec::new();
    game.resolve_top(&mut resolution_events);
    assert!(
        resolution_events.is_empty(),
        "debug no-op resolution itself emits no engine events"
    );
    assert!(game.stack().is_empty());

    push_public_stack_ghost(&mut game, P1, public).unwrap();
    game.submit(crate::Intent::PassPriority { player: P0 })
        .expect("first player passes");
    game.submit(crate::Intent::PassPriority { player: P1 })
        .expect("second pass safely resolves the ghost");
    assert!(game.stack().is_empty());
}

#[test]
fn stack_render_source_public_ghost_metadata_enforces_byte_bounds_atomically() {
    let known_card_id = card("Lightning Bolt").id.to_string();
    let cases = [
        (
            "empty name",
            PublicStackGhost {
                name: String::new(),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidName,
        ),
        (
            "name over bound",
            PublicStackGhost {
                name: "é".repeat(65),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidName,
        ),
        (
            "empty label",
            PublicStackGhost {
                label: " ".to_string(),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidLabel,
        ),
        (
            "label over bound",
            PublicStackGhost {
                label: "é".repeat(257),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidLabel,
        ),
        (
            "empty printing",
            PublicStackGhost {
                printing_id: String::new(),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidPrintingId,
        ),
        (
            "printing over bound",
            PublicStackGhost {
                printing_id: "é".repeat(33),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidPrintingId,
        ),
        (
            "empty card id",
            PublicStackGhost {
                card_id: Some(String::new()),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidCardId,
        ),
        (
            "card id over bound",
            PublicStackGhost {
                card_id: Some("é".repeat(33)),
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidCardId,
        ),
        (
            "unknown card id",
            PublicStackGhost {
                card_id: Some("unknown-public-card".to_string()),
                ..valid_public_ghost()
            },
            PublicStackGhostError::UnknownCardId,
        ),
        (
            "too many sentences",
            PublicStackGhost {
                printed_sentences: vec!["ok".to_string(); 9],
                ..valid_public_ghost()
            },
            PublicStackGhostError::TooManyPrintedSentences,
        ),
        (
            "empty sentence",
            PublicStackGhost {
                printed_sentences: vec![" ".to_string()],
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidPrintedSentence,
        ),
        (
            "sentence over bound",
            PublicStackGhost {
                printed_sentences: vec!["é".repeat(257)],
                ..valid_public_ghost()
            },
            PublicStackGhostError::InvalidPrintedSentence,
        ),
    ];

    for (case, mut public, expected) in cases {
        if public.card_id.as_deref() == Some("known") {
            public.card_id = Some(known_card_id.clone());
        }
        let mut game = Game::with_players(2, 7);
        let allocator_before = game.next_stack_entry_id;
        assert_eq!(
            push_public_stack_ghost(&mut game, P0, public),
            Err(expected),
            "{case}"
        );
        assert_eq!(
            game.next_stack_entry_id, allocator_before,
            "{case} consumed an id"
        );
        assert!(game.stack.is_empty(), "{case} partially inserted");
    }

    let mut game = Game::with_players(2, 7);
    assert_eq!(
        push_public_stack_ghost(&mut game, PlayerId(2), valid_public_ghost()),
        Err(PublicStackGhostError::InvalidController)
    );
    assert_eq!(game.next_stack_entry_id.unwrap().get(), 1);
    assert!(game.stack.is_empty());
}

#[test]
fn stack_render_source_public_ghost_accepts_exact_metadata_byte_limits() {
    let mut game = Game::with_players(2, 7);
    let public = PublicStackGhost {
        name: "n".repeat(MAX_PUBLIC_STACK_GHOST_NAME_BYTES),
        label: "l".repeat(MAX_PUBLIC_STACK_GHOST_LABEL_BYTES),
        printing_id: "p".repeat(MAX_PUBLIC_STACK_GHOST_PRINTING_ID_BYTES),
        card_id: Some(card("Lightning Bolt").id.to_string()),
        printed_sentences: vec![
            "s".repeat(MAX_PUBLIC_STACK_GHOST_SENTENCE_BYTES);
            MAX_PUBLIC_STACK_GHOST_SENTENCES
        ],
    };
    push_public_stack_ghost(&mut game, P0, public).expect("exact bounds are accepted");
    assert!(validate_structural(&game).is_ok());
}

#[test]
fn stack_render_source_structural_validator_rejects_every_mismatched_pair() {
    let base = valid_public_ghost();
    let mut ghost_game = Game::with_players(2, 7);
    push_public_stack_ghost(&mut ghost_game, P0, base.clone()).unwrap();
    let inline = ghost_game.stack[0].render_source.clone();

    let mut object_debug = ghost_game.clone();
    object_debug.stack[0].render_source = StackRenderSource::Object(0);
    assert!(codes(&object_debug).contains(&"stack_pairing"));

    let mut game = Game::with_players(2, 7);
    let source = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    game.push_stack_item(
        StackRenderSource::Object(source),
        StackPayload::Ability {
            controller: P0,
            source,
            effect: Effect::Draw(crate::DrawEffect::Cards {
                who: crate::PlayerSet::You,
                count: crate::Amount::Fixed(1),
            }),
            activated: false,
            target: None,
            targets_second: Default::default(),
            x: 0,
            spent_mana: [0; 6],
        },
    )
    .unwrap();
    game.stack[0].render_source = inline.clone();
    assert!(codes(&game).contains(&"stack_pairing"));

    let mut spell_game = Game::with_players(2, 7);
    let card_id = spell_game.spawn_in_hand(P0, card("Lightning Bolt"));
    spell_game.fund_mana(P0);
    spell_game
        .cast(
            P0,
            card_id,
            Some(Target::Player(P1)),
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
        .unwrap();
    spell_game.stack[0].render_source = inline;
    assert!(codes(&spell_game).contains(&"stack_pairing"));
}

#[test]
fn stack_render_source_public_ghost_exhaustion_is_atomic() {
    let mut game = Game::with_players(2, 7);
    game.next_stack_entry_id = None;
    let allocator_before = game.next_stack_entry_id;

    assert_eq!(
        push_public_stack_ghost(&mut game, P0, valid_public_ghost()),
        Err(PublicStackGhostError::StackEntryIdExhausted)
    );
    assert_eq!(game.next_stack_entry_id, allocator_before);
    assert!(game.stack.is_empty());
}

#[test]
fn stack_render_source_public_ghost_preserves_accepted_whitespace() {
    let mut game = Game::with_players(2, 7);
    let public = PublicStackGhost {
        name: "  Fixture  ".to_string(),
        label: "  Public no-op  ".to_string(),
        printing_id: "  print  ".to_string(),
        card_id: None,
        printed_sentences: vec!["  public sentence  ".to_string()],
    };

    push_public_stack_ghost(&mut game, P0, public.clone()).expect("nonempty metadata is accepted");
    assert!(matches!(
        &game.stack()[0].kind,
        crate::StackEntryKind::DebugNoOp { public: stored, .. } if stored == &public
    ));
}

#[test]
fn stack_render_source_public_ghost_card_id_lookup_is_exact() {
    let mut game = Game::with_players(2, 7);
    let public = PublicStackGhost {
        card_id: Some(format!(" {} ", card("Lightning Bolt").id)),
        ..valid_public_ghost()
    };

    assert_eq!(
        push_public_stack_ghost(&mut game, P0, public),
        Err(PublicStackGhostError::UnknownCardId)
    );
    assert_eq!(game.next_stack_entry_id.unwrap().get(), 1);
    assert!(game.stack.is_empty());
}

fn valid_public_ghost() -> PublicStackGhost {
    PublicStackGhost {
        name: "Fixture".to_string(),
        label: "Public no-op".to_string(),
        printing_id: "print".to_string(),
        card_id: None,
        printed_sentences: vec![],
    }
}

fn known_spell_spec(
    entry_id: u64,
    from_object_id: ObjectId,
    spell_object_id: ObjectId,
) -> DebugStackEntrySpec {
    DebugStackEntrySpec::KnownSpell {
        entry_id: StackEntryId(entry_id),
        from_object_id,
        spell_object_id,
        controller: P0,
        targets: vec![],
        targets_second: vec![],
        x: 0,
    }
}

#[test]
fn debug_stack_mutation_replaces_with_six_real_spells_and_a_public_ghost() {
    let mut game = Game::with_players(2, 7);
    let names = [
        "Dark Ritual",
        "Vision Skeins",
        "Night's Whisper",
        "Harmonize",
        "Fog",
        "Time Walk",
    ];
    let cards: Vec<_> = names
        .iter()
        .map(|name| game.spawn_in_hand(P0, card(name)))
        .collect();
    let first_spell = game.next_object_id();
    let mut specs: Vec<_> = cards
        .iter()
        .enumerate()
        .map(|(index, &from)| {
            known_spell_spec((10 + index) as u64, from, first_spell + index as ObjectId)
        })
        .collect();
    specs.push(DebugStackEntrySpec::PublicGhost {
        entry_id: StackEntryId(16),
        controller: P1,
        public: valid_public_ghost(),
    });

    let ids = replace_stack(&mut game, &specs).expect("supported stack replacement");
    assert_eq!(ids, (10..=16).map(StackEntryId).collect::<Vec<_>>());
    let stack = inspect(&game).stack;
    assert_eq!(
        stack.iter().map(|row| row.entry_id.0).collect::<Vec<_>>(),
        (10..=16).collect::<Vec<_>>()
    );
    assert_eq!(stack[0].source_object_id, Some(first_spell));
    assert_eq!(stack[6].source_object_id, None);
    assert_eq!(stack[6].public_ghost, Some(valid_public_ghost()));
    assert!(stack.iter().all(|row| row.targets.is_empty()));
    assert_eq!(game.next_stack_entry_id.unwrap().get(), 17);

    let p0_cards = vec![card("Forest"); 8];
    let p1_cards = vec![card("Island"); 2];
    let p0_library = game.stack_library(P0, &p0_cards);
    let p1_library = game.stack_library(P1, &p1_cards);
    let life_before = game.life(P0);
    let spell_objects: Vec<_> = (first_spell..first_spell + names.len() as ObjectId).collect();

    let mut events = vec![];
    game.resolve_top(&mut events);
    assert!(events.is_empty());
    assert_eq!(game.stack().len(), 6);
    while !game.stack().is_empty() {
        game.resolve_top(&mut events);
    }

    assert!(game.stack().is_empty());
    assert!(
        spell_objects
            .iter()
            .all(|&object| !matches!(game.objects[object as usize], Object::Spell(_)))
    );
    assert_eq!(game.mana_in_pool(P0, crate::Color::Black), 3);
    assert_eq!(game.life(P0), life_before - 2);
    assert_eq!(game.players[0].library.len(), p0_library.len() - 7);
    assert_eq!(game.players[1].library.len(), p1_library.len() - 2);
}

#[test]
fn debug_stack_mutation_rejects_identity_object_shape_and_unsupported_spells_atomically() {
    let cases = [
        ("zero", 0, "Dark Ritual", ErrorReason::InvalidValue.into()),
        ("old id", 0, "Dark Ritual", ErrorReason::InvalidValue.into()),
        (
            "modal",
            1,
            "Witherbloom Command",
            StackEditReason::UnsupportedStackConstruction,
        ),
        (
            "divided",
            1,
            "Magma Opus",
            StackEditReason::UnsupportedStackConstruction,
        ),
        (
            "additional cost",
            1,
            "Big Score",
            StackEditReason::UnsupportedStackConstruction,
        ),
    ];
    for (label, entry, name, reason) in cases {
        let mut game = Game::with_players(2, 7);
        if label == "old id" {
            push_public_stack_ghost(&mut game, P0, valid_public_ghost()).unwrap();
        }
        let from = game.spawn_in_hand(P0, card(name));
        let before = game.clone();
        let spec = known_spell_spec(entry, from, game.next_object_id());
        assert_eq!(
            push_stack(&mut game, &spec),
            Err(StackEditError {
                entry_index: Some(0),
                reason
            }),
            "{label}"
        );
        assert_eq!(inspect(&game), inspect(&before), "{label} must roll back");
        assert_eq!(game.next_stack_entry_id, before.next_stack_entry_id);
    }

    let mut game = Game::with_players(2, 7);
    let first = game.spawn_in_hand(P0, card("Dark Ritual"));
    let second = game.spawn_in_hand(P0, card("Fog"));
    let before = game.clone();
    let next = game.next_object_id();
    let specs = [
        known_spell_spec(5, first, next),
        known_spell_spec(5, second, next + 1),
    ];
    assert_eq!(
        replace_stack(&mut game, &specs),
        Err(StackEditError {
            entry_index: Some(1),
            reason: ErrorReason::DuplicateId.into()
        })
    );
    assert_eq!(inspect(&game), inspect(&before));

    let mut game = Game::with_players(2, 7);
    let from = game.spawn_in_hand(P0, card("Dark Ritual"));
    let spec = known_spell_spec(1, from, game.next_object_id() + 1);
    assert_eq!(
        push_stack(&mut game, &spec).unwrap_err().reason,
        ErrorReason::InvalidValue.into()
    );
}

#[test]
fn debug_stack_mutation_constructs_public_authored_ability_and_validates_source() {
    let mut game = Game::with_players(2, 7);
    let source = game.spawn_on_battlefield(P0, card("Llanowar Elves"));
    let spec = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(4),
        controller: P0,
        source_object_id: source,
        ability_index: 0,
        target: None,
        targets_second: vec![],
        x: 0,
    };
    push_stack(&mut game, &spec).expect("authored activated ability");
    assert_eq!(inspect(&game).stack[0].source_object_id, Some(source));
    assert!(
        !game.permanent(source).tapped,
        "debug construction does not pay the tap cost"
    );
    let mut events = vec![];
    game.resolve_top(&mut events);
    assert!(!game.permanent(source).tapped);
    assert_eq!(game.mana_in_pool(P0, crate::Color::Green), 1);

    let mut game = Game::with_players(2, 7);
    game.stack_library(P0, &[card("Forest")]);
    let triggered_source = game.spawn_in_graveyard(P0, card("Solemn Simulacrum"));
    let triggered = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(5),
        controller: P0,
        source_object_id: triggered_source,
        ability_index: 1,
        target: None,
        targets_second: vec![],
        x: 0,
    };
    push_stack(&mut game, &triggered).expect("context-free authored triggered ability");
    assert_eq!(
        inspect(&game).stack[0].source_object_id,
        Some(triggered_source)
    );
    game.resolve_top(&mut events);
    assert!(game.stack().is_empty());
    assert_eq!(game.hand(P0).len(), 1);

    let mut game = Game::with_players(2, 7);
    let source = game.spawn_on_battlefield(P0, card("Llanowar Elves"));
    let hidden = game.spawn_in_hand(P0, card("Llanowar Elves"));
    let hidden_spec = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(6),
        controller: P0,
        source_object_id: hidden,
        ability_index: 0,
        target: None,
        targets_second: vec![],
        x: 0,
    };
    assert_eq!(
        push_stack(&mut game, &hidden_spec).unwrap_err().reason,
        StackEditReason::UnsupportedStackConstruction
    );

    let bad = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(6),
        controller: P0,
        source_object_id: source,
        ability_index: 99,
        target: None,
        targets_second: vec![],
        x: 0,
    };
    assert_eq!(
        push_stack(&mut game, &bad).unwrap_err().reason,
        ErrorReason::UnknownEntity.into()
    );
}

#[test]
fn debug_stack_mutation_saturates_admitted_mana_when_the_pool_fills_before_resolution() {
    for (initial, repeat) in [(1, 4096), (u8::MAX, 1)] {
        let mut def = card("Llanowar Elves");
        let mut abilities = def.abilities.to_vec();
        let Effect::Mana(crate::ManaEffect::Add { repeat: count, .. }) = &mut abilities[0].effect
        else {
            panic!("Llanowar Elves has an add-mana ability");
        };
        *count = crate::Amount::Fixed(repeat);
        def.abilities = abilities.into();

        let mut game = Game::with_players(2, 7);
        game.players[0]
            .mana_pool
            .add(crate::Mana::Color(crate::Color::Green), initial);
        let source = game.spawn_on_battlefield(P0, def);
        push_stack(&mut game, &authored_ability_spec(source, 0))
            .expect("bounded authored mana ability is admitted");

        let mut events = vec![];
        game.resolve_top(&mut events);

        assert_eq!(game.mana_in_pool(P0, crate::Color::Green), u8::MAX);
    }
}

#[test]
fn debug_stack_mutation_rejects_activated_mana_with_activation_side_channels() {
    for (name, ability_index) in [
        ("Lotus Field", 1),
        ("Kami of Whispered Hopes", 1),
        ("Glistening Sphere", 2),
        ("Path of Ancestry", 0),
    ] {
        let mut game = Game::with_players(2, 7);
        let source = game.spawn_on_battlefield(P0, card(name));
        let spec = authored_ability_spec(source, ability_index);

        assert_eq!(
            push_stack(&mut game, &spec).unwrap_err().reason,
            StackEditReason::UnsupportedStackConstruction,
            "{name} ability {ability_index}"
        );
        assert!(game.stack().is_empty(), "{name} must not be admitted");
    }
}

#[test]
fn debug_stack_mutation_rejects_every_arithmetic_endpoint_before_resolution() {
    let hazards = [
        crate::Amount::Combine {
            left: &crate::Amount::Fixed(i32::MAX),
            op: crate::ArithOp::Add,
            right: &crate::Amount::Fixed(1),
        },
        crate::Amount::Combine {
            left: &crate::Amount::Fixed(i32::MIN),
            op: crate::ArithOp::Subtract,
            right: &crate::Amount::Fixed(1),
        },
        crate::Amount::Combine {
            left: &crate::Amount::Fixed(i32::MAX),
            op: crate::ArithOp::Multiply,
            right: &crate::Amount::Fixed(2),
        },
        crate::Amount::Combine {
            left: &crate::Amount::Fixed(1),
            op: crate::ArithOp::DivideRoundingDown,
            right: &crate::Amount::Fixed(0),
        },
        crate::Amount::Combine {
            left: &crate::Amount::Fixed(i32::MIN),
            op: crate::ArithOp::DivideRoundingUp,
            right: &crate::Amount::Fixed(-1),
        },
    ];

    for count in hazards {
        let mut def = card("Llanowar Elves");
        let mut abilities = def.abilities.to_vec();
        abilities[0].effect = Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count,
        });
        def.abilities = abilities.into();

        let mut game = Game::with_players(2, 7);
        let source = game.spawn_on_battlefield(P0, def);
        let spec = authored_ability_spec(source, 0);
        assert_eq!(
            push_stack(&mut game, &spec).unwrap_err().reason,
            StackEditReason::UnsupportedStackConstruction
        );
        assert!(game.stack().is_empty());
    }
}

fn authored_ability_spec(source_object_id: ObjectId, ability_index: usize) -> DebugStackEntrySpec {
    DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(1),
        controller: P0,
        source_object_id,
        ability_index,
        target: None,
        targets_second: vec![],
        x: 0,
    }
}

#[test]
fn debug_stack_mutation_rejects_trigger_effects_that_need_missing_event_context() {
    let mut game = Game::with_players(2, 7);
    let source = game.spawn_in_graveyard(P0, card("Creature Bond"));
    let spec = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(1),
        controller: P0,
        source_object_id: source,
        ability_index: 0,
        target: None,
        targets_second: vec![],
        x: 0,
    };

    assert_eq!(
        push_stack(&mut game, &spec).unwrap_err().reason,
        StackEditReason::UnsupportedStackConstruction
    );
    assert!(game.stack().is_empty());
}

#[test]
fn debug_stack_mutation_rejects_pending_and_pop_underflow_and_cleans_spells() {
    let mut game = Game::with_players(2, 7);
    assert_eq!(
        pop_stack(&mut game, 0).unwrap_err().reason,
        ErrorReason::InvalidValue.into()
    );
    assert_eq!(
        pop_stack(&mut game, 1).unwrap_err().reason,
        ErrorReason::InvalidValue.into()
    );

    let source = game.spawn_in_hand(P0, card("Dark Ritual"));
    game.pending_choice = Some(PendingChoice::MayDrawUpTo {
        player: P0,
        max: 1,
        effect: Effect::Draw(crate::DrawEffect::Cards {
            who: crate::PlayerSet::You,
            count: crate::Amount::Fixed(1),
        }),
        resume: crate::MayDrawUpToResume::TradeSecretsRepeat {
            opponent: P1,
            source,
        },
    });
    let spec = known_spell_spec(1, source, game.next_object_id());
    assert_eq!(
        push_stack(&mut game, &spec).unwrap_err().reason,
        StackEditReason::PendingOrchestration
    );
    clear_pending_orchestration(&mut game, true);
    push_stack(&mut game, &spec).unwrap();
    let spell = game.next_object_id() - 1;
    pop_stack(&mut game, 1).unwrap();
    assert!(matches!(
        game.objects[spell as usize],
        Object::Removed { .. }
    ));
}

#[test]
fn debug_stack_mutation_supports_checked_single_targets_x_and_explicit_id_gaps() {
    let mut game = Game::with_players(2, 7);
    let bolt = game.spawn_in_hand(P0, card("Lightning Bolt"));
    let hurricane = game.spawn_in_hand(P0, card("Hurricane"));
    let next = game.next_object_id();
    let specs = [
        DebugStackEntrySpec::KnownSpell {
            entry_id: StackEntryId(20),
            from_object_id: bolt,
            spell_object_id: next,
            controller: P0,
            targets: vec![Target::Player(P1)],
            targets_second: vec![],
            x: 0,
        },
        DebugStackEntrySpec::KnownSpell {
            entry_id: StackEntryId(24),
            from_object_id: hurricane,
            spell_object_id: next + 1,
            controller: P0,
            targets: vec![],
            targets_second: vec![],
            x: 3,
        },
    ];
    replace_stack(&mut game, &specs).expect("single target and chosen X are represented exactly");
    assert_eq!(inspect(&game).stack[0].targets, vec![Target::Player(P1)]);
    assert_eq!(game.next_stack_entry_id.unwrap().get(), 25);
}

#[test]
fn debug_stack_mutation_negative_matrix_rejects_bad_sources_targets_timing_and_exhaustion() {
    let mut game = Game::with_players(2, 7);
    let next = game.next_object_id();
    let missing = known_spell_spec(1, 999, next);
    assert_eq!(
        push_stack(&mut game, &missing).unwrap_err().reason,
        ErrorReason::UnknownEntity.into()
    );

    let permanent = game.spawn_on_battlefield(P0, card("Grizzly Bears"));
    let wrong_kind = known_spell_spec(1, permanent, game.next_object_id());
    assert_eq!(
        push_stack(&mut game, &wrong_kind).unwrap_err().reason,
        ErrorReason::WrongObjectKind.into()
    );

    let ritual = game.spawn_in_hand(P0, card("Dark Ritual"));
    let invalid_target = DebugStackEntrySpec::KnownSpell {
        entry_id: StackEntryId(1),
        from_object_id: ritual,
        spell_object_id: game.next_object_id(),
        controller: P0,
        targets: vec![Target::Player(P1)],
        targets_second: vec![],
        x: 0,
    };
    assert_eq!(
        push_stack(&mut game, &invalid_target).unwrap_err().reason,
        ErrorReason::InvalidValue.into()
    );

    let source = game.spawn_on_battlefield(P0, card("Bonesplitter"));
    let static_ability = DebugStackEntrySpec::AuthoredAbility {
        entry_id: StackEntryId(1),
        controller: P0,
        source_object_id: source,
        ability_index: 0,
        target: None,
        targets_second: vec![],
        x: 0,
    };
    assert_eq!(
        push_stack(&mut game, &static_ability).unwrap_err().reason,
        ErrorReason::WrongObjectKind.into()
    );

    let ghost = DebugStackEntrySpec::PublicGhost {
        entry_id: StackEntryId(u64::MAX),
        controller: P0,
        public: valid_public_ghost(),
    };
    push_stack(&mut game, &ghost).expect("the maximum identity is issued once");
    assert!(game.next_stack_entry_id.is_none());
    let another = DebugStackEntrySpec::PublicGhost {
        entry_id: StackEntryId(u64::MAX),
        controller: P0,
        public: valid_public_ghost(),
    };
    assert_eq!(
        push_stack(&mut game, &another).unwrap_err().reason,
        ErrorReason::DuplicateId.into()
    );
}
