//! Workflow automation and the set-and-forget scheduler.
//!
//! - **Automations** — rules an operator configures once (`POST
//!   /api/automations`). When a task crosses the Kanban board (or a
//!   review verdict lands) [`fire`] evaluates them with the pure engine
//!   in [`crate::automation`] and applies the matching actions.
//! - **Scheduled actions** — the deadline queue. An action configured
//!   once is held until `due_at`, then fired **exactly once** by the
//!   sweep (`POST /api/scheduled-actions/sweep`, or the env-gated
//!   in-process ticker).
//! - **Runs** — every firing writes an `automation_runs` row, applied
//!   or not, so an automatic change is always traceable to the rule
//!   that made it.
//!
//! Three deliberate limits:
//!
//! - **An automation never breaks the operator's action.** If a rule
//!   fails, the failure is recorded as a `failed` run and the move that
//!   triggered it still stands. Automation is an assistant, not a gate.
//! - **No cascades.** Actions are applied without re-entering the
//!   engine, so one board move fires one generation of rules.
//! - **Exactly once.** The sweep claims each due row with a conditional
//!   update and only acts when it won the claim, so two concurrent
//!   sweeps cannot double-fire a deadline.

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::pagination::{Page, with_page_headers};
use crate::auth::MaybeAuthUser;
use crate::automation as rules;
use crate::engineering::TASK_STATUSES;
use crate::models::_entities::{plans, reviews, scheduled_actions, tasks};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::capabilities as cap_models;
use crate::validation::MAX_TEXT_LEN;

/// Default page size for every list endpoint here — the cap each one
/// applied before `?limit=`/`?offset=` pagination (agents/share/restful.md)
/// landed; a bigger `limit` clamps rather than errors
/// ([`super::pagination::MAX_LIMIT`]).
pub const LIST_CAP: u64 = 200;

/// Highest number of due actions one sweep will fire. A bounded sweep
/// keeps a backlog from turning into an unbounded write burst; the
/// response reports whether the cap was hit.
pub const SWEEP_CAP: u64 = 500;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// `POST /api/automations` body.
#[derive(Debug, Deserialize)]
struct AutomationPayload {
    /// Scope the rule to one plan's board; absent = every plan.
    #[serde(default)]
    plan_pid: Option<Uuid>,
    name: String,
    trigger_kind: String,
    #[serde(default)]
    from_status: Option<String>,
    #[serde(default)]
    to_status: Option<String>,
    action_kind: String,
    #[serde(default)]
    action_value: serde_json::Value,
}

/// `POST /api/automations` — configure one rule. The trigger and the
/// action are validated here, at write time, so a malformed rule can
/// never fail silently later when nobody is watching.
#[debug_handler]
async fn create_automation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AutomationPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if payload.name.trim().is_empty() || payload.name.len() > MAX_TEXT_LEN {
        problems.push(format!(
            "name is required and capped at {MAX_TEXT_LEN} characters"
        ));
    }
    if let Err(e) = rules::validate_trigger(
        &payload.trigger_kind,
        payload.from_status.as_deref(),
        payload.to_status.as_deref(),
        TASK_STATUSES,
    ) {
        problems.push(e);
    }
    let action_value = if payload.action_value.is_null() {
        serde_json::json!({})
    } else {
        payload.action_value.clone()
    };
    if let Err(e) = rules::validate_action(&payload.action_kind, &action_value, TASK_STATUSES) {
        problems.push(e);
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    if let Some(plan_pid) = payload.plan_pid {
        plans::Entity::find()
            .filter(plans::Column::Pid.eq(plan_pid))
            .filter(plans::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .ok_or(Error::NotFound)?;
    }
    let row = crate::models::_entities::automations::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(payload.plan_pid),
        name: ActiveValue::set(payload.name.trim().to_string()),
        trigger_kind: ActiveValue::set(payload.trigger_kind.clone()),
        from_status: ActiveValue::set(payload.from_status.clone()),
        to_status: ActiveValue::set(payload.to_status.clone()),
        action_kind: ActiveValue::set(payload.action_kind.clone()),
        action_value: ActiveValue::set(action_value),
        enabled: ActiveValue::set(true),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        row.pid,
        "automation_created",
        caller.actor(),
        Some(serde_json::json!({
            "trigger_kind": row.trigger_kind,
            "action_kind": row.action_kind,
            "plan_pid": row.plan_pid.map(|p| p.to_string()),
        })),
    )
    .await
    .ok();
    format::json(row)
}

