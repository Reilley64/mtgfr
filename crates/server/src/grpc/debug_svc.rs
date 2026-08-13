//! Debug-build-only, deliberately unauthenticated gRPC adapter.

use engine::debug::{
    DebugZone, ErrorReason, Inspection, Mutation, ObjectInspection, PlayerInspection,
    StackInspection,
};
use engine::{PlayerCounterKind, PlayerId, Step, Zone};
use prost::Message;
use tonic::{Code, Request, Response, Status};

use super::debug_pb as pb;
use crate::debug::{
    CheckpointCommand, DebugAbortReason, DebugFailure, JournalKind, JournalRecord, MutateCommand,
    ResourceLimit, RestoreCommand, TableMutation, checkpoint_table as checkpoint_domain,
    mutate_table as mutate_domain, restore_checkpoint as restore_domain,
};
use crate::{AppState, lock};

const MAX_STATUS_VIOLATIONS: usize = 16;
const MAX_STATUS_TEXT_BYTES: usize = 64;
const PRIVATE_FAILURE_MESSAGE: &str = "debug request rejected";

#[derive(Clone, Copy)]
struct MappingError;

#[used]
static DEBUG_IMPLEMENTATION_MARKER: [u8; 36] = *b"MTGFR_DEBUG_IMPLEMENTATION_MARKER_V1";

#[derive(Clone)]
pub(crate) struct DebugSvc {
    state: AppState,
    #[cfg(test)]
    inspection_override: Option<Inspection>,
    #[cfg(test)]
    chrome_override: Option<crate::chrome::DebugChromeSnapshot>,
    #[cfg(test)]
    pending_override: Option<engine::debug::PendingOrchestrationInspection>,
}

