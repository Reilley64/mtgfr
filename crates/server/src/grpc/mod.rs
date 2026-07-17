//! The tonic gRPC server: `.proto` is the sole wire contract, served on `Settings::grpc_port`.
//! Each `*_svc` calls the same transport-agnostic cores as before (`*_core`, `stream::subscribe`).

pub mod auth_ctx;
mod auth_svc;
mod cards_svc;
mod decks_svc;
mod game_svc;
pub(crate) mod map;
mod tables_svc;
mod trace;
#[cfg(test)]
mod tests;

/// Generated types and service traits from `proto/mtgfr/v1/*.proto`.
pub mod pb {
    tonic::include_proto!("mtgfr.v1");
}

use std::future::Future;
use std::net::SocketAddr;

use tonic::transport::Server;

use crate::AppState;

use self::trace::TraceLayer;

/// Build and serve every gRPC service on `addr`, sharing `state` with the Axum app. Runs until
/// `shutdown` resolves (SIGTERM/Ctrl-C — see `main.rs::await_shutdown_signal`).
pub async fn serve(
    addr: SocketAddr,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
        .layer(TraceLayer)
        .add_service(pb::auth_server::AuthServer::new(auth_svc::AuthSvc::new(
            state.clone(),
        )))
        .add_service(pb::decks_server::DecksServer::new(
            decks_svc::DecksSvc::new(state.clone()),
        ))
        .add_service(pb::cards_server::CardsServer::new(
            cards_svc::CardsSvc::new(state.clone()),
        ))
        .add_service(pb::game_server::GameServer::new(game_svc::GameSvc::new(
            state.clone(),
        )))
        .add_service(pb::tables_server::TablesServer::new(
            tables_svc::TablesSvc::new(state),
        ))
        .serve_with_shutdown(addr, shutdown)
        .await
}
