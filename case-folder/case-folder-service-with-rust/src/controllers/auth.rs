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
    Extension, Json, debug_handler,
    http::{HeaderMap, HeaderValue, StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::{AuthState, Identity};
use crate::responses;

/// Request body for `POST /api/auth/request`.
#[derive(Debug, Deserialize)]
pub struct RequestParams {
    /// The email address to send a magic link to. Optional in the wire
    /// format so a missing/blank value yields a `422` rather than a
    /// deserialization failure.
    pub email: Option<String>,
}

/// Request body for `POST /api/auth/verify`.
#[derive(Debug, Deserialize)]
pub struct VerifyParams {
    /// The magic-link token to exchange for a session. Optional in the
    /// wire format so a missing value is handled as an invalid token
    /// (`401`) rather than a deserialization failure.
    pub token: Option<String>,
}

/// Public, serialisable view of a signed-in identity (the safe subset
/// returned to clients).
#[derive(Serialize)]
struct UserView {
    /// The user's email address.
    email: String,
    /// The user's display name.
    name: String,
    /// The user's role, if any (e.g. records clerk / admin).
    role: Option<String>,
}

/// Project an internal `Identity` into the client-facing `UserView`,
/// dropping any non-public fields.
impl From<Identity> for UserView {
    fn from(i: Identity) -> Self {
        Self {
            email: i.email,
            name: i.name,
            role: i.role,
        }
    }
}

/// Response envelope wrapping the current user, used by `verify` and
/// `me`.
#[derive(Serialize)]
struct UserBody {
    /// The signed-in user.
    user: UserView,
}

/// Response envelope for `POST /api/auth/request`.
#[derive(Serialize)]
struct RequestBody {
    /// Always `true` — the response is deliberately identical whether or
    /// not the email was recognised, to prevent address enumeration.
    sent: bool,
    /// The magic link itself, included **only** in dev/test (when
    /// `auth.expose_magic_link` is set) and only when the email matched.
    /// Omitted from JSON entirely when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    magic_link: Option<String>,
}

/// Mint and email a magic sign-in link for a known email address.
///
/// Route: `POST /api/auth/request`. Method: `POST`. Request:
/// `RequestParams` JSON (`{"email": "..."}`). Response: `RequestBody`
/// JSON (`{"sent": true, "magic_link"?: "..."}`).
///
/// Status codes:
/// - `200` — always returned when an email is supplied, regardless of
///   whether it is on the allowlist. This **prevents email
///   enumeration**: a caller cannot tell a known from an unknown address
///   by the response. A token is minted + emailed only on a match, and
///   the link is echoed in the body only when `expose_magic_link` is on
///   (dev/test).
/// - `422` — the email field is missing or blank (validation error map
///   keyed by `email`).
///
/// The minted token has the **magic** audience (single-use sign-in),
/// distinct from the session-cookie audience issued by `verify`.
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
    if let Some(identity) = auth.identity_for_email(email)
        && let Ok(token) = auth.mint_magic_token(&identity)
    {
        let link = auth.magic_link(&token);
        auth.mailer.send_magic_link(&identity, &link);
        if auth.expose_magic_link() {
            magic_link = Some(link);
        }
    }

    Json(RequestBody {
        sent: true,
        magic_link,
    })
    .into_response()
}

/// Exchange a valid magic-link token for a session cookie.
///
/// Route: `POST /api/auth/verify`. Method: `POST`. Request:
/// `VerifyParams` JSON (`{"token": "..."}`). Response: `UserBody` JSON
/// (`{"user": {...}}`) plus a `Set-Cookie` header carrying the session.
///
/// Status codes:
/// - `200` — magic token valid; an **opaque server-side session** is
///   established and its id attached as the HttpOnly session cookie, and
///   the signed-in user is returned.
/// - `401` — the magic token is missing, invalid, or expired.
///
/// This is where the single-use **magic** link is traded for a server-side
/// session (an opaque id in the cookie, per `agents/share/jwt.md`).
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
    // Establish an opaque server-side session (not a token) and carry its
    // id in the HttpOnly cookie.
    let sid = auth.create_session(&identity);

    let mut response = Json(UserBody {
        user: identity.into(),
    })
    .into_response();
    set_cookie(&mut response, &auth.session_cookie(&sid));
    response
}

/// Return the currently signed-in user, derived from the session cookie.
///
/// Route: `GET /api/auth/me`. Method: `GET`. Request: none (reads the
/// session cookie from request headers). Response: `UserBody` JSON.
///
/// Status codes:
/// - `200` — a valid session cookie was present; returns the user.
/// - `401` — no valid session (not signed in).
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

/// Sign out by clearing the session cookie.
///
/// Route: `POST /api/auth/logout`. Method: `POST`. Request: none.
/// Response: empty body with a `Set-Cookie` header that expires/clears
/// the session.
///
/// Status codes:
/// - `204` — always; logout is idempotent (clearing an absent session
///   is a no-op).
#[debug_handler]
pub async fn logout(Extension(auth): Extension<Arc<AuthState>>, headers: HeaderMap) -> Response {
    // Revoke the server-side session (not just the cookie) so the opaque id
    // can never be replayed, then clear the cookie.
    auth.revoke_from_headers(&headers);
    let mut response = StatusCode::NO_CONTENT.into_response();
    set_cookie(&mut response, &auth.clear_cookie());
    response
}

/// Attach a `Set-Cookie` header to a response.
///
/// Helper shared by `verify` (set session) and `logout` (clear
/// session). Silently skips the header if the cookie string is not a
/// valid header value, so a malformed cookie never aborts the response.
fn set_cookie(response: &mut Response, cookie: &str) {
    if let Ok(value) = HeaderValue::from_str(cookie) {
        response.headers_mut().insert(SET_COOKIE, value);
    }
}

/// Mount the authentication routes under the `/api/auth` prefix:
/// `POST /request`, `POST /verify`, `GET /me`, `POST /logout`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/auth")
        .add("/request", post(request_link))
        .add("/verify", post(verify))
        .add("/me", get(me))
        .add("/logout", post(logout))
}
