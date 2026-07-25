//! Reasonable adjustments (WPM-R33 / WPM-D25) — barrier-based
//! requests ("what gets in the way, how it affects the work, what
//! change would reduce it"), a decision lifecycle with in-app
//! notification, and content-tier masking: a masked read keeps
//! category + status and withholds the words. No diagnosis is ever
//! required, and the schema has nowhere to put one.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::adjustment_requests;
use crate::models::audit_logs::Model as Audit;
use crate::models::notifications::Model as Notification;
use crate::models::records;
use crate::rules::adjustments as rules;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

/// `POST /api/employees/{pid}/adjustment-requests` body — the useful
/// bit and nothing else: barrier, impact, change.
#[derive(Debug, Deserialize)]
struct RequestPayload {
    category: String,
    barrier: String,
    impact: String,
    adjustment: String,
}

/// `POST /api/employees/{pid}/adjustment-requests` — put it in
/// writing. All three texts required; `$sub` ownership applies (HR
/// may file on behalf per policy). Audited.
#[debug_handler]
async fn create_request(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<RequestPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("category", rules::ADJUSTMENT_CATEGORIES, &payload.category);
    problems.require_text("barrier", &payload.barrier);
    problems.cap_text("barrier", &payload.barrier);
    problems.require_text("impact", &payload.impact);
    problems.cap_text("impact", &payload.impact);
    problems.require_text("adjustment", &payload.adjustment);
    problems.cap_text("adjustment", &payload.adjustment);
    ensure_valid(&problems.into_vec())?;
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let row = adjustment_requests::ActiveModel {
        pid: ActiveValue::set(uuid::Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        category: ActiveValue::set(payload.category.clone()),
        barrier: ActiveValue::set(payload.barrier.clone()),
        impact: ActiveValue::set(payload.impact.clone()),
        adjustment: ActiveValue::set(payload.adjustment.clone()),
        status: ActiveValue::set("requested".to_string()),
        decision_note: ActiveValue::set(None),
        decided_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    // The audit row records that a request was made — not its words.
    Audit::record(
        &ctx.db,
        "adjustment_request",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "category": payload.category })),
    )
    .await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/employees/{pid}/adjustment-requests` — the employee's
/// requests, newest first. `$sub`-owned; content-tier: a masked read
/// keeps category + status and **withholds the words**; unmasked
/// reads are audited.
#[debug_handler]
async fn list_requests(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let obligations = auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let masked = obligations.iter().any(|o| o == "mask");
    let rows = adjustment_requests::Entity::find()
        .filter(adjustment_requests::Column::EmployeePid.eq(employee.pid))
        .filter(adjustment_requests::Column::DeletedAt.is_null())
        .order_by_desc(adjustment_requests::Column::Id)
        .all(&ctx.db)
        .await?;
    if !masked && !rows.is_empty() {
        Audit::record(
            &ctx.db,
            "adjustment_request",
            employee.pid,
            "adjustments_read",
            caller.actor(),
            None,
        )
        .await?;
    }
    let view: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            if masked {
                serde_json::json!({
                    "pid": row.pid, "category": row.category, "status": row.status,
                    "decided_on": row.decided_on, "words_withheld": true,
                })
            } else {
                serde_json::json!({
                    "pid": row.pid, "category": row.category, "status": row.status,
                    "barrier": row.barrier, "impact": row.impact,
                    "adjustment": row.adjustment,
                    "decision_note": row.decision_note, "decided_on": row.decided_on,
                    "words_withheld": false,
                })
            }
        })
        .collect();
    format::json(view)
}

/// `POST /api/adjustment-requests/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct DecisionPayload {
    to: String,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/adjustment-requests/{pid}/status` — decide (pure
/// machine); stamps the date, records the practical note, audits, and
/// notifies the employee in-app.
#[debug_handler]
async fn decide(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<DecisionPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.cap_opt("note", payload.note.as_deref());
    ensure_valid(&problems.into_vec())?;
    let row = adjustment_requests::Entity::find()
        .filter(adjustment_requests::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(adjustment_requests::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    rules::transition(&row.status, &payload.to).map_err(|reason| unprocessable(&reason))?;
    let from = row.status.clone();
    let row_pid = row.pid;
    let employee_pid = row.employee_pid;
    let category = row.category.clone();
    let mut active: adjustment_requests::ActiveModel = row.into();
    active.status = ActiveValue::set(payload.to.clone());
    active.decided_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    if payload.note.is_some() {
        active.decision_note = ActiveValue::set(payload.note.clone());
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "adjustment_request",
        row_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    // Tell the employee — the body names the category and the state,
    // never the words (WPM-D23 reference-only posture).
    Notification::push(
        &ctx.db,
        employee_pid,
        "adjustment_update",
        &format!("Adjustment request ({category}): {}", payload.to),
        serde_json::json!({ "adjustment_request_pid": row_pid }),
    )
    .await?;
    format::json(updated)
}

/// The adjustment routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/employees/{pid}/adjustment-requests", post(create_request))
        .add("/employees/{pid}/adjustment-requests", get(list_requests))
        .add("/adjustment-requests/{pid}/status", post(decide))
}
