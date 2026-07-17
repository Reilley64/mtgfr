//! `mtgfr.v1.Decks` — deck CRUD over native protobuf messages (ADR 0032): each payload is a
//! generated `pb` type, mapped to/from `schema`'s wire DTOs at the boundary (`grpc::map`).
#![allow(clippy::result_large_err)] // `tonic::Status` is a large `Err` by design; see auth_ctx.rs.

use tonic::{Request, Response, Status};

use crate::AppState;
use crate::decks_api::{
    DeckOpError, create_deck_core, delete_deck_core, get_deck_core, list_decks_core,
    update_deck_core,
};
use crate::grpc::auth_ctx;
use crate::grpc::map;
use crate::grpc::pb;

pub struct DecksSvc {
    state: AppState,
}

impl DecksSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

impl From<DeckOpError> for Status {
    fn from(err: DeckOpError) -> Status {
        match err {
            DeckOpError::Illegal(problems) => {
                Status::invalid_argument(format!("illegal deck: {}", problems.join("; ")))
            }
            DeckOpError::PreconReadonly => Status::permission_denied("precon decks are read-only"),
            DeckOpError::NotFound => Status::not_found("deck not found"),
            DeckOpError::Corrupt => Status::internal("stored deck is corrupt"),
            DeckOpError::Internal => Status::internal("deck operation failed"),
        }
    }
}

#[tonic::async_trait]
impl pb::decks_server::Decks for DecksSvc {
    async fn create(
        &self,
        request: Request<pb::SaveDeckRequest>,
    ) -> Result<Response<pb::DeckDetail>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let req = map::save_deck_request_from_pb(request.into_inner());
        let deck = create_deck_core(&self.state, user.id, req).await?;
        Ok(Response::new(map::deck_detail_to_pb(deck)))
    }

    async fn list(&self, request: Request<pb::Empty>) -> Result<Response<pb::DeckList>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let decks = list_decks_core(&self.state, user.id).await?;
        Ok(Response::new(pb::DeckList {
            decks: decks.into_iter().map(map::deck_summary_to_pb).collect(),
        }))
    }

    async fn get(&self, request: Request<pb::DeckId>) -> Result<Response<pb::DeckDetail>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let id = request.into_inner().id;
        let deck = get_deck_core(&self.state, user.id, id).await?;
        Ok(Response::new(map::deck_detail_to_pb(deck)))
    }

    async fn update(
        &self,
        request: Request<pb::UpdateDeckRequest>,
    ) -> Result<Response<pb::DeckDetail>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let req = map::save_deck_request_from_pb(
            inner
                .request
                .ok_or_else(|| Status::invalid_argument("missing SaveDeckRequest"))?,
        );
        let deck = update_deck_core(&self.state, user.id, inner.id, req).await?;
        Ok(Response::new(map::deck_detail_to_pb(deck)))
    }

    async fn delete(&self, request: Request<pb::DeckId>) -> Result<Response<pb::Empty>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let id = request.into_inner().id;
        delete_deck_core(&self.state, user.id, id).await?;
        Ok(Response::new(pb::Empty {}))
    }
}
