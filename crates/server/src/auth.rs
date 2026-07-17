//! Self-hosted email+password accounts. `AuthUser` remains a cookie extractor for parity with
//! how sessions used to reach the API directly; the live path is the gRPC `Auth` service
//! (`grpc::auth_svc`), which calls the transport-agnostic helpers below and mints/reads the
//! session token as `AuthSession.session_token` / `x-session-token` metadata (ADR 0032) — the
//! BFF is the one that terminates the browser's cookie and does the `Set-Cookie`. An expired
//! session is lazily swept so a leaked token can't be probed repeatedly.

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::cookie::CookieJar;
use rand::RngCore;

use crate::AppState;
use crate::db::{Session, User};

/// The session cookie name.
const SESSION_COOKIE: &str = "session";

/// How long a session stays valid: 30 days.
const SESSION_TTL_SECS: i64 = 30 * 24 * 60 * 60;

/// Current unix time in seconds. The server may read the wall clock — only the *engine* is pure.
fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The signed-in user, resolved from the session cookie. Protected handlers take this as an
/// argument; absence or an unknown session is a 401.
#[derive(Debug)]
pub struct AuthUser(pub User);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let mut db = state.db.clone();
        resolve_session_token(&mut db, &token).await.map(AuthUser)
    }
}

/// Resolve a raw session token to its user — the transport-agnostic half of [`AuthUser`]'s
/// cookie lookup, shared with the gRPC `x-session-token` metadata path (ADR 0032,
/// `grpc::auth_ctx`). An expired session is lazily swept so a leaked token can't be probed
/// repeatedly.
pub(crate) async fn resolve_session_token(
    db: &mut toasty::Db,
    token: &str,
) -> Result<User, StatusCode> {
    let session = Session::filter_by_token(token)
        .get(db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if session.expires_at <= now_unix() {
        let _ = session.delete().exec(db).await;
        return Err(StatusCode::UNAUTHORIZED);
    }
    User::filter_by_id(session.user_id)
        .get(db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

/// argon2 PHC hash of a password.
pub(crate) fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

/// Whether `password` matches the stored PHC hash.
pub(crate) fn verify_password(password: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// A random opaque session token (256 bits, hex).
fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Create a session row for `user_id` and return its raw token, for the gRPC `Auth` service
/// (signup/login) to hand back as `AuthSession.session_token` — the BFF does the `Set-Cookie`
/// itself from that.
pub(crate) async fn mint_session(db: &mut toasty::Db, user_id: i64) -> Result<String, StatusCode> {
    let token = random_token();
    Session::create()
        .token(&token)
        .user_id(user_id)
        .expires_at(now_unix() + SESSION_TTL_SECS)
        .exec(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(token)
}

/// Delete the session row for `token`, if any — idempotent, for the gRPC `Auth.Logout` path.
pub(crate) async fn revoke_session(db: &mut toasty::Db, token: &str) -> Result<(), StatusCode> {
    if let Ok(session) = Session::filter_by_token(token).get(db).await {
        session
            .delete()
            .exec(db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    Ok(())
}

/// Trim and length-check a signup username; 400 on empty or too long.
pub(crate) fn validate_username(raw: &str) -> Result<String, StatusCode> {
    const MAX: usize = 32;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(trimmed.to_string())
}

/// Duplicate email is a 409; anything else (schema drift, connectivity) is a 500 so verify/dev
/// doesn't misread a broken DB as "email taken". Called by the gRPC `Auth.Signup` service.
pub(crate) fn signup_create_error(err: toasty::Error) -> StatusCode {
    let msg = err.to_string().to_lowercase();
    if msg.contains("unique") || msg.contains("duplicate") {
        return StatusCode::CONFLICT;
    }
    eprintln!("signup create failed: {err}");
    StatusCode::INTERNAL_SERVER_ERROR
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect;

    async fn test_state() -> AppState {
        AppState::for_test(connect("sqlite::memory:").await.expect("sqlite"))
    }

    /// A `Parts` carrying the given session cookie, for driving the `AuthUser` extractor.
    fn parts_with_cookie(token: &str) -> Parts {
        axum::http::Request::builder()
            .header("cookie", format!("{SESSION_COOKIE}={token}"))
            .body(())
            .unwrap()
            .into_parts()
            .0
    }

    #[test]
    fn a_hash_verifies_its_own_password_and_rejects_others() {
        let phc = hash_password("hunter2");
        assert!(verify_password("hunter2", &phc));
        assert!(!verify_password("wrong", &phc));
    }

    #[tokio::test]
    async fn a_minted_session_authenticates_the_cookie_extractor() {
        let state = test_state().await;
        let mut db = state.db.clone();
        let user = User::create()
            .email("a@b.c")
            .username("alice")
            .password_hash(hash_password("pw"))
            .exec(&mut db)
            .await
            .expect("create user");
        let token = mint_session(&mut db, user.id).await.expect("mint session");

        let mut parts = parts_with_cookie(&token);
        let AuthUser(resolved) = AuthUser::from_request_parts(&mut parts, &state)
            .await
            .expect("cookie resolves to the user");
        assert_eq!(resolved.email, "a@b.c");
    }

    #[tokio::test]
    async fn an_expired_session_is_rejected_and_swept() {
        let state = test_state().await;
        let mut db = state.db.clone();
        let user = User::create()
            .email("x@y.z")
            .username("x")
            .password_hash("h")
            .exec(&mut db)
            .await
            .expect("create user");
        // A session that expired an hour ago.
        Session::create()
            .token("stale")
            .user_id(user.id)
            .expires_at(now_unix() - 3600)
            .exec(&mut db)
            .await
            .expect("create stale session");

        let mut parts = parts_with_cookie("stale");
        let err = AuthUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err();
        assert_eq!(err, StatusCode::UNAUTHORIZED);

        // Lazy sweep: the stale row is gone, so it can't be probed again.
        assert!(
            Session::filter_by_token("stale")
                .get(&mut db)
                .await
                .is_err(),
            "expired session should be deleted on the failed resolve"
        );
    }

    #[tokio::test]
    async fn a_missing_cookie_is_unauthorized() {
        let state = test_state().await;
        let mut parts = axum::http::Request::builder()
            .body(())
            .unwrap()
            .into_parts()
            .0;
        let err = AuthUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap_err();
        assert_eq!(err, StatusCode::UNAUTHORIZED);
    }
}
