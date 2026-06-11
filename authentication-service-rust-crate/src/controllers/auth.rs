//! Passwordless, magic-link authentication.
//!
//! Flow:
//! 1. `POST /api/auth/signup`     `{email, name?}` — create the account, issue a magic link.
//! 2. `POST /api/auth/magic-link` `{email}`        — issue a magic link for an existing account (sign in).
//! 3. `GET  /api/auth/magic-link/{token}`          — consume the link → RS256 access token + session.
//! 4. `GET  /api/auth/me`                          — current user (bearer token required).
//! 5. `POST /api/auth/signout`                     — revoke the current session (bearer token required).
//!
//! Tokens are RS256 and verifiable offline by peer services via the
//! JWKS at `/.well-known/jwks.json`. In development the magic link is
//! written to the tracing log (no SMTP required).

use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    mailers::auth::AuthMailer,
    models::{sessions, users},
    views::auth::{CurrentResponse, LoginResponse},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct SignupParams {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MagicLinkParams {
    pub email: String,
}

fn default_name(email: &str) -> String {
    let local = email.split('@').next().unwrap_or("user");
    if local.chars().count() >= 2 {
        local.to_string()
    } else {
        "user".to_string()
    }
}

/// Logs the magic link to the console (dev) and best-effort emails it
/// (prod). The console log is authoritative in development.
async fn deliver_magic_link(ctx: &AppContext, user: &users::Model) {
    let Some(token) = user.magic_link_token.as_ref() else {
        return;
    };
    let frontend = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:5173".to_string());
    let link = format!("{frontend}/verify?token={token}");
    tracing::info!(
        email = %user.email,
        magic_link = %link,
        "magic link issued (dev: open the link, or GET /api/auth/magic-link/{{token}})"
    );
    if let Err(err) = AuthMailer::send_magic_link(ctx, user).await {
        tracing::debug!(error = %err, "magic link email not sent; console log above is authoritative");
    }
}

/// Create a passwordless account and send a magic link. To avoid
/// leaking whether an email is already registered, an existing email
/// still receives a fresh link and the response is always 200.
#[debug_handler]
async fn signup(State(ctx): State<AppContext>, Json(params): Json<SignupParams>) -> Result<Response> {
    let name = params
        .name
        .clone()
        .filter(|n| n.chars().count() >= 2)
        .unwrap_or_else(|| default_name(&params.email));

    let user = match users::Model::create_passwordless(&ctx.db, &params.email, &name).await {
        Ok(user) => user,
        Err(ModelError::EntityAlreadyExists) => {
            match users::Model::find_by_email(&ctx.db, &params.email).await {
                Ok(user) => user,
                Err(_) => return format::empty_json(),
            }
        }
        Err(err) => {
            tracing::info!(error = %err, email = %params.email, "signup rejected");
            return format::empty_json();
        }
    };

    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    deliver_magic_link(&ctx, &user).await;
    format::empty_json()
}

/// Request a magic link for an existing account (sign in). Always
/// returns 200, even for unknown emails, to avoid account enumeration.
#[debug_handler]
async fn request_magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    let Ok(user) = users::Model::find_by_email(&ctx.db, &params.email).await else {
        tracing::debug!(email = %params.email, "magic link requested for unknown email");
        return format::empty_json();
    };
    let user = user.into_active_model().create_magic_link(&ctx.db).await?;
    deliver_magic_link(&ctx, &user).await;
    format::empty_json()
}

/// Consume a magic link: validate the token, verify the email, issue an
/// RS256 access token, and record the session for revocation.
#[debug_handler]
async fn verify(Path(token): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let Ok(user) = users::Model::find_by_magic_token(&ctx.db, &token).await else {
        return unauthorized("invalid or expired magic link");
    };

    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;
    let user = if user.email_verified_at.is_none() {
        user.into_active_model().verified(&ctx.db).await?
    } else {
        user
    };

    let (access_token, jti, exp) =
        crate::auth::sign_access_token(&user.pid.to_string(), &user.email, &user.name)
            .map_err(|e| Error::string(&e.to_string()))?;

    let expires_at = chrono::DateTime::<chrono::Utc>::from_timestamp(exp, 0)
        .unwrap_or_else(chrono::Utc::now)
        .fixed_offset();
    sessions::Model::issue(&ctx.db, &jti, user.pid, expires_at, None).await?;

    format::json(LoginResponse::new(&user, &access_token))
}

/// Current authenticated user. Honors local revocation: a signed-out
/// session is rejected even though its JWT signature is still valid.
#[debug_handler]
async fn me(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.jti).await {
        if !session.is_active() {
            return unauthorized("session signed out");
        }
    }
    let user = users::Model::find_by_pid(&ctx.db, &claims.sub).await?;
    format::json(CurrentResponse::new(&user))
}

/// Revoke the current session. Peer services that cached the token keep
/// honoring it until expiry (offline JWKS verification) — that's the
/// documented tradeoff of stateless tokens; we keep TTLs short.
#[debug_handler]
async fn signout(auth: AuthUser, State(ctx): State<AppContext>) -> Result<Response> {
    let AuthUser(claims) = auth;
    if let Ok(session) = sessions::Model::find_by_jid(&ctx.db, &claims.jti).await {
        session.into_active_model().revoke(&ctx.db).await?;
    }
    format::empty_json()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/signup", post(signup))
        .add("/magic-link", post(request_magic_link))
        .add("/magic-link/{token}", get(verify))
        .add("/me", get(me))
        .add("/signout", post(signout))
}
