//! `/health/live` + `/health/ready` stay 200 while draining (owned tables keep traffic).
//! `/health/drain` reports `{active_tables, draining}` (SIGTERM sets draining).

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct LiveStatus {
    pub version: String,
    pub faithful_count: u32,
    pub faithful_by_set: BTreeMap<String, u32>,
}

pub fn faithful_by_set() -> BTreeMap<String, u32> {
    let mut map = BTreeMap::new();
    for def in cards::registry().values() {
        if def.approximates.is_some() {
            continue;
        }
        for code in def.sets.iter() {
            if code.is_empty() {
                continue;
            }
            *map.entry((*code).to_lowercase()).or_default() += 1;
        }
    }
    map
}

pub fn faithful_pool_count() -> u32 {
    cards::registry()
        .values()
        .filter(|def| def.approximates.is_none())
        .count() as u32
}

pub async fn live(State(state): State<AppState>) -> Json<LiveStatus> {
    Json(LiveStatus {
        version: state.settings.version.clone(),
        faithful_count: faithful_pool_count(),
        faithful_by_set: faithful_by_set(),
    })
}

pub async fn ready() -> &'static str {
    "ok"
}

#[derive(Debug, Clone, Serialize)]
pub struct DrainStatus {
    pub active_tables: usize,
    pub draining: bool,
}

pub(crate) fn drain_status(state: &AppState) -> DrainStatus {
    DrainStatus {
        active_tables: crate::lock(&state.reg).active_table_count(),
        draining: state.draining.load(Ordering::Relaxed),
    }
}

pub async fn drain(State(state): State<AppState>) -> Json<DrainStatus> {
    Json(drain_status(&state))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::db;

    async fn test_state() -> AppState {
        AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"))
    }

    #[tokio::test]
    async fn live_reports_the_configured_version() {
        let state = test_state().await;
        let expected = state.settings.version.clone();
        let Json(status) = live(State(state)).await;
        assert_eq!(status.version, expected);
    }

    #[tokio::test]
    async fn live_reports_faithful_count_matching_registry() {
        let state = test_state().await;
        let expected = cards::registry()
            .values()
            .filter(|d| d.approximates.is_none())
            .count() as u32;
        assert!(expected > 0, "pool should have faithful cards in test env");
        let Json(status) = live(State(state)).await;
        assert_eq!(status.faithful_count, expected);
        assert!(
            status.faithful_count < cards::registry().len() as u32
                || cards::registry().values().all(|d| d.approximates.is_none()),
            "count must exclude approximates when any exist"
        );
    }

    #[tokio::test]
    async fn live_faithful_count_excludes_approximated_cards() {
        let approximated = cards::registry()
            .values()
            .filter(|d| d.approximates.is_some())
            .count() as u32;
        let state = test_state().await;
        let Json(status) = live(State(state)).await;
        assert_eq!(
            status.faithful_count + approximated,
            cards::registry().len() as u32
        );
    }

    #[tokio::test]
    async fn live_reports_faithful_by_set_matching_registry() {
        let mut expected: BTreeMap<String, u32> = BTreeMap::new();
        for def in cards::registry().values() {
            if def.approximates.is_some() {
                continue;
            }
            for code in def.sets.iter() {
                if code.is_empty() {
                    continue;
                }
                *expected.entry((*code).to_lowercase()).or_default() += 1;
            }
        }
        assert!(!expected.is_empty());
        let state = test_state().await;
        let Json(status) = live(State(state)).await;
        assert_eq!(status.faithful_by_set, expected);
        // Multi-credit: sum may exceed faithful_count — do not assert sum <= faithful_count.
    }

    #[tokio::test]
    async fn live_faithful_by_set_multi_credits_across_sets() {
        let multi = cards::registry()
            .values()
            .filter(|d| d.approximates.is_none() && d.sets.len() > 1)
            .count();
        assert!(multi > 0, "backfilled pool should have multi-set cards");
        let state = test_state().await;
        let Json(status) = live(State(state)).await;
        let sum: u32 = status.faithful_by_set.values().sum();
        assert!(sum > status.faithful_count);
    }

    #[tokio::test]
    async fn live_faithful_by_set_omits_empty_and_approximates() {
        let state = test_state().await;
        let Json(status) = live(State(state)).await;
        assert!(!status.faithful_by_set.contains_key(""));
        for (code, n) in &status.faithful_by_set {
            let registry_faithful = cards::registry()
                .values()
                .filter(|d| {
                    d.approximates.is_none() && d.sets.iter().any(|s| s.eq_ignore_ascii_case(code))
                })
                .count() as u32;
            assert_eq!(*n, registry_faithful);
        }
    }

    #[tokio::test]
    async fn ready_is_ok_even_while_draining() {
        let state = test_state().await;
        state.draining.store(true, Ordering::Relaxed);
        assert_eq!(ready().await, "ok");
    }

    #[tokio::test]
    async fn drain_status_reports_zero_active_tables_for_a_fresh_registry() {
        let state = test_state().await;
        let Json(status) = drain(State(state)).await;
        assert_eq!(status.active_tables, 0);
        assert!(!status.draining);
    }
}
