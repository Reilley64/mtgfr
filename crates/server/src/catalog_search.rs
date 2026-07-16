//! Deck-builder card search, backed by a Postgres projection of the engine's card pool.
//!
//! The playable pool is `cards::registry()` — bounded, load-once, engine-owned. On boot we
//! project it into a `catalog_cards` table (one row per card: a lowercased `search_blob` haystack
//! plus the card's full wire JSON), and `/cards/search` runs a token `LIKE` query against it.
//! This scales as the pool grows toward the corpus without shipping the whole pool to the client.
//!
//! DDL for `catalog_cards` lives in Toasty migrations. This module only refreshes data via
//! Toasty's raw-SQL escape hatch. Bind placeholders differ by driver (`$1` on Postgres, `?1` on
//! sqlite in tests) — [`placeholder`] reads the driver's capability to pick.

use schema::{CatalogCard, WireKind, catalog_card};
use toasty::SqlPlaceholder;

const TABLE: &str = "catalog_cards";
const COLOR_WORDS: [&str; 5] = ["white", "blue", "black", "red", "green"];
/// Cap on rows returned by one search, so a broad/empty query can't dump the whole pool.
const MAX_LIMIT: u32 = 200;

/// The bind placeholder for parameter `n` (1-based) in the connected driver's dialect.
fn placeholder(db: &toasty::Db, n: usize) -> String {
    match db.capability().sql_placeholder {
        Some(SqlPlaceholder::DollarNumber) => format!("${n}"),
        // NumberedQuestionMark (sqlite) and the plain-`?` fallback both accept `?n` here.
        _ => format!("?{n}"),
    }
}

