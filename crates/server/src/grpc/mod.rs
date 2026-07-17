//! The tonic gRPC server (ADR 0032): `.proto` is the sole wire contract, and this module hosts
//! every RPC on its own port (`Settings::grpc_port`) alongside the legacy Axum `app()` — a hard
//! cutover to gRPC-only happens once the BFF client lands (see `docs/adr/0032-*.md`).
//!
//! Each `*_svc` submodule implements one proto service by calling the same transport-agnostic
//! handler logic the HTTP routes call (`game_loop::*_core`, `decks_api::*_core`,
//! `lobby::seed_table_core`, `stream::subscribe`) — HTTP and gRPC can't drift apart on behavior,
//! only on wire shape.

pub mod auth_ctx;
mod auth_svc;
mod cards_svc;
mod decks_svc;
mod game_svc;
pub(crate) mod map;
mod tables_svc;
#[cfg(test)]
mod tests;

/// Generated types (`Empty`, `Ack`, `StreamFrame`, …) and service traits/servers from
/// `proto/mtgfr/v1/*.proto` (native payloads — no JSON-in-string wrappers).
pub mod pb {
    tonic::include_proto!("mtgfr.v1");
}

use std::future::Future;
use std::net::SocketAddr;

use tonic::transport::Server;

use crate::AppState;

/// Build and serve every gRPC service on `addr`, sharing `state` with the Axum app. Runs until
/// `shutdown` resolves (SIGTERM/Ctrl-C — see `main.rs::await_shutdown_signal`).
pub async fn serve(
    addr: SocketAddr,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    Server::builder()
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
