//! Authentication API — passwordless email magic link.
//!
//! - `POST /api/auth/request` — mint + email a magic link for a known
//!   email. Always `200` (no email enumeration); includes the link in
//!   the body only when `auth.expose_magic_link` is set (dev/test).
//! - `POST /api/auth/verify`  — exchange a valid magic token for a
//!   session cookie.
//! - `GET  /api/auth/me`      — the signed-in user, or `401`.
//! - `POST /api/auth/logout`  — clear the session cookie.

use axum::{
    debug_handler,
    http::{header::SET_COOKIE, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::{AuthState, Identity};
use crate::responses;

#[derive(Debug, Deserialize)]
pub struct RequestParams {
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyParams {
    pub token: Option<String>,
}

#[derive(Serialize)]
struct UserView {
    email: String,
    name: String,
    role: Option<String>,
}

impl From<Identity> for UserView {
    fn from(i: Identity) -> Self {
        Self {
            email: i.email,
            name: i.name,
            role: i.role,
        }
    }
}

#[derive(Serialize)]
struct UserBody {
    user: UserView,
}

#[derive(Serialize)]
struct RequestBody {
    sent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    magic_link: Option<String>,
}

#[debug_handler]
pub async fn request_link(
    Extension(auth): Extension<Arc<AuthState>>,
    Json(params): Json<RequestParams>,
) -> Response {
    let email = params.email.unwrap_or_default();
    let email = email.trim();
    if email.is_empty() {
        let mut errors = HashMap::new();
        errors.insert("email".to_string(), "Enter your email address.".to_string());
        return responses::unprocessable(errors);
    }

    // Always answer 200 whether or not the email is known, so the
    // endpoint can't be used to enumerate valid addresses. A token is
    // only minted + emailed when the email matches the allowlist.
    let mut magic_link = None;
    if let Some(identity) = auth.identity_for_email(email) {
        if let Ok(token) = auth.mint_magic_token(&identity) {
            let link = auth.magic_link(&token);
            auth.mailer.send_magic_link(&identity, &link);
            if auth.expose_magic_link() {
                magic_link = Some(link);
            }
        }
    }

    Json(RequestBody {
        sent: true,
        magic_link,
    })
    .into_response()
}

#[debug_handler]
pub async fn verify(
    Extension(auth): Extension<Arc<AuthState>>,
    Json(params): Json<VerifyParams>,
) -> Response {
    let token = params.token.unwrap_or_default();
    let identity = match auth.verify_magic_token(token.trim()) {
        Ok(identity) => identity,
        Err(_) => return responses::unauthorized("Invalid or expired sign-in link."),
    };
    let session = match auth.mint_session_token(&identity) {
        Ok(session) => session,
        Err(_) => return responses::unauthorized("Could not establish a session."),
    };

    let mut response = Json(UserBody {
        user: identity.into(),
    })
    .into_response();
    set_cookie(&mut response, &auth.session_cookie(&session));
    response
}

#[debug_handler]
pub async fn me(Extension(auth): Extension<Arc<AuthState>>, headers: HeaderMap) -> Response {
    match auth.identity_from_headers(&headers) {
        Some(identity) => Json(UserBody {
            user: identity.into(),
        })
        .into_response(),
        None => responses::unauthorized("Authentication required"),
    }
}

#[debug_handler]
pub async fn logout(Extension(auth): Extension<Arc<AuthState>>) -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut response, &auth.clear_cookie());
    response
}

fn set_cookie(response: &mut Response, cookie: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/request", post(request_link))
        .add("/verify", post(verify))
        .add("/me", get(me))
        .add("/logout", post(logout))
}
