//! Deck CRUD, scoped to the signed-in user. Decks are validated for full Commander legality
//! on save (and again at game start, in `start_game`). The card list is stored as a JSON blob
//! in the `Deck.cards` column — read/written whole.
//!
// ponytail: handlers return a ready-made `Response` on error (the natural axum shape); that
// makes the `Err` variant large, which is fine here — allow the lint rather than box it.
#![allow(clippy::result_large_err)]

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use schema::{DeckDetail, DeckError, DeckSummary, SaveDeckRequest};

use crate::AppState;
use crate::auth::AuthUser;
use crate::db::Deck;
use crate::legality;
use crate::precons;

/// A ready-to-return 403 refusing to edit or delete a read-only precon.
fn precon_readonly() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(DeckError {
            problems: vec![
                "Precon decks are read-only and can't be edited or deleted.".to_string(),
            ],
        }),
    )
        .into_response()
}

/// Parse a stored deck row's JSON card list into wire entries.
fn detail_of(deck: Deck) -> Result<DeckDetail, Response> {
    let cards = serde_json::from_str(&deck.cards).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(DeckError {
                problems: vec!["Corrupt deck: unable to parse stored card list".to_string()],
            }),
        )
            .into_response()
    })?;
    Ok(DeckDetail {
        id: deck.id,
        name: deck.name,
        commander: deck.commander,
        commander_print: deck.commander_print,
        cards,
    })
}

/// Validate a save request; `Err` is a ready-to-return 422 with all problems.
fn check(req: &SaveDeckRequest) -> Result<(), Response> {
    legality::validate(&req.commander, &req.cards).map_err(|problems| {
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(DeckError { problems }),
        )
            .into_response()
    })
}

