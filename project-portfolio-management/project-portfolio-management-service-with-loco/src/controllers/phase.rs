//! The **project phase** surface (entity spec §5.9.4 / §9.2b / FR-30).
//!
//! Pure rules live in [`crate::phase`]. Three things this controller is
//! responsible for keeping true:
//!
//! - **The payload and the column agree.** `phase` is a field on the
//!   matcher's `Plan` (informational-only, never scored) *and* a
//!   denormalised column, and both are written together — the same
//!   contract `name` and `parent_pid` already carry.
//! - **The log is append-only.** There is no edit or delete route: a
//!   phase history that can be rewritten cannot support a duration
//!   claim.
//! - **Phase never gates an operational write.** Nothing here is
//!   consulted by the task, issue, or sprint paths, and that is
//!   deliberate — refusing writes on the basis of a phase teaches
//!   operators to misreport it.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use project_portfolio_management_matcher::PlanPhase;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{phase_transitions, plans};
use crate::models::audit_logs::Model as AuditModel;
use crate::phase as rules;

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// `422` naming what was wrong, not merely that something was.
fn refuse(detail: &rules::Refusal) -> Error {
    let message = match detail {
        rules::Refusal::SkippedPhase { skipped } => format!(
            "advancement is one step at a time: this would skip `{skipped}`. \
             Move through it, or record the plan where it truly is."
        ),
        rules::Refusal::SilentRegression => {
            "a backward move is permitted but must carry a `reason`: re-planning is \
             normal, an unexplained regression is not"
                .to_string()
        }
        rules::Refusal::NoChange => "the plan is already in that phase".to_string(),
    };
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", &message),
    )
}

async fn find_plan(ctx: &AppContext, raw: &str) -> Result<plans::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    plans::Entity::find()
        .filter(plans::Column::Pid.eq(pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// `PUT /api/plans/{pid}/phase` body.
#[derive(Debug, Deserialize)]
struct PhasePayload {
    phase: String,
    #[serde(default)]
    reason: Option<String>,
}

/// `PUT /api/plans/{pid}/phase` — advance one step, or move back with a
/// stated reason.
#[debug_handler]
async fn set_phase(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PhasePayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;

    // An unrecognised token is refused, never coerced to a default: a
    // typo must not silently place a plan in Initiating.
    let Some(to) = PlanPhase::parse(&payload.phase) else {
        return Err(Error::CustomError(
            StatusCode::UNPROCESSABLE_ENTITY,
            ErrorDetail::new(
                "unprocessable",
                "phase must be one of initiating, planning, executing, controlling, closing",
            ),
        ));
    };
    let from = plan.phase.as_deref().and_then(PlanPhase::parse);
    rules::check_move(from, to, payload.reason.as_deref()).map_err(|e| refuse(&e))?;

    phase_transitions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        from_phase: ActiveValue::set(from.map(|p| p.token().to_string())),
        to_phase: ActiveValue::set(to.token().to_string()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        reason: ActiveValue::set(payload.reason.clone()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    // Payload and column together, so the two cannot drift.
    let mut data = plan.data.clone();
    if let Some(object) = data.as_object_mut() {
        object.insert(
            "phase".to_string(),
            serde_json::Value::String(format!("{to:?}")),
        );
    }
    let plan_pid = plan.pid;
    let mut active: plans::ActiveModel = plan.into();
    active.phase = ActiveValue::set(Some(to.token().to_string()));
    active.data = ActiveValue::set(data);
    active.update(&ctx.db).await.map_err(db_err)?;

    AuditModel::record(&ctx.db, plan_pid, "phase_changed", caller.actor(), None)
        .await
        .ok();

    // Workflow automation (FR-32). The phase change is already
    // committed, so a rule that fails is logged as a failed run and
    // never undoes the operator's move — the invariant the board move
    // already holds.
    //
    // `plan_phase_changed` is deliberately its own trigger, not folded
    // into `plan_stage_changed`: the gate stage and the project phase
    // are separate ordered vocabularies (§1.5.1), and one rule firing
    // on both would fire on the wrong kind of change half the time.
    super::automation::fire(
        &ctx,
        &crate::automation::TriggerFact {
            kind: "plan_phase_changed".to_string(),
            plan_pid,
            from_status: from.map(|phase| phase.token().to_string()),
            to_status: Some(to.token().to_string()),
        },
        "plan",
        plan_pid,
        caller.actor(),
    )
    .await;

    format::json(serde_json::json!({
        "phase": to.token(),
        "next_phase": rules::next_phase(Some(to)).map(PlanPhase::token),
    }))
}

/// `GET /api/plans/{pid}/phase-history` — the transitions and the time
/// spent in each phase, **every phase present even at zero**.
#[debug_handler]
async fn history(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = phase_transitions::Entity::find()
        .filter(phase_transitions::Column::PlanPid.eq(plan.pid))
        .order_by_asc(phase_transitions::Column::OccurredAt)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let facts: Vec<rules::TransitionFact> = rows
        .iter()
        .filter_map(|row| {
            PlanPhase::parse(&row.to_phase).map(|to| rules::TransitionFact {
                to,
                at_ms: row.occurred_at.timestamp_millis(),
            })
        })
        .collect();

    let current = plan.phase.as_deref().and_then(PlanPhase::parse);
    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "current_phase": current.map(PlanPhase::token),
        "next_phase": rules::next_phase(current).map(PlanPhase::token),
        "transitions": rows,
        "durations": rules::durations(&facts, chrono::Utc::now().timestamp_millis()),
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The phase routes. **No edit or delete route on the log** — that is
/// the append-only property, expressed as an absence.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/phase", put(set_phase))
        .add("/plans/{pid}/phase-history", get(history))
}
