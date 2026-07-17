//! Boundary mappers for `catalog.proto`: deck building and the card catalog.

use schema::{
    CatalogBackFace, CatalogCard, DeckCardEntry, DeckDetail, DeckSummary, SaveDeckRequest,
    SeedRequest, SeedResponse, SeedSeat,
};

use crate::grpc::map::common::{wire_cost_to_pb, wire_kind_to_pb};
use crate::grpc::pb;

pub fn deck_card_entry_to_pb(entry: DeckCardEntry) -> pb::DeckCardEntry {
    pb::DeckCardEntry {
        id: entry.id,
        count: entry.count,
        print: entry.print,
    }
}

pub fn deck_card_entry_from_pb(entry: pb::DeckCardEntry) -> DeckCardEntry {
    DeckCardEntry {
        id: entry.id,
        count: entry.count,
        print: entry.print,
    }
}

pub fn deck_summary_to_pb(deck: DeckSummary) -> pb::DeckSummary {
    pb::DeckSummary {
        id: deck.id,
        name: deck.name,
        commander: deck.commander,
        commander_print: deck.commander_print,
    }
}

pub fn deck_detail_to_pb(deck: DeckDetail) -> pb::DeckDetail {
    pb::DeckDetail {
        id: deck.id,
        name: deck.name,
        commander: deck.commander,
        commander_print: deck.commander_print,
        cards: deck.cards.into_iter().map(deck_card_entry_to_pb).collect(),
    }
}

pub fn save_deck_request_from_pb(req: pb::SaveDeckRequest) -> SaveDeckRequest {
    SaveDeckRequest {
        name: req.name,
        commander: req.commander,
        commander_print: req.commander_print,
        cards: req.cards.into_iter().map(deck_card_entry_from_pb).collect(),
    }
}

pub fn catalog_back_face_to_pb(back: CatalogBackFace) -> pb::CatalogBackFace {
    pb::CatalogBackFace {
        name: back.name,
        oracle: back.oracle,
        approximates: back.approximates,
    }
}

pub fn catalog_card_to_pb(card: CatalogCard) -> pb::CatalogCard {
    pb::CatalogCard {
        id: card.id,
        default_print: card.default_print,
        name: card.name,
        cost: Some(wire_cost_to_pb(card.cost)),
        kind: Some(wire_kind_to_pb(card.kind)),
        keywords: card.keywords,
        summary: card.summary,
        legendary: card.legendary,
        color_identity: card.color_identity.into_iter().map(u32::from).collect(),
        approximates: card.approximates,
        oracle: card.oracle,
        set: card.set,
        subtypes: card.subtypes,
        otags: card.otags,
        back: card.back.map(catalog_back_face_to_pb),
    }
}

pub fn seed_seat_from_pb(seat: pb::SeedSeat) -> SeedSeat {
    SeedSeat {
        user_id: seat.user_id,
        username: seat.username,
        deck_id: seat.deck_id,
    }
}

pub fn seed_request_from_pb(req: pb::SeedRequest) -> SeedRequest {
    SeedRequest {
        table_id: req.table_id,
        host_user_id: req.host_user_id,
        seats: req.seats.into_iter().map(seed_seat_from_pb).collect(),
    }
}

pub fn seed_response_to_pb(resp: SeedResponse) -> pb::SeedResponse {
    pb::SeedResponse {
        table_id: resp.table_id,
        pod_dns: resp.pod_dns,
        version: resp.version,
    }
}
