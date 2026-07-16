//! Shared async test helpers for server integration tests.

#[cfg(test)]
use crate::db::{self, Deck};
#[cfg(test)]
use crate::decks::SeatDeck;
#[cfg(test)]
use schema::DeckCardEntry;

#[cfg(test)]
pub(crate) fn seat_deck() -> SeatDeck {
    SeatDeck {
        commander: cards::get("Tajic, Legion's Edge").unwrap(),
        cards: vec![(cards::get("Plains").unwrap(), 99)],
    }
}

#[cfg(test)]
fn legal_deck_json() -> String {
    let mut cards: Vec<DeckCardEntry> = [
        "Savannah Lions",
        "Goblin Guide",
        "Serra Angel",
        "Glorious Anthem",
        "Shock",
        "Brute Force",
    ]
    .iter()
    .map(|n| DeckCardEntry {
        name: n.to_string(),
        count: 1,
    })
    .collect();
    cards.push(DeckCardEntry {
        name: "Plains".to_string(),
        count: 93,
    });
    serde_json::to_string(&cards).unwrap()
}

#[cfg(test)]
pub(crate) async fn user_with_deck(state: &crate::AppState, email: &str) -> i64 {
    let mut db = state.db.clone();
    let u = db::User::create()
        .email(email)
        .username(email.split('@').next().unwrap_or("player"))
        .password_hash("x")
        .exec(&mut db)
        .await
        .expect("create user");
    Deck::create()
        .user_id(u.id)
        .name("deck")
        .commander("Tajic, Legion's Edge")
        .cards(legal_deck_json())
        .exec(&mut db)
        .await
        .expect("create deck")
        .id
}

#[cfg(test)]
pub(crate) async fn as_user(state: &crate::AppState, email: &str) -> crate::AuthUser {
    let mut db = state.db.clone();
    crate::AuthUser(
        db::User::filter_by_email(email)
            .get(&mut db)
            .await
            .expect("user exists"),
    )
}
