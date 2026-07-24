//! Workforce management (HCM-R4–R6): time & attendance, leave, and
//! shift scheduling. Approval races serialize on the request row
//! (`FOR UPDATE`, HCM-D9); balances change in the decision's
//! transaction.

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{
    leave_entitlements, leave_requests, shift_assignments, shifts, time_entries,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{leave, lifecycle, tokens, workforce};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/employees/{pid}/time-entries` body.
#[derive(Debug, Deserialize)]
struct TimeEntryPayload {
    worked_on: chrono::NaiveDate,
    minutes: i32,
    #[serde(default = "default_kind")]
    kind: String,
    #[serde(default)]
    notes: Option<String>,
}

/// `POST /api/employees/{pid}/leave-entitlements` body.
#[derive(Debug, Deserialize)]
struct EntitlementPayload {
    kind: String,
    year: i32,
    entitled_days: i32,
}

/// `POST /api/employees/{pid}/leave-requests` body.
#[derive(Debug, Deserialize)]
struct LeaveRequestPayload {
    kind: String,
    start_on: chrono::NaiveDate,
    end_on: chrono::NaiveDate,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/shifts` body.
#[derive(Debug, Deserialize)]
struct ShiftPayload {
    department: String,
    starts_at: chrono::DateTime<chrono::FixedOffset>,
    ends_at: chrono::DateTime<chrono::FixedOffset>,
    #[serde(default = "default_headcount")]
    required_headcount: i32,
}

/// `POST /api/shifts/{pid}/assignments` body.
#[derive(Debug, Deserialize)]
struct AssignmentPayload {
    employee_pid: Uuid,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

fn default_kind() -> String {
    "regular".to_string()
}
const fn default_headcount() -> i32 {
    1
}

/// `POST /api/employees/{pid}/time-entries` — record time; the day
/// total is capped at 24 h (HCM-R4).
#[debug_handler]
async fn create_time_entry(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TimeEntryPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_token("kind", tokens::TIME_KINDS, &payload.kind);
    problems.cap_opt("notes", payload.notes.as_deref());
    ensure_valid(&problems.into_vec())?;
    let existing: Vec<time_entries::Model> = time_entries::Entity::find()
        .filter(time_entries::Column::EmployeePid.eq(employee.pid))
        .filter(time_entries::Column::WorkedOn.eq(payload.worked_on))
        .filter(time_entries::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let existing_minutes: i32 = existing.iter().map(|e| e.minutes).sum();
    workforce::check_day_minutes(existing_minutes, payload.minutes)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let row = time_entries::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        worked_on: ActiveValue::set(payload.worked_on),
        minutes: ActiveValue::set(payload.minutes),
        kind: ActiveValue::set(payload.kind.clone()),
        status: ActiveValue::set("recorded".to_string()),
        notes: ActiveValue::set(payload.notes.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "time_entry", row.pid, "time_recorded", caller.actor(), None).await?;
    streaming::emit_on(&txn, "time_entry", "time_recorded", &row.pid.to_string(), &employee.employee_number, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/employees/{pid}/time-entries?from=&to=` — entries plus
/// the derived per-day overtime (HCM-R4).
#[derive(Debug, Deserialize)]
struct TimeListParams {
    #[serde(default)]
    from: Option<chrono::NaiveDate>,
    #[serde(default)]
    to: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn list_time_entries(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<TimeListParams>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut query = time_entries::Entity::find()
        .filter(time_entries::Column::EmployeePid.eq(employee.pid))
        .filter(time_entries::Column::DeletedAt.is_null());
    if let Some(from) = params.from {
        query = query.filter(time_entries::Column::WorkedOn.gte(from));
    }
    if let Some(to) = params.to {
        query = query.filter(time_entries::Column::WorkedOn.lte(to));
    }
    let rows = query
        .order_by_asc(time_entries::Column::WorkedOn)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    // Per-day overtime derivation over the returned window.
    let mut days: std::collections::BTreeMap<chrono::NaiveDate, (i32, i32)> =
        std::collections::BTreeMap::new();
    for entry in &rows {
        let slot = days.entry(entry.worked_on).or_default();
        if entry.kind == "overtime" {
            slot.1 += entry.minutes;
        } else {
            slot.0 += entry.minutes;
        }
    }
    let overtime: Vec<serde_json::Value> = days
        .iter()
        .map(|(day, (regular, explicit))| {
            serde_json::json!({
                "worked_on": day,
                "overtime_minutes": workforce::overtime_minutes(*regular, *explicit, employee.fte_percent),
            })
        })
        .collect();
    format::json(serde_json::json!({ "entries": rows, "overtime": overtime }))
}

/// `POST /api/time-entries/{pid}/approve` — manager approval; only
/// approved time feeds payroll (HCM-R4).
#[debug_handler]
async fn approve_time_entry(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let entry = records::find_time_entry(&ctx.db, records::parse_pid(&pid)?).await?;
    if entry.status == "approved" {
        return Err(unprocessable("time entry is already approved"));
    }
    let txn = ctx.db.begin().await?;
    let mut active: time_entries::ActiveModel = entry.into();
    active.status = ActiveValue::set("approved".to_string());
    let row = active.update(&txn).await?;
    Audit::record(&txn, "time_entry", row.pid, "time_approved", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/employees/{pid}/leave-entitlements` — grant.
#[debug_handler]
async fn create_entitlement(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<EntitlementPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_token("kind", tokens::LEAVE_KINDS, &payload.kind);
    if !(2000..=2100).contains(&payload.year) {
        problems.push(format!("year {} out of range", payload.year));
    }
    if payload.entitled_days < 0 || payload.entitled_days > 366 {
        problems.push(format!("entitled_days {} out of range 0-366", payload.entitled_days));
    }
    ensure_valid(&problems.into_vec())?;
    let existing = leave_entitlements::Entity::find()
        .filter(leave_entitlements::Column::EmployeePid.eq(employee.pid))
        .filter(leave_entitlements::Column::Kind.eq(&payload.kind))
        .filter(leave_entitlements::Column::Year.eq(payload.year))
        .filter(leave_entitlements::Column::DeletedAt.is_null())
        .count(&ctx.db)
        .await?;
    if existing > 0 {
        return Err(unprocessable("entitlement already exists for this kind and year"));
    }
    let txn = ctx.db.begin().await?;
    let row = leave_entitlements::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        year: ActiveValue::set(payload.year),
        entitled_days: ActiveValue::set(payload.entitled_days),
        used_days: ActiveValue::set(0),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "leave_entitlement", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/employees/{pid}/leave-entitlements` — balances.
#[debug_handler]
async fn list_entitlements(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = leave_entitlements::Entity::find()
        .filter(leave_entitlements::Column::EmployeePid.eq(employee.pid))
        .filter(leave_entitlements::Column::DeletedAt.is_null())
        .order_by_asc(leave_entitlements::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/employees/{pid}/leave-requests` — request leave. The
/// balance is **checked** here (annual over-balance ⇒ 422; sick may
/// flag negative) and **decremented on approval** (HCM-R5).
#[debug_handler]
async fn create_leave_request(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<LeaveRequestPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_token("kind", tokens::LEAVE_KINDS, &payload.kind);
    problems.cap_opt("reason", payload.reason.as_deref());
    ensure_valid(&problems.into_vec())?;
    let days = leave::day_span(payload.start_on, payload.end_on).map_err(|e| unprocessable(&e))?;
    let check = balance_check(&ctx.db, &employee, &payload.kind, payload.start_on, days).await?;
    let negative = match check {
        leave::BalanceCheck::Ok { .. } => false,
        leave::BalanceCheck::NegativeFlagged { .. } => true,
        leave::BalanceCheck::OverBalance { remaining, requested } => {
            return Err(unprocessable(&format!(
                "requested {requested} days exceeds remaining balance {remaining}"
            )));
        }
    };
    let txn = ctx.db.begin().await?;
    let row = leave_requests::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        start_on: ActiveValue::set(payload.start_on),
        end_on: ActiveValue::set(payload.end_on),
        days: ActiveValue::set(days),
        status: ActiveValue::set("requested".to_string()),
        negative_balance: ActiveValue::set(negative),
        reason: ActiveValue::set(payload.reason.clone()),
        decided_by: ActiveValue::set(None),
        decided_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "leave_request", row.pid, "leave_requested", caller.actor(), None).await?;
    streaming::emit_on(&txn, "leave_request", "leave_requested", &row.pid.to_string(), &employee.employee_number, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": row.pid, "days": days, "negative_balance": negative }))
}

/// The balance verdict for `kind` in the request's starting year.
async fn balance_check(
    db: &DatabaseConnection,
    employee: &crate::models::_entities::employees::Model,
    kind: &str,
    start_on: chrono::NaiveDate,
    days: i32,
) -> Result<leave::BalanceCheck> {
    use chrono::Datelike;
    let year = start_on.year();
    let entitlement = leave_entitlements::Entity::find()
        .filter(leave_entitlements::Column::EmployeePid.eq(employee.pid))
        .filter(leave_entitlements::Column::Kind.eq(kind))
        .filter(leave_entitlements::Column::Year.eq(year))
        .filter(leave_entitlements::Column::DeletedAt.is_null())
        .one(db)
        .await?;
    let (entitled, used) = entitlement.map_or((0, 0), |e| (e.entitled_days, e.used_days));
    Ok(leave::check_balance(kind, entitled, used, days))
}

/// `GET /api/employees/{pid}/leave-requests`.
#[debug_handler]
async fn list_leave_requests(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = leave_requests::Entity::find()
        .filter(leave_requests::Column::EmployeePid.eq(employee.pid))
        .filter(leave_requests::Column::DeletedAt.is_null())
        .order_by_asc(leave_requests::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// One leave decision (`approved` / `rejected` / `cancelled`),
/// serialized on the locked request row; approval decrements the
/// balance in the same transaction (HCM-R5, HCM-D9).
async fn decide_leave(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: &str,
    to: &str,
) -> Result<Response> {
    use chrono::Datelike;
    let pid = records::parse_pid(pid)?;
    let txn = ctx.db.begin().await?;
    // Lock the request row: two racing approvers serialize here; the
    // loser sees the already-decided status and gets the 422.
    let request = leave_requests::Entity::find()
        .filter(leave_requests::Column::Pid.eq(pid))
        .filter(leave_requests::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    lifecycle::check("leave request", lifecycle::LEAVE, &request.status, to)
        .map_err(|e| unprocessable(&e))?;
    let was = request.status.clone();
    let kind = request.kind.clone();
    let year = request.start_on.year();
    let days = request.days;
    let employee_pid = request.employee_pid;
    let mut active: leave_requests::ActiveModel = request.into();
    active.status = ActiveValue::set(to.to_string());
    active.decided_by = ActiveValue::set(caller.actor().map(ToString::to_string));
    active.decided_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&txn).await?;
    // Balance mutation: approve uses days; cancelling an approved
    // request restores them.
    let delta = if to == "approved" {
        days
    } else if to == "cancelled" && was == "approved" {
        -days
    } else {
        0
    };
    if delta != 0 {
        let entitlement = leave_entitlements::Entity::find()
            .filter(leave_entitlements::Column::EmployeePid.eq(employee_pid))
            .filter(leave_entitlements::Column::Kind.eq(&kind))
            .filter(leave_entitlements::Column::Year.eq(year))
            .filter(leave_entitlements::Column::DeletedAt.is_null())
            .lock_exclusive()
            .one(&txn)
            .await?;
        if let Some(entitlement) = entitlement {
            let mut active: leave_entitlements::ActiveModel = entitlement.clone().into();
            active.used_days = ActiveValue::set(entitlement.used_days + delta);
            active.update(&txn).await?;
        }
    }
    let event = format!("leave_{to}");
    Audit::record(
        &txn,
        "leave_request",
        row.pid,
        &event,
        caller.actor(),
        Some(serde_json::json!({ "from": was, "days": days })),
    )
    .await?;
    streaming::emit_on(&txn, "leave_request", &event, &row.pid.to_string(), "", caller.actor(), None).await?;
    txn.commit().await?;
    if to == "approved" || to == "rejected" {
        Metrics::global().leave_decided_total.inc();
    }
    format::json(row)
}

/// `POST /api/leave-requests/{pid}/approve`.
#[debug_handler]
async fn approve_leave(State(ctx): State<AppContext>, caller: MaybeAuthUser, Path(pid): Path<String>) -> Result<Response> {
    decide_leave(&ctx, &caller, &pid, "approved").await
}

/// `POST /api/leave-requests/{pid}/reject`.
#[debug_handler]
async fn reject_leave(State(ctx): State<AppContext>, caller: MaybeAuthUser, Path(pid): Path<String>) -> Result<Response> {
    decide_leave(&ctx, &caller, &pid, "rejected").await
}

/// `POST /api/leave-requests/{pid}/cancel`.
#[debug_handler]
async fn cancel_leave(State(ctx): State<AppContext>, caller: MaybeAuthUser, Path(pid): Path<String>) -> Result<Response> {
    decide_leave(&ctx, &caller, &pid, "cancelled").await
}

/// `POST /api/shifts` — plan a shift.
#[debug_handler]
async fn create_shift(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ShiftPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("department", &payload.department);
    if payload.ends_at <= payload.starts_at {
        problems.push("ends_at must be after starts_at".to_string());
    }
    if payload.required_headcount < 1 {
        problems.push("required_headcount must be at least 1".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = shifts::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        department: ActiveValue::set(payload.department.clone()),
        starts_at: ActiveValue::set(payload.starts_at),
        ends_at: ActiveValue::set(payload.ends_at),
        required_headcount: ActiveValue::set(payload.required_headcount),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "shift", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/shifts?department=&date=` — the day rota (HCM-R6).
#[derive(Debug, Deserialize)]
struct RotaParams {
    #[serde(default)]
    department: Option<String>,
    #[serde(default)]
    date: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn list_shifts(
    State(ctx): State<AppContext>,
    Query(params): Query<RotaParams>,
) -> Result<Response> {
    let mut query = shifts::Entity::find().filter(shifts::Column::DeletedAt.is_null());
    if let Some(department) = &params.department {
        query = query.filter(shifts::Column::Department.eq(department));
    }
    let rows = query
        .order_by_asc(shifts::Column::StartsAt)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    let rows: Vec<_> = if let Some(date) = params.date {
        rows.into_iter().filter(|s| s.starts_at.date_naive() == date).collect()
    } else {
        rows
    };
    // Attach assignments per shift.
    let mut out = Vec::with_capacity(rows.len());
    for shift in rows {
        let assignments = shift_assignments::Entity::find()
            .filter(shift_assignments::Column::ShiftPid.eq(shift.pid))
            .filter(shift_assignments::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?;
        out.push(serde_json::json!({ "shift": shift, "assignments": assignments }));
    }
    format::json(out)
}

/// `POST /api/shifts/{pid}/assignments` — assign an employee. Refuses
/// a double booking (overlapping assigned shift) and an assignment
/// overlapping approved leave (HCM-R6).
#[debug_handler]
async fn assign_shift(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<AssignmentPayload>,
) -> Result<Response> {
    let shift = records::find_shift(&ctx.db, records::parse_pid(&pid)?).await?;
    let employee = records::find_employee(&ctx.db, payload.employee_pid).await?;
    // Double-booking: any live assignment to an overlapping shift.
    let their_assignments = shift_assignments::Entity::find()
        .filter(shift_assignments::Column::EmployeePid.eq(employee.pid))
        .filter(shift_assignments::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    for assignment in &their_assignments {
        if let Some(other) = shifts::Entity::find()
            .filter(shifts::Column::Pid.eq(assignment.shift_pid))
            .filter(shifts::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await?
            && workforce::windows_overlap(shift.starts_at, shift.ends_at, other.starts_at, other.ends_at) {
                return Err(unprocessable(&format!(
                    "double booking: employee already assigned to an overlapping shift ({})",
                    other.pid
                )));
            }
    }
    // Leave conflict: any approved leave overlapping the shift's dates.
    let approved_leave = leave_requests::Entity::find()
        .filter(leave_requests::Column::EmployeePid.eq(employee.pid))
        .filter(leave_requests::Column::Status.eq("approved"))
        .filter(leave_requests::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    let shift_start = shift.starts_at.date_naive();
    let shift_end = shift.ends_at.date_naive();
    for request in &approved_leave {
        if leave::ranges_overlap(shift_start, shift_end, request.start_on, request.end_on) {
            return Err(unprocessable(&format!(
                "employee is on approved {} leave {} to {}",
                request.kind, request.start_on, request.end_on
            )));
        }
    }
    let txn = ctx.db.begin().await?;
    let row = shift_assignments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        shift_pid: ActiveValue::set(shift.pid),
        employee_pid: ActiveValue::set(employee.pid),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "shift_assignment", row.pid, "shift_assigned", caller.actor(), None).await?;
    streaming::emit_on(&txn, "shift_assignment", "shift_assigned", &row.pid.to_string(), &employee.employee_number, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `DELETE /api/shift-assignments/{pid}` — unassign (soft delete).
#[debug_handler]
async fn unassign_shift(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = records::find_shift_assignment(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let pid = row.pid;
    let mut active: shift_assignments::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(&txn, "shift_assignment", pid, "deleted", caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// The workforce routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/employees/{pid}/time-entries", post(create_time_entry))
        .add("/employees/{pid}/time-entries", get(list_time_entries))
        .add("/time-entries/{pid}/approve", post(approve_time_entry))
        .add("/employees/{pid}/leave-entitlements", post(create_entitlement))
        .add("/employees/{pid}/leave-entitlements", get(list_entitlements))
        .add("/employees/{pid}/leave-requests", post(create_leave_request))
        .add("/employees/{pid}/leave-requests", get(list_leave_requests))
        .add("/leave-requests/{pid}/approve", post(approve_leave))
        .add("/leave-requests/{pid}/reject", post(reject_leave))
        .add("/leave-requests/{pid}/cancel", post(cancel_leave))
        .add("/shifts", post(create_shift))
        .add("/shifts", get(list_shifts))
        .add("/shifts/{pid}/assignments", post(assign_shift))
        .add("/shift-assignments/{pid}", delete(unassign_shift))
}
