//! `/health/live` + `/health/ready` stay 200 while draining (owned tables keep traffic).
//! `/health/drain` reports `{active_tables, draining}` (SIGTERM sets draining).

use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct LiveStatus {
    pub version: String,
}

pub async fn live(State(state): State<AppState>) -> Json<LiveStatus> {
    Json(LiveStatus {
        version: state.settings.version.clone(),
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