/// `GET /api/automations` filter.
#[derive(Debug, Deserialize)]
struct AutomationParams {
    #[serde(default)]
    plan: Option<Uuid>,
    #[serde(default)]
    trigger: Option<String>,
    /// `true` ⇒ only enabled rules.
    #[serde(default)]
    enabled: Option<bool>,
    /// Page size; absent ⇒ [`LIST_CAP`]. Declared inline rather than
    /// `#[serde(flatten)]`-ing [`Page`] in — see that type's docs.
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

/// `GET /api/automations?plan=&trigger=&enabled=&limit=&offset=` — the
/// configured rules, oldest first. Paginated (agents/share/restful.md):
/// `X-Total-Count`/`X-Limit`/`X-Offset` on the response.
///
/// # Errors
///
/// `400` when `offset` exceeds [`super::pagination::MAX_OFFSET`]; a DB
/// error when the query fails.
#[debug_handler]
async fn list_automations(
    State(ctx): State<AppContext>,
    Query(params): Query<AutomationParams>,
) -> Result<Response> {
    use crate::models::_entities::automations;
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_CAP);
    let mut query = automations::Entity::find().filter(automations::Column::DeletedAt.is_null());
    if let Some(plan) = params.plan {
        query = query.filter(automations::Column::PlanPid.eq(plan));
    }
    if let Some(trigger) = params.trigger.as_deref() {
        query = query.filter(automations::Column::TriggerKind.eq(trigger));
    }
    if let Some(enabled) = params.enabled {
        query = query.filter(automations::Column::Enabled.eq(enabled));
    }
    let total = query.clone().count(&ctx.db).await.map_err(db_err)?;
    let rows = query
        .order_by_asc(automations::Column::Id)
        .offset(offset)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(with_page_headers(format::json(rows)?, total, limit, offset))
}

