use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use engine::{Event, Game, PlayerId};
use toasty::SqlPlaceholder;

use crate::Seat;
use crate::db::User;
use crate::elo;

const USERS_TABLE: &str = "users";
pub const DEFAULT_LEADERBOARD_LIMIT: u32 = 50;
pub const MAX_LEADERBOARD_LIMIT: u32 = 100;

#[derive(Debug, PartialEq, Eq)]
pub struct LeaderboardRow {
    pub user_id: i64,
    pub username: String,
    pub rating: i32,
}

/// Account users still in the game at the start of this event batch:
/// not lost after apply, or listed in this batch's PlayerLost events.
pub fn active_at_batch_start(game: &Game, lost_in_batch: &[PlayerId]) -> Vec<PlayerId> {
    (0..game.player_count() as u8)
        .map(PlayerId)
        .filter(|player| !game.has_lost(*player) || lost_in_batch.contains(player))
        .collect()
}

pub async fn persist_player_lost(db: &toasty::Db, seats: &[Seat], game: &Game, events: &[Event]) {
    let lost_in_batch: Vec<PlayerId> = events
        .iter()
        .filter_map(|event| match event {
            Event::PlayerLost { player } => Some(*player),
            _ => None,
        })
        .collect();
    if lost_in_batch.is_empty() {
        return;
    }

    let mut alive = active_at_batch_start(game, &lost_in_batch);
    // ponytail: simultaneous multi-loss batches (e.g. SBA) process in event order — later
    // losers are credited with wins over earlier losers; MTG would treat those as mutual draws.
    for loser in lost_in_batch {
        let Some(loser_id) = user_id_for_player(seats, loser) else {
            alive.retain(|player| *player != loser);
            continue;
        };
        let winner_ids: Vec<i64> = alive
            .iter()
            .filter(|player| **player != loser)
            .filter_map(|player| user_id_for_player(seats, *player))
            .collect();
        if winner_ids.is_empty() {
            alive.retain(|player| *player != loser);
            continue;
        }

        if let Err(err) = apply_one(db, loser_id, &winner_ids).await {
            tracing::warn!(error = %err, loser_id, "rating update failed after PlayerLost");
        }
        alive.retain(|player| *player != loser);
    }
}

fn user_id_for_player(seats: &[Seat], player: PlayerId) -> Option<i64> {
    seats.get(player.0 as usize).and_then(|seat| seat.user_id)
}

async fn apply_one(db: &toasty::Db, loser_id: i64, winner_ids: &[i64]) -> Result<(), String> {
    let mut ratings = HashMap::new();
    let mut conn = db.clone();
    for user_id in std::iter::once(loser_id).chain(winner_ids.iter().copied()) {
        if ratings.contains_key(&user_id) {
            continue;
        }
        let user = User::filter_by_id(user_id)
            .get(&mut conn)
            .await
            .map_err(|err| err.to_string())?;
        ratings.insert(user_id, user.rating);
    }

    let updates = elo::apply_elimination(&ratings, loser_id, winner_ids);
    let now = unix_seconds();
    for update in updates {
        if !update.rating_changed {
            continue;
        }

        let mut user = User::filter_by_id(update.user_id)
            .get(&mut conn)
            .await
            .map_err(|err| err.to_string())?;
        user.update()
            .rating(update.new_rating)
            .rating_set_at(now)
            .exec(&mut conn)
            .await
            .map_err(|err| err.to_string())?;
    }

    Ok(())
}

pub fn normalize_leaderboard_limit(limit: u32) -> u32 {
    if limit == 0 {
        return DEFAULT_LEADERBOARD_LIMIT;
    }
    limit.min(MAX_LEADERBOARD_LIMIT)
}

pub async fn leaderboard(
    db: &toasty::Db,
    limit: u32,
    offset: u32,
) -> toasty::Result<(Vec<LeaderboardRow>, u32)> {
    let limit = normalize_leaderboard_limit(limit);
    let mut conn = db.clone();
    let total = leaderboard_total(&mut conn).await?;
    let rows = leaderboard_rows(&mut conn, limit, offset).await?;
    Ok((rows, total))
}

fn placeholder(db: &toasty::Db, n: usize) -> String {
    match db.capability().sql_placeholder {
        Some(SqlPlaceholder::DollarNumber) => format!("${n}"),
        _ => format!("?{n}"),
    }
}

async fn leaderboard_total(db: &mut toasty::Db) -> toasty::Result<u32> {
    let sql = format!("SELECT COUNT(*) FROM {USERS_TABLE}");
    let rows = toasty::sql::query(sql).exec(db).await?;
    let total = rows
        .first()
        .and_then(|row| row.as_record())
        .and_then(|record| record.fields.first())
        .and_then(|value| value.to_i64())
        .ok_or_else(|| toasty::Error::from_args(format_args!("leaderboard total row missing")))?;
    u32::try_from(total)
        .map_err(|_| toasty::Error::from_args(format_args!("leaderboard total overflowed u32")))
}

