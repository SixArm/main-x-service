//! Builds [`AuthState`] from the `settings.auth` config block, injects
//! it as an Axum extension, and layers the session-guard middleware over
//! `/api/*`.
//!
//! The guard requires a valid session JWT on every `/api/*` request
//! except `/api/auth/*` (and `/healthz`). It is a pass-through when
//! `auth.require_session` is `false` (the `test` environment) so the
//! existing domain request-tests run without logging in.

use async_trait::async_trait;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::{from_fn, Next},
    response::{IntoResponse, Response},
    Json, Router as AxumRouter,
};
use loco_rs::{
    app::{AppContext, Initializer},
    Result,
};
use std::sync::Arc;

use crate::auth::{mailer::LogMailer, AuthConfig, AuthState};

/// Loco initializer that wires up authentication: it constructs the
/// shared [`AuthState`], layers the session-guard middleware, and
/// injects the state as an Axum extension.
pub struct AuthInitializer;

#[async_trait]
impl Initializer for AuthInitializer {
    /// Stable identifier for this initializer in Loco's registry.
    fn name(&self) -> String {
        "auth".to_string()
    }

    /// Builds [`AuthState`] from the `settings.auth` config block, layers
    /// the [`guard`] middleware, then injects the state as an extension.
    ///
    /// Side effects: warns if the signing secret is empty/insecure, and
    /// logs an info line when `require_session` is off (guard is a
    /// pass-through). The guard layer is added *before* the extension so
    /// both are present on every `/api/*` request.
    async fn after_routes(&self, router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let auth_value = ctx
            .config
            .settings
            .clone()
            .and_then(|s| s.get("auth").cloned())
            .unwrap_or(serde_json::Value::Null);
        let config: AuthConfig = serde_json::from_value(auth_value).unwrap_or_default();

        if config.secret_is_insecure() {
            tracing::warn!(
                "auth.secret is empty — set the AUTH_SECRET environment variable before production"
            );
        }
        if !config.require_session {
            tracing::info!(
                "auth.require_session is false — the /api/* guard is in pass-through mode"
            );
        }

        let state = Arc::new(AuthState::new(config, Box::new(LogMailer)));
        let guard_state = state.clone();

        let router = router
            .layer(from_fn(move |req: Request, next: Next| {
                let state = guard_state.clone();
                async move { guard(state, req, next).await }
            }))
            .layer(axum::Extension(state));

        Ok(router)
    }
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
