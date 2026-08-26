//! **Recorded effort** and **utilisation** — the surface over
//! [`crate::effort`] (entity spec §9.2c / FR-28, FR-35).
//!
//! # What this deliberately does not serve
//!
//! Per-person **utilisation** is here, by the owner decision of
//! 2026-08-25. Per-person **cycle time, throughput and flow
//! efficiency** are not, and there is no endpoint from which they could
//! be derived by arithmetic — that refusal
//! (`agents/share/time-based-analysis.md` §7.1) was narrowed, not
//! repealed.
//!
//! # Why the roll-ups are not sorted by utilisation
//!
//! Obligation 4: it is never the sole ranking key. The per-person view
//! returns in a stable `actor_ref` order and ships each figure beside
//! its numerator, its denominator, and the deduction that produced the
//! denominator — so a reader has to look at the working, not just the
//! ratio.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::effort as rules;
use crate::models::_entities::{non_working_periods, plans, time_entries, working_time_configs};
use crate::models::audit_logs::Model as AuditModel;

/// Rows scanned per read.
const MAX_ROWS: u64 = 5000;

/// Default window for a utilisation read.
const DEFAULT_WINDOW_DAYS: i64 = 28;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
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

/// `POST /api/plans/{pid}/time-entries` body.
#[derive(Debug, Deserialize)]
struct EntryPayload {
    actor_ref: String,
    spent_on: chrono::NaiveDate,
    minutes: i64,
    #[serde(default)]
    task_pid: Option<Uuid>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    billable: bool,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/plans/{pid}/time-entries` — record effort.
#[debug_handler]
async fn create_entry(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<EntryPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    if !payload.actor_ref.starts_with("person:") && !payload.actor_ref.starts_with("worker:") {
        return Err(unprocessable("actor_ref must be a person: or worker: URN"));
    }
    // A day has 1440 minutes. More than that against one date is a
    // typo or a fabrication, and either way it is not effort.
    if payload.minutes <= 0 || payload.minutes > 1_440 {
        return Err(unprocessable(
            "minutes must be between 1 and 1440 — a single day cannot hold more",
        ));
    }

    let row = time_entries::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        task_pid: ActiveValue::set(payload.task_pid),
        actor_ref: ActiveValue::set(payload.actor_ref.clone()),
        spent_on: ActiveValue::set(payload.spent_on),
        minutes: ActiveValue::set(payload.minutes),
        category: ActiveValue::set(
            rules::EffortCategory::parse(payload.category.as_deref())
                .token()
                .to_string(),
        ),
        billable: ActiveValue::set(payload.billable),
        note: ActiveValue::set(payload.note.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(&ctx.db, plan.pid, "effort_recorded", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/plans/{pid}/time-entries` — newest first.
#[debug_handler]
async fn list_entries(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = time_entries::Entity::find()
        .filter(time_entries::Column::PlanPid.eq(plan.pid))
        .filter(time_entries::Column::DeletedAt.is_null())
        .order_by_desc(time_entries::Column::SpentOn)
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!(rows))
}

fn facts_of(rows: &[time_entries::Model]) -> Vec<rules::EffortFact> {
    rows.iter()
        .map(|row| rules::EffortFact {
            actor_ref: row.actor_ref.clone(),
            minutes: row.minutes,
            category: rules::EffortCategory::parse(Some(&row.category)),
            billable: row.billable,
        })
        .collect()
}

