#![cfg(debug_assertions)]

use std::sync::{Arc, Mutex};

use prost::Message;
use server::grpc::debug_pb as pb;
use tokio_stream::wrappers::TcpListenerStream;

#[derive(Clone, Default)]
struct RecordingService {
    calls: Arc<Mutex<Vec<String>>>,
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
