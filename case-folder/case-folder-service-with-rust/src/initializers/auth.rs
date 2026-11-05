//! Builds [`AuthState`] from [`AuthConfig`], injects it as an Axum
//! extension, and layers the session-guard middleware over
//! `/api/*`.
//!
//! The guard requires a valid session JWT on every `/api/*` request
//! except `/api/auth/*` (and `/healthz`). It is a pass-through when
//! `auth.require_session` is `false` (the `test` environment) so the
//! existing domain request-tests run without logging in.

use axum::{
    Json, Router,
    extract::Request,
    http::StatusCode,
    middleware::{Next, from_fn},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::auth::{AuthConfig, AuthState, mailer::LogMailer};

/// Layer the session guard and shared [`AuthState`] onto `router`.
///
/// Logs an info line when `require_session` is off (the guard then passes
/// everything through). The guard is added *before* the extension so both
/// are present on every `/api/*` request.
pub fn install(router: Router, config: AuthConfig) -> Router {
    if !config.require_session {
        tracing::info!("auth.require_session is false — the /api/* guard is in pass-through mode");
    }
    let state = Arc::new(AuthState::new(config, Box::new(LogMailer)));
    let guard_state = state.clone();
    router
        .layer(from_fn(move |req: Request, next: Next| {
            let state = guard_state.clone();
            async move { guard(state, req, next).await }
        }))
        .layer(axum::Extension(state))
}

/// Session-guard middleware for `/api/*`.
///
/// A request is exempt (passes straight through) when any of:
///   - `require_session` is `false` (e.g. the `test` environment), or
///   - the path is `/healthz`, or
///   - the path is not under `/api/`, or
///   - the path is under `/api/auth/` (login endpoints must stay open).
///
/// Otherwise the request must carry a valid session identity in its
/// headers; if it does not, respond `401` with a JSON error body.
async fn guard(state: Arc<AuthState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let exempt = !state.require_session()
        || path == "/healthz"
        || !path.starts_with("/api/")
        || path.starts_with("/api/auth/");

    if exempt || state.identity_from_headers(req.headers()).is_some() {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Authentication required" })),
        )
            .into_response()
    }
}