impl DebugSvc {
    pub(crate) fn new(state: AppState) -> Self {
        std::hint::black_box(&DEBUG_IMPLEMENTATION_MARKER);
        Self {
            state,
            #[cfg(test)]
            inspection_override: None,
            #[cfg(test)]
            chrome_override: None,
            #[cfg(test)]
            pending_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_inspection_for_test(state: AppState, inspection: Inspection) -> Self {
        Self {
            state,
            inspection_override: Some(inspection),
            chrome_override: None,
            pending_override: None,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_inspection_state_for_test(
        state: AppState,
        inspection: Inspection,
        chrome: crate::chrome::DebugChromeSnapshot,
        pending: engine::debug::PendingOrchestrationInspection,
    ) -> Self {
        Self {
            state,
            inspection_override: Some(inspection),
            chrome_override: Some(chrome),
            pending_override: Some(pending),
        }
    }
}

#[tonic::async_trait]
impl pb::debug_service_server::DebugService for DebugSvc {
    async fn list_tables(
        &self,
        _request: Request<pb::ListTablesRequest>,
    ) -> Result<Response<pb::ListTablesResponse>, Status> {
        let registry = lock(&self.state.reg);
        let tables = registry
            .debug_table_summaries()
            .into_iter()
            .map(|(table_id, debug_revision, table_seq)| pb::TableSummary {
                table_id,
                debug_revision,
                table_seq,
            })
            .collect();
        Ok(Response::new(pb::ListTablesResponse { tables }))
    }

    async fn inspect_table(
        &self,
        request: Request<pb::InspectTableRequest>,
    ) -> Result<Response<pb::InspectTableResponse>, Status> {
        let request = request.into_inner();
        if request.table_id.is_empty() {
            return Err(invalid_status(None));
        }
        let registry = lock(&self.state.reg);
        let Some(table) = registry.get(&request.table_id) else {
            return Err(status(DebugFailure::NotFound {
                operation_index: None,
            }));
        };
        let Some(game) = table.game.as_ref() else {
            return Err(status(DebugFailure::NotFound {
                operation_index: None,
            }));
        };
        let inspection = engine::debug::inspect(game);
        #[cfg(test)]
        let inspection = self.inspection_override.clone().unwrap_or(inspection);
        let pending = engine::debug::inspect_pending_orchestration(game);
        #[cfg(test)]
        let pending = self.pending_override.clone().unwrap_or(pending);
        let chrome = table.chrome.debug_snapshot();
        #[cfg(test)]
        let chrome = self.chrome_override.unwrap_or(chrome);
        let response = pb::InspectTableResponse {
            table_id: request.table_id,
            debug_revision: table.debug.revision,
            table_seq: table.seq,
            debug_mutated: table.debug.debug_mutated,
            game: Some(map_inspection(inspection)?),
            checkpoint_names: table.debug.checkpoints.keys().cloned().collect(),
            chrome: Some(pb::LogicalChromeView {
                yields: chrome.yields.to_vec(),
                turn_yields: chrome.turn_yields.to_vec(),
                hold_requested: chrome.hold_requested,
            }),
            pending_orchestration: Some(map_pending_orchestration(pending)?),
        };
        Ok(Response::new(response))
    }

    async fn mutate_table(
        &self,
        request: Request<pb::MutateTableRequest>,
    ) -> Result<Response<pb::MutateTableResponse>, Status> {
        let encoded_request_bytes = request.get_ref().encoded_len();
        let request = request.into_inner();
        if request.table_id.is_empty() {
            return Err(invalid_status(None));
        }
        let operation_count =
            u32::try_from(request.operations.len()).map_err(|_| invalid_status(None))?;
        let mut operations = Vec::with_capacity(request.operations.len());
        for (operation_index, operation) in request.operations.into_iter().enumerate() {
            let mapped = match map_mutation(operation) {
                Ok(mapped) => mapped,
                Err(error) if error.code() == Code::Unimplemented => return Err(error),
                Err(_) => return Err(invalid_status(Some(operation_index))),
            };
            operations.push(TableMutation::Engine(mapped));
        }
        let receipt = mutate_domain(
            &self.state,
            MutateCommand {
                table_id: request.table_id,
                expected_debug_revision: request.expected_debug_revision,
                expected_table_seq: request.expected_table_seq,
                operations,
                encoded_request_bytes,
            },
        )
        .map_err(status)?;
        debug_assert_eq!(receipt.applied_operation_count, operation_count as usize);
        Ok(Response::new(pb::MutateTableResponse {
            debug_revision: receipt.debug_revision,
            table_seq: receipt.table_seq,
            applied_operation_count: operation_count,
        }))
    }

    async fn checkpoint_table(
        &self,
        request: Request<pb::CheckpointTableRequest>,
    ) -> Result<Response<pb::CheckpointTableResponse>, Status> {
        let encoded_request_bytes = request.get_ref().encoded_len();
        let request = request.into_inner();
        if request.table_id.is_empty() {
            return Err(invalid_status(None));
        }
        let receipt = checkpoint_domain(
            &self.state,
            CheckpointCommand {
                table_id: request.table_id,
                name: request.name,
                replace_existing: request.replace_existing,
                expected_table_seq: request.expected_table_seq,
                encoded_request_bytes,
            },
        )
        .map_err(status)?;
        Ok(Response::new(pb::CheckpointTableResponse {
            debug_revision: receipt.debug_revision,
            table_seq: receipt.table_seq,
            object_slots: receipt.object_slots,
            replaced: receipt.replaced,
        }))
    }

    async fn restore_checkpoint(
        &self,
        request: Request<pb::RestoreCheckpointRequest>,
    ) -> Result<Response<pb::RestoreCheckpointResponse>, Status> {
        let encoded_request_bytes = request.get_ref().encoded_len();
        let request = request.into_inner();
        if request.table_id.is_empty() {
            return Err(invalid_status(None));
        }
        let receipt = restore_domain(
            &self.state,
            RestoreCommand {
                table_id: request.table_id,
                name: request.name,
                expected_debug_revision: request.expected_debug_revision,
                expected_table_seq: request.expected_table_seq,
                encoded_request_bytes,
            },
        )
        .map_err(status)?;
        Ok(Response::new(pb::RestoreCheckpointResponse {
            debug_revision: receipt.debug_revision,
            table_seq: receipt.table_seq,
            restored_source_table_seq: receipt.restored_source_table_seq,
        }))
    }

    async fn get_debug_journal(
        &self,
        request: Request<pb::GetDebugJournalRequest>,
    ) -> Result<Response<pb::GetDebugJournalResponse>, Status> {
        let request = request.into_inner();
        if request.table_id.is_empty() {
            return Err(invalid_status(None));
        }
        let registry = lock(&self.state.reg);
        let Some(table) = registry.get(&request.table_id) else {
            return Err(status(DebugFailure::NotFound {
                operation_index: None,
            }));
        };
        let records = table
            .debug
            .journal
            .iter()
            .cloned()
            .map(map_journal_record)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Response::new(pb::GetDebugJournalResponse { records }))
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn map_mutation(mutation: pb::Mutation) -> Result<Mutation, Status> {
    use pb::mutation::Operation;

    if matches!(
        mutation.operation.as_ref(),
        Some(
            Operation::ReplaceStack(_)
                | Operation::PushStack(_)
                | Operation::PopStack(_)
                | Operation::SetObjectPrintOverride(_)
        )
    ) {
        return Err(Status::new(Code::Unimplemented, PRIVATE_FAILURE_MESSAGE));
    }
    map_mutation_inner(mutation).map_err(|_| invalid_status(None))
}

fn map_mutation_inner(mutation: pb::Mutation) -> Result<Mutation, MappingError> {
    use pb::mutation::Operation;

    let Some(operation) = mutation.operation else {
        return Err(MappingError);
    };
    match operation {
        Operation::SetLife(edit) => Ok(Mutation::SetLife {
            player: player(edit.player)?,
            life: edit.life,
        }),
        Operation::SetPlayerCounter(edit) => Ok(Mutation::SetPlayerCounter {
            player: player(edit.player)?,
            counter: player_counter(edit.counter)?,
            value: narrow_u8(edit.value)?,
        }),
        Operation::SetTurnState(edit) => Ok(Mutation::SetTurnState {
            active_player: player(edit.active_player)?,
            step: step(edit.step)?,
            priority_player: player(edit.priority_player)?,
            consecutive_passes: narrow_u8(edit.consecutive_passes)?,
        }),
        Operation::SetPermanentState(edit) => Ok(Mutation::SetPermanentState {
            object_id: edit.object_id,
            tapped: edit.tapped,
            marked_damage: edit.marked_damage,
            plus_one_counters: edit.plus_one_counters,
        }),
        Operation::SetController(edit) => Ok(Mutation::SetController {
            object_id: edit.object_id,
            controller: player(edit.controller)?,
        }),
        Operation::SetAttachment(edit) => Ok(Mutation::SetAttachment {
            object_id: edit.object_id,
            attached_to: edit.attached_to,
        }),
        Operation::CreateCard(edit) => {
            if edit.card_id.is_empty() {
                return Err(MappingError);
            }
            Ok(Mutation::CreateCard {
                object_id: edit.object_id,
                card_id: edit.card_id,
                owner: player(edit.owner)?,
                controller: player(edit.controller)?,
                destination: debug_zone(edit.destination)?,
                commander: edit.commander,
                face_down: edit.face_down,
            })
        }
        Operation::MoveCard(edit) => Ok(Mutation::MoveCard {
            object_id: edit.object_id,
            new_object_id: edit.new_object_id,
            destination: debug_zone(edit.destination)?,
            controller: player(edit.controller)?,
            face_down: edit.face_down,
        }),
        Operation::SetLibraryOrder(edit) => Ok(Mutation::SetLibraryOrder {
            player: player(edit.player)?,
            object_ids: edit.object_ids,
        }),
        Operation::RemoveCard(edit) => Ok(Mutation::RemoveCard {
            object_id: edit.object_id,
        }),
        Operation::ClearPendingOrchestration(edit) => Ok(Mutation::ClearPendingOrchestration {
            clear_queued_triggers: edit.clear_queued_triggers,
        }),
        Operation::ReplaceStack(_)
        | Operation::PushStack(_)
        | Operation::PopStack(_)
        | Operation::SetObjectPrintOverride(_) => Err(MappingError),
    }
}

fn player(value: u32) -> Result<PlayerId, MappingError> {
    u8::try_from(value).map(PlayerId).map_err(|_| MappingError)
}

fn narrow_u8(value: u32) -> Result<u8, MappingError> {
    u8::try_from(value).map_err(|_| MappingError)
}

fn player_counter(value: i32) -> Result<PlayerCounterKind, MappingError> {
    match pb::PlayerCounter::try_from(value) {
        Ok(pb::PlayerCounter::Poison) => Ok(PlayerCounterKind::Poison),
        Ok(pb::PlayerCounter::Rad) => Ok(PlayerCounterKind::Rad),
        Ok(pb::PlayerCounter::Unspecified) | Err(_) => Err(MappingError),
    }
}

fn step(value: i32) -> Result<Step, MappingError> {
    match pb::Step::try_from(value) {
        Ok(pb::Step::Untap) => Ok(Step::Untap),
        Ok(pb::Step::Upkeep) => Ok(Step::Upkeep),
        Ok(pb::Step::Draw) => Ok(Step::Draw),
        Ok(pb::Step::Main1) => Ok(Step::Main1),
        Ok(pb::Step::BeginCombat) => Ok(Step::BeginCombat),
        Ok(pb::Step::DeclareAttackers) => Ok(Step::DeclareAttackers),
        Ok(pb::Step::DeclareBlockers) => Ok(Step::DeclareBlockers),
        Ok(pb::Step::FirstStrikeCombatDamage) => Ok(Step::FirstStrikeCombatDamage),
        Ok(pb::Step::CombatDamage) => Ok(Step::CombatDamage),
        Ok(pb::Step::EndCombat) => Ok(Step::EndCombat),
        Ok(pb::Step::Main2) => Ok(Step::Main2),
        Ok(pb::Step::End) => Ok(Step::End),
        Ok(pb::Step::Cleanup) => Ok(Step::Cleanup),
        Ok(pb::Step::Unspecified) | Err(_) => Err(MappingError),
    }
}

fn debug_zone(value: i32) -> Result<DebugZone, MappingError> {
    match pb::Zone::try_from(value) {
        Ok(pb::Zone::Library) => Ok(DebugZone::Library),
        Ok(pb::Zone::Hand) => Ok(DebugZone::Hand),
        Ok(pb::Zone::Battlefield) => Ok(DebugZone::Battlefield),
        Ok(pb::Zone::Graveyard) => Ok(DebugZone::Graveyard),
        Ok(pb::Zone::Exile) => Ok(DebugZone::Exile),
        Ok(pb::Zone::Command) => Ok(DebugZone::Command),
        Ok(pb::Zone::Unspecified) | Err(_) => Err(MappingError),
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn map_inspection(inspection: Inspection) -> Result<pb::GameInspection, Status> {
    map_inspection_inner(inspection).map_err(|_| internal_status())
}

fn map_inspection_inner(inspection: Inspection) -> Result<pb::GameInspection, MappingError> {
    let players = inspection
        .players
        .into_iter()
        .map(map_player_inspection)
        .collect();
    let objects = inspection
        .objects
        .into_iter()
        .map(map_object_inspection)
        .collect::<Result<Vec<_>, _>>()?;
    let stack = inspection
        .stack
        .into_iter()
        .map(map_stack_inspection)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pb::GameInspection {
        players,
        objects,
        stack,
        active_player: u32::from(inspection.active_player.0),
        step: map_step(inspection.step) as i32,
        priority_player: u32::from(inspection.priority_player.0),
        consecutive_passes: u32::from(inspection.consecutive_passes),
        has_pending_choice: inspection.has_pending_choice,
        has_deferred_resume: inspection.has_deferred_resume,
    })
}

fn map_player_inspection(player: PlayerInspection) -> pb::PlayerInspection {
    pb::PlayerInspection {
        player_id: u32::from(player.player_id.0),
        life: player.life,
        poison: u32::from(player.poison),
        rad: u32::from(player.rad),
        library: player.library,
        hand: player.hand,
        graveyard: player.graveyard,
        exile: player.exile,
        command: player.command,
    }
}

fn map_object_inspection(object: ObjectInspection) -> Result<pb::ObjectInspection, MappingError> {
    use pb::object_inspection::State;

    Ok(match object {
        ObjectInspection::Card {
            object_id,
            card_id,
            owner,
            zone,
            commander,
            face_down,
        } => pb::ObjectInspection {
            object_id,
            state: Some(State::Card(pb::CardInspection {
                card_id,
                owner: u32::from(owner.0),
                zone: map_zone(zone)? as i32,
                commander,
                face_down,
            })),
        },
        ObjectInspection::Permanent {
            object_id,
            card_id,
            owner,
            controller,
            tapped,
            marked_damage,
            plus_one_counters,
            attached_to,
            commander,
            token,
            face_down,
        } => pb::ObjectInspection {
            object_id,
            state: Some(State::Permanent(pb::PermanentInspection {
                card_id,
                owner: u32::from(owner.0),
                controller: u32::from(controller.0),
                tapped,
                marked_damage,
                plus_one_counters,
                attached_to,
                commander,
                token,
                face_down,
            })),
        },
        ObjectInspection::Spell {
            object_id,
            card_id,
            controller,
        } => pb::ObjectInspection {
            object_id,
            state: Some(State::Spell(pb::SpellInspection {
                card_id,
                controller: u32::from(controller.0),
            })),
        },
        ObjectInspection::Moved {
            object_id,
            to_object_id,
        } => pb::ObjectInspection {
            object_id,
            state: Some(State::Moved(pb::MovedInspection { to_object_id })),
        },
        ObjectInspection::Removed {
            object_id,
            card_id,
            owner,
        } => pb::ObjectInspection {
            object_id,
            state: Some(State::Removed(pb::RemovedInspection {
                card_id,
                owner: u32::from(owner.0),
            })),
        },
    })
}

fn map_stack_inspection(stack: StackInspection) -> Result<pb::StackInspection, MappingError> {
    Ok(pb::StackInspection {
        position_from_bottom: u32::try_from(stack.position_from_bottom)
            .map_err(|_| MappingError)?,
        kind: stack.kind.to_string(),
        source_object_id: stack.source_object_id,
        controller: stack.controller.map(|controller| u32::from(controller.0)),
        label: stack.label,
    })
}

fn map_zone(zone: Zone) -> Result<pb::Zone, MappingError> {
    match zone {
        Zone::Library => Ok(pb::Zone::Library),
        Zone::Hand => Ok(pb::Zone::Hand),
        Zone::Battlefield => Ok(pb::Zone::Battlefield),
        Zone::Graveyard => Ok(pb::Zone::Graveyard),
        Zone::Exile => Ok(pb::Zone::Exile),
        Zone::Command => Ok(pb::Zone::Command),
        Zone::Stack => Err(MappingError),
    }
}

fn map_step(step: Step) -> pb::Step {
    match step {
        Step::Untap => pb::Step::Untap,
        Step::Upkeep => pb::Step::Upkeep,
        Step::Draw => pb::Step::Draw,
        Step::Main1 => pb::Step::Main1,
        Step::BeginCombat => pb::Step::BeginCombat,
        Step::DeclareAttackers => pb::Step::DeclareAttackers,
        Step::DeclareBlockers => pb::Step::DeclareBlockers,
        Step::FirstStrikeCombatDamage => pb::Step::FirstStrikeCombatDamage,
        Step::CombatDamage => pb::Step::CombatDamage,
        Step::EndCombat => pb::Step::EndCombat,
        Step::Main2 => pb::Step::Main2,
        Step::End => pb::Step::End,
        Step::Cleanup => pb::Step::Cleanup,
    }
}

#[allow(clippy::result_large_err)]
fn map_pending_orchestration(
    pending: engine::debug::PendingOrchestrationInspection,
) -> Result<pb::PendingOrchestrationView, Status> {
    Ok(pb::PendingOrchestrationView {
        has_pending_choice: pending.has_pending_choice,
        has_resume: pending.has_resume,
        has_resolution_frame: pending.has_resolution_frame,
        has_resolution_finish: pending.has_resolution_finish,
        pending_enter_bonus_counters: u32::try_from(pending.pending_enter_bonus_counters)
            .map_err(|_| internal_status())?,
        pending_trigger_groups: u32::try_from(pending.pending_trigger_groups)
            .map_err(|_| internal_status())?,
        pending_obligations: u32::try_from(pending.pending_obligations)
            .map_err(|_| internal_status())?,
    })
}

#[allow(clippy::result_large_err)]
fn map_journal_record(record: JournalRecord) -> Result<pb::DebugJournalRecord, Status> {
    use pb::debug_journal_record::Kind;

    let kind = match record.kind {
        JournalKind::MutationCommitted { operations } => {
            Kind::MutationCommitted(pb::MutationCommitted {
                operations: operations
                    .into_iter()
                    .map(map_table_mutation)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        }
        JournalKind::CheckpointCreated { name, replaced } => {
            Kind::CheckpointCreated(pb::CheckpointCreated { name, replaced })
        }
        JournalKind::CheckpointRestored {
            name,
            source_table_seq,
        } => Kind::CheckpointRestored(pb::CheckpointRestored {
            name,
            source_table_seq,
        }),
    };
    Ok(pb::DebugJournalRecord {
        ordinal: record.ordinal,
        timestamp_unix_ms: record.timestamp_unix_ms,
        debug_revision: record.debug_revision,
        table_seq: record.table_seq,
        kind: Some(kind),
        encoded_request_bytes: u64::try_from(record.encoded_request_bytes)
            .map_err(|_| internal_status())?,
    })
}

#[allow(clippy::result_large_err)]
fn map_table_mutation(operation: TableMutation) -> Result<pb::Mutation, Status> {
    use pb::mutation::Operation;

    let operation = match operation {
        TableMutation::Engine(operation) => return Ok(map_domain_mutation(operation)),
        TableMutation::ReplaceStack(entries) => Operation::ReplaceStack(pb::ReplaceStack {
            entries: entries
                .into_iter()
                .map(map_stack_entry_spec)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        TableMutation::PushStack(entry) => Operation::PushStack(pb::PushStack {
            entry: Some(map_stack_entry_spec(entry)?),
        }),
        TableMutation::PopStack { count } => Operation::PopStack(pb::PopStack {
            count: u32::try_from(count).map_err(|_| internal_status())?,
        }),
        TableMutation::SetObjectPrintOverride {
            object_id,
            printing_id,
        } => Operation::SetObjectPrintOverride(pb::SetObjectPrintOverride {
            object_id,
            printing_id,
        }),
    };
    Ok(pb::Mutation {
        operation: Some(operation),
    })
}

#[allow(clippy::result_large_err)]
fn map_stack_entry_spec(
    entry: engine::debug::DebugStackEntrySpec,
) -> Result<pb::StackEntrySpec, Status> {
    use engine::debug::DebugStackEntrySpec;
    use pb::stack_entry_spec::Kind;

    let kind = match entry {
        DebugStackEntrySpec::KnownSpell {
            entry_id,
            from_object_id,
            spell_object_id,
            controller,
            targets,
            targets_second,
            x,
        } => Kind::KnownSpell(pb::KnownSpellStackEntry {
            entry_id: entry_id.0,
            from_object_id,
            spell_object_id,
            controller: u32::from(controller.0),
            targets: targets.into_iter().map(map_debug_target).collect(),
            targets_second: targets_second.into_iter().map(map_debug_target).collect(),
            x,
        }),
        DebugStackEntrySpec::AuthoredAbility {
            entry_id,
            controller,
            source_object_id,
            ability_index,
            target,
            targets_second,
            x,
        } => Kind::AuthoredAbility(pb::AuthoredAbilityStackEntry {
            entry_id: entry_id.0,
            controller: u32::from(controller.0),
            source_object_id,
            ability_index: u32::try_from(ability_index).map_err(|_| internal_status())?,
            target: target.map(map_debug_target),
            targets_second: targets_second.into_iter().map(map_debug_target).collect(),
            x,
        }),
        DebugStackEntrySpec::PublicGhost {
            entry_id,
            controller,
            public,
        } => Kind::PublicGhost(pb::PublicGhostStackEntry {
            entry_id: entry_id.0,
            controller: u32::from(controller.0),
            name: public.name,
            label: public.label,
            printing_id: public.printing_id,
            card_id: public.card_id,
            printed_sentences: public.printed_sentences,
        }),
    };
    Ok(pb::StackEntrySpec { kind: Some(kind) })
}

fn map_debug_target(target: engine::Target) -> pb::DebugTarget {
    use pb::debug_target::Kind;

    let kind = match target {
        engine::Target::Object(object_id) => Kind::ObjectId(object_id),
        engine::Target::Player(player) => Kind::Player(u32::from(player.0)),
    };
    pb::DebugTarget { kind: Some(kind) }
}

fn map_domain_mutation(mutation: Mutation) -> pb::Mutation {
    use pb::mutation::Operation;

    let operation = match mutation {
        Mutation::SetLife { player, life } => Operation::SetLife(pb::SetLife {
            player: u32::from(player.0),
            life,
        }),
        Mutation::SetPlayerCounter {
            player,
            counter,
            value,
        } => Operation::SetPlayerCounter(pb::SetPlayerCounter {
            player: u32::from(player.0),
            counter: match counter {
                PlayerCounterKind::Poison => pb::PlayerCounter::Poison as i32,
                PlayerCounterKind::Rad => pb::PlayerCounter::Rad as i32,
            },
            value: u32::from(value),
        }),
        Mutation::SetTurnState {
            active_player,
            step,
            priority_player,
            consecutive_passes,
        } => Operation::SetTurnState(pb::SetTurnState {
            active_player: u32::from(active_player.0),
            step: map_step(step) as i32,
            priority_player: u32::from(priority_player.0),
            consecutive_passes: u32::from(consecutive_passes),
        }),
        Mutation::SetPermanentState {
            object_id,
            tapped,
            marked_damage,
            plus_one_counters,
        } => Operation::SetPermanentState(pb::SetPermanentState {
            object_id,
            tapped,
            marked_damage,
            plus_one_counters,
        }),
        Mutation::SetController {
            object_id,
            controller,
        } => Operation::SetController(pb::SetController {
            object_id,
            controller: u32::from(controller.0),
        }),
        Mutation::SetAttachment {
            object_id,
            attached_to,
        } => Operation::SetAttachment(pb::SetAttachment {
            object_id,
            attached_to,
        }),
        Mutation::CreateCard {
            object_id,
            card_id,
            owner,
            controller,
            destination,
            commander,
            face_down,
        } => Operation::CreateCard(pb::CreateCard {
            object_id,
            card_id,
            owner: u32::from(owner.0),
            controller: u32::from(controller.0),
            destination: map_debug_zone(destination) as i32,
            commander,
            face_down,
        }),
        Mutation::MoveCard {
            object_id,
            new_object_id,
            destination,
            controller,
            face_down,
        } => Operation::MoveCard(pb::MoveCard {
            object_id,
            new_object_id,
            destination: map_debug_zone(destination) as i32,
            controller: u32::from(controller.0),
            face_down,
        }),
        Mutation::SetLibraryOrder { player, object_ids } => {
            Operation::SetLibraryOrder(pb::SetLibraryOrder {
                player: u32::from(player.0),
                object_ids,
            })
        }
        Mutation::RemoveCard { object_id } => Operation::RemoveCard(pb::RemoveCard { object_id }),
        Mutation::ClearPendingOrchestration {
            clear_queued_triggers,
        } => Operation::ClearPendingOrchestration(pb::ClearPendingOrchestration {
            clear_queued_triggers,
        }),
    };
    pb::Mutation {
        operation: Some(operation),
    }
}

fn map_debug_zone(zone: DebugZone) -> pb::Zone {
    match zone {
        DebugZone::Library => pb::Zone::Library,
        DebugZone::Hand => pb::Zone::Hand,
        DebugZone::Battlefield => pb::Zone::Battlefield,
        DebugZone::Graveyard => pb::Zone::Graveyard,
        DebugZone::Exile => pb::Zone::Exile,
        DebugZone::Command => pb::Zone::Command,
    }
}

pub(crate) fn status(failure: DebugFailure) -> Status {
    let (code, operation_index, reason, violations) = match failure {
        DebugFailure::CheckpointNotFound => (
            Code::NotFound,
            None,
            pb::DebugErrorReason::CheckpointNotFound,
            vec![],
        ),
        DebugFailure::CheckpointAlreadyExists => (
            Code::AlreadyExists,
            None,
            pb::DebugErrorReason::CheckpointExists,
            vec![],
        ),
        DebugFailure::InvalidCheckpointName => (
            Code::InvalidArgument,
            None,
            pb::DebugErrorReason::CheckpointNameInvalid,
            vec![],
        ),
        DebugFailure::ResourceExhausted { reason } => (
            Code::ResourceExhausted,
            None,
            match reason {
                ResourceLimit::CheckpointCount => pb::DebugErrorReason::CheckpointCountLimit,
                ResourceLimit::CheckpointObjectSlots => pb::DebugErrorReason::CheckpointObjectLimit,
                ResourceLimit::JournalRecords | ResourceLimit::JournalRequestBytes => {
                    pb::DebugErrorReason::JournalLimit
                }
            },
            vec![],
        ),
        DebugFailure::NotFound { operation_index } => (
            Code::NotFound,
            operation_index,
            pb::DebugErrorReason::UnknownEntity,
            vec![],
        ),
        DebugFailure::AlreadyExists { operation_index } => (
            Code::AlreadyExists,
            Some(operation_index),
            pb::DebugErrorReason::DuplicateId,
            vec![],
        ),
        DebugFailure::Aborted { reason, .. } => (
            Code::Aborted,
            None,
            match reason {
                DebugAbortReason::DebugRevisionMismatch => pb::DebugErrorReason::StaleDebugRevision,
                DebugAbortReason::TableSeqMismatch => pb::DebugErrorReason::StaleTableSeq,
            },
            vec![],
        ),
        DebugFailure::Invalid {
            operation_index,
            reason,
        } => (
            Code::InvalidArgument,
            operation_index,
            map_error_reason(reason),
            vec![],
        ),
        DebugFailure::UnsupportedStackConstruction {
            operation_index,
            entry_index: _,
        } => (
            Code::InvalidArgument,
            Some(operation_index),
            pb::DebugErrorReason::InvalidValue,
            vec![],
        ),
        DebugFailure::FailedPrecondition {
            operation_index,
            violations,
        } => {
            let reason = if violations
                .iter()
                .any(|violation| violation.code == "projection_failed")
            {
                pb::DebugErrorReason::ProjectionFailed
            } else {
                pb::DebugErrorReason::InvalidValue
            };
            (
                Code::FailedPrecondition,
                operation_index,
                reason,
                violations,
            )
        }
    };
    let operation_index = match operation_index.map(u32::try_from).transpose() {
        Ok(operation_index) => operation_index,
        Err(_) => return internal_status(),
    };
    let violations = violations
        .into_iter()
        .take(MAX_STATUS_VIOLATIONS)
        .map(|violation| pb::Violation {
            code: bounded_text(violation.code),
            message: "candidate violates a structural invariant".to_string(),
        })
        .collect();
    encoded_status(code, operation_index, reason, violations)
}

fn map_error_reason(reason: ErrorReason) -> pb::DebugErrorReason {
    match reason {
        ErrorReason::EmptyBatch => pb::DebugErrorReason::EmptyBatch,
        ErrorReason::UnknownEntity => pb::DebugErrorReason::UnknownEntity,
        ErrorReason::DuplicateId => pb::DebugErrorReason::DuplicateId,
        ErrorReason::InvalidValue => pb::DebugErrorReason::InvalidValue,
        ErrorReason::WrongObjectKind => pb::DebugErrorReason::WrongObjectKind,
        ErrorReason::ZoneDisagreement => pb::DebugErrorReason::ZoneDisagreement,
        ErrorReason::ReferencedObject => pb::DebugErrorReason::ReferencedObject,
        ErrorReason::AttachmentCycle => pb::DebugErrorReason::AttachmentCycle,
    }
}

fn invalid_status(operation_index: Option<usize>) -> Status {
    let operation_index = match operation_index.map(u32::try_from).transpose() {
        Ok(operation_index) => operation_index,
        Err(_) => return internal_status(),
    };
    encoded_status(
        Code::InvalidArgument,
        operation_index,
        pb::DebugErrorReason::InvalidValue,
        vec![],
    )
}

fn internal_status() -> Status {
    encoded_status(
        Code::Internal,
        None,
        pb::DebugErrorReason::InvalidValue,
        vec![],
    )
}

fn encoded_status(
    code: Code,
    operation_index: Option<u32>,
    reason: pb::DebugErrorReason,
    violations: Vec<pb::Violation>,
) -> Status {
    let details = pb::ErrorDetail {
        operation_index,
        reason: reason as i32,
        violations,
    }
    .encode_to_vec();
    Status::with_details(code, PRIVATE_FAILURE_MESSAGE, details.into())
}

fn bounded_text(value: &str) -> String {
    if value.len() <= MAX_STATUS_TEXT_BYTES {
        return value.to_string();
    }
    let mut end = MAX_STATUS_TEXT_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}