/// `GET /api/plans/{pid}/effort` — roll-ups per plan, per task and per
/// assignee, every one labelled **asserted**.
#[debug_handler]
async fn effort_rollup(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = time_entries::Entity::find()
        .filter(time_entries::Column::PlanPid.eq(plan.pid))
        .filter(time_entries::Column::DeletedAt.is_null())
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let mut by_actor: std::collections::BTreeMap<String, Vec<time_entries::Model>> =
        std::collections::BTreeMap::new();
    let mut by_task: std::collections::BTreeMap<String, Vec<time_entries::Model>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        by_actor
            .entry(row.actor_ref.clone())
            .or_default()
            .push(row.clone());
        if let Some(task_pid) = row.task_pid {
            by_task
                .entry(task_pid.to_string())
                .or_default()
                .push(row.clone());
        }
    }

    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "plan": rules::totals(&facts_of(&rows)),
        // Stable key order, and deliberately not sorted by size:
        // obligation 4, never the sole ranking key.
        "by_assignee": by_actor.iter().map(|(actor, entries)| serde_json::json!({
            "actor_ref": actor,
            "totals": rules::totals(&facts_of(entries)),
        })).collect::<Vec<_>>(),
        "by_task": by_task.iter().map(|(task, entries)| serde_json::json!({
            "task_pid": task,
            "totals": rules::totals(&facts_of(entries)),
        })).collect::<Vec<_>>(),
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// `POST /api/working-time` body.
#[derive(Debug, Deserialize)]
struct WorkingTimePayload {
    #[serde(default)]
    scope_ref: Option<String>,
    minutes_per_day: i32,
    working_days_per_week: i32,
}