async fn leaderboard_rows(
    db: &mut toasty::Db,
    limit: u32,
    offset: u32,
) -> toasty::Result<Vec<LeaderboardRow>> {
    let limit_slot = placeholder(db, 1);
    let offset_slot = placeholder(db, 2);
    // ponytail: use Toasty's raw-SQL escape hatch for ordered paging until model queries expose
    // stable multi-column ORDER BY + LIMIT/OFFSET with the same clarity as the catalog search path.
    let sql = format!(
        "SELECT id, username, rating \
         FROM {USERS_TABLE} \
         ORDER BY rating DESC, rating_set_at ASC, id ASC \
         LIMIT {limit_slot} OFFSET {offset_slot}"
    );
    let rows = toasty::sql::query(sql)
        .bind(i64::from(limit))
        .bind(i64::from(offset))
        .exec(db)
        .await?;
    rows_to_leaderboard_rows(rows)
}

fn rows_to_leaderboard_rows(rows: Vec<toasty::stmt::Value>) -> toasty::Result<Vec<LeaderboardRow>> {
    let mut entries = Vec::with_capacity(rows.len());
    for row in rows {
        let record = row.as_record().ok_or_else(|| {
            toasty::Error::from_args(format_args!("leaderboard row missing record"))
        })?;
        let user_id = record
            .fields
            .first()
            .and_then(|value| value.to_i64())
            .ok_or_else(|| {
                toasty::Error::from_args(format_args!("leaderboard row missing user id"))
            })?;
        let username = record
            .fields
            .get(1)
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                toasty::Error::from_args(format_args!("leaderboard row missing username"))
            })?
            .to_string();
        let rating = record
            .fields
            .get(2)
            .and_then(|value| value.to_i32())
            .ok_or_else(|| {
                toasty::Error::from_args(format_args!("leaderboard row missing rating"))
            })?;
        entries.push(LeaderboardRow {
            user_id,
            username,
            rating,
        });
    }
    Ok(entries)
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::elo::STARTING_RATING;
    use engine::Intent;

    const TEST_RATING_SET_AT: i64 = 1_700_000_000;

    async fn user(db: &mut toasty::Db, email: &str) -> i64 {
        db::User::create()
            .email(email)
            .username(email.split('@').next().unwrap_or("player"))
            .password_hash("x")
            .rating(STARTING_RATING)
            .rating_set_at(TEST_RATING_SET_AT)
            .exec(db)
            .await
            .expect("create user")
            .id
    }

    async fn rating(db: &mut toasty::Db, id: i64) -> i32 {
        db::User::filter_by_id(id)
            .get(db)
            .await
            .expect("user exists")
            .rating
    }

    #[test]
    fn active_at_batch_start_includes_current_survivors_and_batch_losers() {
        let mut game = Game::with_players(4, 0);
        game.submit(Intent::Concede {
            player: PlayerId(1),
        })
        .expect("player 1 concedes before this batch");
        game.submit(Intent::Concede {
            player: PlayerId(2),
        })
        .expect("player 2 loses in this batch");
        game.submit(Intent::Concede {
            player: PlayerId(3),
        })
        .expect("player 3 loses in this batch");

        assert_eq!(
            active_at_batch_start(&game, &[PlayerId(2), PlayerId(3)]),
            vec![PlayerId(0), PlayerId(2), PlayerId(3)]
        );
    }

    #[test]
    fn normalize_leaderboard_limit_clamps_values_above_the_maximum() {
        assert_eq!(
            normalize_leaderboard_limit(MAX_LEADERBOARD_LIMIT + 1),
            MAX_LEADERBOARD_LIMIT
        );
    }

    #[tokio::test]
    async fn persist_player_lost_processes_multi_loss_batches_in_event_order() {
        let db = db::connect("sqlite::memory:").await.expect("sqlite");
        let mut writable = db.clone();
        let winner = user(&mut writable, "winner@example.test").await;
        let first_loser = user(&mut writable, "first-loser@example.test").await;
        let second_loser = user(&mut writable, "second-loser@example.test").await;
        let seats = [
            Seat {
                user_id: Some(winner),
                username: None,
            },
            Seat {
                user_id: Some(first_loser),
                username: None,
            },
            Seat {
                user_id: Some(second_loser),
                username: None,
            },
        ];
        let mut game = Game::with_players(3, 0);
        game.submit(Intent::Concede {
            player: PlayerId(1),
        })
        .expect("first player loses");
        game.submit(Intent::Concede {
            player: PlayerId(2),
        })
        .expect("second player loses");

        persist_player_lost(
            &db,
            &seats,
            &game,
            &[
                Event::PlayerLost {
                    player: PlayerId(1),
                },
                Event::PlayerLost {
                    player: PlayerId(2),
                },
            ],
        )
        .await;

        assert_eq!(rating(&mut writable, winner).await, 1032);
        assert_eq!(rating(&mut writable, first_loser).await, 968);
        assert_eq!(rating(&mut writable, second_loser).await, 1000);
    }
}
