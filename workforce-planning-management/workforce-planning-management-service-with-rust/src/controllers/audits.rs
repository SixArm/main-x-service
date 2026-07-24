//! Audit + event read surface: the recent trail, the per-record
//! slice, the **department-scoped query** (spec `audit.md`), and the
//! recent event stream.

use loco_rs::prelude::*;
use serde::Deserialize;

use crate::models::audit_logs::Model as Audit;
use crate::models::event_outbox::Model as Outbox;
use crate::models::records;
use crate::streaming;

/// `GET /api/audits/recent` — the most recent entries (cap 100).
#[debug_handler]
async fn recent(State(ctx): State<AppContext>) -> Result<Response> {
    format::json(Audit::recent(&ctx.db, 100).await?)
}

/// `GET /api/audits/{entity_pid}` — one record's trail.
#[debug_handler]
async fn for_entity(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    format::json(Audit::for_entity(&ctx.db, records::parse_pid(&pid)?).await?)
}

/// `GET /api/audits?department=<name>&since=<rfc3339>` — everything
/// that changed in one department since a timestamp (cap 200).
#[derive(Debug, Deserialize)]
struct DepartmentParams {
    department: String,
    since: chrono::DateTime<chrono::FixedOffset>,
}

#[debug_handler]
async fn department(
    State(ctx): State<AppContext>,
    Query(params): Query<DepartmentParams>,
) -> Result<Response> {
    format::json(Audit::for_department_since(&ctx.db, &params.department, params.since, 200).await?)
}

/// `GET /api/events/recent` — the flat event view (memory ring under
/// the default transport; the outbox table under `outbox`).
#[debug_handler]
async fn events_recent(State(ctx): State<AppContext>) -> Result<Response> {
    if streaming::transport().is_outbox() {
        format::json(Outbox::recent(&ctx.db, 100).await?)
    } else {
        format::json(streaming::recent(100))
    }
}

/// The audit/event routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/audits/recent", get(recent))
        .add("/audits", get(department))
        .add("/audits/{entity_pid}", get(for_entity))
        .add("/events/recent", get(events_recent))
}
