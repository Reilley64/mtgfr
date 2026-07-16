//! Shared async test helpers for server integration tests.

#[cfg(test)]
use crate::db::{self, Deck};
#[cfg(test)]
use crate::decks::SeatDeck;
#[cfg(test)]
use schema::DeckCardEntry;

#[cfg(test)]
pub(crate) fn seat_deck() -> SeatDeck {
    let commander = cards::get_by_name("Tajic, Legion's Edge").unwrap();
    let plains = cards::get_by_name("Plains").unwrap();
    let mut prints = std::collections::HashMap::new();
    prints.insert(
        commander.id.to_string(),
        commander.default_print.to_string(),
    );
    prints.insert(plains.id.to_string(), plains.default_print.to_string());
    SeatDeck {
        commander,
        cards: vec![(plains, 99)],
        prints,
    }
}

#[cfg(test)]
fn legal_deck_json() -> String {
    let entry = |name: &str, count: u32| {
        let def = cards::get_by_name(name).unwrap();
        DeckCardEntry {
            id: def.id.to_string(),
            count,
            print: def.default_print.to_string(),
        }
    };
    let mut cards: Vec<DeckCardEntry> = [
        "Savannah Lions",
        "Goblin Guide",
        "Serra Angel",
        "Glorious Anthem",
        "Shock",
        "Brute Force",
    ]
    .iter()
    .map(|n| entry(n, 1))
    .collect();
    cards.push(entry("Plains", 93));
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
        .commander(cards::get_by_name("Tajic, Legion's Edge").unwrap().id)
        .commander_print(
            cards::get_by_name("Tajic, Legion's Edge")
                .unwrap()
                .default_print,
        )
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
