#![cfg(debug_assertions)]

use std::sync::{Arc, Mutex};

use prost::Message;
use server::grpc::debug_pb as pb;
use tokio_stream::wrappers::TcpListenerStream;

#[derive(Clone)]
struct RecordingService {
    calls: Arc<Mutex<Vec<String>>>,
    mutation_requests: Arc<Mutex<Vec<pb::MutateTableRequest>>>,
    inspection: Arc<Mutex<pb::GameInspection>>,
}

impl Default for RecordingService {
    fn default() -> Self {
        Self {
            calls: Arc::default(),
            mutation_requests: Arc::default(),
            inspection: Arc::new(Mutex::new(pb::GameInspection {
                active_player: 1,
                next_object_id: Some(17),
                next_stack_entry_id: Some(41),
                ..Default::default()
            })),
        }
    }
}

#[tonic::async_trait]
impl pb::debug_service_server::DebugService for RecordingService {
    async fn list_tables(
        &self,
        _: tonic::Request<pb::ListTablesRequest>,
    ) -> Result<tonic::Response<pb::ListTablesResponse>, tonic::Status> {
        self.calls.lock().unwrap().push("tables".into());
        Ok(tonic::Response::new(pb::ListTablesResponse {
            tables: vec![],
        }))
    }

    async fn inspect_table(
        &self,
        request: tonic::Request<pb::InspectTableRequest>,
    ) -> Result<tonic::Response<pb::InspectTableResponse>, tonic::Status> {
        let table = request.into_inner().table_id;
        self.calls.lock().unwrap().push(format!("inspect:{table}"));
        Ok(tonic::Response::new(pb::InspectTableResponse {
            table_id: table,
            debug_revision: 2,
            table_seq: 3,
            game: Some(self.inspection.lock().unwrap().clone()),
            ..Default::default()
        }))
    }

    async fn mutate_table(
        &self,
        request: tonic::Request<pb::MutateTableRequest>,
    ) -> Result<tonic::Response<pb::MutateTableResponse>, tonic::Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(format!(
            "mutate:{}:{:?}",
            request.table_id, request.expected_table_seq
        ));
        self.mutation_requests.lock().unwrap().push(request.clone());
        Ok(tonic::Response::new(pb::MutateTableResponse {
            debug_revision: 3,
            table_seq: 4,
            applied_operation_count: request.operations.len() as u32,
        }))
    }

    async fn checkpoint_table(
        &self,
        request: tonic::Request<pb::CheckpointTableRequest>,
    ) -> Result<tonic::Response<pb::CheckpointTableResponse>, tonic::Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(format!(
            "checkpoint:{}:{}:{}:{:?}",
            request.table_id, request.name, request.replace_existing, request.expected_table_seq
        ));
        if request.name.starts_with("SECRET") {
            let detail = pb::ErrorDetail {
                operation_index: None,
                reason: pb::DebugErrorReason::CheckpointNotFound.into(),
                violations: vec![],
            };
            return Err(tonic::Status::with_details(
                tonic::Code::NotFound,
                format!("missing {} for {}", request.name, request.table_id),
                detail.encode_to_vec().into(),
            ));
        }
        Ok(tonic::Response::new(pb::CheckpointTableResponse {
            debug_revision: 3,
            table_seq: 4,
            object_slots: 5,
            replaced: request.replace_existing,
        }))
    }

    async fn restore_checkpoint(
        &self,
        request: tonic::Request<pb::RestoreCheckpointRequest>,
    ) -> Result<tonic::Response<pb::RestoreCheckpointResponse>, tonic::Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(format!(
            "restore:{}:{}:{:?}:{:?}",
            request.table_id,
            request.name,
            request.expected_debug_revision,
            request.expected_table_seq
        ));
        Ok(tonic::Response::new(pb::RestoreCheckpointResponse {
            debug_revision: 4,
            table_seq: 5,
            restored_source_table_seq: 3,
        }))
    }

    async fn get_debug_journal(
        &self,
        request: tonic::Request<pb::GetDebugJournalRequest>,
    ) -> Result<tonic::Response<pb::GetDebugJournalResponse>, tonic::Status> {
        let table = request.into_inner().table_id;
        self.calls.lock().unwrap().push(format!("journal:{table}"));
        Ok(tonic::Response::new(pb::GetDebugJournalResponse {
            records: vec![pb::DebugJournalRecord {
                ordinal: 1,
                timestamp_unix_ms: 2,
                debug_revision: 3,
                table_seq: 4,
                kind: Some(pb::debug_journal_record::Kind::CheckpointCreated(
                    pb::CheckpointCreated {
                        name: "baseline".into(),
                        replaced: false,
                    },
                )),
                encoded_request_bytes: 7,
            }],
        }))
    }
}

