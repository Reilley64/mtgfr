//! `mtgfr.v1.Cards` — the card pool and its search/lookup projection. Public, like the HTTP
//! `/cards/*` routes: the pool isn't private.

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

fn card_list(cards: Vec<schema::CatalogCard>) -> pb::CardList {
    pb::CardList {
        cards: cards.into_iter().map(map::catalog_card_to_pb).collect(),
    }
}

#[tonic::async_trait]
impl pb::cards_server::Cards for CardsSvc {
    async fn catalog(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::CardList>, Status> {
        let cards = cards::registry().values().map(catalog_card).collect();
        Ok(Response::new(card_list(cards)))
    }

    async fn search(
        &self,
        request: Request<pb::SearchCardsRequest>,
    ) -> Result<Response<pb::CardList>, Status> {
        let req = request.into_inner();
        let mut db = self.state.db.clone();
        let cards = catalog_search::search(&mut db, &req.q, req.limit, req.offset)
            .await
            .unwrap_or_default();
        Ok(Response::new(card_list(cards)))
    }

    async fn lookup(
        &self,
        request: Request<pb::LookupCardsRequest>,
    ) -> Result<Response<pb::CardList>, Status> {
        let req = request.into_inner();
        let mut db = self.state.db.clone();
        let cards = catalog_search::lookup(&mut db, &req.ids)
            .await
            .unwrap_or_default();
        Ok(Response::new(card_list(cards)))
    }
}
