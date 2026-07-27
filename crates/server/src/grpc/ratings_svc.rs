//! `mtgfr.v1.RatingsService` — leaderboard reads over the persistent user ratings table.

use tonic::{Request, Response, Status};

use crate::AppState;
use crate::grpc::auth_ctx;
use crate::grpc::pb;
use crate::ratings;

pub struct RatingsSvc {
    state: AppState,
}

impl RatingsSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl pb::ratings_service_server::RatingsService for RatingsSvc {
    async fn get_leaderboard(
        &self,
        request: Request<pb::GetLeaderboardRequest>,
    ) -> Result<Response<pb::GetLeaderboardResponse>, Status> {
        auth_ctx::authenticate(&self.state, &request).await?;
        let req = request.into_inner();
        let (rows, total) = ratings::leaderboard(&self.state.db, req.limit, req.offset)
            .await
            .map_err(|_| Status::internal("leaderboard query failed"))?;
        Ok(Response::new(pb::GetLeaderboardResponse {
            entries: rows
                .into_iter()
                .enumerate()
                .map(|(index, row)| pb::LeaderboardEntry {
                    user_id: row.user_id,
                    username: row.username,
                    rating: row.rating,
                    rank: req.offset.saturating_add(index as u32).saturating_add(1),
                })
                .collect(),
            total,
        }))
    }
}