/// Create a deck for the signed-in user (422 with all legality problems if illegal).
#[utoipa::path(post, path = "/decks/v1", request_body = SaveDeckRequest, responses((status = 200, description = "Created", body = DeckDetail), (status = 422, description = "Illegal deck", body = DeckError)))]
pub async fn create_deck(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<SaveDeckRequest>,
) -> Result<Json<DeckDetail>, Response> {
    check(&req)?;
    let mut db = state.db.clone();
    let cards = serde_json::to_string(&req.cards).expect("serialize decklist");
    let deck = Deck::create()
        .user_id(user.0.id)
        .name(&req.name)
        .commander(&req.commander)
        .commander_print(&req.commander_print)
        .cards(&cards)
        .exec(&mut db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(Json(detail_of(deck)?))
}

/// List the signed-in user's decks.
#[utoipa::path(get, path = "/decks/v1", responses((status = 200, description = "The user's decks", body = [DeckSummary])))]
pub async fn list_decks(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<DeckSummary>>, Response> {
    let mut db = state.db.clone();
    let decks = Deck::filter_by_user_id(user.0.id)
        .exec(&mut db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    // A player's own decks list first, then the read-only precons everyone shares.
    let mut summaries: Vec<DeckSummary> = decks
        .into_iter()
        .map(|d| DeckSummary {
            id: d.id,
            name: d.name,
            commander: d.commander,
        })
        .collect();
    summaries.extend(precons::summaries());
    Ok(Json(summaries))
}

/// Load one of the user's decks by id (404 if it isn't theirs).
async fn owned_deck(state: &AppState, user_id: i64, id: i64) -> Result<Deck, Response> {
    let mut db = state.db.clone();
    let deck = Deck::filter_by_id(id)
        .get(&mut db)
        .await
        .map_err(|_| StatusCode::NOT_FOUND.into_response())?;
    if deck.user_id != user_id {
        return Err(StatusCode::NOT_FOUND.into_response());
    }
    Ok(deck)
}

/// Get a deck's full contents.
#[utoipa::path(get, path = "/decks/{id}/v1", params(("id" = i64, Path, description = "deck id")), responses((status = 200, description = "The deck", body = DeckDetail), (status = 404, description = "Not found")))]
pub async fn get_deck(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<DeckDetail>, Response> {
    if let Some(precon) = precons::get(id) {
        return Ok(Json(precon.clone())); // a precon belongs to everyone
    }
    let deck = owned_deck(&state, user.0.id, id).await?;
    Ok(Json(detail_of(deck)?))
}

/// Update a deck (re-validated; 422 if the new list is illegal).
#[utoipa::path(put, path = "/decks/{id}/v1", params(("id" = i64, Path, description = "deck id")), request_body = SaveDeckRequest, responses((status = 200, description = "Updated", body = DeckDetail), (status = 404, description = "Not found"), (status = 422, description = "Illegal deck", body = DeckError)))]
pub async fn update_deck(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<SaveDeckRequest>,
) -> Result<Json<DeckDetail>, Response> {
    if precons::is_precon(id) {
        return Err(precon_readonly());
    }
    check(&req)?;
    let mut deck = owned_deck(&state, user.0.id, id).await?;
    let mut db = state.db.clone();
    let cards = serde_json::to_string(&req.cards).expect("serialize decklist");
    deck.update()
        .name(&req.name)
        .commander(&req.commander)
        .commander_print(&req.commander_print)
        .cards(&cards)
        .exec(&mut db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    // Re-read for the canonical stored form.
    let deck = owned_deck(&state, user.0.id, id).await?;
    Ok(Json(detail_of(deck)?))
}

/// Delete a deck.
#[utoipa::path(delete, path = "/decks/{id}/v1", params(("id" = i64, Path, description = "deck id")), responses((status = 204, description = "Deleted"), (status = 404, description = "Not found")))]
pub async fn delete_deck(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, Response> {
    if precons::is_precon(id) {
        return Err(precon_readonly());
    }
    let deck = owned_deck(&state, user.0.id, id).await?;
    let mut db = state.db.clone();
    deck.delete()
        .exec(&mut db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{User, connect};
    use schema::DeckCardEntry;

    async fn test_state() -> AppState {
        AppState::for_test(connect("sqlite::memory:").await.expect("sqlite"))
    }

    /// Create a user row and wrap it as an authenticated caller.
    async fn user(state: &AppState, email: &str) -> AuthUser {
        let mut db = state.db.clone();
        let u = User::create()
            .email(email)
            .username(email.split('@').next().unwrap_or("player"))
            .password_hash("x")
            .exec(&mut db)
            .await
            .expect("create user");
        AuthUser(u)
    }

    /// A legal Tajic (RW) deck: the pool's RW nonbasics padded to 99 with Plains.
    fn legal_deck() -> SaveDeckRequest {
        let nonbasics = [
            "Savannah Lions",
            "Goblin Guide",
            "Serra Angel",
            "Glorious Anthem",
            "Shock",
            "Brute Force",
        ];
        let entry = |name: &str, count: u32| {
            let def = cards::get_by_name(name).expect("pool card");
            DeckCardEntry {
                id: def.id.to_string(),
                count,
                print: def.default_print.to_string(),
            }
        };
        let mut cards: Vec<DeckCardEntry> = nonbasics.iter().map(|n| entry(n, 1)).collect();
        cards.push(entry("Plains", 99 - nonbasics.len() as u32));
        let tajic = cards::get_by_name("Tajic, Legion's Edge").expect("pool card");
        SaveDeckRequest {
            name: "My Deck".to_string(),
            commander: tajic.id.to_string(),
            commander_print: tajic.default_print.to_string(),
            cards,
        }
    }

    #[tokio::test]
    async fn a_legal_deck_saves_lists_and_reads_back() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;

        let created = create_deck(State(state.clone()), alice, Json(legal_deck()))
            .await
            .expect("save legal deck");
        let id = created.0.id;

        let alice = user2(&state, "a@b.c").await;
        let list = list_decks(State(state.clone()), alice)
            .await
            .expect("list")
            .0;
        assert!(list.iter().any(|d| d.id == id));

        let alice = user2(&state, "a@b.c").await;
        let got = get_deck(State(state.clone()), alice, Path(id))
            .await
            .expect("get")
            .0;
        assert_eq!(
            got.commander,
            cards::get_by_name("Tajic, Legion's Edge").unwrap().id
        );
        assert_eq!(got.cards.iter().map(|c| c.count).sum::<u32>(), 99);
    }

    #[tokio::test]
    async fn an_illegal_deck_is_rejected_with_problems() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let mut deck = legal_deck();
        deck.cards.pop(); // now well short of 99
        let resp = create_deck(State(state.clone()), alice, Json(deck))
            .await
            .unwrap_err();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn update_deck_renames_and_replaces_cards() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let created = create_deck(State(state.clone()), alice, Json(legal_deck()))
            .await
            .expect("save")
            .0;

        let mut revised = legal_deck();
        revised.name = "Renamed".to_string();
        let bolt = cards::get_by_name("Lightning Bolt").expect("pool");
        revised.cards[0] = DeckCardEntry {
            id: bolt.id.to_string(),
            count: 1,
            print: bolt.default_print.to_string(),
        };

        let alice = user2(&state, "a@b.c").await;
        let updated = update_deck(
            State(state.clone()),
            alice,
            Path(created.id),
            Json(revised.clone()),
        )
        .await
        .expect("update")
        .0;
        assert_eq!(updated.name, "Renamed");
        assert!(
            updated.cards.iter().any(|c| c.id == bolt.id),
            "got {:?}",
            updated.cards,
        );
    }

    #[tokio::test]
    async fn update_deck_rejects_precons() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let resp = update_deck(State(state.clone()), alice, Path(-1), Json(legal_deck()))
            .await
            .unwrap_err();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_user_cannot_read_another_users_deck() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let created = create_deck(State(state.clone()), alice, Json(legal_deck()))
            .await
            .expect("save")
            .0;

        let bob = user(&state, "b@b.c").await;
        let resp = get_deck(State(state.clone()), bob, Path(created.id))
            .await
            .unwrap_err();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    /// Re-fetch an `AuthUser` for the same email (handlers consume it by value).
    async fn user2(state: &AppState, email: &str) -> AuthUser {
        let mut db = state.db.clone();
        let u = User::filter_by_email(email)
            .get(&mut db)
            .await
            .expect("user exists");
        AuthUser(u)
    }
}
