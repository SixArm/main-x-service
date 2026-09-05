//! Operator control-plane endpoint: force one reconciliation pass on
//! demand (T-36).
//!
//! Before this, the only lever an operator had for a stuck/diverged
//! read-model was restarting the process (or restarting with a smaller
//! `LINK_GRAPH_RECONCILE_SECS` temporarily) — the reconciliation runbook
//! documented this as a confirmed gap, not a design choice. This
//! endpoint reuses [`crate::reconcile::reconcile`] directly rather than
//! duplicating its diff/repair/metrics logic; it is not a new write path
//! of this service's own (`AGENTS.md` "read-only to the world" governs
//! *link* writes — edge creation/withdrawal, which still live only in
//! the owning entity service). Triggering a repair of this aggregator's
//! own read-model against a source it is already configured to trust is
//! the same category of action the periodic worker already performs.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;

use crate::auth;
use crate::reconcile::{self, HttpAuthoritativeSource};

/// Map an auth rejection `(status, reason)` to a loco error response.
fn auth_rejection((status, reason): (StatusCode, String)) -> Error {
    let code = if status == StatusCode::FORBIDDEN {
        "forbidden"
    } else {
        "unauthorized"
    };
    Error::CustomError(status, ErrorDetail::new(code, &reason))
}

/// `POST /api/admin/reconcile/{entity}` — run one reconciliation pass
/// for `entity` immediately (e.g. `case`, `person`, `worker`,
/// `care_pathway_instance`), updating the same
/// `reconciliation_divergence` / `reconciliation_last_success_unixtime`
/// gauges the periodic worker updates. `Action::Destructive`-gated
/// ([`auth::authorize_reconcile`]).
///
/// # Errors
///
/// `401`/`403` per [`auth::authorize_reconcile`]; `404` when `entity` is
/// not a recognised type or has no `LINK_GRAPH_RECONCILE_URL_<ENTITY>`
/// configured (nothing to force); a `500` on a fetch/DB error from the
/// pass itself.
#[debug_handler]
async fn force_reconcile(
    Path(entity): Path<String>,
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
) -> Result<Response> {
    auth::authorize_reconcile(&headers).map_err(auth_rejection)?;
    let Some(source) = HttpAuthoritativeSource::from_env_for(&entity) else {
        return Err(Error::CustomError(
            StatusCode::NOT_FOUND,
            ErrorDetail::new(
                "not_configured",
                format!(
                    "no reconciliation source configured for entity `{entity}` \
                     (unknown type, or LINK_GRAPH_RECONCILE_URL_{} unset)",
                    entity.to_ascii_uppercase()
                ),
            ),
        ));
    };
    let divergence_count = reconcile::reconcile(&ctx.db, &source).await?;
    format::json(serde_json::json!({
        "entity": entity,
        "divergence_count": divergence_count,
        "as_of": chrono::Utc::now(),
    }))
}

/// The admin control-plane routes, mounted under `/api/admin`.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/admin")
        .add("/reconcile/{entity}", post(force_reconcile))
}
