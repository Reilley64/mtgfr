//! gRPC session auth: resolve the caller from the `x-session-token` metadata key, mirroring the
//! cookie-based [`crate::auth::AuthUser`] extractor (ADR 0032). Cookies terminate at the BFF —
//! by the time a call reaches tonic, the session token travels as gRPC metadata instead.
//!
// ponytail: `tonic::Status` is a large `Err` (it carries the RPC status shape); boxing it here
// buys nothing since every caller immediately maps it into a `Response` anyway (see
// `decks_api.rs`'s identical rationale for `Response`).
#![allow(clippy::result_large_err)]

use tonic::{Request, Status};

use crate::AppState;
use crate::auth::resolve_session_token;
use crate::db::User;

/// The gRPC metadata key carrying the session token (lowercase — tonic/h2 metadata keys are
/// ASCII-lowercase only).
pub const SESSION_METADATA_KEY: &str = "x-session-token";

/// The raw session token from `req`'s metadata, or `UNAUTHENTICATED` if absent/not valid ASCII.
pub fn session_token<T>(req: &Request<T>) -> Result<&str, Status> {
    req.metadata()
        .get(SESSION_METADATA_KEY)
        .ok_or_else(|| Status::unauthenticated("missing x-session-token metadata"))?
        .to_str()
        .map_err(|_| Status::unauthenticated("x-session-token is not valid ASCII"))
}

/// Resolve the signed-in user from `req`'s session metadata, or `UNAUTHENTICATED` — the gRPC
/// analogue of the cookie-based `AuthUser` extractor.
pub async fn authenticate<T>(state: &AppState, req: &Request<T>) -> Result<User, Status> {
    let token = session_token(req)?;
    let mut db = state.db.clone();
    resolve_session_token(&mut db, token)
        .await
        .map_err(|_| Status::unauthenticated("invalid or expired session"))
}