/// Flip one rule's `enabled` flag.
async fn set_enabled(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: &str,
    enabled: bool,
) -> Result<Response> {
    let row =
        cap_models::find_automation(&ctx.db, crate::models::governance::parse_pid(pid)?).await?;
    let row_pid = row.pid;
    let mut active: crate::models::_entities::automations::ActiveModel = row.into();
    active.enabled = ActiveValue::set(enabled);
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    let action = if enabled {
        "automation_enabled"
    } else {
        "automation_disabled"
    };
    AuditModel::record(&ctx.db, row_pid, action, caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/automations/{pid}/enable`.
#[debug_handler]
async fn enable_automation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    set_enabled(&ctx, &caller, &pid, true).await
}

/// `POST /api/automations/{pid}/disable` — the off switch. Disabling
/// keeps the rule and its run history; deleting is a separate call.
#[debug_handler]
async fn disable_automation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    set_enabled(&ctx, &caller, &pid, false).await
}

/// `DELETE /api/automations/{pid}` — soft delete.
#[debug_handler]
async fn delete_automation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row =
        cap_models::find_automation(&ctx.db, crate::models::governance::parse_pid(&pid)?).await?;
    let row_pid = row.pid;
    let mut active: crate::models::_entities::automations::ActiveModel = row.into();
    active.enabled = ActiveValue::set(false);
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(&ctx.db, row_pid, "automation_deleted", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// `GET /api/automations/runs` filter.
#[derive(Debug, Deserialize)]
struct RunParams {
    #[serde(default)]
    automation: Option<Uuid>,
    #[serde(default)]
    subject: Option<Uuid>,
    #[serde(default)]
    outcome: Option<String>,
    /// Page size; absent ⇒ [`LIST_CAP`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

/// `GET /api/automations/runs?automation=&subject=&outcome=&limit=&offset=`
/// — what the automations actually did, newest first. Paginated
/// (agents/share/restful.md): `X-Total-Count`/`X-Limit`/`X-Offset` on
/// the response.
///
/// # Errors
///
/// `400` when `offset` exceeds [`super::pagination::MAX_OFFSET`]; a DB
/// error when the query fails.
#[debug_handler]
async fn list_runs(
    State(ctx): State<AppContext>,
    Query(params): Query<RunParams>,
) -> Result<Response> {
    use crate::models::_entities::automation_runs;
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_CAP);
    let mut query = automation_runs::Entity::find();
    if let Some(automation) = params.automation {
        query = query.filter(automation_runs::Column::AutomationPid.eq(automation));
    }
    if let Some(subject) = params.subject {
        query = query.filter(automation_runs::Column::SubjectPid.eq(subject));
    }
    if let Some(outcome) = params.outcome.as_deref() {
        query = query.filter(automation_runs::Column::Outcome.eq(outcome));
    }
    let total = query.clone().count(&ctx.db).await.map_err(db_err)?;
    let rows = query
        .order_by_desc(automation_runs::Column::Id)
        .offset(offset)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(with_page_headers(format::json(rows)?, total, limit, offset))
}

/// Read a non-blank string field from an action's stored value.
fn action_str(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

/// `assign` — put the named person on the task.
async fn act_assign(
    ctx: &AppContext,
    rule: &crate::models::_entities::automations::Model,
    subject_kind: &str,
    subject_pid: Uuid,
) -> (&'static str, serde_json::Value) {
    let Some(assignee) = action_str(&rule.action_value, "assignee_ref") else {
        return ("failed", serde_json::json!({ "reason": "no assignee_ref" }));
    };
    if subject_kind != "task" {
        return (
            "skipped",
            serde_json::json!({ "reason": "assign only applies to a task" }),
        );
    }
    let Ok(task) = find_live_task(ctx, subject_pid).await else {
        return (
            "skipped",
            serde_json::json!({ "reason": "task no longer live" }),
        );
    };
    let previous = task.assignee_ref.clone();
    let mut active: tasks::ActiveModel = task.into();
    active.assignee_ref = ActiveValue::set(Some(assignee.clone()));
    match active.update(&ctx.db).await {
        Ok(_) => {
            cap_models::notify(
                &ctx.db,
                &assignee,
                "task",
                subject_pid,
                "task_assigned",
                &format!("Assigned to you by automation `{}`", rule.name),
            )
            .await
            .ok();
            (
                "applied",
                serde_json::json!({ "from": previous, "to": assignee }),
            )
        }
        Err(e) => ("failed", serde_json::json!({ "reason": e.to_string() })),
    }
}

/// `set_task_status` — move the task itself.
///
/// Applied **without** re-entering the engine: automations do not
/// cascade.
async fn act_set_task_status(
    ctx: &AppContext,
    rule: &crate::models::_entities::automations::Model,
    subject_kind: &str,
    subject_pid: Uuid,
) -> (&'static str, serde_json::Value) {
    let Some(status) = action_str(&rule.action_value, "status") else {
        return ("failed", serde_json::json!({ "reason": "no status" }));
    };
    if subject_kind != "task" {
        return (
            "skipped",
            serde_json::json!({ "reason": "set_task_status only applies to a task" }),
        );
    }
    let Ok(task) = find_live_task(ctx, subject_pid).await else {
        return (
            "skipped",
            serde_json::json!({ "reason": "task no longer live" }),
        );
    };
    if task.status == status {
        return (
            "skipped",
            serde_json::json!({ "reason": "task already in that status" }),
        );
    }
    let from = task.status.clone();
    let first_done = task.done_at.is_none() && status == "done";
    let mut active: tasks::ActiveModel = task.into();
    active.status = ActiveValue::set(status.clone());
    active.status_changed_at = ActiveValue::set(chrono::Utc::now().into());
    if first_done {
        active.done_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    }
    match active.update(&ctx.db).await {
        Ok(_) => ("applied", serde_json::json!({ "from": from, "to": status })),
        Err(e) => ("failed", serde_json::json!({ "reason": e.to_string() })),
    }
}

/// `notify` — write one in-app notification.
async fn act_notify(
    ctx: &AppContext,
    rule: &crate::models::_entities::automations::Model,
    subject_kind: &str,
    subject_pid: Uuid,
) -> (&'static str, serde_json::Value) {
    let Some(recipient) = action_str(&rule.action_value, "recipient_ref") else {
        return (
            "failed",
            serde_json::json!({ "reason": "no recipient_ref" }),
        );
    };
    let message = action_str(&rule.action_value, "message")
        .unwrap_or_else(|| format!("Automation `{}` fired", rule.name));
    match cap_models::notify(
        &ctx.db,
        &recipient,
        subject_kind,
        subject_pid,
        "automation",
        &message,
    )
    .await
    {
        Ok(_) => ("applied", serde_json::json!({ "recipient_ref": recipient })),
        Err(e) => ("failed", serde_json::json!({ "reason": e.to_string() })),
    }
}

/// `schedule_action` — hand a deadline to the set-and-forget queue.
async fn act_schedule(
    ctx: &AppContext,
    rule: &crate::models::_entities::automations::Model,
    subject_kind: &str,
    subject_pid: Uuid,
    actor: Option<&str>,
) -> (&'static str, serde_json::Value) {
    let Some(action_kind) = action_str(&rule.action_value, "action_kind") else {
        return ("failed", serde_json::json!({ "reason": "no action_kind" }));
    };
    let Some(days) = rule
        .action_value
        .get("in_days")
        .and_then(serde_json::Value::as_i64)
    else {
        return ("failed", serde_json::json!({ "reason": "no in_days" }));
    };
    let due_at = chrono::Utc::now() + chrono::Duration::days(days);
    match cap_models::schedule(
        &ctx.db,
        cap_models::NewScheduledAction {
            subject_kind: subject_kind.to_string(),
            subject_pid,
            action_kind,
            payload: rule.action_value.clone(),
            due_at: due_at.into(),
            source_automation_pid: Some(rule.pid),
            created_by: actor.map(std::string::ToString::to_string),
        },
    )
    .await
    {
        Ok(row) => (
            "applied",
            serde_json::json!({
                "scheduled_action_pid": row.pid.to_string(),
                "due_at": row.due_at.to_rfc3339(),
            }),
        ),
        Err(e) => ("failed", serde_json::json!({ "reason": e.to_string() })),
    }
}

/// Apply one matched rule's action. Returns the run outcome and the
/// detail recorded against it.
async fn apply_action(
    ctx: &AppContext,
    rule: &crate::models::_entities::automations::Model,
    fact: &rules::TriggerFact,
    subject_kind: &str,
    subject_pid: Uuid,
    actor: Option<&str>,
) -> (&'static str, serde_json::Value) {
    match rule.action_kind.as_str() {
        "assign" => act_assign(ctx, rule, subject_kind, subject_pid).await,
        "set_task_status" => act_set_task_status(ctx, rule, subject_kind, subject_pid).await,
        "notify" => act_notify(ctx, rule, subject_kind, subject_pid).await,
        "schedule_action" => act_schedule(ctx, rule, subject_kind, subject_pid, actor).await,
        "add_label" => {
            let Some(label) = action_str(&rule.action_value, "label") else {
                return ("failed", serde_json::json!({ "reason": "no label" }));
            };
            match add_plan_label(ctx, fact.plan_pid, &label).await {
                Ok(true) => ("applied", serde_json::json!({ "label": label })),
                Ok(false) => (
                    "skipped",
                    serde_json::json!({ "reason": "plan already carries that label" }),
                ),
                Err(e) => ("failed", serde_json::json!({ "reason": e.to_string() })),
            }
        }
        other => (
            "failed",
            serde_json::json!({ "reason": format!("unknown action_kind `{other}`") }),
        ),
    }
}

/// One live task by pid.
async fn find_live_task(ctx: &AppContext, pid: Uuid) -> Result<tasks::Model> {
    tasks::Entity::find()
        .filter(tasks::Column::Pid.eq(pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// Append a label to a plan's payload tags, case-insensitively
/// de-duplicated (the tag convention in the entity spec §5.4).
/// `Ok(false)` when the plan already carries it.
async fn add_plan_label(ctx: &AppContext, plan_pid: Uuid, label: &str) -> Result<bool> {
    let model = plans::Entity::find()
        .filter(plans::Column::Pid.eq(plan_pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    let mut plan = model.to_plan().map_err(Error::Model)?;
    if plan
        .tags
        .iter()
        .any(|t| t.trim().eq_ignore_ascii_case(label))
    {
        return Ok(false);
    }
    plan.tags.push(label.to_string());
    let active: plans::ActiveModel = model.into();
    active
        .update_data(&ctx.db, &plan)
        .await
        .map_err(Error::Model)?;
    Ok(true)
}

/// Evaluate every enabled rule against one thing that just happened
/// and apply what matches.
///
/// Deliberately infallible from the caller's point of view: a rule
/// that fails is recorded as a `failed` run, never propagated, so an
/// automation can never undo or block the operator action that
/// triggered it.
pub(crate) async fn fire(
    ctx: &AppContext,
    fact: &rules::TriggerFact,
    subject_kind: &str,
    subject_pid: Uuid,
    actor: Option<&str>,
) {
    let candidates = match cap_models::enabled_automations(&ctx.db, &fact.kind).await {
        Ok(rows) => rows,
        Err(err) => {
            tracing::warn!("automation lookup failed: {err}");
            return;
        }
    };
    for rule in candidates {
        let rule_fact = rules::RuleFact {
            enabled: rule.enabled,
            plan_pid: rule.plan_pid,
            trigger_kind: rule.trigger_kind.clone(),
            from_status: rule.from_status.clone(),
            to_status: rule.to_status.clone(),
        };
        if !rules::rule_matches(&rule_fact, fact) {
            continue;
        }
        let (outcome, detail) =
            apply_action(ctx, &rule, fact, subject_kind, subject_pid, actor).await;
        if let Err(err) = cap_models::record_run(
            &ctx.db,
            rule.pid,
            subject_kind,
            subject_pid,
            outcome,
            detail.clone(),
        )
        .await
        {
            tracing::warn!("automation run log failed: {err}");
        }
        AuditModel::record(
            &ctx.db,
            subject_pid,
            "automation_fired",
            actor,
            Some(serde_json::json!({
                "automation_pid": rule.pid.to_string(),
                "name": rule.name,
                "action_kind": rule.action_kind,
                "outcome": outcome,
                "detail": detail,
            })),
        )
        .await
        .ok();
    }
}

/// `POST /api/scheduled-actions` body.
#[derive(Debug, Deserialize)]
struct SchedulePayload {
    subject_kind: String,
    subject_pid: Uuid,
    action_kind: String,
    /// Deadline in days from now (1..=[`crate::automation::MAX_SCHEDULE_DAYS`]).
    in_days: i64,
    #[serde(default)]
    recipient_ref: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// `POST /api/scheduled-actions` — configure one deadline and forget
/// it.
#[debug_handler]
async fn create_scheduled_action(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SchedulePayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if !rules::is_token(
        crate::collaboration::REVIEW_SUBJECT_KINDS,
        &payload.subject_kind,
    ) && payload.subject_kind != "task"
    {
        problems.push(
            "subject_kind must be one of [\"idea\", \"proposal\", \"plan\", \"task\"]".to_string(),
        );
    }
    if !rules::is_token(rules::SCHEDULED_ACTION_KINDS, &payload.action_kind) {
        problems.push(format!(
            "action_kind must be one of {:?}",
            rules::SCHEDULED_ACTION_KINDS
        ));
    }
    if payload.in_days < 1 || payload.in_days > rules::MAX_SCHEDULE_DAYS {
        problems.push(format!(
            "in_days must be between 1 and {}",
            rules::MAX_SCHEDULE_DAYS
        ));
    }
    if payload.action_kind == "notify"
        && !payload
            .recipient_ref
            .as_deref()
            .is_some_and(rules::person_like_ref)
    {
        problems.push(
            "a `notify` action needs a recipient_ref (person:/worker:/organization: URN)"
                .to_string(),
        );
    }
    if let Some(message) = payload.message.as_deref()
        && message.len() > MAX_TEXT_LEN
    {
        problems.push(format!("message exceeds {MAX_TEXT_LEN} characters"));
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let due_at = chrono::Utc::now() + chrono::Duration::days(payload.in_days);
    let row = cap_models::schedule(
        &ctx.db,
        cap_models::NewScheduledAction {
            subject_kind: payload.subject_kind.clone(),
            subject_pid: payload.subject_pid,
            action_kind: payload.action_kind.clone(),
            payload: serde_json::json!({
                "recipient_ref": payload.recipient_ref,
                "message": payload.message,
            }),
            due_at: due_at.into(),
            source_automation_pid: None,
            created_by: caller.actor().map(std::string::ToString::to_string),
        },
    )
    .await?;
    AuditModel::record(
        &ctx.db,
        row.pid,
        "scheduled_action_created",
        caller.actor(),
        Some(serde_json::json!({
            "action_kind": row.action_kind,
            "due_at": row.due_at.to_rfc3339(),
        })),
    )
    .await
    .ok();
    format::json(row)
}

/// `GET /api/scheduled-actions` filter.
#[derive(Debug, Deserialize)]
struct ScheduledParams {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    subject: Option<Uuid>,
    /// `true` ⇒ only rows already past their deadline.
    #[serde(default)]
    overdue: Option<bool>,
    /// Page size; absent ⇒ [`LIST_CAP`].
    #[serde(default)]
    limit: Option<u64>,
    /// Rows to skip; absent ⇒ 0.
    #[serde(default)]
    offset: Option<u64>,
}

/// `GET /api/scheduled-actions?status=&subject=&overdue=&limit=&offset=`
/// — the deadline queue, soonest first. Paginated
/// (agents/share/restful.md): `X-Total-Count`/`X-Limit`/`X-Offset` on
/// the response; the soonest-first order holds across pages, so page 2
/// picks up exactly where page 1 left off.
///
/// # Errors
///
/// `400` when `offset` exceeds [`super::pagination::MAX_OFFSET`]; a DB
/// error when the query fails.
#[debug_handler]
async fn list_scheduled_actions(
    State(ctx): State<AppContext>,
    Query(params): Query<ScheduledParams>,
) -> Result<Response> {
    let page = Page {
        limit: params.limit,
        offset: params.offset,
    };
    page.check_offset()?;
    let (limit, offset) = page.resolve(LIST_CAP);
    let mut query =
        scheduled_actions::Entity::find().filter(scheduled_actions::Column::DeletedAt.is_null());
    if let Some(status) = params.status.as_deref() {
        query = query.filter(scheduled_actions::Column::Status.eq(status));
    }
    if let Some(subject) = params.subject {
        query = query.filter(scheduled_actions::Column::SubjectPid.eq(subject));
    }
    if params.overdue.unwrap_or(false) {
        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        query = query
            .filter(scheduled_actions::Column::Status.eq("pending"))
            .filter(scheduled_actions::Column::DueAt.lte(now));
    }
    let total = query.clone().count(&ctx.db).await.map_err(db_err)?;
    let rows = query
        .order_by_asc(scheduled_actions::Column::DueAt)
        .offset(offset)
        .limit(limit)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(with_page_headers(format::json(rows)?, total, limit, offset))
}

/// `DELETE /api/scheduled-actions/{pid}` — cancel a pending deadline.
/// An action that already fired cannot be cancelled after the fact.
#[debug_handler]
async fn cancel_scheduled_action(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row =
        cap_models::find_scheduled_action(&ctx.db, crate::models::governance::parse_pid(&pid)?)
            .await?;
    if row.status != "pending" {
        return Err(unprocessable(&format!(
            "only a pending action can be cancelled; this one is `{}`",
            row.status
        )));
    }
    let row_pid = row.pid;
    let mut active: scheduled_actions::ActiveModel = row.into();
    active.status = ActiveValue::set("cancelled".to_string());
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        row_pid,
        "scheduled_action_cancelled",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::empty_json()
}

/// Fire everything due, at most [`SWEEP_CAP`] rows. Returns
/// `(fired, skipped, capped)`.
///
/// Each row is **claimed** with a conditional update
/// (`status = 'pending'` → `'fired'`) and only acted on when the claim
/// won, so two concurrent sweeps — or the ticker racing the endpoint —
/// cannot double-fire one deadline.
///
/// # Errors
///
/// Propagates database errors from the due-row query.
pub async fn sweep_due(ctx: &AppContext) -> Result<(usize, usize, bool)> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let due = cap_models::due_actions(&ctx.db, now, SWEEP_CAP).await?;
    let capped = due.len() as u64 >= SWEEP_CAP;
    let (mut fired, mut skipped) = (0, 0);
    for row in due {
        // Claim it: only the sweep that flips pending → fired acts.
        let claimed = scheduled_actions::Entity::update_many()
            .col_expr(
                scheduled_actions::Column::Status,
                sea_orm::sea_query::Expr::value("fired"),
            )
            .col_expr(
                scheduled_actions::Column::FiredAt,
                sea_orm::sea_query::Expr::value(now),
            )
            .filter(scheduled_actions::Column::Pid.eq(row.pid))
            .filter(scheduled_actions::Column::Status.eq("pending"))
            .exec(&ctx.db)
            .await
            .map_err(db_err)?;
        if claimed.rows_affected != 1 {
            skipped += 1;
            continue;
        }
        let outcome = perform_scheduled(ctx, &row).await;
        scheduled_actions::Entity::update_many()
            .col_expr(
                scheduled_actions::Column::Outcome,
                sea_orm::sea_query::Expr::value(outcome.clone()),
            )
            .filter(scheduled_actions::Column::Pid.eq(row.pid))
            .exec(&ctx.db)
            .await
            .ok();
        AuditModel::record(
            &ctx.db,
            row.pid,
            "scheduled_action_fired",
            None,
            Some(serde_json::json!({
                "action_kind": row.action_kind,
                "outcome": outcome,
            })),
        )
        .await
        .ok();
        fired += 1;
    }
    Ok((fired, skipped, capped))
}

/// Carry out one due action. Returns the recorded outcome.
async fn perform_scheduled(ctx: &AppContext, row: &scheduled_actions::Model) -> String {
    match row.action_kind.as_str() {
        "notify" => {
            let recipient = row
                .payload
                .get("recipient_ref")
                .and_then(serde_json::Value::as_str);
            let message = row
                .payload
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("A scheduled reminder came due");
            match recipient {
                Some(recipient) => match cap_models::notify(
                    &ctx.db,
                    recipient,
                    &row.subject_kind,
                    row.subject_pid,
                    "scheduled",
                    message,
                )
                .await
                {
                    Ok(_) => "notified".to_string(),
                    Err(e) => format!("failed: {e}"),
                },
                None => "failed: no recipient_ref".to_string(),
            }
        }
        "expire_review" => {
            // Expire only invitations still awaiting a verdict; a
            // submitted or declined review is left exactly as it is.
            let expired = reviews::Entity::update_many()
                .col_expr(
                    reviews::Column::Status,
                    sea_orm::sea_query::Expr::value("expired"),
                )
                .filter(reviews::Column::SubjectPid.eq(row.subject_pid))
                .filter(
                    reviews::Column::Status
                        .is_in(crate::collaboration::LIVE_REVIEW_STATUSES.iter().copied()),
                )
                .filter(reviews::Column::DeletedAt.is_null())
                .exec(&ctx.db)
                .await;
            match expired {
                Ok(result) => format!("expired {} review(s)", result.rows_affected),
                Err(e) => format!("failed: {e}"),
            }
        }
        other => format!("failed: unknown action_kind `{other}`"),
    }
}

/// `POST /api/scheduled-actions/sweep` — fire every action now due.
/// Safe to call repeatedly and from more than one caller: claiming is
/// atomic, so an action fires exactly once.
#[debug_handler]
async fn sweep(State(ctx): State<AppContext>) -> Result<Response> {
    let (fired, skipped, capped) = sweep_due(&ctx).await?;
    format::json(serde_json::json!({
        "fired": fired,
        "skipped_already_claimed": skipped,
        "capped": capped,
        "cap": SWEEP_CAP,
        "as_of": chrono::Utc::now().to_rfc3339(),
    }))
}

/// The automation routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/automations", post(create_automation))
        .add("/automations", get(list_automations))
        .add("/automations/runs", get(list_runs))
        .add("/automations/{pid}/enable", post(enable_automation))
        .add("/automations/{pid}/disable", post(disable_automation))
        .add("/automations/{pid}", delete(delete_automation))
        .add("/scheduled-actions", post(create_scheduled_action))
        .add("/scheduled-actions", get(list_scheduled_actions))
        .add("/scheduled-actions/sweep", post(sweep))
        .add("/scheduled-actions/{pid}", delete(cancel_scheduled_action))
}
