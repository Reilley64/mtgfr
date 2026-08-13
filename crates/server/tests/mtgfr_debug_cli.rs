#![cfg(debug_assertions)]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use server::grpc::debug_pb as pb;
use tokio_stream::wrappers::TcpListenerStream;

#[derive(Clone, Default)]
struct RecordingService {
    calls: Arc<AtomicUsize>,
}

#[tonic::async_trait]
impl pb::debug_service_server::DebugService for RecordingService {
    async fn list_tables(
        &self,
        _: tonic::Request<pb::ListTablesRequest>,
    ) -> Result<tonic::Response<pb::ListTablesResponse>, tonic::Status> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(tonic::Response::new(pb::ListTablesResponse {
            tables: vec![],
        }))
    }

    async fn inspect_table(
        &self,
        _: tonic::Request<pb::InspectTableRequest>,
    ) -> Result<tonic::Response<pb::InspectTableResponse>, tonic::Status> {
        unreachable!("environment test only invokes tables")
    }

    async fn mutate_table(
        &self,
        _: tonic::Request<pb::MutateTableRequest>,
    ) -> Result<tonic::Response<pb::MutateTableResponse>, tonic::Status> {
        unreachable!("environment test only invokes tables")
    }

    async fn checkpoint_table(
        &self,
        _: tonic::Request<pb::CheckpointTableRequest>,
    ) -> Result<tonic::Response<pb::CheckpointTableResponse>, tonic::Status> {
        unreachable!("environment test only invokes tables")
    }

    async fn restore_checkpoint(
        &self,
        _: tonic::Request<pb::RestoreCheckpointRequest>,
    ) -> Result<tonic::Response<pb::RestoreCheckpointResponse>, tonic::Status> {
        unreachable!("environment test only invokes tables")
    }

    async fn get_debug_journal(
        &self,
        _: tonic::Request<pb::GetDebugJournalRequest>,
    ) -> Result<tonic::Response<pb::GetDebugJournalResponse>, tonic::Status> {
        unreachable!("environment test only invokes tables")
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

#[tokio::test]
async fn subprocess_reads_environment_endpoint_and_cli_endpoint_takes_precedence() {
    let executable = env!("CARGO_BIN_EXE_mtgfr-debug");
    let (env_endpoint, env_service, env_shutdown) = bind_recording_service().await;
    let (cli_endpoint, cli_service, cli_shutdown) = bind_recording_service().await;
    let env_endpoint_for_first_call = env_endpoint.clone();

    let env_output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(executable)
            .arg("tables")
            .env("MTGFR_DEBUG_ENDPOINT", env_endpoint_for_first_call)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(env_output.status.success(), "{env_output:?}");
    assert_eq!(env_service.calls.load(Ordering::SeqCst), 1);
    assert_eq!(cli_service.calls.load(Ordering::SeqCst), 0);

    let cli_output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(executable)
            .args(["tables", "--endpoint", &cli_endpoint])
            .env("MTGFR_DEBUG_ENDPOINT", env_endpoint)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(cli_output.status.success(), "{cli_output:?}");

    assert_eq!(env_service.calls.load(Ordering::SeqCst), 1);
    assert_eq!(cli_service.calls.load(Ordering::SeqCst), 1);
    let _ = env_shutdown.send(());
    let _ = cli_shutdown.send(());
}
