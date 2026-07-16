//! Self-hosted email+password accounts with cookie sessions.
//!
//! Passwords are argon2 PHC hashes; a login/signup mints a random session token, stores it with a
//! 30-day expiry, and sets it as an HttpOnly `SameSite=Lax` cookie (`Secure` when
//! `settings.cookie_secure`, since dev is http on localhost; `Domain` when
//! `settings.cookie_domain` is set, for prod's shared-subdomain auth). The [`AuthUser`] extractor
//! resolves that cookie to a [`db::User`] for protected routes, rejecting and sweeping sessions
//! past `expires_at`.

use std::time::{SystemTime, UNIX_EPOCH};

use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use rand::RngCore;
use schema::{Credentials, Me, SignupCredentials};

use crate::AppState;
use crate::db::{Session, User};
use crate::settings::Settings;

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
        let session = Session::filter_by_token(&token)
            .get(&mut db)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        if session.expires_at <= now_unix() {
            // Lazy sweep: drop the dead row so a leaked token can't be probed repeatedly.
            let _ = session.delete().exec(&mut db).await;
            return Err(StatusCode::UNAUTHORIZED);
        }
        let user = User::filter_by_id(session.user_id)
            .get(&mut db)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        Ok(AuthUser(user))
    }
}

/// argon2 PHC hash of a password.
fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

/// Whether `password` matches the stored PHC hash.
fn verify_password(password: &str, phc: &str) -> bool {
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

/// Session cookie `Domain` from settings — omitted when empty (host-only). Must match on logout.
fn cookie_domain_attr(settings: &Settings) -> Option<String> {
    (!settings.cookie_domain.is_empty()).then(|| settings.cookie_domain.clone())
}

/// Create a session row and return the jar with the session cookie set.
async fn start_session(
    jar: CookieJar,
    db: &mut toasty::Db,
    user_id: i64,
    settings: &Settings,
) -> Result<CookieJar, StatusCode> {
    let token = random_token();
    Session::create()
        .token(&token)
        .user_id(user_id)
        .expires_at(now_unix() + SESSION_TTL_SECS)
        .exec(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut builder = Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .secure(settings.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/");
    if let Some(domain) = cookie_domain_attr(settings) {
        builder = builder.domain(domain);
    }
    Ok(jar.add(builder.build()))
}

/// Trim and length-check a signup username; 400 on empty or too long.
fn validate_username(raw: &str) -> Result<String, StatusCode> {
    const MAX: usize = 32;
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(trimmed.to_string())
}

fn me_of(user: &User) -> Me {
    Me {
        email: user.email.clone(),
        username: user.username.clone(),
    }
}

/// Duplicate email is a 409; anything else (schema drift, connectivity) is a 500 so verify/dev
/// doesn't misread a broken DB as "email taken".
fn signup_create_error(err: toasty::Error) -> StatusCode {
    let msg = err.to_string().to_lowercase();
    if msg.contains("unique") || msg.contains("duplicate") {
        return StatusCode::CONFLICT;
    }
    eprintln!("signup create failed: {err}");
    StatusCode::INTERNAL_SERVER_ERROR
}

/// Register a new account and sign in. A duplicate email is a 409 — deliberately *not* declared
/// as a response, so the generated client surfaces it as a catchable `HttpClientError` (a
/// documented bodiless status is instead swallowed to void). The client reads the 409 off the error.
#[utoipa::path(post, path = "/auth/signup/v1", request_body = SignupCredentials, responses((status = 200, description = "Signed up", body = Me)))]
pub async fn signup(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(cred): Json<SignupCredentials>,
) -> Result<(CookieJar, Json<Me>), StatusCode> {
    let username = validate_username(&cred.username)?;
    let mut db = state.db.clone();
    let hash = hash_password(&cred.password);
    // A duplicate email violates the unique index — surface it as a conflict.
    let user = User::create()
        .email(&cred.email)
        .username(&username)
        .password_hash(&hash)
        .exec(&mut db)
        .await
        .map_err(signup_create_error)?;
    let jar = start_session(jar, &mut db, user.id, &state.settings).await?;
    Ok((jar, Json(me_of(&user))))
}

/// Sign in to an existing account.
/// response (see `signup`), so the generated client surfaces it as a catchable `HttpClientError`
/// rather than swallowing it to void.
#[utoipa::path(post, path = "/auth/login/v1", request_body = Credentials, responses((status = 200, description = "Signed in", body = Me)))]
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(cred): Json<Credentials>,
) -> Result<(CookieJar, Json<Me>), StatusCode> {
    let mut db = state.db.clone();
    let user = User::filter_by_email(&cred.email)
        .get(&mut db)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    if !verify_password(&cred.password, &user.password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let jar = start_session(jar, &mut db, user.id, &state.settings).await?;
    Ok((jar, Json(me_of(&user))))
}

/// Sign out: delete the session row and clear the cookie.
#[utoipa::path(post, path = "/auth/logout/v1", responses((status = 200, description = "Signed out"), (status = 500, description = "Session deletion failed")))]
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<CookieJar, StatusCode> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        let token = cookie.value().to_string();
        let mut db = state.db.clone();
        if let Ok(session) = Session::filter_by_token(&token).get(&mut db).await {
            session
                .delete()
                .exec(&mut db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    let mut builder = Cookie::build(SESSION_COOKIE).path("/");
    if let Some(domain) = cookie_domain_attr(&state.settings) {
        builder = builder.domain(domain);
    }
    Ok(jar.remove(builder.build()))
}

/// The currently signed-in user (401 if not signed in).
#[utoipa::path(get, path = "/auth/me/v1", responses((status = 200, description = "Current user", body = Me), (status = 401, description = "Not signed in")))]
pub async fn me(user: AuthUser) -> Json<Me> {
    Json(me_of(&user.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connect;

    async fn test_state() -> AppState {
        AppState::for_test(connect("sqlite::memory:").await.expect("sqlite"))
    }

    fn signup_creds(email: &str, password: &str, username: &str) -> SignupCredentials {
        SignupCredentials {
            email: email.to_string(),
            password: password.to_string(),
            username: username.to_string(),
        }
    }

    fn creds(email: &str, password: &str) -> Credentials {
        Credentials {
            email: email.to_string(),
            password: password.to_string(),
        }
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
    async fn signup_sets_a_session_that_the_extractor_resolves() {
        let state = test_state().await;
        let (jar, Json(me)) = signup(
            State(state.clone()),
            CookieJar::new(),
            Json(signup_creds("a@b.c", "pw", "alice")),
        )
        .await
        .expect("signup");
        assert_eq!(me.email, "a@b.c");
        assert_eq!(me.username, "alice");

        let token = jar
            .get(SESSION_COOKIE)
            .expect("session cookie set")
            .value()
            .to_string();
        let mut parts = parts_with_cookie(&token);
        let AuthUser(user) = AuthUser::from_request_parts(&mut parts, &state)
            .await
            .expect("cookie resolves to the user");
        assert_eq!(user.email, "a@b.c");
    }

    #[tokio::test]
    async fn login_rejects_a_wrong_password() {
        let state = test_state().await;
        let _ = signup(
            State(state.clone()),
            CookieJar::new(),
            Json(signup_creds("a@b.c", "right", "alice")),
        )
        .await
        .expect("signup");

        let err = login(
            State(state.clone()),
            CookieJar::new(),
            Json(creds("a@b.c", "wrong")),
        )
        .await
        .unwrap_err();
        assert_eq!(err, StatusCode::UNAUTHORIZED);
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
