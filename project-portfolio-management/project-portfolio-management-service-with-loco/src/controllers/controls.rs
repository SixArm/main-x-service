//! **Controls** — the Controlling-process register (entity spec §5.9.8
//! / FR-38, FR-39). Pure rules live in [`crate::controls`].
//!
//! The four process steps are records here, not habits: set a standard
//! (a `controls` row), measure (a `control_readings` row), compare (the
//! verdict and gap, derived at write from the standard), act (a
//! `control_actions` row).
//!
//! **The timing decides what a failing control may do**, and that is
//! enforced rather than documented: only a `feedforward` control is
//! allowed to block, because acting before the fact is that timing's
//! entire purpose; a `feedback` control judges finished work and may
//! only record.
//!
//! Two honesty rules the read path depends on:
//!
//! - **A control naming a metric this service does not produce is
//!   refused at write** (`422`). Left registered, it would read
//!   `unmeasured` forever — and a check nobody can evaluate is
//!   indistinguishable from one that passes.
//! - **`unmeasured` is a third verdict**, excluded from pass rates
//!   rather than counted as either half. An all-unmeasured control
//!   reports `null`, never `0%`.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::controls as rules;
use crate::models::_entities::{control_actions, control_readings, controls, plans, tasks};
use crate::models::audit_logs::Model as AuditModel;
use crate::validation::MAX_TEXT_LEN;

/// The metrics a control may name. A control's source must resolve to
/// something this service actually produces (FR-38); the list is the
/// derived figures the read surfaces already serve.
pub const KNOWN_METRICS: &[&str] = &[
    "flow_efficiency",
    "cycle_time_p85",
    "lead_time_p85",
    "throughput",
    "work_in_progress",
    "first_pass_yield",
    "burndown_remaining",
    "smart_score",
    "dipp",
    "dipp_progress_index",
    "roi",
    "budget_variance",
    "risk_exposure",
    "gate_readiness",
    "adoption_rate",
    "value_realization_rate",
    "time_to_value",
];

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

fn find_plan_pid(raw: &str) -> Result<Uuid> {
    Uuid::parse_str(raw).map_err(|_| Error::NotFound)
}

