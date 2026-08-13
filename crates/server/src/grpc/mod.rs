//! The tonic gRPC server: `.proto` is the sole wire contract, served on `Settings::grpc_port`.
//! Each `*_svc` calls the same transport-agnostic cores as before (`*_core`, `stream::subscribe`).

pub mod auth_ctx;
mod auth_svc;
mod cards_svc;
#[cfg(debug_assertions)]
mod debug_svc;
mod decks_svc;
mod game_svc;
pub(crate) mod map;
mod ratings_svc;
mod tables_svc;
#[cfg(test)]
mod tests;
mod trace;

/// Generated types and service traits from `proto/mtgfr/v1/*.proto`.
pub mod pb {
    tonic::include_proto!("mtgfr.v1");
}

/// Generated debug-only types, clients, service traits, and protobuf-JSON mappings.
#[cfg(debug_assertions)]
#[allow(clippy::useless_borrows_in_formatting)]
pub mod debug_pb {
    tonic::include_proto!("mtgfr.debug.v1");
    include!(concat!(env!("OUT_DIR"), "/mtgfr.debug.v1.serde.rs"));
}

use std::future::Future;
use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;

use crate::AppState;

use self::trace::TraceLayer;

/// Bind `addr` and serve every gRPC service, sharing `state` with the Axum app. Runs until
/// `shutdown` resolves (SIGTERM/Ctrl-C — see `main.rs::await_shutdown_signal`).
pub async fn serve(
    addr: SocketAddr,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    serve_with_listener(listener, state, shutdown).await
}

/// Serve every gRPC service from an already-bound listener.
///
/// Keeping ownership of the listener across startup lets callers bind port zero without a
/// drop-and-rebind race.
pub async fn serve_with_listener(
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> anyhow::Result<()> {
    let router = production_router(&state);
    serve_router(router, listener, state, shutdown).await?;
    Ok(())
}

type ProductionRouter = tonic::transport::server::Router<
    tower::layer::util::Stack<TraceLayer, tower::layer::util::Identity>,
>;

fn production_router(state: &AppState) -> ProductionRouter {
    Server::builder()
        .layer(TraceLayer)
        .add_service(pb::auth_service_server::AuthServiceServer::new(
            auth_svc::AuthSvc::new(state.clone()),
        ))
        .add_service(pb::decks_service_server::DecksServiceServer::new(
            decks_svc::DecksSvc::new(state.clone()),
        ))
        .add_service(pb::ratings_service_server::RatingsServiceServer::new(
            ratings_svc::RatingsSvc::new(state.clone()),
        ))
        .add_service(pb::cards_service_server::CardsServiceServer::new(
            cards_svc::CardsSvc::new(state.clone()),
        ))
        .add_service(pb::game_service_server::GameServiceServer::new(
            game_svc::GameSvc::new(state.clone()),
        ))
        .add_service(pb::tables_service_server::TablesServiceServer::new(
            tables_svc::TablesSvc::new(state.clone()),
        ))
}

#[cfg(debug_assertions)]
async fn serve_router(
    router: ProductionRouter,
    listener: TcpListener,
    state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    router
        .add_service(debug_pb::debug_service_server::DebugServiceServer::new(
            debug_svc::DebugSvc::new(state),
        ))
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await
}

#[cfg(not(debug_assertions))]
async fn serve_router(
    router: ProductionRouter,
    listener: TcpListener,
    _state: AppState,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    router
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await
}