/// The lowercased haystack a card is matched against: everything a single search box should hit —
/// name, card type, printed subtypes, set code, colors, keywords, and Scryfall oracle-tag slugs.
fn search_blob(c: &CatalogCard) -> String {
    let kind = match c.kind {
        WireKind::Creature { .. } => "creature",
        WireKind::Instant => "instant",
        WireKind::Sorcery => "sorcery",
        WireKind::Enchantment => "enchantment",
        WireKind::Artifact => "artifact",
        WireKind::Planeswalker { .. } => "planeswalker",
        WireKind::Land { .. } => "land",
    };
    let colors = if c.color_identity.is_empty() {
        "colorless".to_string()
    } else {
        c.color_identity
            .iter()
            .filter_map(|&i| COLOR_WORDS.get(i as usize).copied())
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut parts = vec![c.name.clone(), kind.to_string(), c.set.clone(), colors];
    if c.legendary {
        parts.push("legendary".to_string());
    }
    parts.extend(c.subtypes.iter().cloned());
    parts.extend(c.keywords.iter().cloned());
    parts.extend(c.otags.iter().cloned());
    for slug in &c.otags {
        parts.push(slug.replace('-', " "));
    }
    parts.join(" ").to_lowercase()
}

/// (Re)build the `catalog_cards` projection from the engine registry. Idempotent: safe to call on
/// every boot. Replaces the table's contents wholesale (the pool is bounded and load-once, so a
/// truncate+reinsert is simpler than diffing).
pub async fn project(db: &mut toasty::Db) -> toasty::Result<()> {
    // Postgres DDL is in Toasty migrations; sqlite tests create the table here.
    let is_postgres = matches!(
        db.capability().sql_placeholder,
        Some(SqlPlaceholder::DollarNumber)
    );
    if !is_postgres {
        toasty::sql::statement(format!(
            "CREATE TABLE IF NOT EXISTS {TABLE} \
             (name TEXT PRIMARY KEY, search_blob TEXT NOT NULL, card_json TEXT NOT NULL)"
        ))
        .exec(db)
        .await?;
    }

    let p1 = placeholder(db, 1);
    let p2 = placeholder(db, 2);
    let p3 = placeholder(db, 3);
    let insert =
        format!("INSERT INTO {TABLE} (name, search_blob, card_json) VALUES ({p1}, {p2}, {p3})");

    let mut tx = db.transaction().await?;
    toasty::sql::statement(format!("DELETE FROM {TABLE}"))
        .exec(&mut tx)
        .await?;
    for def in cards::registry().values() {
        let card = catalog_card(def);
        let json = serde_json::to_string(&card).expect("a catalog card serializes");
        toasty::sql::statement(&insert)
            .bind(card.name.clone())
            .bind(search_blob(&card))
            .bind(json)
            .exec(&mut tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Cards matching every whitespace-separated token of `q` (each a case-insensitive substring of the
/// haystack), ordered by name and paged. An empty query returns the first page of the whole pool.
/// ponytail: `%`/`_` in the query act as SQL `LIKE` wildcards — not escaped, since the pool is
/// trusted card text and a stray wildcard only widens a search. Escape if user-authored data lands
/// in the blob.
pub async fn search(
    db: &mut toasty::Db,
    q: &str,
    limit: u32,
    offset: u32,
) -> toasty::Result<Vec<CatalogCard>> {
    let tokens: Vec<String> = q
        .split_whitespace()
        .map(|t| format!("%{}%", t.to_lowercase()))
        .collect();
    let limit = limit.clamp(1, MAX_LIMIT);

    let where_clause = if tokens.is_empty() {
        String::new()
    } else {
        let clauses: Vec<String> = (1..=tokens.len())
            .map(|i| format!("search_blob LIKE {}", placeholder(db, i)))
            .collect();
        format!("WHERE {}", clauses.join(" AND "))
    };
    // limit/offset are server-controlled integers (not user text), so inlining them is injection-safe.
    let sql = format!(
        "SELECT card_json FROM {TABLE} {where_clause} ORDER BY name LIMIT {limit} OFFSET {offset}"
    );

    let mut query = toasty::sql::query(sql);
    for token in tokens {
        query = query.bind(token);
    }
    rows_to_cards(query.exec(db).await?)
}

/// The cards with exactly these names (for hydrating a decklist/commander without fetching the
/// whole pool). Order is unspecified; the caller keys by name. Empty input → empty result.
pub async fn lookup(db: &mut toasty::Db, names: &[String]) -> toasty::Result<Vec<CatalogCard>> {
    if names.is_empty() {
        return Ok(vec![]);
    }
    let placeholders: Vec<String> = (1..=names.len()).map(|i| placeholder(db, i)).collect();
    let sql = format!(
        "SELECT card_json FROM {TABLE} WHERE name IN ({})",
        placeholders.join(", ")
    );
    let mut query = toasty::sql::query(sql);
    for name in names {
        query = query.bind(name.clone());
    }
    rows_to_cards(query.exec(db).await?)
}

/// Decode the single-column (`card_json`) rows a query returns into cards.
fn rows_to_cards(rows: Vec<toasty::stmt::Value>) -> toasty::Result<Vec<CatalogCard>> {
    let mut cards = Vec::with_capacity(rows.len());
    for row in rows {
        let json = row
            .as_record()
            .and_then(|rec| rec.fields.first())
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                toasty::Error::from_args(format_args!("catalog row missing card_json"))
            })?;
        cards.push(
            serde_json::from_str(json)
                .map_err(|e| toasty::Error::from_args(format_args!("bad catalog json: {e}")))?,
        );
    }
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn projected() -> toasty::Db {
        let mut db = crate::db::connect("sqlite::memory:").await.expect("sqlite");
        project(&mut db).await.expect("project the pool");
        db
    }

    fn names(cards: &[CatalogCard]) -> Vec<&str> {
        cards.iter().map(|c| c.name.as_str()).collect()
    }

    #[tokio::test]
    async fn one_box_matches_name_type_subtype_set_and_color() {
        // Ambush Viper (inr): a green creature — Snake. Exercises every search dimension, and the
        // all-tokens-must-match (AND) semantics.
        let mut db = projected().await;
        for q in ["ambush", "Snake", "inr", "green creature", "snake viper"] {
            let hits = search(&mut db, q, 100, 0).await.expect("search");
            assert!(
                names(&hits).contains(&"Ambush Viper"),
                "{q:?} should find Ambush Viper"
            );
        }
        // A non-matching token filters it out (it is a creature, not an instant).
        let hits = search(&mut db, "snake instant", 100, 0)
            .await
            .expect("search");
        assert!(
            !names(&hits).contains(&"Ambush Viper"),
            "AND semantics: the instant token excludes it"
        );
    }

    #[tokio::test]
    async fn otag_slugs_are_searchable() {
        let mut db = projected().await;
        for q in [
            "typal-spirit",
            "spirit",
            "cost-reducer-enchantment",
            "enchantment",
        ] {
            let hits = search(&mut db, q, 100, 0).await.expect("search");
            assert!(
                names(&hits).contains(&"Vanguard of the Restless")
                    || names(&hits).contains(&"Starfield Mystic"),
                "{q:?} should find a tagged card"
            );
        }
        let spirit_hits = search(&mut db, "typal-spirit", 100, 0)
            .await
            .expect("search");
        assert!(
            names(&spirit_hits).contains(&"Vanguard of the Restless"),
            "typal-spirit slug should find Vanguard of the Restless"
        );
    }

    #[tokio::test]
    async fn empty_query_pages_the_pool_and_respects_limit() {
        let mut db = projected().await;
        let page = search(&mut db, "", 10, 0).await.expect("search");
        assert_eq!(page.len(), 10, "empty query returns a capped first page");
        let next = search(&mut db, "  ", 10, 10).await.expect("search");
        assert_ne!(names(&page), names(&next), "offset advances the page");
    }

    #[tokio::test]
    async fn lookup_hydrates_exact_names() {
        let mut db = projected().await;
        let got = lookup(&mut db, &["Ambush Viper".to_string(), "Forest".to_string()])
            .await
            .expect("lookup");
        let mut got = names(&got);
        got.sort();
        assert_eq!(got, ["Ambush Viper", "Forest"]);
        assert!(
            lookup(&mut db, &[]).await.expect("lookup").is_empty(),
            "no names → no rows"
        );
    }
}
