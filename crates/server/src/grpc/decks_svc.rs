//! `mtgfr.v1.DecksService` — deck CRUD; payloads map through `grpc::map`.
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
impl pb::decks_service_server::DecksService for DecksSvc {
    async fn create(
        &self,
        request: Request<pb::CreateRequest>,
    ) -> Result<Response<pb::CreateResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let req = map::create_request_from_pb(request.into_inner());
        let deck = create_deck_core(&self.state, user.id, req).await?;
        Ok(Response::new(map::deck_detail_to_create_response(deck)))
    }

    async fn list(
        &self,
        request: Request<pb::ListRequest>,
    ) -> Result<Response<pb::ListResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let decks = list_decks_core(&self.state, user.id).await?;
        Ok(Response::new(pb::ListResponse {
            decks: decks.into_iter().map(map::deck_summary_to_pb).collect(),
        }))
    }

    async fn get(
        &self,
        request: Request<pb::GetRequest>,
    ) -> Result<Response<pb::GetResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let id = request.into_inner().id;
        let deck = get_deck_core(&self.state, user.id, id).await?;
        Ok(Response::new(map::deck_detail_to_get_response(deck)))
    }

    async fn update(
        &self,
        request: Request<pb::UpdateRequest>,
    ) -> Result<Response<pb::UpdateResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let inner = request.into_inner();
        let req = map::deck_save_body_from_pb(
            inner
                .request
                .ok_or_else(|| Status::invalid_argument("missing DeckSaveBody"))?,
        );
        let deck = update_deck_core(&self.state, user.id, inner.id, req).await?;
        Ok(Response::new(map::deck_detail_to_update_response(deck)))
    }

    async fn delete(
        &self,
        request: Request<pb::DeleteRequest>,
    ) -> Result<Response<pb::DeleteResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let id = request.into_inner().id;
        delete_deck_core(&self.state, user.id, id).await?;
        Ok(Response::new(pb::DeleteResponse {}))
    }
}
