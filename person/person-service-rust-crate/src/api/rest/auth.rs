//! Bearer-token authentication for the REST surface.
//!
//! [`AuthUser`] is an Axum extractor that pulls `Authorization: Bearer
//! <jwt>`, verifies the RS256 signature and claims against the
//! authentication-service JWKS (carried in [`AppState::verifier`]), and
//! yields the verified [`Claims`]. Verification is stateless and offline
//! — no database hit, no introspection call — so any handler can require
//! authentication by taking an `AuthUser` argument.

use authentication_verifier::Claims;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::IntoResponse;
use axum::Json;

use super::state::AppState;

/// A request whose bearer token passed RS256 / issuer / audience / expiry
/// verification. The wrapped [`Claims`] identify the caller (`sub` is the
/// user `pid`).
pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app = AppState::from_ref(state);
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "missing authorization header".to_string(),
            ))?;
        let token = header
            .strip_prefix("Bearer ")
            .or_else(|| header.strip_prefix("bearer "))
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "expected bearer token".to_string(),
            ))?;
        let claims = app
            .verifier
            .verify(token.trim())
            .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
        Ok(AuthUser(claims))
    }
}

/// `GET /api/whoami` — echo the verified claims of the bearer token.
/// Returns `401` when the token is missing, malformed, or fails
/// verification. Useful for confirming peer JWT verification end to end.
#[utoipa::path(
    get,
    path = "/api/whoami",
    tag = "auth",
    responses(
        (status = 200, description = "Verified token claims"),
        (status = 401, description = "Missing or invalid bearer token"),
    ),
    security(("bearer" = [])),
)]
pub async fn whoami(AuthUser(claims): AuthUser) -> impl IntoResponse {
    Json(claims)
}
