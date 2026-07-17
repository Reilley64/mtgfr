//! `mtgfr.v1.Auth` — signup/login mint a session and hand the raw token back as
//! `AuthSession.session_token` (cookies terminate at the BFF, which does the `Set-Cookie`).

use axum::http::StatusCode;
use tonic::{Request, Response, Status};

use crate::AppState;
use crate::auth::{
    hash_password, mint_session, revoke_session, signup_create_error, validate_username,
    verify_password,
};
use crate::db::User;
use crate::grpc::auth_ctx;
use crate::grpc::pb;

pub struct AuthSvc {
    state: AppState,
}

impl AuthSvc {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

/// Map an auth HTTP-style status into a tonic `Status`.
fn status_of(code: StatusCode) -> Status {
    match code {
        StatusCode::CONFLICT => Status::already_exists("email already registered"),
        StatusCode::BAD_REQUEST => Status::invalid_argument("invalid signup credentials"),
        _ => Status::internal("account operation failed"),
    }
}

fn pb_me(user: &User) -> pb::Me {
    pb::Me {
        id: user.id,
        email: user.email.clone(),
        username: user.username.clone(),
    }
}

#[tonic::async_trait]
impl pb::auth_server::Auth for AuthSvc {
    async fn signup(
        &self,
        request: Request<pb::SignupCredentials>,
    ) -> Result<Response<pb::AuthSession>, Status> {
        let cred = request.into_inner();
        let username = validate_username(&cred.username).map_err(status_of)?;
        let mut db = self.state.db.clone();
        let hash = hash_password(&cred.password);
        let user = User::create()
            .email(&cred.email)
            .username(&username)
            .password_hash(&hash)
            .exec(&mut db)
            .await
            .map_err(|e| status_of(signup_create_error(e)))?;
        let session_token = mint_session(&mut db, user.id).await.map_err(status_of)?;
        Ok(Response::new(pb::AuthSession {
            me: Some(pb_me(&user)),
            session_token,
        }))
    }

    async fn login(
        &self,
        request: Request<pb::Credentials>,
    ) -> Result<Response<pb::AuthSession>, Status> {
        let cred = request.into_inner();
        let mut db = self.state.db.clone();
        let user = User::filter_by_email(&cred.email)
            .get(&mut db)
            .await
            .map_err(|_| Status::unauthenticated("invalid email or password"))?;
        if !verify_password(&cred.password, &user.password_hash) {
            return Err(Status::unauthenticated("invalid email or password"));
        }
        let session_token = mint_session(&mut db, user.id).await.map_err(status_of)?;
        Ok(Response::new(pb::AuthSession {
            me: Some(pb_me(&user)),
            session_token,
        }))
    }

    async fn logout(&self, request: Request<pb::Empty>) -> Result<Response<pb::Empty>, Status> {
        let token = auth_ctx::session_token(&request)?.to_string();
        let mut db = self.state.db.clone();
        revoke_session(&mut db, &token).await.map_err(status_of)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_me(&self, request: Request<pb::Empty>) -> Result<Response<pb::Me>, Status> {
        let user = auth_ctx::authenticate(&self.state, &request).await?;
        Ok(Response::new(pb_me(&user)))
    }
}
