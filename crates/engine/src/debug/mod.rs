//! Debug-build-only authoritative inspection and structural safety checks.
//!
//! This module deliberately depends only on engine domain types. It is the private-state seam
//! consumed by the debug server; protobuf and schema projections do not belong here.

use std::collections::{HashMap, HashSet};

use crate::{
    CounterKind, EffectMessage, Game, Object, ObjectId, PlayerCounterKind, PlayerId, StackItem,
    Step, Target, Zone, card_def,
};

const MAX_VIOLATIONS: usize = 16;
const MAX_VIOLATION_MESSAGE_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    pub players: Vec<PlayerInspection>,
    pub objects: Vec<ObjectInspection>,
    pub stack: Vec<StackInspection>,
    pub active_player: PlayerId,
    pub step: Step,
    pub priority_player: PlayerId,
    pub consecutive_passes: u8,
    pub has_pending_choice: bool,
    pub has_deferred_resume: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerInspection {
    pub player_id: PlayerId,
    pub life: i32,
    pub poison: u8,
    pub rad: u8,
    pub library: Vec<ObjectId>,
    pub hand: Vec<ObjectId>,
    pub graveyard: Vec<ObjectId>,
    pub exile: Vec<ObjectId>,
    pub command: Vec<ObjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectInspection {
    Card {
        object_id: ObjectId,
        card_id: String,
        owner: PlayerId,
        zone: Zone,
        commander: bool,
        face_down: bool,
    },
    Permanent {
        object_id: ObjectId,
        card_id: String,
        owner: PlayerId,
        controller: PlayerId,
        tapped: bool,
        marked_damage: i32,
        plus_one_counters: i32,
        attached_to: Option<ObjectId>,
        commander: bool,
        token: bool,
        face_down: bool,
    },
    Spell {
        object_id: ObjectId,
        card_id: String,
        controller: PlayerId,
    },
    Moved {
        object_id: ObjectId,
        to_object_id: ObjectId,
    },
    Removed {
        object_id: ObjectId,
        card_id: String,
        owner: PlayerId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackInspection {
    pub position_from_bottom: usize,
    pub kind: &'static str,
    pub source_object_id: Option<ObjectId>,
    /// The authoritative controller, or `None` when corrupt stack state has no matching object.
    pub controller: Option<PlayerId>,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mutation {
    SetLife {
        player: PlayerId,
        life: i32,
    },
    SetPlayerCounter {
        player: PlayerId,
        counter: PlayerCounterKind,
        value: u8,
    },
    SetTurnState {
        active_player: PlayerId,
        step: Step,
        priority_player: PlayerId,
        consecutive_passes: u8,
    },
    SetPermanentState {
        object_id: ObjectId,
        tapped: Option<bool>,
        marked_damage: Option<i32>,
        plus_one_counters: Option<i32>,
    },
    SetController {
        object_id: ObjectId,
        controller: PlayerId,
    },
    SetAttachment {
        object_id: ObjectId,
        attached_to: Option<ObjectId>,
    },
    CreateCard {
        object_id: ObjectId,
        card_id: String,
        owner: PlayerId,
        controller: PlayerId,
        destination: DebugZone,
        commander: bool,
        face_down: bool,
    },
    MoveCard {
        object_id: ObjectId,
        new_object_id: ObjectId,
        destination: DebugZone,
        controller: PlayerId,
        face_down: bool,
    },
    SetLibraryOrder {
        player: PlayerId,
        object_ids: Vec<ObjectId>,
    },
    RemoveCard {
        object_id: ObjectId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugZone {
    Library,
    Hand,
    Battlefield,
    Graveyard,
    Exile,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorReason {
    EmptyBatch,
    UnknownEntity,
    DuplicateId,
    InvalidValue,
    WrongObjectKind,
    ZoneDisagreement,
    ReferencedObject,
    AttachmentCycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditError {
    pub operation_index: Option<usize>,
    pub reason: ErrorReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub code: &'static str,
    pub message: String,
}

pub fn inspect(game: &Game) -> Inspection {
    let mut players = Vec::with_capacity(game.players.len());
    for (index, player) in game.players.iter().enumerate() {
        let player_id = PlayerId(index as u8);
        let mut hand = zone_cards(game, player_id, Zone::Hand);
        let mut graveyard = zone_cards(game, player_id, Zone::Graveyard);
        let mut exile = zone_cards(game, player_id, Zone::Exile);
        let mut command = zone_cards(game, player_id, Zone::Command);
        hand.sort_unstable();
        graveyard.sort_unstable();
        exile.sort_unstable();
        command.sort_unstable();
        players.push(PlayerInspection {
            player_id,
            life: player.life,
            poison: player.kind_counters[PlayerCounterKind::Poison as usize],
            rad: player.kind_counters[PlayerCounterKind::Rad as usize],
            library: player.library.clone(),
            hand,
            graveyard,
            exile,
            command,
        });
    }

    let objects = game
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| inspect_object(game, index as ObjectId, object))
        .collect();
    let stack = game
        .stack
        .iter()
        .enumerate()
        .map(|(position_from_bottom, item)| inspect_stack_item(game, position_from_bottom, item))
        .collect();

    Inspection {
        players,
        objects,
        stack,
        active_player: game.active_player,
        step: game.step,
        priority_player: game.priority,
        consecutive_passes: game.consecutive_passes,
        has_pending_choice: game.pending_choice.is_some(),
        has_deferred_resume: game.resume.clash_scry.is_some()
            || game.resume.sequence.is_some()
            || game.resume.demonstrate_opponent_copy.is_some()
            || game.resume.spell_finish.is_some()
            || game.resume.draw_batch.is_some(),
    }
}

fn zone_cards(game: &Game, owner: PlayerId, zone: Zone) -> Vec<ObjectId> {
    game.objects
        .iter()
        .enumerate()
        .filter_map(|(index, object)| match object {
            Object::Card(card) if card.owner == owner && card.zone == zone => {
                Some(index as ObjectId)
            }
            _ => None,
        })
        .collect()
}

fn inspect_object(game: &Game, object_id: ObjectId, object: &Object) -> ObjectInspection {
    match object {
        Object::Card(card) => ObjectInspection::Card {
            object_id,
            card_id: card_def(card.def).id.to_string(),
            owner: card.owner,
            zone: card.zone,
            commander: card.commander,
            face_down: card.face_down,
        },
        Object::Permanent(permanent) => ObjectInspection::Permanent {
            object_id,
            card_id: card_def(permanent.def).id.to_string(),
            owner: permanent.owner,
            controller: game.controller_of(object_id),
            tapped: permanent.tapped,
            marked_damage: permanent.marked_damage,
            plus_one_counters: permanent.plus_counters,
            attached_to: permanent.attached_to,
            commander: permanent.commander,
            token: permanent.token,
            face_down: permanent.face_down,
        },
        Object::Spell(spell) => ObjectInspection::Spell {
            object_id,
            card_id: card_def(spell.def).id.to_string(),
            controller: spell.controller,
        },
        Object::Moved { to } => ObjectInspection::Moved {
            object_id,
            to_object_id: *to,
        },
        Object::Removed { def, owner } => ObjectInspection::Removed {
            object_id,
            card_id: card_def(*def).id.to_string(),
            owner: *owner,
        },
    }
}

fn inspect_stack_item(
    game: &Game,
    position_from_bottom: usize,
    item: &StackItem,
) -> StackInspection {
    match item {
        StackItem::Spell(object_id) => StackInspection {
            position_from_bottom,
            kind: "spell",
            source_object_id: Some(*object_id),
            controller: match game.objects.get(*object_id as usize) {
                Some(Object::Spell(spell)) => Some(spell.controller),
                _ => None,
            },
            label: match game.objects.get(*object_id as usize) {
                Some(Object::Spell(spell)) => card_def(spell.def).name.to_string(),
                _ => format!("spell object {object_id}"),
            },
        },
        StackItem::Ability {
            controller,
            source,
            effect,
            ..
        } => StackInspection {
            position_from_bottom,
            kind: "ability",
            source_object_id: Some(*source),
            controller: Some(*controller),
            label: effect.clone().message().key.as_str().to_string(),
        },
    }
}

/// Applies raw debug edits in request order, then rebuilds derived state once for the batch.
pub fn apply_operations(game: &mut Game, operations: &[Mutation]) -> Result<(), EditError> {
    if operations.is_empty() {
        return Err(EditError {
            operation_index: None,
            reason: ErrorReason::EmptyBatch,
        });
    }

    for (operation_index, operation) in operations.iter().enumerate() {
        apply_operation(game, operation).map_err(|reason| EditError {
            operation_index: Some(operation_index),
            reason,
        })?;
    }

    game.characteristics_cache = crate::characteristics_cache::CharacteristicsCacheCell::default();
    game.refresh_actions();
    validate_structural(game).map_err(|_| EditError {
        operation_index: None,
        reason: ErrorReason::InvalidValue,
    })
}

fn apply_operation(game: &mut Game, operation: &Mutation) -> Result<(), ErrorReason> {
    match operation {
        Mutation::SetLife { player, life } => {
            let Some(state) = game.players.get_mut(player.0 as usize) else {
                return Err(ErrorReason::UnknownEntity);
            };
            state.life = *life;
        }
        Mutation::SetPlayerCounter {
            player,
            counter,
            value,
        } => {
            let Some(state) = game.players.get_mut(player.0 as usize) else {
                return Err(ErrorReason::UnknownEntity);
            };
            state.kind_counters[*counter as usize] = *value;
        }
        Mutation::SetTurnState {
            active_player,
            step,
            priority_player,
            consecutive_passes,
        } => {
            let Some(active) = game.players.get(active_player.0 as usize) else {
                return Err(ErrorReason::UnknownEntity);
            };
            let Some(priority) = game.players.get(priority_player.0 as usize) else {
                return Err(ErrorReason::UnknownEntity);
            };
            if active.lost || priority.lost {
                return Err(ErrorReason::InvalidValue);
            }
            let living_players = game.living_player_count();
            if living_players == 0 || *consecutive_passes >= living_players {
                return Err(ErrorReason::InvalidValue);
            }
            game.active_player = *active_player;
            game.step = *step;
            game.priority = *priority_player;
            game.consecutive_passes = *consecutive_passes;
        }
        Mutation::SetPermanentState {
            object_id,
            tapped,
            marked_damage,
            plus_one_counters,
        } => {
            if marked_damage.is_some_and(|value| value < 0)
                || plus_one_counters.is_some_and(|value| value < 0)
            {
                return Err(ErrorReason::InvalidValue);
            }
            let Some(object) = game.objects.get(*object_id as usize) else {
                return Err(ErrorReason::UnknownEntity);
            };
            if !matches!(object, Object::Permanent(_)) {
                return Err(ErrorReason::WrongObjectKind);
            }
            {
                let permanent = game.permanent_mut(*object_id);
                if let Some(value) = tapped {
                    permanent.tapped = *value;
                }
                if let Some(value) = marked_damage {
                    permanent.marked_damage = *value;
                }
            }
            if let Some(value) = plus_one_counters {
                game.set_plus_counter_aggregate(*object_id, *value);
            }
        }
        Mutation::SetController {
            object_id,
            controller,
        } => set_controller(game, *object_id, *controller)?,
        Mutation::SetAttachment {
            object_id,
            attached_to,
        } => set_attachment(game, *object_id, *attached_to)?,
        Mutation::CreateCard { .. }
        | Mutation::MoveCard { .. }
        | Mutation::SetLibraryOrder { .. }
        | Mutation::RemoveCard { .. } => return Err(ErrorReason::InvalidValue),
    }
    Ok(())
}

fn set_controller(
    game: &mut Game,
    object_id: ObjectId,
    controller: PlayerId,
) -> Result<(), ErrorReason> {
    if game.players.get(controller.0 as usize).is_none() {
        return Err(ErrorReason::UnknownEntity);
    }
    let Some(object) = game.objects.get(object_id as usize) else {
        return Err(ErrorReason::UnknownEntity);
    };
    if !matches!(object, Object::Permanent(_)) {
        return Err(ErrorReason::WrongObjectKind);
    }

    game.play_permissions
        .control_overrides
        .retain(|&(object, ..)| object != object_id);
    game.play_permissions
        .permanent_control_overrides
        .retain(|&(object, ..)| object != object_id);
    game.play_permissions
        .conditioned_control_overrides
        .retain(|&(object, ..)| object != object_id);
    let timestamp = game.stamp_control_timestamp();
    game.play_permissions
        .permanent_control_overrides
        .push((object_id, controller, timestamp));

    game.combat.attackers.retain(|&object| object != object_id);
    game.combat
        .attack_targets
        .retain(|&(attacker, _)| attacker != object_id);
    game.combat
        .blocks
        .retain(|&(blocker, attacker)| blocker != object_id && attacker != object_id);
    game.combat
        .blocked_ever
        .retain(|&(_, attacker)| attacker != object_id);
    Ok(())
}

fn set_attachment(
    game: &mut Game,
    object_id: ObjectId,
    attached_to: Option<ObjectId>,
) -> Result<(), ErrorReason> {
    let Some(object) = game.objects.get(object_id as usize) else {
        return Err(ErrorReason::UnknownEntity);
    };
    if !matches!(object, Object::Permanent(_)) {
        return Err(ErrorReason::WrongObjectKind);
    }
    let Some(host) = attached_to else {
        let Object::Permanent(permanent) = &mut game.objects[object_id as usize] else {
            unreachable!("kind checked above");
        };
        permanent.attached_to = None;
        return Ok(());
    };
    let Some(host_object) = game.objects.get(host as usize) else {
        return Err(ErrorReason::UnknownEntity);
    };
    if !matches!(host_object, Object::Permanent(_)) {
        return Err(ErrorReason::WrongObjectKind);
    }
    if attachment_would_cycle(game, object_id, host) {
        return Err(ErrorReason::AttachmentCycle);
    }
    let Object::Permanent(permanent) = &mut game.objects[object_id as usize] else {
        unreachable!("kind checked above");
    };
    let previous_host = permanent.attached_to.replace(host);
    game.characteristics_cache = crate::characteristics_cache::CharacteristicsCacheCell::default();

    let effective_subtypes = game.effective_subtypes(object_id);
    let valid_result = (effective_subtypes.contains(&"Aura")
        || effective_subtypes.contains(&"Equipment"))
        && game.attachment_host_legal(object_id, host);
    if valid_result {
        return Ok(());
    }

    let Object::Permanent(permanent) = &mut game.objects[object_id as usize] else {
        unreachable!("kind checked above");
    };
    permanent.attached_to = previous_host;
    // The prospective relationship may have populated derived reads. Discard them so a rejected
    // local operation leaves its candidate coherent for the transaction layer to inspect/drop.
    game.characteristics_cache = crate::characteristics_cache::CharacteristicsCacheCell::default();
    Err(ErrorReason::InvalidValue)
}

fn attachment_would_cycle(game: &Game, object_id: ObjectId, mut host: ObjectId) -> bool {
    let mut visited = HashSet::new();
    loop {
        if host == object_id || !visited.insert(host) {
            return true;
        }
        let Some(Object::Permanent(permanent)) = game.objects.get(host as usize) else {
            return false;
        };
        let Some(next) = permanent.attached_to else {
            return false;
        };
        host = next;
    }
}

pub fn validate_structural(game: &Game) -> Result<(), Vec<Violation>> {
    let mut violations = ViolationCollector::default();
    validate_players(game, &mut violations);
    validate_libraries(game, &mut violations);
    validate_owners_and_scalars(game, &mut violations);
    validate_stack(game, &mut violations);
    validate_attachments(game, &mut violations);
    validate_references(game, &mut violations);

    if violations.items.is_empty() {
        return Ok(());
    }
    Err(violations.items)
}

fn validate_players(game: &Game, violations: &mut ViolationCollector) {
    let player_count = game.players.len();
    if game.active_player.0 as usize >= player_count {
        violations.push(
            "player_range",
            format!(
                "active player {} is outside 0..{player_count}",
                game.active_player.0
            ),
        );
    }
    if game.priority.0 as usize >= player_count {
        violations.push(
            "player_range",
            format!(
                "priority player {} is outside 0..{player_count}",
                game.priority.0
            ),
        );
    }
    let living_player_count = game.living_player_count() as usize;
    // A stable priority state has fewer passes than living seats. Once nobody is living there is
    // no priority round to complete, so only the neutral terminal value zero is safe to inspect.
    let stable_pass_bound = if living_player_count == 0 {
        1
    } else {
        living_player_count
    };
    if game.consecutive_passes as usize >= stable_pass_bound {
        violations.push(
            "consecutive_passes",
            format!(
                "consecutive passes {} is outside 0..{stable_pass_bound} for {living_player_count} living players",
                game.consecutive_passes,
            ),
        );
    }
    for (player, site) in all_player_references(game) {
        if player.0 as usize >= player_count {
            violations.push(
                "player_range",
                format!(
                    "{site} references player {} outside 0..{player_count}",
                    player.0
                ),
            );
        }
    }
}

fn push_target_player(
    target: Target,
    site: &'static str,
    players: &mut Vec<(PlayerId, &'static str)>,
) {
    if let Target::Player(player) = target {
        players.push((player, site));
    }
}

// Destructure every authoritative player field so adding a field forces this visitor to decide
// whether it carries a PlayerId. Object-id carriers are validated by `all_references`.
fn push_player_references(player: &crate::Player, players: &mut Vec<(PlayerId, &'static str)>) {
    let crate::Player {
        life: _,
        mana_pool: _,
        library: _,
        attempted_empty_draw: _,
        mulligans_taken: _,
        hand_kept: _,
        lands_played: _,
        life_gained_this_turn: _,
        spells_cast_this_turn: _,
        damage_taken_this_turn: _,
        untapped_lands_at_turn_start: _,
        x_spells_cast_this_turn: _,
        draws_this_turn: _,
        draws_this_draw_step: _,
        life_losses_this_turn: _,
        creatures_died_this_turn: _,
        modified_creature_died_this_turn: _,
        nontoken_creatures_entered_this_turn: _,
        land_entered_under_your_control_this_turn: _,
        card_left_graveyard_this_turn: _,
        instant_or_sorcery_cast_this_turn: _,
        sorcery_cast_this_turn: _,
        greatest_instant_or_sorcery_mana_value_cast_this_turn: _,
        instants_and_sorceries_cast_this_turn: _,
        instant_spells_cast_this_turn: _,
        flash_permission_this_turn: _,
        channel_colorless_mana_this_turn: _,
        spend_mana_as_any_type_this_turn: _,
        graveyard_play_used_this_turn: _,
        attacked_this_turn: _,
        nontoken_permanent_entered_this_turn: _,
        acted_on_last_own_turn: _,
        op_iteration: _,
        command_casts: _,
        commander_damage,
        kind_counters: _,
        lost: _,
        has_citys_blessing: _,
        war_choices: _,
        mana_provenance: _,
        persistent_mana: _,
    } = player;
    players.extend(
        commander_damage
            .iter()
            .map(|&(commander_owner, _)| (commander_owner, "commander damage")),
    );
}

fn all_player_references(game: &Game) -> Vec<(PlayerId, &'static str)> {
    let mut players = Vec::new();

    for player in &game.players {
        push_player_references(player, &mut players);
    }

    for object in &game.objects {
        match object {
            Object::Card(card) => players.push((card.owner, "card owner")),
            Object::Permanent(permanent) => {
                players.push((permanent.owner, "permanent owner"));
                if let Some(player) = permanent.chosen_opponent {
                    players.push((player, "chosen opponent"));
                }
                if let Some(player) = permanent.vow_protected {
                    players.push((player, "vow protection"));
                }
            }
            Object::Spell(spell) => {
                players.push((spell.controller, "spell controller"));
                for target in spell.targets.iter().chain(spell.targets_second.iter()) {
                    push_target_player(target, "spell target", &mut players);
                }
                for (_, target) in spell.modes.chosen() {
                    if let Some(target) = target {
                        push_target_player(target, "spell mode target", &mut players);
                    }
                }
                for (player, _) in spell.damage_division_players.iter().flatten() {
                    players.push((*player, "spell damage target"));
                }
            }
            Object::Removed { owner, .. } => players.push((*owner, "removed owner")),
            Object::Moved { .. } => {}
        }
    }

    for item in &game.stack {
        match item {
            StackItem::Spell(_) => {}
            StackItem::Ability {
                controller,
                target,
                targets_second,
                ..
            } => {
                players.push((*controller, "stack controller"));
                if let Some(target) = target {
                    push_target_player(*target, "stack target", &mut players);
                }
                for target in targets_second.iter() {
                    push_target_player(target, "stack target", &mut players);
                }
            }
        }
    }

    if let Some(choice) = &game.pending_choice {
        push_pending_choice_player_references(choice, &mut players);
    }

    players.extend(
        game.extra_turns
            .iter()
            .copied()
            .map(|player| (player, "extra turn")),
    );
    for &(_, defender) in &game.combat.attack_targets {
        if let crate::Defender::Player(player) = defender {
            players.push((player, "combat defender"));
        }
    }
    players.extend(
        game.combat
            .blocked_by
            .iter()
            .copied()
            .map(|player| (player, "combat blocks")),
    );
    players.extend(
        game.combat
            .attacks_dont_tap
            .iter()
            .map(|&(player, _)| (player, "combat permission")),
    );

    for &(_, player, _) in &game.combat_extras.goaded {
        players.push((player, "goad"));
    }
    for &(_, player) in &game.combat_extras.must_attack {
        players.push((player, "must attack"));
    }
    for &(player, _) in &game.combat_extras.combat_damage_prevention_shields {
        players.push((player, "combat prevention"));
    }
    for player in [
        game.combat_extras.attack_declarer,
        game.combat_extras.block_declarer,
    ]
    .into_iter()
    .flatten()
    {
        players.push((player, "combat declarer"));
    }
    for &(player, _) in &game.combat_extras.repelled_until_next_turn {
        players.push((player, "combat restriction"));
    }
    for &(_, _, player) in &game.combat_extras.blocked_this_turn {
        players.push((player, "combat history"));
    }

    for &(_, player, _, _) in &game.play_permissions.control_overrides {
        players.push((player, "control override"));
    }
    for &(_, player, _) in &game.play_permissions.permanent_control_overrides {
        players.push((player, "control override"));
    }
    for &(_, player, _, _) in &game.play_permissions.conditioned_control_overrides {
        players.push((player, "control override"));
    }
    for &(_, player, _) in &game.play_permissions.play_from_exile {
        players.push((player, "play permission"));
    }
    for &(_, player, _) in &game.play_permissions.play_from_exile_free_while_source {
        players.push((player, "play permission"));
    }
    for &(_, player) in &game.play_permissions.cast_from_exile_free {
        players.push((player, "play permission"));
    }
    if let Some((_, player)) = game.play_permissions.compelled_play {
        players.push((player, "compelled play"));
    }
    for &(_, player) in &game.play_permissions.on_adventure {
        players.push((player, "adventure"));
    }

    for &(_, target, _) in &game.damage_dealt_this_turn {
        push_target_player(target, "damage history", &mut players);
    }
    for &(player, _) in &game.hand_cards_seen {
        players.push((player, "hand visibility"));
    }
    for &(_, player) in &game.revealed_unplayable_until_next_turn {
        players.push((player, "revealed hand"));
    }
    for &(player, _) in &game.batch_trigger_scratch.graveyard_exits_this_batch {
        players.push((player, "batch history"));
    }
    players.extend(
        game.batch_trigger_scratch
            .library_or_graveyard_exits_this_batch
            .iter()
            .copied()
            .map(|p| (p, "batch history")),
    );
    players.extend(
        game.batch_trigger_scratch
            .creature_tokens_created_this_batch
            .iter()
            .copied()
            .map(|p| (p, "batch history")),
    );
    players.extend(
        game.batch_trigger_scratch
            .creatures_dealt_combat_damage_this_batch
            .iter()
            .copied()
            .map(|p| (p, "batch history")),
    );
    for &(player, _) in &game.batch_trigger_scratch.discards_this_batch {
        players.push((player, "batch history"));
    }
    for &(_, _, player, _) in &game.batch_trigger_scratch.dying_creature_attachments {
        players.push((player, "batch history"));
    }
    for stats in &game.batch_trigger_scratch.dying_creature_stats {
        players.push((stats.controller, "batch history"));
    }
    for &(_, _, player) in &game.batch_trigger_scratch.dying_creature_lki {
        players.push((player, "batch history"));
    }

    for group in &game.pending_trigger_groups {
        players.push((group.controller, "pending trigger"));
    }
    for action in &game.actions {
        players.push((action.player, "legal action"));
    }
    for rows in [&game.delayed_triggers.scheduled] {
        for &(player, ..) in rows {
            players.push((player, "delayed trigger"));
        }
    }
    for &(player, ..) in &game.delayed_triggers.scheduled_your_upkeep {
        players.push((player, "delayed trigger"));
    }
    for &(player, ..) in &game.delayed_triggers.pending_next_cast {
        players.push((player, "delayed trigger"));
    }
    for &(player, ..) in &game.delayed_triggers.pending_combat_damage_watch {
        players.push((player, "delayed trigger"));
    }
    for &(player, ..) in &game.delayed_triggers.pending_dies_this_turn {
        players.push((player, "delayed trigger"));
    }
    for &(player, ..) in &game.delayed_triggers.pending_combat_damage_copy {
        players.push((player, "delayed trigger"));
    }
    for &(player, ..) in &game.delayed_triggers.pending_attacker_damage_life {
        players.push((player, "delayed trigger"));
    }

    for shield in &game.damage_prevention_shields {
        push_target_player(shield.target, "prevention target", &mut players);
        if let Some(target) = shield.redirect_to {
            push_target_player(target, "prevention redirect", &mut players);
        }
    }
    for offer in &game.standing_preventions {
        push_target_player(offer.target, "standing prevention", &mut players);
        players.push((offer.player, "standing prevention controller"));
    }
    for target in &game.resolution_frame.resolving_targets {
        push_target_player(*target, "resolution target", &mut players);
    }
    if let Some((_, player)) = game.resolution_frame.vanished_permanent_owner {
        players.push((player, "resolution history"));
    }
    for entry in &game.resolution_frame.destroyed_this_way {
        players.push((entry.controller, "resolution history"));
    }
    for entry in &game.resolution_frame.power_exiled_this_way {
        players.push((entry.controller, "resolution history"));
    }
    if let Some(player) = game.resolution_frame.discard_cause {
        players.push((player, "resolution discard"));
    }
    if let Some(fanout) = &game.resolution_frame.search_fanout {
        players.extend(
            fanout
                .remaining
                .iter()
                .copied()
                .map(|p| (p, "resolution search")),
        );
    }
    for modifier in &game.modifier_provenance.modifiers {
        match modifier.duration {
            crate::ModifierDuration::EndOfNextUpkeep { player, .. }
            | crate::ModifierDuration::UntilNextUpkeep { player } => {
                players.push((player, "modifier duration"))
            }
            crate::ModifierDuration::EndOfTurn
            | crate::ModifierDuration::EndOfCombat
            | crate::ModifierDuration::Indefinite => {}
        }
    }

    if let Some(player) = game.resume.clash_scry {
        players.push((player, "resume clash"));
    }
    if let Some((player, _)) = game.resume.demonstrate_opponent_copy {
        players.push((player, "resume copy"));
    }
    if let Some(sequence) = &game.resume.sequence {
        players.push((sequence.ctx.controller, "resume controller"));
        if let Some(target) = sequence.ctx.target {
            push_target_player(target, "resume target", &mut players);
        }
        for target in sequence.ctx.targets_second.iter() {
            push_target_player(target, "resume target", &mut players);
        }
    }
    if let Some(batch) = &game.resume.draw_batch {
        players.extend(
            batch
                .seats
                .iter()
                .map(|&(player, _)| (player, "resume draw")),
        );
        match batch.after {
            crate::resolution::DrawAfter::TradeSecretsCaster {
                caster, opponent, ..
            }
            | crate::resolution::DrawAfter::TradeSecretsRepeat {
                caster, opponent, ..
            } => {
                players.push((caster, "resume draw"));
                players.push((opponent, "resume draw"));
            }
            crate::resolution::DrawAfter::Nothing | crate::resolution::DrawAfter::DrawStep => {}
        }
    }
    players
}

fn push_pending_choice_player_references(
    choice: &crate::PendingChoice,
    players: &mut Vec<(PlayerId, &'static str)>,
) {
    players.push((choice.player(), "pending chooser"));
    use crate::PendingChoice as P;
    match choice {
        P::ChooseTarget {
            controller,
            legal,
            target,
            ..
        } => {
            players.push((*controller, "pending controller"));
            for target in legal {
                push_target_player(*target, "pending target", players);
            }
            if let Some(target) = target {
                push_target_player(*target, "pending target", players);
            }
        }
        P::MayYesNo {
            resume: crate::MayYesNoResume::TradeSecretsRepeat { caster, .. },
            ..
        } => players.push((*caster, "pending resume")),
        P::MayYesNo {
            resume:
                crate::MayYesNoResume::Default
                | crate::MayYesNoResume::ResolveInline
                | crate::MayYesNoResume::SkipDrawStepDraw
                | crate::MayYesNoResume::SkipTurnWhileSourceTapped,
            ..
        } => {}
        P::MayDrawUpTo {
            resume: crate::MayDrawUpToResume::TradeSecretsRepeat { opponent, .. },
            ..
        } => players.push((*opponent, "pending resume")),
        P::MayDrawUpTo {
            resume: crate::MayDrawUpToResume::Default,
            ..
        } => {}
        P::PayOrControllerDraws { controller, .. } => {
            players.push((*controller, "pending controller"))
        }
        P::ArrangeTop { library, .. } | P::ShuffleFromGraveyard { owner: library, .. } => {
            players.push((*library, "pending library owner"))
        }
        P::Proliferate { options, .. } => {
            for option in options {
                if let crate::ProliferateTarget::Player(player) = option {
                    players.push((*player, "pending option"));
                }
            }
        }
        P::SacrificeEdict {
            remaining,
            controller,
            ..
        } => {
            players.extend(
                remaining
                    .iter()
                    .copied()
                    .map(|p| (p, "pending continuation")),
            );
            players.push((*controller, "pending controller"));
        }
        P::ChooseTargetPlayers { legal, .. } | P::ChooseSplittingOpponent { legal, .. } => {
            players.extend(legal.iter().copied().map(|p| (p, "pending option")))
        }
        P::ExileFromGraveyard { remaining, .. }
        | P::DiscardEdict { remaining, .. }
        | P::JoinForcesPayment { remaining, .. }
        | P::CastVote { remaining, .. } => players.extend(
            remaining
                .iter()
                .copied()
                .map(|p| (p, "pending continuation")),
        ),
        P::CasterKeepPermanents {
            target_player,
            remaining,
            ..
        }
        | P::ChooseCounterTargetForPlayer {
            target_player,
            remaining,
            ..
        } => {
            players.push((*target_player, "pending subject"));
            players.extend(
                remaining
                    .iter()
                    .copied()
                    .map(|p| (p, "pending continuation")),
            );
        }
        P::MayReturnFromGraveyard {
            then_graveyards, ..
        } => players.extend(
            then_graveyards
                .iter()
                .copied()
                .map(|p| (p, "pending continuation")),
        ),
        P::PutCreatureFromHand {
            defender, round, ..
        } => {
            if let Some(p) = defender {
                players.push((*p, "pending defender"));
            }
            if let Some(round) = round {
                players.extend(round.iter().copied().map(|p| (p, "pending continuation")));
            }
        }
        P::ChooseCardInHandToPlay { subject, .. } => players.push((*subject, "pending subject")),
        P::SplitBlockersIntoPiles {
            left, defenders, ..
        } => {
            players.extend(left.iter().map(|(p, _)| (*p, "pending pile")));
            players.extend(defenders.iter().copied().map(|p| (p, "pending defender")));
        }
        P::DivideBlockersIntoPiles { defenders, .. } => {
            players.extend(defenders.iter().copied().map(|p| (p, "pending defender")))
        }
        P::ChoosePileForAttacker { left, .. } => {
            players.extend(left.iter().map(|(p, _)| (*p, "pending pile")))
        }
        P::OpponentChoosesPile { controller, .. }
        | P::OpponentChoosesExiledNonland { controller, .. }
        | P::PartitionRevealed { controller, .. }
        | P::OpponentChoosesRevealedToGraveyard { controller, .. } => {
            players.push((*controller, "pending controller"))
        }
        P::ChooseCardName {
            remaining, use_, ..
        } => {
            players.extend(
                remaining
                    .iter()
                    .copied()
                    .map(|p| (p, "pending continuation")),
            );
            if let crate::CardNameUse::SubjectRevealsHandAtRandomThenDiscards { subject, .. } = use_
            {
                players.push((*subject, "pending subject"));
            }
        }
        P::DivideSpellDamage { targets, .. } => {
            for target in targets {
                push_target_player(*target, "pending target", players);
            }
        }
        P::ChooseActivationCostTargets { target, legal, .. } => {
            if let Some(target) = target {
                push_target_player(*target, "pending target", players);
            }
            for target in legal {
                push_target_player(*target, "pending target", players);
            }
        }
        P::ChooseMode { target, .. } => {
            if let Some(target) = target {
                push_target_player(*target, "pending target", players);
            }
        }
        // Every choice's primary chooser is visited above through the typed `player()` API.
        // These variants carry no additional player-valued fields; spelling them out keeps new
        // variants compiler-auditable instead of silently falling through a wildcard.
        P::OrderTriggers { .. }
        | P::MayRevealLandFromHand { .. }
        | P::DeclineUntap { .. }
        | P::ChooseDredge { .. }
        | P::PayCost { .. }
        | P::PayOrCounter { .. }
        | P::ChooseCounteredSpellDestination { .. }
        | P::PayEchoOrSacrifice { .. }
        | P::PayCumulativeUpkeepOrSacrifice { .. }
        | P::PayRecoverOrExile { .. }
        | P::PayOrElse { .. }
        | P::PayLifeOrEntersTapped { .. }
        | P::SacrificeUnlessReturnLand { .. }
        | P::AssignCombatDamage { .. }
        | P::DivideCounters { .. }
        | P::DivideMovedCounters { .. }
        | P::SelectFromTop { .. }
        | P::DanceExileMore { .. }
        | P::DistributeTop { .. }
        | P::PhaseOut { .. }
        | P::SearchLibrary { .. }
        | P::ChooseTriggerModes { .. }
        | P::MaySacrifice { .. }
        | P::MayExileDiscardedToPlay { .. }
        | P::MayDiscard { .. }
        | P::MayPutCounterOnCreature { .. }
        | P::ChooseBlockTarget { .. }
        | P::DiscardToHandSize { .. }
        | P::DiscardCards { .. }
        | P::PutFromHandOnTop { .. }
        | P::PutLandFromHand { .. }
        | P::CastCreatureFaceDown { .. }
        | P::ChooseExiledWithCard { .. }
        | P::ChooseExiledWithCardToCast { .. }
        | P::ChooseExiledDigToCastFree { .. }
        | P::ChoosePileForHand { .. }
        | P::ChooseExiledToCastFree { .. }
        | P::RevealedCardToBattlefieldOrHand { .. }
        | P::ChooseOwnSacrifices { .. }
        | P::SacrificeAnyNumber { .. }
        | P::Devour { .. }
        | P::ChooseManaColor { .. }
        | P::ChooseCreatureType { .. }
        | P::ChooseColor { .. }
        | P::ChooseCopyTarget { .. }
        | P::ChooseTokenToCopy { .. }
        | P::ChooseCopyCardFromList { .. }
        | P::ChooseAttachHost { .. }
        | P::ChooseLegendaryKeep { .. }
        | P::ChooseDamageSource { .. } => {}
    }
}

fn validate_libraries(game: &Game, violations: &mut ViolationCollector) {
    let mut memberships: HashMap<ObjectId, usize> = HashMap::new();
    for (player_index, player) in game.players.iter().enumerate() {
        for &object_id in &player.library {
            *memberships.entry(object_id).or_default() += 1;
            match game.objects.get(object_id as usize) {
                Some(Object::Card(card)) if card.zone == Zone::Library => {
                    if card.owner.0 as usize != player_index {
                        violations.push(
                            "library_membership",
                            format!(
                                "library object {object_id} is listed for player {player_index}"
                            ),
                        );
                    }
                }
                Some(_) => violations.push(
                    "zone_disagreement",
                    format!("library entry {object_id} is not a library card"),
                ),
                None => violations.push(
                    "library_membership",
                    format!("library entry {object_id} has no arena slot"),
                ),
            }
        }
    }
    let mut memberships_in_id_order: Vec<_> = memberships.iter().collect();
    memberships_in_id_order.sort_unstable_by_key(|&(object_id, _)| *object_id);
    for (&object_id, &count) in memberships_in_id_order {
        if count != 1 {
            violations.push(
                "library_membership",
                format!("library object {object_id} occurs {count} times"),
            );
        }
    }
    for (index, object) in game.objects.iter().enumerate() {
        if matches!(object, Object::Card(card) if card.zone == Zone::Library)
            && memberships.get(&(index as ObjectId)).copied().unwrap_or(0) != 1
        {
            violations.push(
                "library_membership",
                format!("library object {index} is not listed exactly once"),
            );
        }
    }
}

fn validate_owners_and_scalars(game: &Game, violations: &mut ViolationCollector) {
    let player_count = game.players.len();
    for (index, object) in game.objects.iter().enumerate() {
        match object {
            Object::Card(card) => {
                if matches!(card.zone, Zone::Battlefield | Zone::Stack) {
                    violations.push(
                        "zone_disagreement",
                        format!("card object {index} has live-object zone {:?}", card.zone),
                    );
                }
                if card.owner.0 as usize >= player_count {
                    violations.push(
                        "owner_range",
                        format!(
                            "card object {index} owner {} is outside 0..{player_count}",
                            card.owner.0
                        ),
                    );
                }
            }
            Object::Permanent(permanent) => {
                if permanent.owner.0 as usize >= player_count {
                    violations.push(
                        "owner_range",
                        format!(
                            "permanent object {index} owner {} is outside 0..{player_count}",
                            permanent.owner.0
                        ),
                    );
                } else {
                    let controller = game.controller_of(index as ObjectId);
                    if controller.0 as usize >= player_count {
                        violations.push(
                            "controller_range",
                            format!("permanent object {index} controller {} is outside 0..{player_count}", controller.0),
                        );
                    }
                }
                if permanent.marked_damage < 0 {
                    violations.push(
                        "negative_damage",
                        format!("permanent object {index} has negative marked damage"),
                    );
                }
                if permanent.plus_counters < 0 {
                    violations.push(
                        "negative_counter",
                        format!("permanent object {index} has negative plus-one counter total"),
                    );
                }
            }
            Object::Spell(spell) => {
                if spell.controller.0 as usize >= player_count {
                    violations.push(
                        "controller_range",
                        format!(
                            "spell object {index} controller {} is outside 0..{player_count}",
                            spell.controller.0
                        ),
                    );
                }
            }
            Object::Removed { owner, .. } => {
                if owner.0 as usize >= player_count {
                    violations.push(
                        "owner_range",
                        format!(
                            "removed object {index} owner {} is outside 0..{player_count}",
                            owner.0
                        ),
                    );
                }
            }
            Object::Moved { to } => {
                if (*to as usize) >= game.objects.len() || *to <= index as ObjectId {
                    violations.push(
                        "moved_lineage",
                        format!("moved object {index} does not point to a later arena slot"),
                    );
                }
            }
        }
    }
    for (index, _) in game.players.iter().enumerate() {
        let player = PlayerId(index as u8);
        for kind in PlayerCounterKind::ALL {
            let _ = game.players[index].kind_counters[kind as usize];
        }
        let _ = player;
    }
    let _ = CounterKind::ALL;
}

fn validate_stack(game: &Game, violations: &mut ViolationCollector) {
    let mut stack_spells = HashSet::new();
    for (position, item) in game.stack.iter().enumerate() {
        match item {
            StackItem::Spell(object_id) => {
                if !matches!(
                    game.objects.get(*object_id as usize),
                    Some(Object::Spell(_))
                ) {
                    violations.push(
                        "stack_pairing",
                        format!("stack position {position} names non-spell object {object_id}"),
                    );
                }
                if !stack_spells.insert(*object_id) {
                    violations.push(
                        "stack_pairing",
                        format!("spell object {object_id} occurs more than once on stack"),
                    );
                }
            }
            StackItem::Ability { controller, .. } => {
                if controller.0 as usize >= game.players.len() {
                    violations.push(
                        "controller_range",
                        format!(
                            "stack position {position} controller {} is out of range",
                            controller.0
                        ),
                    );
                }
            }
        }
    }
    for (index, object) in game.objects.iter().enumerate() {
        if matches!(object, Object::Spell(_)) && !stack_spells.contains(&(index as ObjectId)) {
            violations.push(
                "stack_pairing",
                format!("spell object {index} has no matching stack item"),
            );
        }
    }
}

fn validate_attachments(game: &Game, violations: &mut ViolationCollector) {
    for (index, object) in game.objects.iter().enumerate() {
        let Object::Permanent(permanent) = object else {
            continue;
        };
        let Some(host) = permanent.attached_to else {
            continue;
        };
        let effective_subtypes = game.effective_subtypes(index as ObjectId);
        if !effective_subtypes.contains(&"Aura") && !effective_subtypes.contains(&"Equipment") {
            violations.push(
                "attachment_type",
                format!("attachment object {index} is neither Aura nor Equipment"),
            );
            continue;
        }
        if !matches!(game.objects.get(host as usize), Some(Object::Permanent(_))) {
            violations.push(
                "attachment_target",
                format!("attachment object {index} names non-permanent host {host}"),
            );
            continue;
        }
        if !game.attachment_host_legal(index as ObjectId, host) {
            violations.push(
                "attachment_target",
                format!("attachment object {index} cannot attach to host {host}"),
            );
        }
    }

    let mut colors = vec![0u8; game.objects.len()];
    for index in 0..game.objects.len() {
        if colors[index] == 0 {
            visit_attachment(game, index as ObjectId, &mut colors, violations);
        }
    }
}

fn visit_attachment(
    game: &Game,
    object_id: ObjectId,
    colors: &mut [u8],
    violations: &mut ViolationCollector,
) {
    let Some(color) = colors.get_mut(object_id as usize) else {
        return;
    };
    if *color == 2 {
        return;
    }
    if *color == 1 {
        violations.push(
            "attachment_cycle",
            format!("attachment cycle reaches object {object_id}"),
        );
        return;
    }
    *color = 1;
    if let Some(Object::Permanent(permanent)) = game.objects.get(object_id as usize)
        && let Some(host) = permanent.attached_to
        && (host as usize) < colors.len()
    {
        visit_attachment(game, host, colors, violations);
    }
    colors[object_id as usize] = 2;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReferenceDisposition {
    /// The referencing state must be explicitly resolved before the object can leave its slot.
    Blocking,
    /// Ordered zone membership that a move updates as part of changing zones.
    Membership,
    /// A derived cache discarded and rebuilt after an edit batch.
    Recomputed,
    /// Last-known information or history whose retired object IDs remain meaningful.
    Historical,
    /// CR 400.7 tombstone lineage; moving the current object extends this chain.
    Lineage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReferenceSite {
    kind: &'static str,
    disposition: ReferenceDisposition,
}

fn reference_disposition(kind: &'static str) -> ReferenceDisposition {
    match kind {
        "library" => ReferenceDisposition::Membership,
        "legal_action" => ReferenceDisposition::Recomputed,
        "moved_to" => ReferenceDisposition::Lineage,
        "damage_history" | "draw_history" | "hand_seen" | "block_history" | "batch_scratch"
        | "batch_lki" | "resolution_lki" | "activation_limit" | "trigger_limit"
        | "mana_provenance" | "player_war_choice" => ReferenceDisposition::Historical,
        _ => ReferenceDisposition::Blocking,
    }
}

/// Return every Phase A state location that names `object_id`.
///
/// Keeping this walker in the module beside `Game`'s private fields makes future state additions
/// visible during engine review. Move/remove operations consume this seam in Tasks 4 and 5.
#[allow(dead_code)]
fn references_to(game: &Game, object_id: ObjectId) -> Vec<ReferenceSite> {
    all_references(game)
        .into_iter()
        .filter_map(|(referenced, site)| (referenced == object_id).then_some(site))
        .collect()
}

/// Dependencies that cannot be repaired by ordinary zone membership updates, cache refresh, or
/// CR 400.7 lineage extension. Task 5 consumes this query before moving or removing an object.
#[allow(dead_code)]
fn blocking_references_to(game: &Game, object_id: ObjectId) -> Vec<ReferenceSite> {
    references_to(game, object_id)
        .into_iter()
        .filter(|site| site.disposition == ReferenceDisposition::Blocking)
        .collect()
}

fn all_references(game: &Game) -> Vec<(ObjectId, ReferenceSite)> {
    let mut references = Vec::new();
    let mut push = |object_id, kind| {
        references.push((
            object_id,
            ReferenceSite {
                kind,
                disposition: reference_disposition(kind),
            },
        ));
    };

    for player in &game.players {
        for &object_id in &player.library {
            push(object_id, "library");
        }
    }
    for item in &game.stack {
        match item {
            StackItem::Spell(object_id) => push(*object_id, "stack_spell"),
            StackItem::Ability {
                source,
                target,
                targets_second,
                ..
            } => {
                push(*source, "stack_source");
                if let Some(Target::Object(object_id)) = target {
                    push(*object_id, "stack_target");
                }
                for target in targets_second.iter() {
                    if let Target::Object(object_id) = target {
                        push(object_id, "stack_target");
                    }
                }
            }
        }
    }
    if let Some(choice) = &game.pending_choice {
        push_pending_choice_references(choice, &mut push);
    }
    for (index, object) in game.objects.iter().enumerate() {
        match object {
            Object::Moved { to } => push(*to, "moved_to"),
            Object::Permanent(permanent) => {
                for referenced in [
                    permanent.attached_to,
                    permanent.cast_time_enchant_target,
                    permanent.linked_twin,
                    permanent.enchant_rewrite_host,
                    permanent
                        .subtypes_set_while_source_remains
                        .map(|(_, source, _)| source),
                ]
                .into_iter()
                .flatten()
                {
                    push(referenced, "permanent_state");
                }
            }
            Object::Spell(spell) => {
                for target in spell.targets.iter().chain(spell.targets_second.iter()) {
                    if let Target::Object(referenced) = target {
                        push(referenced, "spell_target");
                    }
                }
                for (_, target) in spell.modes.chosen() {
                    if let Some(Target::Object(referenced)) = target {
                        push(referenced, "spell_mode_target");
                    }
                }
                for (referenced, _) in spell.damage_division.pairs() {
                    push(referenced, "spell_damage");
                }
                for (referenced, _) in spell.counter_division.pairs() {
                    push(referenced, "spell_counter");
                }
            }
            Object::Card(_) | Object::Removed { .. } => {}
        }
        let _ = index;
    }

    for &object_id in &game.combat.attackers {
        push(object_id, "combat_attacker");
    }
    for &(attacker, defender) in &game.combat.attack_targets {
        push(attacker, "combat_attacker");
        if let crate::Defender::Planeswalker(object_id) = defender {
            push(object_id, "combat_defender");
        }
    }
    for &(first, second) in game
        .combat
        .blocks
        .iter()
        .chain(&game.combat.blocked_ever)
        .chain(&game.combat.cant_block_this_combat)
    {
        push(first, "combat");
        push(second, "combat");
    }
    for &object_id in &game.combat.attacked_or_blocked {
        push(object_id, "combat");
    }
    for (attacker, assignments) in &game.combat.damage {
        push(*attacker, "combat_damage");
        for &(blocker, _) in assignments {
            push(blocker, "combat_damage");
        }
    }
    for band in &game.combat.bands {
        for &object_id in band {
            push(object_id, "combat_band");
        }
    }
    for &object_id in &game.combat.cant_attack_this_combat {
        push(object_id, "combat");
    }
    for &(_, object_id) in &game.combat.attacks_dont_tap {
        push(object_id, "combat");
    }

    for &object_id in &game.skip_next_untap {
        push(object_id, "skip_untap");
    }
    for &(source, victim) in &game.damaged_this_turn {
        push(source, "damage_history");
        push(victim, "damage_history");
    }
    for &(source, target, _) in &game.damage_dealt_this_turn {
        push(source, "damage_history");
        if let Target::Object(object_id) = target {
            push(object_id, "damage_history");
        }
    }
    for &object_id in &game.drawn_this_turn {
        push(object_id, "draw_history");
    }
    for &(_, object_id) in &game.hand_cards_seen {
        push(object_id, "hand_seen");
    }
    for &(object_id, _) in &game.revealed_unplayable_until_next_turn {
        push(object_id, "revealed_hand");
    }
    for &(target, _) in &game.abilities_granted_until_eot {
        push(target, "granted_ability");
    }
    for &(_, source) in &game.abilities_granted_until_eot {
        push(source, "granted_ability_source");
    }
    for &(object_id, _) in &game.pending_enter_bonus_counters {
        push(object_id, "enter_counter");
    }
    for &(object_id, _) in &game.exile_time_counters {
        push(object_id, "exile_counter");
    }
    if let Some(object_id) = game.resume.spell_finish {
        push(object_id, "resume_spell");
    }
    if let Some((_, object_id)) = game.resume.demonstrate_opponent_copy {
        push(object_id, "resume_copy");
    }
    if let Some(batch) = &game.resume.draw_batch {
        push_draw_after_references(batch.after, &mut push);
    }
    for obligation in &game.pending_obligations {
        push(obligation.object(), "obligation");
    }
    for group in &game.pending_trigger_groups {
        push(group.source, "pending_trigger");
    }
    for &(object_id, _) in &game.once_per_turn.activated {
        push(object_id, "activation_limit");
    }
    for &object_id in &game.once_per_turn.triggered {
        push(object_id, "trigger_limit");
    }
    for &(source, exiled) in &game.exile_links.until_source_leaves {
        push(source, "exile_link");
        push(exiled, "exile_link");
    }
    for &(source, exiled) in &game.exile_links.illusion_on_source_leave {
        push(source, "exile_link");
        push(exiled, "exile_link");
    }
    for &(source, exiled) in &game.exile_links.with_source {
        push(source, "exile_link");
        push(exiled, "exile_link");
    }
    for &(_, source, _, _) in &game.delayed_triggers.scheduled {
        push(source, "delayed_trigger");
    }
    for &(_, source, _) in &game.delayed_triggers.scheduled_your_upkeep {
        push(source, "delayed_trigger");
    }
    for modifier in &game.modifier_provenance.modifiers {
        push(modifier.host, "modifier");
    }
    for &(host, _, _) in &game.modifier_provenance.counter_batches {
        push(host, "counter_batch");
    }

    for player in &game.players {
        for &(object_id, _) in &player.war_choices {
            push(object_id, "player_war_choice");
        }
        for &(object_id, _) in &player.mana_provenance {
            push(object_id, "mana_provenance");
        }
    }
    for &(object_id, _, _) in &game.combat_extras.goaded {
        push(object_id, "goad");
    }
    for &(object_id, _) in &game.combat_extras.must_attack {
        push(object_id, "must_attack");
    }
    for &object_id in &game.combat_extras.may_block_any_number {
        push(object_id, "combat_permission");
    }
    for &object_id in &game.combat_extras.must_block_all {
        push(object_id, "combat_requirement");
    }
    for &(first, second, _) in &game.combat_extras.blocked_this_turn {
        push(first, "block_history");
        push(second, "block_history");
    }
    for &object_id in game
        .combat_extras
        .cant_attack_next_own_turn
        .iter()
        .chain(&game.combat_extras.cant_attack_this_own_turn)
        .chain(&game.combat_extras.assigns_no_combat_damage_this_turn)
    {
        push(object_id, "combat_restriction");
    }

    for &(object_id, _, _, _) in &game.play_permissions.control_overrides {
        push(object_id, "control_override");
    }
    for &(object_id, _, _) in &game.play_permissions.permanent_control_overrides {
        push(object_id, "control_override");
    }
    for &(object_id, _, condition, _) in &game.play_permissions.conditioned_control_overrides {
        push(object_id, "control_override");
        push_control_condition_references(condition, &mut push);
    }
    for &(object_id, _) in &game.play_permissions.aura_control_timestamps {
        push(object_id, "aura_control");
    }
    for &(object_id, _, _) in &game.play_permissions.play_from_exile {
        push(object_id, "exile_permission");
    }
    for &(object_id, _, source) in &game.play_permissions.play_from_exile_free_while_source {
        push(object_id, "exile_permission");
        push(source, "permission_source");
    }
    for &(object_id, _) in &game.play_permissions.cast_from_exile_free {
        push(object_id, "exile_permission");
    }
    if let Some((object_id, _)) = game.play_permissions.compelled_play {
        push(object_id, "compelled_play");
    }
    for &object_id in &game.play_permissions.stack_object_bottoms_library_on_leave {
        push(object_id, "stack_permission");
    }
    for &(object_id, _) in &game.play_permissions.on_adventure {
        push(object_id, "adventure");
    }
    for &(object_id, _) in &game.play_permissions.adventure_fronts {
        push(object_id, "adventure");
    }
    for &(object_id, _) in &game.play_permissions.split_halves_on_stack {
        push(object_id, "split_spell");
    }

    for &(token, exiled) in &game.exile_links.token_leaves_returns_exiled {
        push(token, "exile_link");
        push(exiled, "exile_link");
    }
    for &(_, source, _, _) in &game.delayed_triggers.pending_next_cast {
        push(source, "delayed_trigger");
    }
    for rows in [
        &game.delayed_triggers.pending_combat_damage_watch,
        &game.delayed_triggers.pending_combat_damage_copy,
        &game.delayed_triggers.pending_attacker_damage_life,
    ] {
        for &(_, source, watched) in rows {
            push(source, "delayed_trigger");
            push(watched, "delayed_watch");
        }
    }
    for &(_, source, watched, _) in &game.delayed_triggers.pending_dies_this_turn {
        push(source, "delayed_trigger");
        push(watched, "delayed_watch");
    }

    for action in &game.actions {
        use crate::MeaningfulAction as A;
        let object_id = match action.kind {
            A::PlayLand { card, .. }
            | A::Cast { card, .. }
            | A::Cycle { card }
            | A::ActivateHandAbility { card, .. }
            | A::Suspend { card }
            | A::Encore { card }
            | A::CastSplitHalf { card, .. }
            | A::CastFaceDown { card } => Some(card),
            A::Activate { source, .. } | A::CastPrepared { source } => Some(source),
            A::TurnFaceUp { permanent } => Some(permanent),
            A::KeepHand
            | A::Mulligan
            | A::PayStandingPrevention { .. }
            | A::DeclareAttackers
            | A::DeclareBlockers => None,
        };
        if let Some(object_id) = object_id {
            push(object_id, "legal_action");
        }
    }
    for shield in &game.damage_prevention_shields {
        if let Target::Object(object_id) = shield.target {
            push(object_id, "prevention_target");
        }
        if let Some(object_id) = shield.from_source {
            push(object_id, "prevention_source");
        }
        if let Some(Target::Object(object_id)) = shield.redirect_to {
            push(object_id, "prevention_redirect");
        }
    }
    for offer in &game.standing_preventions {
        if let Target::Object(object_id) = offer.target {
            push(object_id, "standing_prevention");
        }
    }
    for &(_, object_id) in &game.batch_trigger_scratch.graveyard_exits_this_batch {
        push(object_id, "batch_scratch");
    }
    for &(_, object_id) in &game.batch_trigger_scratch.discards_this_batch {
        push(object_id, "batch_scratch");
    }
    for &(creature, aura, _, _) in &game.batch_trigger_scratch.dying_creature_attachments {
        push(creature, "batch_lki");
        push(aura, "batch_lki");
    }
    for &object_id in &game
        .batch_trigger_scratch
        .permanents_put_into_graveyard_from_battlefield
    {
        push(object_id, "batch_lki");
    }
    for &(object_id, host) in &game.batch_trigger_scratch.permanents_left_battlefield {
        push(object_id, "batch_lki");
        if let Some(host) = host {
            push(host, "batch_lki");
        }
    }
    for &object_id in &game.batch_trigger_scratch.serra_recursion_deaths {
        push(object_id, "batch_lki");
    }
    for stats in &game.batch_trigger_scratch.dying_creature_stats {
        push(stats.id, "batch_lki");
    }
    for &(object_id, _, _) in &game.batch_trigger_scratch.dying_creature_lki {
        push(object_id, "batch_lki");
    }

    for target in &game.resolution_frame.resolving_targets {
        if let Target::Object(object_id) = target {
            push(*object_id, "resolution_target");
        }
    }
    if let Some((object_id, _)) = game.resolution_frame.surge_exiled_card {
        push(object_id, "resolution_exile");
    }
    if let Some((object_id, _)) = game.resolution_frame.vanished_permanent_owner {
        push(object_id, "resolution_lki");
    }
    if let Some(object_id) = game.resolution_frame.chosen_damage_source {
        push(object_id, "resolution_source");
    }
    if let Some(sequence) = &game.resume.sequence {
        push(sequence.ctx.source, "resume_source");
        if let Some(Target::Object(object_id)) = sequence.ctx.target {
            push(object_id, "resume_target");
        }
        for target in sequence.ctx.targets_second.iter() {
            if let Target::Object(object_id) = target {
                push(object_id, "resume_target");
            }
        }
    }

    references
}

fn push_draw_after_references(
    after: crate::resolution::DrawAfter,
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    match after {
        crate::resolution::DrawAfter::Nothing | crate::resolution::DrawAfter::DrawStep => {}
        crate::resolution::DrawAfter::TradeSecretsCaster { source, .. }
        | crate::resolution::DrawAfter::TradeSecretsRepeat { source, .. } => {
            push(source, "resume_draw_after");
        }
    }
}

fn push_may_draw_up_to_resume_references(
    resume: &crate::MayDrawUpToResume,
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    match resume {
        crate::MayDrawUpToResume::Default => {}
        crate::MayDrawUpToResume::TradeSecretsRepeat { source, .. } => {
            push(*source, "pending_choice");
        }
    }
}

fn push_control_condition_references(
    condition: crate::ControlCondition,
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    let crate::ControlCondition {
        source,
        needs_tapped: _,
    } = condition;
    push(source, "control_condition");
}

fn push_player_object_piles_references(
    piles: &[(PlayerId, Vec<ObjectId>)],
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    for (_, pile) in piles {
        for &object_id in pile {
            push(object_id, "pending_choice");
        }
    }
}

fn push_splitting_continuation_references(
    continuation: &crate::SplittingContinuation,
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    match continuation {
        crate::SplittingContinuation::ExilePiles { pile_a, pile_b } => {
            for &object_id in pile_a.iter().chain(pile_b) {
                push(object_id, "pending_choice");
            }
        }
        crate::SplittingContinuation::Partition { revealed }
        | crate::SplittingContinuation::PickOneToGraveyard { revealed } => {
            for &object_id in revealed {
                push(object_id, "pending_choice");
            }
        }
        crate::SplittingContinuation::GainControlOf { objects } => {
            for &object_id in objects {
                push(object_id, "pending_choice");
            }
        }
        crate::SplittingContinuation::Clash
        | crate::SplittingContinuation::RememberAsChosenOpponent => {}
    }
}

fn push_pending_choice_references(
    choice: &crate::PendingChoice,
    push: &mut impl FnMut(ObjectId, &'static str),
) {
    match choice {
        crate::PendingChoice::OrderTriggers { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseTarget {
            source,
            legal,
            target,
            ..
        } => {
            push(*source, "pending_choice");
            for target in legal {
                if let Target::Object(object_id) = target {
                    push(*object_id, "pending_choice");
                }
            }
            if let Some(Target::Object(object_id)) = target {
                push(*object_id, "pending_choice");
            }
        }
        crate::PendingChoice::MayYesNo { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::MayRevealLandFromHand { land, .. } => {
            push(*land, "pending_choice");
        }
        crate::PendingChoice::MayDrawUpTo { resume, .. } => {
            push_may_draw_up_to_resume_references(resume, push);
        }
        crate::PendingChoice::DeclineUntap {
            permanents,
            at_most_one,
            ..
        } => {
            for &object_id in permanents {
                push(object_id, "pending_choice");
            }
            for group in at_most_one {
                for &object_id in group {
                    push(object_id, "pending_choice");
                }
            }
        }
        crate::PendingChoice::ChooseDredge { eligible, .. } => {
            for &(object_id, ..) in eligible {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::PayCost { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::PayOrCounter { spell, .. } => {
            push(*spell, "pending_choice");
        }
        crate::PendingChoice::PayOrControllerDraws { .. } => {}
        crate::PendingChoice::ChooseCounteredSpellDestination { spell, .. } => {
            push(*spell, "pending_choice");
        }
        crate::PendingChoice::PayEchoOrSacrifice { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::PayCumulativeUpkeepOrSacrifice {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::PayRecoverOrExile { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::PayOrElse { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::PayLifeOrEntersTapped { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::SacrificeUnlessReturnLand {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::AssignCombatDamage {
            source, recipients, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in recipients {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DivideSpellDamage { spell, targets, .. } => {
            push(*spell, "pending_choice");
            for target in targets {
                if let Target::Object(object_id) = target {
                    push(*object_id, "pending_choice");
                }
            }
        }
        crate::PendingChoice::DivideCounters { spell, targets, .. } => {
            push(*spell, "pending_choice");
            for &object_id in targets {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DivideMovedCounters { from, legal, .. } => {
            push(*from, "pending_choice");
            for &object_id in legal {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ArrangeTop { cards, .. } => {
            for &object_id in cards {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::SelectFromTop { cards, .. } => {
            for &object_id in cards {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DanceExileMore { source, exiled, .. } => {
            push(*source, "pending_choice");
            for &object_id in exiled {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DistributeTop { cards, .. } => {
            for &object_id in cards {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::Proliferate {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for option in options {
                if let crate::ProliferateTarget::Permanent(object_id) = option {
                    push(*object_id, "pending_choice");
                }
            }
        }
        crate::PendingChoice::PhaseOut {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseActivationCostTargets {
            source,
            target,
            legal,
            ..
        } => {
            push(*source, "pending_choice");
            if let Some(Target::Object(object_id)) = target {
                push(*object_id, "pending_choice");
            }
            for target in legal {
                if let Target::Object(object_id) = target {
                    push(*object_id, "pending_choice");
                }
            }
        }
        crate::PendingChoice::ShuffleFromGraveyard {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::SearchLibrary { matches, .. } => {
            for &object_id in matches {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseMode { source, target, .. } => {
            push(*source, "pending_choice");
            if let Some(Target::Object(object_id)) = target {
                push(*object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseTriggerModes { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::SacrificeEdict {
            options, source, ..
        } => {
            for &object_id in options {
                push(object_id, "pending_choice");
            }
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseTargetPlayers { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ExileFromGraveyard {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DiscardEdict {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::CasterKeepPermanents {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseCounterTargetForPlayer {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::JoinForcesPayment { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::CastVote { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::MaySacrifice {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::MayReturnFromGraveyard {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::MayExileDiscardedToPlay {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::MayDiscard {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::MayPutCounterOnCreature {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseBlockTarget {
            source,
            blocker,
            options,
            ..
        } => {
            push(*source, "pending_choice");
            push(*blocker, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DiscardToHandSize { hand, .. } => {
            for &object_id in hand {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DiscardCards { hand, .. } => {
            for &object_id in hand {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::PutFromHandOnTop { hand, .. } => {
            for &object_id in hand {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::PutLandFromHand { candidates, .. } => {
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::PutCreatureFromHand {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::CastCreatureFaceDown { candidates, .. } => {
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseExiledWithCard {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseExiledWithCardToCast {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseExiledDigToCastFree {
            source,
            candidates,
            exiled,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
            for &object_id in exiled {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseCardInHandToPlay {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::SplitBlockersIntoPiles {
            source,
            options,
            left,
            attackers,
            ..
        } => {
            push(*source, "pending_choice");
            push_player_object_piles_references(left, push);
            for &object_id in options {
                push(object_id, "pending_choice");
            }
            for &object_id in attackers {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::DivideBlockersIntoPiles {
            source,
            options,
            piles,
            attackers,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
            for group in piles {
                for &object_id in group {
                    push(object_id, "pending_choice");
                }
            }
            for &object_id in attackers {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChoosePileForAttacker {
            source,
            attacker,
            left,
            remaining,
            ..
        } => {
            push(*source, "pending_choice");
            push(*attacker, "pending_choice");
            push_player_object_piles_references(left, push);
            for &object_id in remaining {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::OpponentChoosesPile {
            source,
            pile_a,
            pile_b,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in pile_a {
                push(object_id, "pending_choice");
            }
            for &object_id in pile_b {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::OpponentChoosesExiledNonland {
            source,
            nonlands,
            exiled,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in nonlands {
                push(object_id, "pending_choice");
            }
            for &object_id in exiled {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseSplittingOpponent { source, then, .. } => {
            push(*source, "pending_choice");
            push_splitting_continuation_references(then, push);
        }
        crate::PendingChoice::PartitionRevealed {
            source, revealed, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in revealed {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::OpponentChoosesRevealedToGraveyard {
            source, revealed, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in revealed {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChoosePileForHand {
            source,
            pile_a,
            pile_b,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in pile_a {
                push(object_id, "pending_choice");
            }
            for &object_id in pile_b {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseExiledToCastFree {
            source,
            candidates,
            exiled,
            ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
            for &object_id in exiled {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::RevealedCardToBattlefieldOrHand { card, .. } => {
            push(*card, "pending_choice");
        }
        crate::PendingChoice::ChooseOwnSacrifices {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::SacrificeAnyNumber {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::Devour {
            source, options, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseManaColor { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseCreatureType { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseColor { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseCardName { source, .. } => {
            push(*source, "pending_choice");
        }
        crate::PendingChoice::ChooseCopyTarget {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseTokenToCopy {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseCopyCardFromList {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseAttachHost {
            attachment,
            candidates,
            ..
        } => {
            push(*attachment, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseLegendaryKeep { options, .. } => {
            for &object_id in options {
                push(object_id, "pending_choice");
            }
        }
        crate::PendingChoice::ChooseDamageSource {
            source, candidates, ..
        } => {
            push(*source, "pending_choice");
            for &object_id in candidates {
                push(object_id, "pending_choice");
            }
        }
    }
}

fn validate_references(game: &Game, violations: &mut ViolationCollector) {
    for (object_id, site) in all_references(game) {
        if game.objects.get(object_id as usize).is_none() {
            violations.push(
                "object_reference",
                format!("{} references missing object {object_id}", site.kind),
            );
        }
    }
}

#[derive(Default)]
struct ViolationCollector {
    items: Vec<Violation>,
}

impl ViolationCollector {
    fn push(&mut self, code: &'static str, message: String) {
        if self.items.len() >= MAX_VIOLATIONS {
            return;
        }
        let mut safe: String = message
            .chars()
            .map(|character| if character.is_ascii() { character } else { '?' })
            .collect();
        safe.truncate(safe.len().min(MAX_VIOLATION_MESSAGE_BYTES));
        self.items.push(Violation {
            code,
            message: safe,
        });
    }
}

#[cfg(test)]
mod tests;
