//! Deck CRUD, scoped to the signed-in user. Decks are validated for full Commander legality
//! on save (and again at game start, in `start_game`). The card list is stored as a JSON blob
//! in the `Deck.cards` column — read/written whole. The gRPC `Decks` service
//! (`grpc::decks_svc`) is the sole caller of the `*_core` functions below.

use schema::{DeckDetail, DeckSummary, SaveDeckRequest};

use crate::AppState;
use crate::db::Deck;
use crate::legality;
use crate::precons;

/// A deck read/write failure, in transport-agnostic form (ADR 0032): the gRPC `Decks` service
/// maps this to a `tonic::Status` (`grpc::decks_svc`'s `From<DeckOpError> for Status`).
#[derive(Debug)]
pub(crate) enum DeckOpError {
    /// Failed Commander legality — carries every problem found.
    Illegal(Vec<String>),
    /// Precons are read-only.
    PreconReadonly,
    /// Not found, or found but not owned by the caller (never distinguished — see `owned_deck`).
    NotFound,
    /// The stored `cards` JSON blob didn't parse.
    Corrupt,
    Internal,
}

/// Parse a stored deck row's JSON card list into wire entries.
fn detail_of(deck: Deck) -> Result<DeckDetail, DeckOpError> {
    let cards = serde_json::from_str(&deck.cards).map_err(|_| DeckOpError::Corrupt)?;
    Ok(DeckDetail {
        id: deck.id,
        name: deck.name,
        commander: deck.commander,
        commander_print: deck.commander_print,
        cards,
    })
}

/// Validate a save request; `Err` carries every legality problem found.
fn check(req: &SaveDeckRequest) -> Result<(), DeckOpError> {
    legality::validate(&req.commander, &req.commander_print, &req.cards)
        .map_err(DeckOpError::Illegal)
}

/// Create a deck for the signed-in user (422 with all legality problems if illegal). Called by
/// the gRPC `Decks.Create` service.
pub(crate) async fn create_deck_core(
    state: &AppState,
    user_id: i64,
    req: SaveDeckRequest,
) -> Result<DeckDetail, DeckOpError> {
    check(&req)?;
    let mut db = state.db.clone();
    let cards = serde_json::to_string(&req.cards).expect("serialize decklist");
    let deck = Deck::create()
        .user_id(user_id)
        .name(&req.name)
        .commander(&req.commander)
        .commander_print(&req.commander_print)
        .cards(&cards)
        .exec(&mut db)
        .await
        .map_err(|_| DeckOpError::Internal)?;
    detail_of(deck)
}

