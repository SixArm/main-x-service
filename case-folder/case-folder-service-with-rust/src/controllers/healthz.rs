//! Liveness probe. Returns `{"status":"ok"}` without contacting any
//! downstream service — so it remains green even when every external
//! Main-X-Service is unreachable.

use axum::debug_handler;
use axum::Json;
use loco_rs::prelude::*;
use serde_json::json;

/// Liveness handler.
///
/// Route: `GET /healthz`. Method: `GET`. Request: none. Response: JSON
/// body `{"status":"ok"}` with status `200`. Makes **no** downstream
/// calls, so it stays green even when every upstream Main-X-Service is
/// unreachable — it reports only that *this* process is alive.
#[debug_handler]
pub async fn index() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

/// Mount the liveness route at the root: `GET /healthz`.
pub fn routes() -> Routes {
    Routes::new().add("/healthz", get(index))
}
