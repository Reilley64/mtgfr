//! Classic Elo helpers for multiplayer elimination batches. Pure — no I/O.

use std::collections::HashMap;

pub const K: f64 = 32.0;
pub const STARTING_RATING: i32 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RatingUpdate {
    pub user_id: i64,
    pub new_rating: i32,
    pub rating_changed: bool,
}

fn expected(rating_self: i32, rating_opp: i32) -> f64 {
    1.0 / (1.0 + 10f64.powf((f64::from(rating_opp) - f64::from(rating_self)) / 400.0))
}

pub fn apply_elimination(
    ratings: &HashMap<i64, i32>,
    loser_id: i64,
    winner_ids: &[i64],
) -> Vec<RatingUpdate> {
    if winner_ids.is_empty() {
        return Vec::new();
    }
    let r_loser = *ratings
        .get(&loser_id)
        .expect("ratings map must contain loser_id");
    let mut deltas: HashMap<i64, f64> = HashMap::new();
    for &wid in winner_ids {
        let r_w = *ratings
            .get(&wid)
            .expect("ratings map must contain every winner id");
        let e_w = expected(r_w, r_loser);
        let e_l = expected(r_loser, r_w);
        *deltas.entry(wid).or_default() += K * (1.0 - e_w);
        *deltas.entry(loser_id).or_default() += K * (0.0 - e_l);
    }
    let mut out = Vec::with_capacity(deltas.len());
    for (user_id, delta) in deltas {
        let old = ratings[&user_id];
        let new_rating = (f64::from(old) + delta).round() as i32;
        out.push(RatingUpdate {
            user_id,
            new_rating,
            rating_changed: new_rating != old,
        });
    }
    out.sort_by_key(|u| u.user_id);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn map(pairs: &[(i64, i32)]) -> HashMap<i64, i32> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn equal_ratings_one_winner_moves_by_k_over_two() {
        let updates = apply_elimination(&map(&[(1, 1000), (2, 1000)]), 2, &[1]);
        let w = updates.iter().find(|u| u.user_id == 1).unwrap();
        let l = updates.iter().find(|u| u.user_id == 2).unwrap();
        assert_eq!(w.new_rating, 1016); // round(1000 + 32*0.5)
        assert_eq!(l.new_rating, 984);
        assert!(w.rating_changed && l.rating_changed);
    }

    #[test]
    fn two_winners_vs_one_loser_are_order_independent() {
        let ratings = map(&[(1, 1000), (2, 1000), (3, 1000)]);
        let a = apply_elimination(&ratings, 3, &[1, 2]);
        let b = apply_elimination(&ratings, 3, &[2, 1]);
        let rating = |updates: &[RatingUpdate], id: i64| {
            updates.iter().find(|u| u.user_id == id).unwrap().new_rating
        };
        assert_eq!(rating(&a, 1), rating(&b, 1));
        assert_eq!(rating(&a, 2), rating(&b, 2));
        assert_eq!(rating(&a, 3), rating(&b, 3));
    }

    #[test]
    fn empty_winners_returns_no_updates() {
        assert!(apply_elimination(&map(&[(1, 1000)]), 1, &[]).is_empty());
    }

    #[test]
    #[should_panic(expected = "ratings map must contain loser_id")]
    fn missing_loser_is_programmer_error() {
        apply_elimination(&map(&[(1, 1000)]), 2, &[1]);
    }

    #[test]
    #[should_panic(expected = "ratings map must contain every winner id")]
    fn missing_winner_is_programmer_error() {
        apply_elimination(&map(&[(1, 1000), (3, 1000)]), 3, &[1, 2]);
    }
}