/// List the signed-in user's decks: their own decks first, then the read-only precons everyone
/// shares. Called by the gRPC `Decks.List` service.
pub(crate) async fn list_decks_core(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<DeckSummary>, DeckOpError> {
    let mut db = state.db.clone();
    let decks = Deck::filter_by_user_id(user_id)
        .exec(&mut db)
        .await
        .map_err(|_| DeckOpError::Internal)?;
    let mut summaries: Vec<DeckSummary> = decks
        .into_iter()
        .map(|d| DeckSummary {
            id: d.id,
            name: d.name,
            commander: d.commander,
            commander_print: d.commander_print,
        })
        .collect();
    summaries.extend(precons::summaries());
    Ok(summaries)
}

/// Load one of the user's decks by id (not-found if it isn't theirs).
async fn owned_deck(state: &AppState, user_id: i64, id: i64) -> Result<Deck, DeckOpError> {
    let mut db = state.db.clone();
    let deck = Deck::filter_by_id(id)
        .get(&mut db)
        .await
        .map_err(|_| DeckOpError::NotFound)?;
    if deck.user_id != user_id {
        return Err(DeckOpError::NotFound);
    }
    Ok(deck)
}

/// Get a deck's full contents — a precon belongs to everyone; otherwise it must be owned by
/// `user_id`. Called by the gRPC `Decks.Get` service.
pub(crate) async fn get_deck_core(
    state: &AppState,
    user_id: i64,
    id: i64,
) -> Result<DeckDetail, DeckOpError> {
    if let Some(precon) = precons::get(id) {
        return Ok(precon.clone());
    }
    let deck = owned_deck(state, user_id, id).await?;
    detail_of(deck)
}

/// Update a deck (re-validated; 422 if the new list is illegal). Called by the gRPC
/// `Decks.Update` service.
pub(crate) async fn update_deck_core(
    state: &AppState,
    user_id: i64,
    id: i64,
    req: SaveDeckRequest,
) -> Result<DeckDetail, DeckOpError> {
    if precons::is_precon(id) {
        return Err(DeckOpError::PreconReadonly);
    }
    check(&req)?;
    let mut deck = owned_deck(state, user_id, id).await?;
    let mut db = state.db.clone();
    let cards = serde_json::to_string(&req.cards).expect("serialize decklist");
    deck.update()
        .name(&req.name)
        .commander(&req.commander)
        .commander_print(&req.commander_print)
        .cards(&cards)
        .exec(&mut db)
        .await
        .map_err(|_| DeckOpError::Internal)?;
    // Re-read for the canonical stored form.
    let deck = owned_deck(state, user_id, id).await?;
    detail_of(deck)
}

/// Delete a deck. Called by the gRPC `Decks.Delete` service.
pub(crate) async fn delete_deck_core(
    state: &AppState,
    user_id: i64,
    id: i64,
) -> Result<(), DeckOpError> {
    if precons::is_precon(id) {
        return Err(DeckOpError::PreconReadonly);
    }
    let deck = owned_deck(state, user_id, id).await?;
    let mut db = state.db.clone();
    deck.delete()
        .exec(&mut db)
        .await
        .map_err(|_| DeckOpError::Internal)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{User, connect};
    use schema::DeckCardEntry;

    async fn test_state() -> AppState {
        AppState::for_test(connect("sqlite::memory:").await.expect("sqlite"))
    }

    /// Create a user row and return its id.
    async fn user(state: &AppState, email: &str) -> i64 {
        let mut db = state.db.clone();
        User::create()
            .email(email)
            .username(email.split('@').next().unwrap_or("player"))
            .password_hash("x")
            .exec(&mut db)
            .await
            .expect("create user")
            .id
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

        let created = create_deck_core(&state, alice, legal_deck())
            .await
            .expect("save legal deck");
        let id = created.id;

        let list = list_decks_core(&state, alice).await.expect("list");
        assert!(list.iter().any(|d| d.id == id));

        let got = get_deck_core(&state, alice, id).await.expect("get");
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
        let err = create_deck_core(&state, alice, deck).await.unwrap_err();
        assert!(matches!(err, DeckOpError::Illegal(_)));
    }

    #[tokio::test]
    async fn update_deck_renames_and_replaces_cards() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let created = create_deck_core(&state, alice, legal_deck())
            .await
            .expect("save");

        let mut revised = legal_deck();
        revised.name = "Renamed".to_string();
        let bolt = cards::get_by_name("Lightning Bolt").expect("pool");
        revised.cards[0] = DeckCardEntry {
            id: bolt.id.to_string(),
            count: 1,
            print: bolt.default_print.to_string(),
        };

        let updated = update_deck_core(&state, alice, created.id, revised.clone())
            .await
            .expect("update");
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
        let err = update_deck_core(&state, alice, -1, legal_deck())
            .await
            .unwrap_err();
        assert!(matches!(err, DeckOpError::PreconReadonly));
    }

    #[tokio::test]
    async fn a_user_cannot_read_another_users_deck() {
        let state = test_state().await;
        let alice = user(&state, "a@b.c").await;
        let created = create_deck_core(&state, alice, legal_deck())
            .await
            .expect("save");

        let bob = user(&state, "b@b.c").await;
        let err = get_deck_core(&state, bob, created.id).await.unwrap_err();
        assert!(matches!(err, DeckOpError::NotFound));
    }
}