async fn bind_recording_service() -> (String, RecordingService, tokio::sync::oneshot::Sender<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let service = RecordingService::default();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let bound = service.clone();
    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(pb::debug_service_server::DebugServiceServer::new(bound))
            .serve_with_incoming_shutdown(TcpListenerStream::new(listener), async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    (format!("http://{address}"), service, shutdown_tx)
}

async fn run(
    executable: &'static str,
    args: Vec<String>,
    env_endpoint: String,
) -> std::process::Output {
    tokio::task::spawn_blocking(move || {
        std::process::Command::new(executable)
            .args(args)
            .env("MTGFR_DEBUG_ENDPOINT", env_endpoint)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn subprocess_calls_all_six_rpcs_and_cli_endpoint_takes_precedence_over_env() {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (env_endpoint, env_service, env_shutdown) = bind_recording_service().await;
    let (cli_endpoint, cli_service, cli_shutdown) = bind_recording_service().await;
    let root = std::env::temp_dir().join(format!("mtgfr-debug-subprocess-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let request = root.join("request.json");
    std::fs::write(&request, r#"{"tableId":"table-a","operations":[]}"#).unwrap();

    let env_output = run(executable, vec!["tables".into()], env_endpoint.clone()).await;
    assert!(env_output.status.success(), "{env_output:?}");

    let checkpoint_out = root.join("checkpoint.json");
    let restore_out = root.join("restore.json");
    let journal_out = root.join("journal.json");
    let commands = [
        (vec!["inspect".into(), "table-a".into()], None),
        (
            vec![
                "mutate".into(),
                request.to_string_lossy().into_owned(),
                "--expected-table-seq".into(),
                "8".into(),
            ],
            None,
        ),
        (
            vec![
                "checkpoint".into(),
                "table-a".into(),
                "baseline".into(),
                "--replace".into(),
                "--expected-table-seq".into(),
                "8".into(),
                "--out".into(),
                checkpoint_out.to_string_lossy().into_owned(),
            ],
            Some(checkpoint_out),
        ),
        (
            vec![
                "restore".into(),
                "table-a".into(),
                "baseline".into(),
                "--expected-debug-revision".into(),
                "3".into(),
                "--expected-table-seq".into(),
                "8".into(),
                "--out".into(),
                restore_out.to_string_lossy().into_owned(),
            ],
            Some(restore_out),
        ),
        (
            vec![
                "journal".into(),
                "table-a".into(),
                "--out".into(),
                journal_out.to_string_lossy().into_owned(),
            ],
            Some(journal_out),
        ),
    ];
    for (mut args, out) in commands {
        args.extend(["--endpoint".into(), cli_endpoint.clone()]);
        let output = run(executable, args, env_endpoint.clone()).await;
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
        if let Some(out) = out {
            assert!(output.stdout.is_empty(), "{output:?}");
            assert!(std::fs::read(&out).unwrap().starts_with(b"{\n"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                assert_eq!(std::fs::metadata(out).unwrap().mode() & 0o777, 0o600);
            }
        } else {
            assert!(output.stdout.starts_with(b"{\n"), "{output:?}");
        }
    }

    assert_eq!(*env_service.calls.lock().unwrap(), ["tables"]);
    assert_eq!(
        *cli_service.calls.lock().unwrap(),
        [
            "inspect:table-a",
            "mutate:table-a:Some(8)",
            "checkpoint:table-a:baseline:true:Some(8)",
            "restore:table-a:baseline:Some(3):Some(8)",
            "journal:table-a",
        ]
    );
    let _ = env_shutdown.send(());
    let _ = cli_shutdown.send(());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn subprocess_maps_each_mutation_table_seq_source_to_the_request() {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (endpoint, service, shutdown) = bind_recording_service().await;
    let root = std::env::temp_dir().join(format!(
        "mtgfr-debug-mutation-guards-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let cases = [
        ("json-only", Some(11), None),
        ("flag-only", None, Some(22)),
        ("matching-both", Some(33), Some(33)),
    ];
    for (table, json_seq, flag_seq) in cases {
        let request = root.join(format!("{table}.json"));
        let expected_table_seq = json_seq
            .map(|seq| format!(r#","expectedTableSeq":"{seq}""#))
            .unwrap_or_default();
        std::fs::write(
            &request,
            format!(r#"{{"tableId":"{table}"{expected_table_seq},"operations":[]}}"#),
        )
        .unwrap();
        let mut args = vec!["mutate".into(), request.to_string_lossy().into_owned()];
        if let Some(seq) = flag_seq {
            args.extend(["--expected-table-seq".into(), seq.to_string()]);
        }
        args.extend(["--endpoint".into(), endpoint.clone()]);

        let output = run(executable, args, endpoint.clone()).await;
        assert!(output.status.success(), "{table}: {output:?}");
        assert!(output.stderr.is_empty(), "{table}: {output:?}");
    }

    assert_eq!(
        *service.calls.lock().unwrap(),
        [
            "mutate:json-only:Some(11)",
            "mutate:flag-only:Some(22)",
            "mutate:matching-both:Some(33)",
        ]
    );
    let _ = shutdown.send(());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn subprocess_rejects_conflicting_mutation_guards_before_connecting_without_echoes() {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (env_endpoint, service, shutdown) = bind_recording_service().await;
    let root = std::env::temp_dir().join(format!(
        "mtgfr-debug-SECRET-conflict-path-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let request = root.join("SECRET-request-file.json");
    let secret_table = "SECRET-conflict-table";
    let secret_body_value = "SECRET-conflict-body-card";
    let body = format!(
        r#"{{"tableId":"{secret_table}","expectedTableSeq":"44","operations":[{{"createCard":{{"objectId":9001,"cardId":"{secret_body_value}","owner":1,"controller":1}}}}]}}"#
    );
    std::fs::write(&request, &body).unwrap();
    let unreachable_endpoint = "http://127.0.0.1:1/SECRET-unreachable-endpoint";

    let output = run(
        executable,
        vec![
            "mutate".into(),
            request.to_string_lossy().into_owned(),
            "--expected-table-seq".into(),
            "45".into(),
            "--endpoint".into(),
            unreachable_endpoint.into(),
        ],
        env_endpoint,
    )
    .await;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr, "conflicting expected_table_seq values\n");
    let request_path = request.to_string_lossy().into_owned();
    for secret in [
        secret_table,
        secret_body_value,
        body.as_str(),
        request_path.as_str(),
        unreachable_endpoint,
        "connect",
        "Connection refused",
        "UNAVAILABLE",
    ] {
        assert!(
            !stderr.contains(secret),
            "stderr leaked {secret:?}: {stderr:?}"
        );
    }
    assert!(service.calls.lock().unwrap().is_empty());
    let _ = shutdown.send(());
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn subprocess_rpc_errors_do_not_echo_table_or_checkpoint_names() {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (endpoint, _service, shutdown) = bind_recording_service().await;
    let secret_table = "SECRET-table-identifier";
    let secret_checkpoint = "SECRET-checkpoint-name";
    let output = run(
        executable,
        vec![
            "checkpoint".into(),
            secret_table.into(),
            secret_checkpoint.into(),
            "--endpoint".into(),
            endpoint.clone(),
        ],
        endpoint,
    )
    .await;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("NOT_FOUND"));
    assert!(stderr.contains("DEBUG_ERROR_REASON_CHECKPOINT_NOT_FOUND"));
    assert!(!stderr.contains(secret_table));
    assert!(!stderr.contains(secret_checkpoint));
    let _ = shutdown.send(());
}

#[tokio::test]
async fn stack_fixture_uses_authoritative_frontiers_for_one_exact_checked_atomic_batch() {
    use pb::mutation::Operation;
    use pb::stack_entry_spec::Kind;

    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (endpoint, service, shutdown) = bind_recording_service().await;
    let output = run(
        executable,
        vec![
            "stack-fixture".into(),
            "fixture-table".into(),
            "--endpoint".into(),
            endpoint.clone(),
        ],
        endpoint,
    )
    .await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        *service.calls.lock().unwrap(),
        ["inspect:fixture-table", "mutate:fixture-table:Some(3)"]
    );

    let card_ids = [
        "53f7c868-b03e-4fc2-8dcf-a75bbfa3272b",
        "27e9db49-7af7-4bef-ad4c-bf5dfb92030d",
        "d0209d3f-3f7e-4fd5-bce5-10bce6f29c86",
        "f671e3c3-cd59-4d06-a1af-5d04892cf74d",
        "7ffae8f8-3006-4969-a339-6d30678f87ea",
        "6d42dff5-97ec-4768-b112-84a3584d0b87",
    ];
    let printing_ids = [
        "11e12a84-e7be-4afc-a230-c2e644743fa8",
        "ba0a14ac-037a-42f2-8fc9-2a41275dc7da",
        "ade7d00d-4e7b-46e9-ace1-63f628a589fc",
        "fc24f763-4c7f-45e4-933b-573d1ace1ddc",
        "1ed2b60b-5f20-4ae6-93fb-51540ad9c9e7",
        "92b8cffe-4466-48b0-836e-51ba5db36696",
    ];
    let mut expected_operations = Vec::new();
    for (index, card_id) in card_ids.into_iter().enumerate() {
        expected_operations.push(pb::Mutation {
            operation: Some(Operation::CreateCard(pb::CreateCard {
                object_id: 17 + index as u32,
                card_id: card_id.into(),
                owner: 1,
                controller: 1,
                destination: pb::Zone::Hand as i32,
                commander: false,
                face_down: false,
            })),
        });
    }
    for (index, printing_id) in printing_ids.into_iter().enumerate() {
        expected_operations.push(pb::Mutation {
            operation: Some(Operation::SetObjectPrintOverride(
                pb::SetObjectPrintOverride {
                    object_id: 17 + index as u32,
                    printing_id: Some(printing_id.into()),
                },
            )),
        });
    }
    let mut entries = Vec::new();
    for index in 0..6 {
        entries.push(pb::StackEntrySpec {
            kind: Some(Kind::KnownSpell(pb::KnownSpellStackEntry {
                entry_id: 41 + index,
                from_object_id: 17 + index as u32,
                spell_object_id: 23 + index as u32,
                controller: 1,
                targets: vec![],
                targets_second: vec![],
                x: 0,
            })),
        });
    }
    entries.push(pb::StackEntrySpec {
        kind: Some(Kind::PublicGhost(pb::PublicGhostStackEntry {
            entry_id: 47,
            controller: 1,
            name: "Priority Demonstration".into(),
            label: "Debug no-op stack entry".into(),
            printing_id: "91fdb56b-54d5-4272-8319-505ff987fe9b".into(),
            card_id: None,
            printed_sentences: vec!["This debug stack entry resolves without an effect.".into()],
        })),
    });
    expected_operations.push(pb::Mutation {
        operation: Some(Operation::ReplaceStack(pb::ReplaceStack { entries })),
    });
    let expected = pb::MutateTableRequest {
        table_id: "fixture-table".into(),
        expected_debug_revision: Some(2),
        expected_table_seq: Some(3),
        operations: expected_operations,
    };
    assert_eq!(*service.mutation_requests.lock().unwrap(), [expected]);

    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(!stdout.contains("53f7c868"), "{stdout}");
    assert!(!stdout.contains("Priority Demonstration"), "{stdout}");
    assert!(!stdout.contains("operations"), "{stdout}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let _ = shutdown.send(());
}

async fn assert_stack_fixture_rejected_before_mutation(
    game: pb::GameInspection,
    expected_error: &str,
) {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (endpoint, service, shutdown) = bind_recording_service().await;
    *service.inspection.lock().unwrap() = game;
    let output = run(
        executable,
        vec![
            "stack-fixture".into(),
            "fixture-table".into(),
            "--endpoint".into(),
            endpoint.clone(),
        ],
        endpoint,
    )
    .await;
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8(output.stderr.clone())
            .unwrap()
            .contains(expected_error),
        "{output:?}"
    );
    assert_eq!(*service.calls.lock().unwrap(), ["inspect:fixture-table"]);
    assert!(service.mutation_requests.lock().unwrap().is_empty());
    let _ = shutdown.send(());
}

#[tokio::test]
async fn stack_fixture_rejects_absent_exhausted_frontiers_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: None,
            next_stack_entry_id: None,
            ..Default::default()
        },
        "fixture object ids exhausted",
    )
    .await;
}

#[tokio::test]
async fn stack_fixture_rejects_absent_stack_frontier_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: Some(1),
            next_stack_entry_id: None,
            ..Default::default()
        },
        "fixture stack ids exhausted",
    )
    .await;
}

#[tokio::test]
async fn stack_fixture_rejects_object_frontier_without_room_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: Some(u64::from(u32::MAX) - 10),
            next_stack_entry_id: Some(1),
            ..Default::default()
        },
        "fixture object ids exhausted",
    )
    .await;
}

#[tokio::test]
async fn stack_fixture_rejects_object_frontier_arithmetic_overflow_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: Some(u64::MAX),
            next_stack_entry_id: Some(1),
            ..Default::default()
        },
        "fixture object ids exhausted",
    )
    .await;
}

async fn run_accepted_stack_fixture(game: pb::GameInspection) -> pb::MutateTableRequest {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (endpoint, service, shutdown) = bind_recording_service().await;
    *service.inspection.lock().unwrap() = game;
    let output = run(
        executable,
        vec![
            "stack-fixture".into(),
            "fixture-table".into(),
            "--endpoint".into(),
            endpoint.clone(),
        ],
        endpoint,
    )
    .await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        *service.calls.lock().unwrap(),
        ["inspect:fixture-table", "mutate:fixture-table:Some(3)"]
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    let request = service.mutation_requests.lock().unwrap()[0].clone();
    let _ = shutdown.send(());
    request
}

#[tokio::test]
async fn stack_fixture_accepts_zero_object_frontier_and_builds_zero_based_identities() {
    use pb::mutation::Operation;
    use pb::stack_entry_spec::Kind;

    let request = run_accepted_stack_fixture(pb::GameInspection {
        active_player: 1,
        next_object_id: Some(0),
        next_stack_entry_id: Some(1),
        ..Default::default()
    })
    .await;

    let created: Vec<_> = request.operations[..6]
        .iter()
        .map(|mutation| match mutation.operation.as_ref().unwrap() {
            Operation::CreateCard(create) => create.object_id,
            operation => panic!("expected create-card operation, got {operation:?}"),
        })
        .collect();
    assert_eq!(created, (0..6).collect::<Vec<_>>());

    let Operation::ReplaceStack(replace) = request
        .operations
        .last()
        .unwrap()
        .operation
        .as_ref()
        .unwrap()
    else {
        panic!("expected replace-stack operation");
    };
    for (index, entry) in replace.entries[..6].iter().enumerate() {
        let Some(Kind::KnownSpell(spell)) = entry.kind.as_ref() else {
            panic!("expected known-spell entry");
        };
        assert_eq!(spell.entry_id, 1 + index as u64);
        assert_eq!(spell.from_object_id, index as u32);
        assert_eq!(spell.spell_object_id, 6 + index as u32);
    }
    let Some(Kind::PublicGhost(ghost)) = replace.entries[6].kind.as_ref() else {
        panic!("expected public-ghost entry");
    };
    assert_eq!(ghost.entry_id, 7);
}

#[tokio::test]
async fn stack_fixture_accepts_exact_object_frontier_upper_boundary() {
    run_accepted_stack_fixture(pb::GameInspection {
        active_player: 1,
        next_object_id: Some(u64::from(u32::MAX) - 11),
        next_stack_entry_id: Some(1),
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn stack_fixture_accepts_exact_stack_frontier_upper_boundary() {
    run_accepted_stack_fixture(pb::GameInspection {
        active_player: 1,
        next_object_id: Some(0),
        next_stack_entry_id: Some(u64::MAX - 6),
        ..Default::default()
    })
    .await;
}

#[tokio::test]
async fn stack_fixture_rejects_zero_stack_frontier_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: Some(0),
            next_stack_entry_id: Some(0),
            ..Default::default()
        },
        "fixture stack ids exhausted",
    )
    .await;
}

#[tokio::test]
async fn stack_fixture_rejects_stack_frontier_without_room_before_mutation() {
    assert_stack_fixture_rejected_before_mutation(
        pb::GameInspection {
            next_object_id: Some(1),
            next_stack_entry_id: Some(u64::MAX - 5),
            ..Default::default()
        },
        "fixture stack ids exhausted",
    )
    .await;
}

#[test]
fn protobuf_json_round_trips_all_stack_operations_and_entry_kinds() {
    use pb::mutation::Operation;
    use pb::stack_entry_spec::Kind;
    let entries = vec![
        pb::StackEntrySpec {
            kind: Some(Kind::KnownSpell(pb::KnownSpellStackEntry {
                entry_id: 1,
                ..Default::default()
            })),
        },
        pb::StackEntrySpec {
            kind: Some(Kind::AuthoredAbility(pb::AuthoredAbilityStackEntry {
                entry_id: 2,
                ..Default::default()
            })),
        },
        pb::StackEntrySpec {
            kind: Some(Kind::PublicGhost(pb::PublicGhostStackEntry {
                entry_id: 3,
                name: "ghost".into(),
                label: "label".into(),
                printing_id: "print".into(),
                ..Default::default()
            })),
        },
    ];
    let request = pb::MutateTableRequest {
        table_id: "table".into(),
        expected_debug_revision: Some(4),
        expected_table_seq: Some(5),
        operations: vec![
            pb::Mutation {
                operation: Some(Operation::ReplaceStack(pb::ReplaceStack {
                    entries: entries.clone(),
                })),
            },
            pb::Mutation {
                operation: Some(Operation::PushStack(pb::PushStack {
                    entry: Some(entries[2].clone()),
                })),
            },
            pb::Mutation {
                operation: Some(Operation::PopStack(pb::PopStack { count: 1 })),
            },
            pb::Mutation {
                operation: Some(Operation::SetObjectPrintOverride(
                    pb::SetObjectPrintOverride {
                        object_id: 7,
                        printing_id: Some("print".into()),
                    },
                )),
            },
        ],
    };
    let json = serde_json::to_string(&request).unwrap();
    let decoded: pb::MutateTableRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, request);
}
