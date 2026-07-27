//! `mtgfr.v1.CardsService` — public card pool catalog/search/lookup.

use schema::catalog_card;
use tonic::{Request, Response, Status};

use crate::AppState;
use crate::catalog_search;
use crate::grpc::map;
use crate::grpc::pb;

pub struct CardsSvc {
    state: AppState,
}

impl CardsSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

fn card_list(cards: Vec<schema::CatalogCard>) -> Vec<pb::CatalogCard> {
    cards.into_iter().map(map::catalog_card_to_pb).collect()
}

#[tonic::async_trait]
impl pb::cards_service_server::CardsService for CardsSvc {
    async fn catalog(
        &self,
        _request: Request<pb::CatalogRequest>,
    ) -> Result<Response<pb::CatalogResponse>, Status> {
        let cards = cards::registry().values().map(catalog_card).collect();
        Ok(Response::new(pb::CatalogResponse {
            cards: card_list(cards),
        }))
    }

    async fn search(
        &self,
        request: Request<pb::SearchRequest>,
    ) -> Result<Response<pb::SearchResponse>, Status> {
        let req = request.into_inner();
        let mut db = self.state.db.clone();
        let cards = catalog_search::search(&mut db, &req.q, req.limit, req.offset)
            .await
            .map_err(|e| Status::internal(format!("catalog query failed: {e}")))?;
        Ok(Response::new(pb::SearchResponse {
            cards: card_list(cards),
        }))
    }

    async fn lookup(
        &self,
        request: Request<pb::LookupRequest>,
    ) -> Result<Response<pb::LookupResponse>, Status> {
        let req = request.into_inner();
        let mut db = self.state.db.clone();
        let cards = catalog_search::lookup(&mut db, &req.ids)
            .await
            .map_err(|e| Status::internal(format!("catalog query failed: {e}")))?;
        Ok(Response::new(pb::LookupResponse {
            cards: card_list(cards),
        }))
    }
}
