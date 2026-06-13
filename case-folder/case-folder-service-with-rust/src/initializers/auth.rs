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

pub struct AuthInitializer;

#[async_trait]
impl Initializer for AuthInitializer {
    fn name(&self) -> String {
        "auth".to_string()
    }

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
