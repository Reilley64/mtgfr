#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("mtgfr-debug is unavailable in release builds");
    std::process::exit(2);
}

#[cfg(debug_assertions)]
mod debug_cli {
    use std::ffi::{OsStr, OsString};
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use clap::{Parser, Subcommand};
    use prost::Message;
    use serde::Serialize;
    use server::grpc::debug_pb as pb;

    const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:50051";
    const MAX_VIOLATIONS: usize = 16;
    const MAX_VIOLATION_TEXT_BYTES: usize = 256;
    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[derive(Debug, Parser, PartialEq)]
    #[command(
        name = "mtgfr-debug",
        about = "Inspect and mutate authoritative development tables"
    )]
    struct Cli {
        /// Debug gRPC endpoint (overrides MTGFR_DEBUG_ENDPOINT)
        #[arg(long, global = true, value_name = "URL")]
        endpoint: Option<String>,
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Debug, Subcommand, PartialEq)]
    enum Command {
        /// List active tables and revisions
        Tables,
        /// Inspect one authoritative table
        Inspect {
            table: String,
            /// Atomically write protobuf JSON instead of stdout
            #[arg(long)]
            out: Option<PathBuf>,
        },
        /// Apply one protobuf-JSON mutation batch from a named file
        Mutate {
            request_json: PathBuf,
            /// Atomically write protobuf JSON instead of stdout
            #[arg(long)]
            out: Option<PathBuf>,
        },
    }

    #[derive(Debug)]
    enum CliError {
        Status(tonic::Status),
        Generic(&'static str),
    }

    #[derive(Serialize)]
    struct StatusOutput {
        code: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        operation_index: Option<u32>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        violations: Vec<ViolationOutput>,
    }

    #[derive(Serialize)]
    struct ViolationOutput {
        code: String,
        message: String,
    }

    pub async fn run() -> i32 {
        run_cli(Cli::parse()).await
    }

    async fn run_cli(cli: Cli) -> i32 {
        let env_endpoint = std::env::var("MTGFR_DEBUG_ENDPOINT").ok();
        let endpoint = match resolve_endpoint(cli.endpoint.as_deref(), env_endpoint.as_deref()) {
            Ok(endpoint) => endpoint,
            Err(()) => {
                eprintln!("invalid debug endpoint");
                return 2;
            }
        };
        match execute(endpoint, cli.command).await {
            Ok((json, out)) => match write_output(out.as_deref(), &format!("{json}\n")) {
                Ok(()) => 0,
                Err(()) => {
                    eprintln!("failed to write debug response");
                    1
                }
            },
            Err(CliError::Status(status)) => {
                eprintln!("{}", render_status(&status));
                1
            }
            Err(CliError::Generic(message)) => {
                eprintln!("{message}");
                1
            }
        }
    }

    async fn execute(
        endpoint: &str,
        command: Command,
    ) -> Result<(String, Option<PathBuf>), CliError> {
        let command = match command {
            Command::Mutate { request_json, out } => {
                let bytes = fs::read(request_json)
                    .map_err(|_| CliError::Generic("failed to read mutation request file"))?;
                let request = parse_mutation(&bytes)
                    .map_err(|_| CliError::Generic("invalid protobuf JSON mutation request"))?;
                PreparedCommand::Mutate { request, out }
            }
            Command::Tables => PreparedCommand::Tables,
            Command::Inspect { table, out } => PreparedCommand::Inspect { table, out },
        };
        let mut client = pb::debug_service_client::DebugServiceClient::connect(endpoint.to_owned())
            .await
            .map_err(|_| CliError::Generic("failed to connect to debug endpoint"))?;
        match command {
            PreparedCommand::Tables => {
                let response = client
                    .list_tables(pb::ListTablesRequest {})
                    .await
                    .map_err(CliError::Status)?
                    .into_inner();
                let json = protobuf_json(&response)
                    .map_err(|_| CliError::Generic("failed to encode debug response"))?;
                Ok((json, None))
            }
            PreparedCommand::Inspect { table, out } => {
                let response = client
                    .inspect_table(pb::InspectTableRequest { table_id: table })
                    .await
                    .map_err(CliError::Status)?
                    .into_inner();
                let json = protobuf_json(&response)
                    .map_err(|_| CliError::Generic("failed to encode debug response"))?;
                Ok((json, out))
            }
            PreparedCommand::Mutate { request, out } => {
                let response = client
                    .mutate_table(request)
                    .await
                    .map_err(CliError::Status)?
                    .into_inner();
                let json = protobuf_json(&response)
                    .map_err(|_| CliError::Generic("failed to encode debug response"))?;
                Ok((json, out))
            }
        }
    }

    enum PreparedCommand {
        Tables,
        Inspect {
            table: String,
            out: Option<PathBuf>,
        },
        Mutate {
            request: pb::MutateTableRequest,
            out: Option<PathBuf>,
        },
    }

    fn resolve_endpoint<'a>(cli: Option<&'a str>, env: Option<&'a str>) -> Result<&'a str, ()> {
        let endpoint = match cli {
            Some(value) => value,
            None => env
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(DEFAULT_ENDPOINT),
        };
        if endpoint.trim() != endpoint || endpoint.is_empty() {
            return Err(());
        }
        let uri: tonic::codegen::http::Uri = endpoint.parse().map_err(|_| ())?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.authority().is_none() {
            return Err(());
        }
        tonic::transport::Endpoint::from_shared(endpoint.to_owned()).map_err(|_| ())?;
        Ok(endpoint)
    }

    fn parse_mutation(bytes: &[u8]) -> Result<pb::MutateTableRequest, serde_json::Error> {
        serde_json::from_slice(bytes)
    }

    fn protobuf_json(message: &impl Serialize) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(message)
    }

    fn write_output(path: Option<&Path>, contents: &str) -> Result<(), ()> {
        let Some(path) = path else {
            print!("{contents}");
            return std::io::stdout().flush().map_err(|_| ());
        };
        atomic_write(path, contents.as_bytes())
    }

    fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), ()> {
        atomic_write_with_permissions(path, contents, |temporary_path, permissions| {
            fs::set_permissions(temporary_path, permissions)
        })
    }

    fn atomic_write_with_permissions(
        path: &Path,
        contents: &[u8],
        set_permissions: impl FnOnce(&Path, fs::Permissions) -> std::io::Result<()>,
    ) -> Result<(), ()> {
        let file_name = path.file_name().filter(|name| !name.is_empty()).ok_or(())?;
        let intended_permissions = intended_permissions(path);
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let (temporary_path, mut temporary) = create_temporary(parent, file_name)?;
        let result = (|| {
            temporary.write_all(contents).map_err(|_| ())?;
            temporary.sync_all().map_err(|_| ())?;
            if let Some(permissions) = intended_permissions {
                set_permissions(&temporary_path, permissions).map_err(|_| ())?;
                temporary.sync_all().map_err(|_| ())?;
            }
            drop(temporary);
            fs::rename(&temporary_path, path).map_err(|_| ())?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }
        result
    }

    fn intended_permissions(path: &Path) -> Option<fs::Permissions> {
        if let Ok(metadata) = fs::metadata(path) {
            return Some(metadata.permissions());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            Some(fs::Permissions::from_mode(0o600))
        }
        #[cfg(not(unix))]
        {
            None
        }
    }

    fn create_temporary(parent: &Path, file_name: &OsStr) -> Result<(PathBuf, fs::File), ()> {
        for _ in 0..32 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let mut temporary_name = OsString::from(".");
            temporary_name.push(file_name);
            temporary_name.push(format!(".{}.{}.tmp", std::process::id(), sequence));
            let temporary_path = parent.join(temporary_name);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            match options.open(&temporary_path) {
                Ok(file) => return Ok((temporary_path, file)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(()),
            }
        }
        Err(())
    }

    fn render_status(status: &tonic::Status) -> String {
        let detail = pb::ErrorDetail::decode(status.details())
            .ok()
            .and_then(|detail| {
                let reason = pb::DebugErrorReason::try_from(detail.reason).ok()?;
                if reason == pb::DebugErrorReason::Unspecified {
                    return None;
                }
                Some((reason, detail))
            });
        let (reason, operation_index, violations) =
            detail.map_or((None, None, vec![]), |(reason, detail)| {
                let violations = detail
                    .violations
                    .into_iter()
                    .take(MAX_VIOLATIONS)
                    .map(|violation| ViolationOutput {
                        code: bounded_text(&violation.code),
                        message: bounded_text(&violation.message),
                    })
                    .collect();
                (
                    Some(reason.as_str_name().to_string()),
                    detail.operation_index,
                    violations,
                )
            });
        serde_json::to_string_pretty(&StatusOutput {
            code: status_code_name(status.code()),
            reason,
            operation_index,
            violations,
        })
        .expect("status output contains only serializable values")
    }

    fn bounded_text(value: &str) -> String {
        if value.len() <= MAX_VIOLATION_TEXT_BYTES {
            return value.to_string();
        }
        let mut end = MAX_VIOLATION_TEXT_BYTES;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        value[..end].to_string()
    }

    fn status_code_name(code: tonic::Code) -> &'static str {
        match code {
            tonic::Code::Ok => "OK",
            tonic::Code::Cancelled => "CANCELLED",
            tonic::Code::Unknown => "UNKNOWN",
            tonic::Code::InvalidArgument => "INVALID_ARGUMENT",
            tonic::Code::DeadlineExceeded => "DEADLINE_EXCEEDED",
            tonic::Code::NotFound => "NOT_FOUND",
            tonic::Code::AlreadyExists => "ALREADY_EXISTS",
            tonic::Code::PermissionDenied => "PERMISSION_DENIED",
            tonic::Code::ResourceExhausted => "RESOURCE_EXHAUSTED",
            tonic::Code::FailedPrecondition => "FAILED_PRECONDITION",
            tonic::Code::Aborted => "ABORTED",
            tonic::Code::OutOfRange => "OUT_OF_RANGE",
            tonic::Code::Unimplemented => "UNIMPLEMENTED",
            tonic::Code::Internal => "INTERNAL",
            tonic::Code::Unavailable => "UNAVAILABLE",
            tonic::Code::DataLoss => "DATA_LOSS",
            tonic::Code::Unauthenticated => "UNAUTHENTICATED",
        }
    }

    #[cfg(test)]
    mod tests {
        use std::fs;

        use clap::Parser;
        use prost::Message;

        use super::*;

        #[test]
        fn parses_only_phase_a_commands_and_global_endpoint() {
            assert_eq!(
                Cli::try_parse_from(["mtgfr-debug", "tables"]).unwrap(),
                Cli {
                    endpoint: None,
                    command: Command::Tables
                }
            );
            assert_eq!(
                Cli::try_parse_from([
                    "mtgfr-debug",
                    "inspect",
                    "table-a",
                    "--out",
                    "inspection.json",
                    "--endpoint",
                    "http://debug.test:6000"
                ])
                .unwrap(),
                Cli {
                    endpoint: Some("http://debug.test:6000".into()),
                    command: Command::Inspect {
                        table: "table-a".into(),
                        out: Some("inspection.json".into()),
                    },
                }
            );
            assert_eq!(
                Cli::try_parse_from([
                    "mtgfr-debug",
                    "mutate",
                    "request.json",
                    "--out",
                    "response.json"
                ])
                .unwrap(),
                Cli {
                    endpoint: None,
                    command: Command::Mutate {
                        request_json: "request.json".into(),
                        out: Some("response.json".into()),
                    },
                }
            );
            for unavailable in ["checkpoint", "restore", "journal"] {
                assert!(Cli::try_parse_from(["mtgfr-debug", unavailable]).is_err());
            }
            assert!(Cli::try_parse_from(["mtgfr-debug", "mutate"]).is_err());
        }

        #[test]
        fn endpoint_precedence_is_cli_then_env_then_default() {
            assert_eq!(
                resolve_endpoint(Some("http://cli"), Some("http://env")).unwrap(),
                "http://cli"
            );
            assert_eq!(
                resolve_endpoint(None, Some("http://env")).unwrap(),
                "http://env"
            );
            assert_eq!(resolve_endpoint(None, None).unwrap(), DEFAULT_ENDPOINT);
            assert_eq!(resolve_endpoint(None, Some("")).unwrap(), DEFAULT_ENDPOINT);
        }

        #[test]
        fn protobuf_json_accepts_enum_strings_and_oneof_names() {
            let json = r#"{
              "tableId": "table-a",
              "expectedDebugRevision": "3",
              "expectedTableSeq": "7",
              "operations": [
                {"setPlayerCounter": {"player": 1, "counter": "PLAYER_COUNTER_RAD", "value": 2}},
                {"moveCard": {"objectId": 4, "newObjectId": 9, "destination": "ZONE_HAND", "controller": 1}}
              ]
            }"#;
            let request = parse_mutation(json.as_bytes()).unwrap();
            assert_eq!(request.table_id, "table-a");
            assert_eq!(request.expected_debug_revision, Some(3));
            assert_eq!(request.expected_table_seq, Some(7));
            assert!(matches!(
                request.operations[0].operation,
                Some(pb::mutation::Operation::SetPlayerCounter(
                    pb::SetPlayerCounter { counter: 2, .. }
                ))
            ));
            assert!(matches!(
                request.operations[1].operation,
                Some(pb::mutation::Operation::MoveCard(pb::MoveCard {
                    destination: 2,
                    ..
                }))
            ));
        }

        #[test]
        fn protobuf_success_is_pretty_json() {
            let response = pb::MutateTableResponse {
                debug_revision: 8,
                table_seq: 12,
                applied_operation_count: 2,
            };
            let json = protobuf_json(&response).unwrap();
            assert!(json.starts_with("{\n"));
            assert!(json.contains("  \"debugRevision\": \"8\""));
            assert!(json.contains("  \"appliedOperationCount\": 2"));
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&json).unwrap()["tableSeq"],
                "12"
            );
        }

        #[test]
        fn output_is_replaced_atomically_without_leftover_temp_files() {
            let root = std::env::temp_dir().join(format!("mtgfr-debug-cli-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let output = root.join("inspection.json");
            fs::write(&output, "old").unwrap();

            write_output(Some(&output), "new response\n").unwrap();

            assert_eq!(fs::read_to_string(&output).unwrap(), "new response\n");
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn invalid_output_paths_fail_without_temp_file_leaks() {
            let root =
                std::env::temp_dir().join(format!("mtgfr-debug-cli-errors-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            assert!(write_output(Some(root.as_path()), "response").is_err());
            assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
            assert!(write_output(Some(std::path::Path::new("/")), "response").is_err());
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn typed_status_output_is_bounded_and_omits_status_and_request_payloads() {
            let request_secret = "Lightning Bolt SECRET-CARD-PAYLOAD";
            let detail = pb::ErrorDetail {
                operation_index: Some(4),
                reason: pb::DebugErrorReason::InvalidValue.into(),
                violations: (0..20)
                    .map(|index| pb::Violation {
                        code: format!("violation_{index}"),
                        message: "candidate violates a structural invariant".repeat(20),
                    })
                    .collect(),
            };
            let status = tonic::Status::with_details(
                tonic::Code::FailedPrecondition,
                request_secret,
                detail.encode_to_vec().into(),
            );

            let rendered = render_status(&status);
            let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
            assert_eq!(json["code"], "FAILED_PRECONDITION");
            assert_eq!(json["reason"], "DEBUG_ERROR_REASON_INVALID_VALUE");
            assert_eq!(json["operation_index"], 4);
            assert_eq!(json["violations"].as_array().unwrap().len(), 16);
            assert!(json["violations"][0]["message"].as_str().unwrap().len() <= 256);
            assert!(!rendered.contains(request_secret));
            assert!(!rendered.contains("details"));
            assert_eq!(json.as_object().unwrap().len(), 4);
        }

        #[test]
        fn status_without_a_meaningful_known_reason_is_code_only() {
            let empty = tonic::Status::new(tonic::Code::Unimplemented, "SECRET");
            assert_eq!(render_status(&empty), "{\n  \"code\": \"UNIMPLEMENTED\"\n}");

            for reason in [pb::DebugErrorReason::Unspecified as i32, 99_999] {
                let detail = pb::ErrorDetail {
                    operation_index: Some(7),
                    reason,
                    violations: vec![pb::Violation {
                        code: "must-not-appear".into(),
                        message: "SECRET".into(),
                    }],
                };
                let status = tonic::Status::with_details(
                    tonic::Code::InvalidArgument,
                    "SECRET",
                    detail.encode_to_vec().into(),
                );
                assert_eq!(
                    render_status(&status),
                    "{\n  \"code\": \"INVALID_ARGUMENT\"\n}"
                );
            }
        }

        #[cfg(unix)]
        #[test]
        fn atomic_output_is_private_and_preserves_existing_permissions() {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            let root =
                std::env::temp_dir().join(format!("mtgfr-debug-cli-modes-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let fresh = root.join("fresh.json");
            atomic_write(&fresh, b"fresh").unwrap();
            assert_eq!(fs::metadata(&fresh).unwrap().mode() & 0o777, 0o600);

            for mode in [0o000, 0o600, 0o644] {
                let existing = root.join(format!("existing-{mode:o}.json"));
                fs::write(&existing, "old").unwrap();
                fs::set_permissions(&existing, fs::Permissions::from_mode(mode)).unwrap();
                atomic_write(&existing, b"new").unwrap();
                assert_eq!(fs::metadata(&existing).unwrap().mode() & 0o777, mode);
                if mode == 0 {
                    fs::set_permissions(&existing, fs::Permissions::from_mode(0o600)).unwrap();
                }
                assert_eq!(fs::read(&existing).unwrap(), b"new");
            }
            fs::remove_dir_all(root).unwrap();
        }

        #[cfg(unix)]
        #[test]
        fn permission_failure_does_not_commit_atomic_output() {
            use std::os::unix::fs::{MetadataExt, PermissionsExt};

            let root = std::env::temp_dir().join(format!(
                "mtgfr-debug-cli-permission-failure-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let output = root.join("existing.json");
            fs::write(&output, "old").unwrap();
            fs::set_permissions(&output, fs::Permissions::from_mode(0o644)).unwrap();

            let result = atomic_write_with_permissions(&output, b"new", |_, _| {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            });

            assert!(result.is_err());
            assert_eq!(fs::read(&output).unwrap(), b"old");
            assert_eq!(fs::metadata(&output).unwrap().mode() & 0o777, 0o644);
            assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
            fs::remove_dir_all(root).unwrap();
        }

        #[cfg(all(unix, not(target_os = "macos")))]
        #[test]
        fn atomic_output_supports_non_utf8_file_names() {
            use std::ffi::OsString;
            use std::os::unix::ffi::OsStringExt;

            let root = std::env::temp_dir()
                .join(format!("mtgfr-debug-cli-non-utf8-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let output = root.join(OsString::from_vec(b"inspection-\xff.json".to_vec()));
            atomic_write(&output, b"private").unwrap();
            assert_eq!(fs::read(&output).unwrap(), b"private");
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn explicit_empty_whitespace_and_invalid_endpoints_are_rejected() {
            for endpoint in ["", "   ", "not a uri", "localhost:50051", "/relative"] {
                assert!(resolve_endpoint(Some(endpoint), Some("http://env")).is_err());
            }
        }

        #[tokio::test]
        async fn mutation_file_errors_are_reported_before_connecting() {
            let missing = std::env::temp_dir().join(format!(
                "mtgfr-debug-missing-request-{}",
                std::process::id()
            ));
            let error = execute(
                "http://127.0.0.1:1",
                Command::Mutate {
                    request_json: missing,
                    out: None,
                },
            )
            .await
            .unwrap_err();
            assert!(matches!(
                error,
                CliError::Generic("failed to read mutation request file")
            ));
        }

        #[derive(Clone, Default)]
        struct RecordingService {
            calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
        }

        #[tonic::async_trait]
        impl pb::debug_service_server::DebugService for RecordingService {
            async fn list_tables(
                &self,
                _: tonic::Request<pb::ListTablesRequest>,
            ) -> Result<tonic::Response<pb::ListTablesResponse>, tonic::Status> {
                self.calls.lock().unwrap().push("tables".into());
                Ok(tonic::Response::new(pb::ListTablesResponse {
                    tables: vec![pb::TableSummary {
                        table_id: "table-a".into(),
                        debug_revision: 2,
                        table_seq: 3,
                    }],
                }))
            }

            async fn inspect_table(
                &self,
                request: tonic::Request<pb::InspectTableRequest>,
            ) -> Result<tonic::Response<pb::InspectTableResponse>, tonic::Status> {
                let table = request.into_inner().table_id;
                self.calls.lock().unwrap().push(format!("inspect:{table}"));
                if table == "missing" {
                    let detail = pb::ErrorDetail {
                        operation_index: None,
                        reason: pb::DebugErrorReason::UnknownEntity.into(),
                        violations: vec![],
                    };
                    return Err(tonic::Status::with_details(
                        tonic::Code::NotFound,
                        "SECRET",
                        detail.encode_to_vec().into(),
                    ));
                }
                Ok(tonic::Response::new(pb::InspectTableResponse {
                    table_id: table,
                    debug_revision: 2,
                    table_seq: 3,
                    debug_mutated: false,
                    game: None,
                    ..Default::default()
                }))
            }

            async fn mutate_table(
                &self,
                request: tonic::Request<pb::MutateTableRequest>,
            ) -> Result<tonic::Response<pb::MutateTableResponse>, tonic::Status> {
                let request = request.into_inner();
                self.calls.lock().unwrap().push(format!(
                    "mutate:{}:{}",
                    request.table_id,
                    request.operations.len()
                ));
                Ok(tonic::Response::new(pb::MutateTableResponse {
                    debug_revision: 4,
                    table_seq: 5,
                    applied_operation_count: request.operations.len() as u32,
                }))
            }

            async fn checkpoint_table(
                &self,
                _: tonic::Request<pb::CheckpointTableRequest>,
            ) -> Result<tonic::Response<pb::CheckpointTableResponse>, tonic::Status> {
                Err(tonic::Status::unimplemented("not used by this test"))
            }

            async fn restore_checkpoint(
                &self,
                _: tonic::Request<pb::RestoreCheckpointRequest>,
            ) -> Result<tonic::Response<pb::RestoreCheckpointResponse>, tonic::Status> {
                Err(tonic::Status::unimplemented("not used by this test"))
            }

            async fn get_debug_journal(
                &self,
                _: tonic::Request<pb::GetDebugJournalRequest>,
            ) -> Result<tonic::Response<pb::GetDebugJournalResponse>, tonic::Status> {
                Err(tonic::Status::unimplemented("not used by this test"))
            }
        }

        async fn bind_recording_service()
        -> (String, RecordingService, tokio::sync::oneshot::Sender<()>) {
            use tokio_stream::wrappers::TcpListenerStream;

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

        #[tokio::test]
        async fn execute_wires_tables_inspect_mutate_and_typed_rpc_errors() {
            let (endpoint, service, shutdown) = bind_recording_service().await;
            let root = std::env::temp_dir().join(format!("mtgfr-debug-rpc-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            let mutation = root.join("request.json");
            fs::write(&mutation, r#"{"tableId":"table-a","operations":[]}"#).unwrap();

            let (tables, _) = execute(&endpoint, Command::Tables).await.unwrap();
            assert!(tables.contains("table-a"));
            let (inspection, _) = execute(
                &endpoint,
                Command::Inspect {
                    table: "table-a".into(),
                    out: None,
                },
            )
            .await
            .unwrap();
            assert!(inspection.contains("table-a"));
            let (mutation_json, _) = execute(
                &endpoint,
                Command::Mutate {
                    request_json: mutation,
                    out: None,
                },
            )
            .await
            .unwrap();
            assert!(mutation_json.contains("debugRevision"));
            let error = execute(
                &endpoint,
                Command::Inspect {
                    table: "missing".into(),
                    out: None,
                },
            )
            .await
            .unwrap_err();
            let CliError::Status(status) = error else {
                panic!("expected status")
            };
            assert_eq!(
                render_status(&status),
                "{\n  \"code\": \"NOT_FOUND\",\n  \"reason\": \"DEBUG_ERROR_REASON_UNKNOWN_ENTITY\"\n}"
            );
            assert_eq!(
                *service.calls.lock().unwrap(),
                [
                    "tables",
                    "inspect:table-a",
                    "mutate:table-a:0",
                    "inspect:missing"
                ]
            );
            let _ = shutdown.send(());
            fs::remove_dir_all(root).unwrap();
        }

        #[test]
        fn undecodable_status_is_still_generic_and_payload_free() {
            let status = tonic::Status::with_details(
                tonic::Code::InvalidArgument,
                "SECRET REQUEST BODY",
                vec![1, 2, 3].into(),
            );
            let rendered = render_status(&status);
            assert_eq!(rendered, "{\n  \"code\": \"INVALID_ARGUMENT\"\n}");
            assert!(!rendered.contains("SECRET"));
        }
    }
}

#[cfg(debug_assertions)]
#[tokio::main]
async fn main() {
    std::process::exit(debug_cli::run().await);
}
