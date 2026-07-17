//! `mtgfr.v1.Tables` — `Seed` builds a live `Table` from BFF-resolved seats.

use axum::http::StatusCode;
use tonic::{Request, Response, Status};

use crate::AppState;
use crate::grpc::auth_ctx;
use crate::grpc::map;
use crate::grpc::pb;
use crate::lobby::seed_table_core;

pub struct TablesSvc {
    state: AppState,
}

impl TablesSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn status_of(code: StatusCode) -> Status {
    match code {
        StatusCode::FORBIDDEN => Status::permission_denied("caller is not the host"),
        StatusCode::SERVICE_UNAVAILABLE => {
            Status::unavailable("instance draining — retry against another instance")
        }
        StatusCode::BAD_REQUEST => Status::invalid_argument("invalid seed request"),
        _ => Status::internal("seed failed"),
    }
}

#[tonic::async_trait]
impl pb::tables_server::Tables for TablesSvc {
    async fn seed(
        &self,
        request: Request<pb::SeedRequest>,
    ) -> Result<Response<pb::SeedResponse>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        let req = map::seed_request_from_pb(request.into_inner());
        let resp = seed_table_core(&self.state, user.id, req)
            .await
            .map_err(status_of)?;
        Ok(Response::new(map::seed_response_to_pb(resp)))
    }
}