async fn find_plan(ctx: &AppContext, raw: &str) -> Result<plans::Model> {
    let pid = find_plan_pid(raw)?;
    plans::Entity::find()
        .filter(plans::Column::Pid.eq(pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

async fn find_control(ctx: &AppContext, raw: &str) -> Result<controls::Model> {
    let pid = find_plan_pid(raw)?;
    controls::Entity::find()
        .filter(controls::Column::Pid.eq(pid))
        .filter(controls::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

async fn find_reading(ctx: &AppContext, raw: &str) -> Result<control_readings::Model> {
    let pid = find_plan_pid(raw)?;
    control_readings::Entity::find()
        .filter(control_readings::Column::Pid.eq(pid))
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

async fn find_action(ctx: &AppContext, raw: &str) -> Result<control_actions::Model> {
    let pid = find_plan_pid(raw)?;
    control_actions::Entity::find()
        .filter(control_actions::Column::Pid.eq(pid))
        .filter(control_actions::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// Parse a wire token into a [`rules::Timing`].
fn parse_timing(raw: &str) -> Option<rules::Timing> {
    match raw {
        "feedforward" => Some(rules::Timing::Feedforward),
        "concurrent" => Some(rules::Timing::Concurrent),
        "feedback" => Some(rules::Timing::Feedback),
        _ => None,
    }
}

/// The wire token for a [`rules::Timing`].
const fn timing_token(timing: rules::Timing) -> &'static str {
    match timing {
        rules::Timing::Feedforward => "feedforward",
        rules::Timing::Concurrent => "concurrent",
        rules::Timing::Feedback => "feedback",
    }
}

fn parse_comparator(raw: &str) -> Option<rules::Comparator> {
    match raw {
        "at_least" => Some(rules::Comparator::AtLeast),
        "at_most" => Some(rules::Comparator::AtMost),
        "within" => Some(rules::Comparator::Within),
        "equals" => Some(rules::Comparator::Equals),
        _ => None,
    }
}

const fn verdict_token(verdict: rules::Verdict) -> &'static str {
    match verdict {
        rules::Verdict::Pass => "pass",
        rules::Verdict::Fail => "fail",
        rules::Verdict::Unmeasured => "unmeasured",
    }
}

fn parse_verdict(raw: &str) -> rules::Verdict {
    match raw {
        "pass" => rules::Verdict::Pass,
        "fail" => rules::Verdict::Fail,
        // Anything unrecognised reads as unmeasured rather than as a
        // pass: a typo must never become a green control.
        _ => rules::Verdict::Unmeasured,
    }
}

/// Rebuild the pure standard from a stored row.
fn standard_of(row: &controls::Model) -> rules::Standard {
    rules::Standard {
        metric: row.metric.clone(),
        target_value: row.target_value,
        comparator: parse_comparator(&row.comparator).unwrap_or(rules::Comparator::Equals),
        tolerance: row.tolerance,
    }
}

/// `POST /api/plans/{pid}/controls` body.
#[derive(Debug, Deserialize)]
struct ControlPayload {
    name: String,
    timing: String,
    metric: String,
    target_value: i64,
    comparator: String,
    #[serde(default)]
    tolerance: Option<i64>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default = "default_source_kind")]
    source_kind: String,
    #[serde(default)]
    source_ref: Option<String>,
    #[serde(default)]
    cadence_days: Option<i64>,
    #[serde(default)]
    owner_ref: Option<String>,
}

fn default_source_kind() -> String {
    "metric".to_string()
}

/// `POST /api/plans/{pid}/controls` — register a control.
#[debug_handler]
async fn create(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ControlPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;

    let Some(timing) = parse_timing(&payload.timing) else {
        return Err(unprocessable(
            "timing must be feedforward, concurrent, or feedback",
        ));
    };
    let Some(comparator) = parse_comparator(&payload.comparator) else {
        return Err(unprocessable(
            "comparator must be at_least, at_most, within, or equals",
        ));
    };
    if !["metric", "query", "manual"].contains(&payload.source_kind.as_str()) {
        return Err(unprocessable(
            "source_kind must be metric, query, or manual",
        ));
    }
    if payload.name.len() > MAX_TEXT_LEN {
        return Err(unprocessable("name is capped"));
    }
    if payload.cadence_days.is_some_and(|d| d <= 0) {
        return Err(unprocessable("cadence_days must be positive when present"));
    }

    let standard = rules::Standard {
        metric: payload.metric.clone(),
        target_value: payload.target_value,
        comparator,
        tolerance: payload.tolerance,
    };

    // The whole point of validating here rather than at read time: a
    // control that can never be evaluated must not be registerable.
    if let Err(problems) = rules::validate(&payload.name, &standard, KNOWN_METRICS) {
        return Err(unprocessable(&format!(
            "control is not evaluable: {}",
            serde_json::to_string(&problems).unwrap_or_default()
        )));
    }

    let row = controls::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        name: ActiveValue::set(payload.name.trim().to_string()),
        timing: ActiveValue::set(timing_token(timing).to_string()),
        metric: ActiveValue::set(payload.metric.clone()),
        target_value: ActiveValue::set(payload.target_value),
        comparator: ActiveValue::set(payload.comparator.clone()),
        tolerance: ActiveValue::set(payload.tolerance),
        unit: ActiveValue::set(payload.unit.clone()),
        currency: ActiveValue::set(payload.currency.clone()),
        source_kind: ActiveValue::set(payload.source_kind.clone()),
        source_ref: ActiveValue::set(payload.source_ref.clone()),
        cadence_days: ActiveValue::set(payload.cadence_days),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        enabled: ActiveValue::set(true),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        plan.pid,
        "control_registered",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({
        "pid": row.pid.to_string(),
        "permitted_response": rules::permitted_response(timing),
    }))
}

/// `GET /api/plans/{pid}/controls` — the register for one plan.
#[debug_handler]
async fn list(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = controls::Entity::find()
        .filter(controls::Column::PlanPid.eq(plan.pid))
        .filter(controls::Column::DeletedAt.is_null())
        .order_by_asc(controls::Column::Name)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!(rows))
}

/// `DELETE /api/controls/{pid}` — soft-delete a control. The readings
/// stay: they are the history of what was measured, and deleting a
/// control does not unmeasure the past.
#[debug_handler]
async fn remove(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let row = find_control(&ctx, &pid).await?;
    let plan_pid = row.plan_pid;
    let mut active: controls::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(&ctx.db, plan_pid, "control_withdrawn", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// `POST /api/controls/{pid}/readings` body. `value` absent is
/// `unmeasured`, which is deliberate and is not an error.
#[derive(Debug, Deserialize)]
struct ReadingPayload {
    #[serde(default)]
    value: Option<i64>,
    #[serde(default = "default_method")]
    method: String,
}

fn default_method() -> String {
    "manual".to_string()
}

/// `POST /api/controls/{pid}/readings` — measure and compare, in one
/// step, because a reading whose verdict is computed later can disagree
/// with the standard that was in force when it was taken.
#[debug_handler]
async fn add_reading(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ReadingPayload>,
) -> Result<Response> {
    let control = find_control(&ctx, &pid).await?;
    let (verdict, gap) = rules::compare(&standard_of(&control), payload.value);

    let row = control_readings::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        control_pid: ActiveValue::set(control.pid),
        value: ActiveValue::set(payload.value),
        verdict: ActiveValue::set(verdict_token(verdict).to_string()),
        gap: ActiveValue::set(gap),
        method: ActiveValue::set(payload.method.clone()),
        accepted_at: ActiveValue::set(None),
        accepted_reason: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        control.plan_pid,
        "control_reading_recorded",
        caller.actor(),
        None,
    )
    .await
    .ok();

    format::json(serde_json::json!({
        "pid": row.pid.to_string(),
        "verdict": verdict_token(verdict),
        "gap": gap,
        "permitted_response": rules::permitted_response(
            parse_timing(&control.timing).unwrap_or(rules::Timing::Feedback),
        ),
    }))
}

/// `GET /api/controls/{pid}/readings` — newest first.
#[debug_handler]
async fn list_readings(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let control = find_control(&ctx, &pid).await?;
    let rows = control_readings::Entity::find()
        .filter(control_readings::Column::ControlPid.eq(control.pid))
        .order_by_desc(control_readings::Column::ObservedAt)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!(rows))
}

/// `POST /api/readings/{pid}/actions` body.
#[derive(Debug, Deserialize)]
struct ActionPayload {
    kind: String,
    description: String,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/readings/{pid}/actions` — record what a failing reading
/// provoked. `accept` also stamps the reading as explicitly accepted,
/// which is what stops it being reported as **unanswered**.
#[debug_handler]
async fn add_action(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ActionPayload>,
) -> Result<Response> {
    let reading = find_reading(&ctx, &pid).await?;
    if !["correct", "adjust", "retrain", "accept", "escalate"].contains(&payload.kind.as_str()) {
        return Err(unprocessable(
            "kind must be correct, adjust, retrain, accept, or escalate",
        ));
    }
    if payload.description.trim().is_empty() || payload.description.len() > MAX_TEXT_LEN {
        return Err(unprocessable("description is required (and capped)"));
    }

    let row = control_actions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        reading_pid: ActiveValue::set(reading.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        description: ActiveValue::set(payload.description.trim().to_string()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        due_date: ActiveValue::set(None),
        converted_task_pid: ActiveValue::set(None),
        converted_issue_pid: ActiveValue::set(None),
        closed_at: ActiveValue::set(None),
        outcome: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    if payload.kind == "accept" {
        let mut active: control_readings::ActiveModel = reading.clone().into();
        active.accepted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        active.accepted_reason = ActiveValue::set(payload.reason.clone());
        active.update(&ctx.db).await.map_err(db_err)?;
    }

    AuditModel::record(
        &ctx.db,
        reading.control_pid,
        "control_action_recorded",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `POST /api/actions/{pid}/convert` — turn a control action into a
/// task on the control's own plan (T-26: actions convert into the work
/// stores that already exist rather than becoming a fifth one — see
/// `control_actions`' migration doc comment). Created into the
/// **initial** state of the workflow in force for that plan, exactly
/// like an ordinary task create (`engineering::create_task`), so it
/// enters the board and the flow analysis identically to a
/// hand-created one.
///
/// **Issue conversion is deliberately not offered here.** This service
/// has no `issues` store yet (spec §13, FR-14, deferred); the
/// `converted_issue_pid` column stays reserved, `NULL`, until one
/// lands, rather than this endpoint faking an issue as a task.
#[debug_handler]
async fn convert_action(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let action = find_action(&ctx, &pid).await?;
    if action.converted_task_pid.is_some() || action.converted_issue_pid.is_some() {
        return Err(unprocessable("action is already converted"));
    }
    if action.closed_at.is_some() {
        return Err(unprocessable("action is already closed"));
    }
    let reading = control_readings::Entity::find()
        .filter(control_readings::Column::Pid.eq(action.reading_pid))
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    let control = controls::Entity::find()
        .filter(controls::Column::Pid.eq(reading.control_pid))
        .filter(controls::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;

    // The vocabulary comes from the workflow in force for the control's
    // plan, exactly like `engineering::create_task` — a plan with a
    // custom task workflow gets the converted task in *its* initial
    // state, not a hardcoded "todo".
    let workflow = super::workflow::in_force(&ctx, control.plan_pid, "task").await?;
    let initial = workflow
        .states
        .iter()
        .find(|state| state.is_initial)
        .map_or("todo", |state| state.key.as_str());

    let now = chrono::Utc::now();
    // The task and its opening transition commit together with the
    // action's own update, same reasoning as task creation (spec
    // `time-based-analysis.md` §5.1 invariant 3): a converted task
    // without a transition would silently begin its life at its first
    // later move, and an action left unmarked could be converted twice.
    let txn = ctx.db.begin().await.map_err(db_err)?;
    let task = tasks::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(control.plan_pid),
        sprint_pid: ActiveValue::set(None),
        title: ActiveValue::set(action.description.clone()),
        description: ActiveValue::set(Some(format!(
            "Converted from a {} action on control \"{}\".",
            action.kind, control.name
        ))),
        status: ActiveValue::set(initial.to_string()),
        assignee_ref: ActiveValue::set(action.owner_ref.clone()),
        points: ActiveValue::set(None),
        status_changed_at: ActiveValue::set(now.into()),
        flow_type: ActiveValue::set(None),
        done_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(db_err)?;
    crate::tba::transition_row(
        task.pid,
        control.plan_pid,
        None,
        initial.to_string(),
        now,
        caller.actor().map(ToString::to_string),
        action.owner_ref.clone(),
    )
    .insert(&txn)
    .await
    .map_err(db_err)?;
    let mut active: control_actions::ActiveModel = action.clone().into();
    active.converted_task_pid = ActiveValue::set(Some(task.pid));
    active.update(&txn).await.map_err(db_err)?;
    txn.commit().await.map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        control.pid,
        "control_action_converted",
        caller.actor(),
        Some(serde_json::json!({
            "action_pid": action.pid.to_string(),
            "task_pid": task.pid.to_string(),
        })),
    )
    .await
    .ok();
    format::json(serde_json::json!({
        "pid": action.pid.to_string(),
        "task_pid": task.pid.to_string(),
    }))
}

/// Build the coverage facts for a set of controls.
async fn coverage_of(
    ctx: &AppContext,
    control_rows: Vec<controls::Model>,
) -> Result<rules::Coverage> {
    let now = chrono::Utc::now();
    let mut facts = Vec::with_capacity(control_rows.len());

    for control in control_rows {
        let readings = control_readings::Entity::find()
            .filter(control_readings::Column::ControlPid.eq(control.pid))
            .order_by_desc(control_readings::Column::ObservedAt)
            .all(&ctx.db)
            .await
            .map_err(db_err)?;

        let mut reading_facts = Vec::with_capacity(readings.len());
        for reading in readings {
            let answered = reading.accepted_at.is_some()
                || control_actions::Entity::find()
                    .filter(control_actions::Column::ReadingPid.eq(reading.pid))
                    .filter(control_actions::Column::DeletedAt.is_null())
                    .one(&ctx.db)
                    .await
                    .map_err(db_err)?
                    .is_some();
            reading_facts.push(rules::ReadingFact {
                age_days: (now - reading.observed_at.with_timezone(&chrono::Utc))
                    .num_days()
                    .max(0),
                verdict: parse_verdict(&reading.verdict),
                answered,
            });
        }

        facts.push(rules::ControlFact {
            timing: parse_timing(&control.timing).unwrap_or(rules::Timing::Feedback),
            cadence_days: control.cadence_days,
            readings: reading_facts,
            enabled: control.enabled,
        });
    }

    Ok(rules::coverage(&facts))
}

/// `GET /api/plans/{pid}/controls/coverage` — what is **not** being
/// controlled on one plan.
#[debug_handler]
async fn plan_coverage(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = controls::Entity::find()
        .filter(controls::Column::PlanPid.eq(plan.pid))
        .filter(controls::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "coverage": coverage_of(&ctx, rows).await?,
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// `GET /api/controls/coverage` — the portfolio-wide answer to the same
/// question. Plans with **no controls at all** are named, because an
/// empty cell is the finding a register exists to surface.
#[debug_handler]
async fn portfolio_coverage(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let rows = controls::Entity::find()
        .filter(controls::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let controlled: std::collections::HashSet<Uuid> =
        rows.iter().map(|control| control.plan_pid).collect();
    let uncontrolled: Vec<String> = plans::Entity::find()
        .filter(plans::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?
        .into_iter()
        .filter(|plan| !controlled.contains(&plan.pid))
        .map(|plan| plan.pid.to_string())
        .collect();

    let body = serde_json::json!({
        "coverage": coverage_of(&ctx, rows).await?,
        "plans_with_no_controls": uncontrolled,
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The control routes (entity spec §9.2c).
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/controls", post(create))
        .add("/plans/{pid}/controls", get(list))
        .add("/plans/{pid}/controls/coverage", get(plan_coverage))
        .add("/controls/coverage", get(portfolio_coverage))
        .add("/controls/{pid}", delete(remove))
        .add("/controls/{pid}/readings", post(add_reading))
        .add("/controls/{pid}/readings", get(list_readings))
        .add("/readings/{pid}/actions", post(add_action))
        .add("/actions/{pid}/convert", post(convert_action))
}
