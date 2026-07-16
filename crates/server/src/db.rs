//! Durable storage: accounts, sessions, and user-authored decks in Postgres via Toasty.
//!
//! Toasty models are server-private storage types, deliberately distinct from the `schema`
//! wire types. A decklist is stored as one JSON column (`cards`) since a deck is always read
//! and written whole — no per-card queries.
//! Schema changes go through Toasty migrations (`just migrate`); `push_schema` is test-only.

use toasty::Model;

#[derive(Debug, Model)]
pub struct User {
    #[key]
    #[auto]
    pub id: i64,
    #[unique]
    pub email: String,
    /// Display name — not unique; email is the login identifier.
    pub username: String,
    /// argon2 PHC hash string.
    pub password_hash: String,
}

#[derive(Debug, Model)]
pub struct Session {
    /// The random opaque token that is the session cookie value.
    #[key]
    pub token: String,
    pub user_id: i64,
    /// Unix seconds after which the session is dead. Enforced server-side by the `AuthUser`
    /// extractor (a stale row is deleted on the failed resolve), so it holds even if the cookie
    /// outlives it. ponytail: lazy expiry at the extractor — no background sweeper; add one only
    /// if dead rows pile up faster than logins prune them.
    pub expires_at: i64,
}

#[derive(Debug, Model)]
pub struct Deck {
    #[key]
    #[auto]
    pub id: i64,
    /// Indexed so a user's decks can be listed with `filter_by_user_id`.
    #[index]
    pub user_id: i64,
    pub name: String,
    /// Commander card name.
    pub commander: String,
    /// `serde_json` of `Vec<schema::DeckCardEntry>` — the whole 99, read/written as a unit.
    /// ponytail: a JSON blob, not a `deck_cards` join table; add relations only if per-card
    /// queries ever appear (they won't for legality).
    pub cards: String,
}

/// Model set shared by the server and `server migration …`.
pub fn model_set() -> toasty::schema::ModelSet {
    toasty::models!(User, Session, Deck)
}

/// Connect to Postgres (or sqlite in tests). Postgres assumes migrations already ran
/// (`just migrate`). Sqlite tests still use `push_schema`.
pub async fn connect(url: &str) -> toasty::Result<toasty::Db> {
    let mut builder = toasty::Db::builder();
    builder.models(model_set());
    let db = builder.connect(url).await?;
    if url.starts_with("sqlite") {
        // Tests only — Postgres schema comes from `just migrate`.
        db.push_schema().await?;
    }
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_user_and_deck_round_trip_through_the_store() {
        let mut db = connect("sqlite::memory:").await.expect("connect sqlite");

        let user = User::create()
            .email("a@b.c")
            .username("alice")
            .password_hash("hash")
            .exec(&mut db)
            .await
            .expect("create user");

        let found = User::filter_by_email("a@b.c")
            .get(&mut db)
            .await
            .expect("find by unique email");
        assert_eq!(found.id, user.id);

        let deck = Deck::create()
            .user_id(user.id)
            .name("Test")
            .commander("Tajic, Legion's Edge")
            .cards("[]")
            .exec(&mut db)
            .await
            .expect("create deck");
        assert_eq!(deck.user_id, user.id);
    }
}
