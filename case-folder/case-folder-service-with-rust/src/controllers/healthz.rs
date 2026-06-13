//! Liveness probe. Returns `{"status":"ok"}` without contacting any
//! downstream service — so it remains green even when every external
//! Main-X-Service is unreachable.

use axum::debug_handler;
use axum::Json;
use loco_rs::prelude::*;
use serde_json::json;

#[debug_handler]
pub async fn index() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

pub fn routes() -> Routes {
    Routes::new().add("/healthz", get(index))
}
