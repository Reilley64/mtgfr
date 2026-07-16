//! `AdminAuth` for `/health/drain` when `settings.admin_token` is set.

use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;

use crate::AppState;

const ADMIN_TOKEN_HEADER: &str = "x-admin-token";

/// Empty `admin_token` → allow. Otherwise require `Authorization: Bearer …` or `X-Admin-Token`.
#[derive(Debug)]
pub struct AdminAuth;

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if state.settings.admin_token.is_empty() {
            return Ok(AdminAuth);
        }
        let bearer = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "));
        let plain = parts
            .headers
            .get(ADMIN_TOKEN_HEADER)
            .and_then(|v| v.to_str().ok());
        let provided = bearer.or(plain).ok_or(StatusCode::UNAUTHORIZED)?;
        if provided != state.settings.admin_token {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(AdminAuth)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::sync::Arc;

    async fn test_state() -> AppState {
        AppState::for_test(db::connect("sqlite::memory:").await.expect("sqlite"))
    }

    fn parts_with_bearer(token: Option<&str>) -> Parts {
        let mut builder = axum::http::Request::builder();
        if let Some(token) = token {
            builder = builder.header("authorization", format!("Bearer {token}"));
        }
        builder.body(()).unwrap().into_parts().0
    }

    fn parts_with_admin_token_header(token: &str) -> Parts {
        axum::http::Request::builder()
            .header(ADMIN_TOKEN_HEADER, token)
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[tokio::test]
    async fn admin_auth_passes_every_request_when_no_token_is_configured() {
        let state = test_state().await;
        assert!(state.settings.admin_token.is_empty());

        let mut parts = parts_with_bearer(None);
        assert!(
            AdminAuth::from_request_parts(&mut parts, &state)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn admin_auth_requires_a_matching_bearer_token_once_configured() {
        let mut state = test_state().await;
        state.settings = Arc::new(crate::settings::Settings {
            admin_token: "secret".to_string(),
            ..(*state.settings).clone()
        });

        let mut missing = parts_with_bearer(None);
        assert_eq!(
            AdminAuth::from_request_parts(&mut missing, &state)
                .await
                .unwrap_err(),
            StatusCode::UNAUTHORIZED
        );

        let mut wrong = parts_with_bearer(Some("nope"));
        assert_eq!(
            AdminAuth::from_request_parts(&mut wrong, &state)
                .await
                .unwrap_err(),
            StatusCode::UNAUTHORIZED
        );

        let mut right = parts_with_bearer(Some("secret"));
        assert!(
            AdminAuth::from_request_parts(&mut right, &state)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn admin_auth_also_accepts_the_x_admin_token_header() {
        let mut state = test_state().await;
        state.settings = Arc::new(crate::settings::Settings {
            admin_token: "secret".to_string(),
            ..(*state.settings).clone()
        });

        let mut wrong = parts_with_admin_token_header("nope");
        assert_eq!(
            AdminAuth::from_request_parts(&mut wrong, &state)
                .await
                .unwrap_err(),
            StatusCode::UNAUTHORIZED
        );

        let mut right = parts_with_admin_token_header("secret");
        assert!(
            AdminAuth::from_request_parts(&mut right, &state)
                .await
                .is_ok()
        );
    }
}