/// `POST /api/working-time` — declare the capacity basis.
#[debug_handler]
async fn set_working_time(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<WorkingTimePayload>,
) -> Result<Response> {
    if payload.minutes_per_day <= 0 || payload.minutes_per_day > 1_440 {
        return Err(unprocessable("minutes_per_day must be between 1 and 1440"));
    }
    if !(1..=7).contains(&payload.working_days_per_week) {
        return Err(unprocessable(
            "working_days_per_week must be between 1 and 7",
        ));
    }
    let row = working_time_configs::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        scope_ref: ActiveValue::set(payload.scope_ref.clone()),
        minutes_per_day: ActiveValue::set(payload.minutes_per_day),
        working_days_per_week: ActiveValue::set(payload.working_days_per_week),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        row.pid,
        "working_time_declared",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `POST /api/non-working` body.
#[derive(Debug, Deserialize)]
struct NonWorkingPayload {
    person_ref: String,
    starts_on: chrono::NaiveDate,
    ends_on: chrono::NaiveDate,
    kind: String,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/non-working` — record leave or other non-working time.
///
/// This is what stops somebody on leave reporting 0% utilisation, which
/// would read as measured idleness.
#[debug_handler]
async fn create_non_working(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<NonWorkingPayload>,
) -> Result<Response> {
    if !["leave", "holiday", "study_leave", "non_project_duty"].contains(&payload.kind.as_str()) {
        return Err(unprocessable(
            "kind must be leave, holiday, study_leave or non_project_duty",
        ));
    }
    if payload.ends_on < payload.starts_on {
        return Err(unprocessable("ends_on must not precede starts_on"));
    }
    let row = non_working_periods::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        starts_on: ActiveValue::set(payload.starts_on),
        ends_on: ActiveValue::set(payload.ends_on),
        kind: ActiveValue::set(payload.kind.clone()),
        note: ActiveValue::set(payload.note.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        row.pid,
        "non_working_recorded",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/capacity/utilization` query.
#[derive(Debug, Deserialize)]
struct UtilizationParams {
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    window_days: Option<i64>,
    #[serde(default)]
    plan_pid: Option<Uuid>,
}

/// Minutes of declared capacity in a window, from the config in force.
///
/// Integer arithmetic throughout, matching the rest of this crate: a
/// capacity denominator is the sort of figure people reconcile against
/// a payroll system, and a float would introduce a discrepancy nobody
/// could explain. `window_days × working_days ÷ 7 × minutes_per_day`,
/// with the division last so the truncation is a whole minute rather
/// than a fraction of a working week.
fn declared_minutes(config: Option<&working_time_configs::Model>, window_days: i64) -> i64 {
    let Some(config) = config else { return 0 };
    window_days
        .checked_mul(i64::from(config.working_days_per_week))
        .and_then(|d| d.checked_mul(i64::from(config.minutes_per_day)))
        .map_or(0, |total| total / 7)
}

/// `GET /api/capacity/utilization?by=plan|team|person` — effort against
/// declared capacity.
///
/// **No per-person cycle time, throughput or flow efficiency is served
/// from here or derivable from what it returns.**
#[debug_handler]
async fn utilization(
    State(ctx): State<AppContext>,
    Query(params): Query<UtilizationParams>,
    headers: HeaderMap,
) -> Result<Response> {
    let by = params.by.unwrap_or_else(|| "team".to_string());
    if !["plan", "team", "person"].contains(&by.as_str()) {
        return Err(unprocessable("by must be plan, team or person"));
    }
    let window_days = params
        .window_days
        .filter(|d| *d > 0 && *d <= 366)
        .unwrap_or(DEFAULT_WINDOW_DAYS);
    let since = chrono::Utc::now().date_naive() - chrono::Duration::days(window_days);

    let mut query = time_entries::Entity::find()
        .filter(time_entries::Column::DeletedAt.is_null())
        .filter(time_entries::Column::SpentOn.gte(since));
    if let Some(plan_pid) = params.plan_pid {
        query = query.filter(time_entries::Column::PlanPid.eq(plan_pid));
    }
    let entries = query.limit(MAX_ROWS).all(&ctx.db).await.map_err(db_err)?;

    let config = working_time_configs::Entity::find()
        .filter(working_time_configs::Column::ScopeRef.is_null())
        .filter(working_time_configs::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    let declared = declared_minutes(config.as_ref(), window_days);

    let absences = non_working_periods::Entity::find()
        .filter(non_working_periods::Column::DeletedAt.is_null())
        .filter(non_working_periods::Column::EndsOn.gte(since))
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let mut per_person: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for entry in &entries {
        *per_person.entry(entry.actor_ref.clone()).or_default() += entry.minutes;
    }
    // Somebody who was entirely on leave has no effort rows, so they
    // would be invisible. Include them, precisely so the answer is
    // "on leave" rather than a silent absence.
    for absence in &absences {
        per_person.entry(absence.person_ref.clone()).or_insert(0);
    }

    let minutes_per_day = config.as_ref().map_or(0, |c| i64::from(c.minutes_per_day));
    let facts: Vec<rules::CapacityFact> = per_person
        .iter()
        .map(|(actor, minutes)| {
            let non_working: i64 = absences
                .iter()
                .filter(|a| &a.person_ref == actor)
                .map(|a| {
                    let days = (a.ends_on - a.starts_on).num_days() + 1;
                    days.max(0).saturating_mul(minutes_per_day)
                })
                .sum();
            rules::CapacityFact {
                actor_ref: actor.clone(),
                declared_minutes: declared,
                non_working_minutes: non_working.min(declared),
                effort_minutes: *minutes,
            }
        })
        .collect();

    let floor = rules::DEFAULT_SUPPRESSION_FLOOR_MINUTES;
    let body = match by.as_str() {
        "person" => serde_json::json!({
            "by": "person",
            "window_days": window_days,
            // Stable order by actor, **not** sorted by utilisation.
            "utilisation": facts.iter().map(|f| rules::utilisation(f, floor)).collect::<Vec<_>>(),
            "note": "Utilisation near or above 100% is a warning about the queue, \
                     not an achievement. No per-person cycle time, throughput or \
                     flow efficiency is served here or derivable from it.",
        }),
        _ => serde_json::json!({
            "by": by,
            "window_days": window_days,
            "utilisation": rules::team_utilisation(&facts, floor),
            "people_counted": facts.len(),
        }),
    };
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The effort and utilisation routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/time-entries", post(create_entry))
        .add("/plans/{pid}/time-entries", get(list_entries))
        .add("/plans/{pid}/effort", get(effort_rollup))
        .add("/working-time", post(set_working_time))
        .add("/non-working", post(create_non_working))
        .add("/capacity/utilization", get(utilization))
}
