//! Talent acquisition (HCM-R1–R3): requisitions, candidates,
//! applications + interviews, and onboarding checklists. Hiring an
//! application creates the Employee in one transaction (HCM-D9).

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{applications, candidates, employees, interviews, onboarding_items, requisitions};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{lifecycle, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/requisitions` body.
#[derive(Debug, Deserialize)]
struct RequisitionPayload {
    organization_ref: String,
    department: String,
    job_title: String,
    #[serde(default = "default_headcount")]
    headcount: i32,
    #[serde(default)]
    salary_min_minor: Option<i64>,
    #[serde(default)]
    salary_max_minor: Option<i64>,
    #[serde(default)]
    salary_currency: Option<String>,
}

/// `POST /api/candidates` body.
#[derive(Debug, Deserialize)]
struct CandidatePayload {
    display_name: String,
    email: String,
    source: String,
    #[serde(default)]
    person_ref: Option<String>,
    #[serde(default)]
    consent_until: Option<chrono::NaiveDate>,
}

/// `POST /api/requisitions/{pid}/applications` body.
#[derive(Debug, Deserialize)]
struct ApplicationPayload {
    candidate_pid: Uuid,
    #[serde(default)]
    notes: Option<String>,
}

/// `POST /api/applications/{pid}/stage` body. Moving to `hired`
/// requires the employee fields.
#[derive(Debug, Deserialize)]
struct StagePayload {
    to: String,
    /// Required on `hired`: the person URN for the new employee.
    #[serde(default)]
    person_ref: Option<String>,
    /// Required on `hired`: the employee number.
    #[serde(default)]
    employee_number: Option<String>,
    #[serde(default)]
    employment_type: Option<String>,
    #[serde(default)]
    fte_percent: Option<i32>,
    #[serde(default)]
    salary_minor: Option<i64>,
    #[serde(default)]
    salary_currency: Option<String>,
    #[serde(default)]
    hired_on: Option<chrono::NaiveDate>,
}

/// `POST /api/applications/{pid}/interviews` body.
#[derive(Debug, Deserialize)]
struct InterviewPayload {
    scheduled_at: chrono::DateTime<chrono::FixedOffset>,
    interviewer_ref: String,
    #[serde(default)]
    notes: Option<String>,
}

/// `PUT /api/interviews/{pid}` body (outcome).
#[derive(Debug, Deserialize)]
struct OutcomePayload {
    outcome: String,
    #[serde(default)]
    notes: Option<String>,
}

/// `POST /api/employees/{pid}/onboarding` body — add checklist items.
#[derive(Debug, Deserialize)]
struct OnboardingPayload {
    items: Vec<OnboardingItemPayload>,
}

/// One checklist item.
#[derive(Debug, Deserialize)]
struct OnboardingItemPayload {
    name: String,
    #[serde(default = "default_true")]
    mandatory: bool,
}

/// `POST /api/onboarding-items/{pid}/waive` body.
#[derive(Debug, Deserialize)]
struct WaivePayload {
    reason: String,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

const fn default_headcount() -> i32 {
    1
}
const fn default_true() -> bool {
    true
}

/// `POST /api/requisitions` — create in `draft`.
#[debug_handler]
async fn create_requisition(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<RequisitionPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_ref(
        "organization_ref",
        entity_ref::EntityType::Organization,
        &payload.organization_ref,
    );
    problems.require_text("department", &payload.department);
    problems.require_text("job_title", &payload.job_title);
    if payload.headcount < 1 {
        problems.push("headcount must be at least 1".to_string());
    }
    if let (Some(min), Some(max)) = (payload.salary_min_minor, payload.salary_max_minor)
        && min > max {
            problems.push("salary_min_minor exceeds salary_max_minor".to_string());
        }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = requisitions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        organization_ref: ActiveValue::set(payload.organization_ref.clone()),
        department: ActiveValue::set(payload.department.clone()),
        job_title: ActiveValue::set(payload.job_title.clone()),
        headcount: ActiveValue::set(payload.headcount),
        salary_min_minor: ActiveValue::set(payload.salary_min_minor),
        salary_max_minor: ActiveValue::set(payload.salary_max_minor),
        salary_currency: ActiveValue::set(payload.salary_currency.clone()),
        status: ActiveValue::set("draft".to_string()),
        opened_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "requisition", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "requisition", "created", &row.pid.to_string(), &row.job_title, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/requisitions` — active, filterable by `?status=`.
#[derive(Debug, Deserialize)]
struct RequisitionListParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_requisitions(
    State(ctx): State<AppContext>,
    Query(params): Query<RequisitionListParams>,
) -> Result<Response> {
    let mut query = requisitions::Entity::find().filter(requisitions::Column::DeletedAt.is_null());
    if let Some(status) = &params.status {
        query = query.filter(requisitions::Column::Status.eq(status));
    }
    let rows = query
        .order_by_asc(requisitions::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/requisitions/{pid}`.
#[debug_handler]
async fn get_requisition(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    format::json(records::find_requisition(&ctx.db, records::parse_pid(&pid)?).await?)
}

/// `POST /api/requisitions/{pid}/status` — one pipeline transition.
/// `filled` requires hired applications ≥ headcount (HCM-R1).
#[derive(Debug, Deserialize)]
struct RequisitionStatusPayload {
    to: String,
}

#[debug_handler]
async fn requisition_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<RequisitionStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::REQUISITION_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let requisition = records::find_requisition(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("requisition", lifecycle::REQUISITION, &requisition.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    if payload.to == "filled" {
        let hired = applications::Entity::find()
            .filter(applications::Column::RequisitionPid.eq(requisition.pid))
            .filter(applications::Column::DeletedAt.is_null())
            .filter(applications::Column::Stage.eq("hired"))
            .count(&ctx.db)
            .await?;
        if hired < u64::try_from(requisition.headcount).unwrap_or(u64::MAX) {
            return Err(unprocessable(&format!(
                "cannot fill: {hired} hired of headcount {}",
                requisition.headcount
            )));
        }
    }
    let txn = ctx.db.begin().await?;
    let from = requisition.status.clone();
    let title = requisition.job_title.clone();
    let mut active: requisitions::ActiveModel = requisition.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "open" {
        active.opened_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    }
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "open" => "requisition_opened",
        "filled" => "requisition_filled",
        _ => "requisition_status_changed",
    };
    Audit::record(
        &txn,
        "requisition",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(&txn, "requisition", kind, &row.pid.to_string(), &title, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/candidates`.
#[debug_handler]
async fn create_candidate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CandidatePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("display_name", &payload.display_name);
    problems.require_text("email", &payload.email);
    if !payload.email.contains('@') {
        problems.push("email must contain @".to_string());
    }
    problems.require_token("source", tokens::CANDIDATE_SOURCES, &payload.source);
    problems.ref_opt("person_ref", entity_ref::EntityType::Person, payload.person_ref.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = candidates::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        email: ActiveValue::set(payload.email.clone()),
        source: ActiveValue::set(payload.source.clone()),
        consent_until: ActiveValue::set(payload.consent_until),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "candidate", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "candidate", "created", &row.pid.to_string(), &row.display_name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/candidates` — the pool. Consent-expired candidates are
/// **excluded** (HCM-D8); `?expired=1` lists only the purge queue.
#[derive(Debug, Deserialize)]
struct CandidateListParams {
    #[serde(default)]
    expired: Option<String>,
}

#[debug_handler]
async fn list_candidates(
    State(ctx): State<AppContext>,
    Query(params): Query<CandidateListParams>,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let rows = candidates::Entity::find()
        .filter(candidates::Column::DeletedAt.is_null())
        .order_by_asc(candidates::Column::Id)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    let expired_only = params.expired.as_deref().is_some_and(|v| v == "1" || v == "true");
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|c| {
            let expired = c.consent_until.is_some_and(|until| until < today);
            if expired_only { expired } else { !expired }
        })
        .collect();
    format::json(rows)
}

/// `POST /api/requisitions/{pid}/applications` — apply a candidate.
#[debug_handler]
async fn create_application(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ApplicationPayload>,
) -> Result<Response> {
    let requisition = records::find_requisition(&ctx.db, records::parse_pid(&pid)?).await?;
    let candidate = records::find_candidate(&ctx.db, payload.candidate_pid).await?;
    if requisition.status == "draft" || requisition.status == "cancelled" || requisition.status == "filled" {
        return Err(unprocessable(&format!(
            "requisition is {} and not accepting applications",
            requisition.status
        )));
    }
    let today = chrono::Utc::now().date_naive();
    if candidate.consent_until.is_some_and(|until| until < today) {
        return Err(unprocessable("candidate consent has expired"));
    }
    let mut problems = Problems::new();
    problems.cap_opt("notes", payload.notes.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = applications::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        requisition_pid: ActiveValue::set(requisition.pid),
        candidate_pid: ActiveValue::set(candidate.pid),
        stage: ActiveValue::set("received".to_string()),
        notes: ActiveValue::set(payload.notes.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "application", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "application", "created", &row.pid.to_string(), &candidate.display_name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/requisitions/{pid}/applications`.
#[debug_handler]
async fn list_applications(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let requisition = records::find_requisition(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = applications::Entity::find()
        .filter(applications::Column::RequisitionPid.eq(requisition.pid))
        .filter(applications::Column::DeletedAt.is_null())
        .order_by_asc(applications::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/applications/{pid}/stage` — one stage transition.
/// `hired` creates the Employee (onboarding status) **in the same
/// transaction** (HCM-R2, HCM-D9) and emits `employee_hired`.
#[allow(clippy::too_many_lines)] // one linear stage walk incl. the in-tx hire
#[debug_handler]
async fn application_stage(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StagePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::APPLICATION_STAGES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let application = records::find_application(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("application", lifecycle::APPLICATION, &application.stage, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let requisition = records::find_requisition(&ctx.db, application.requisition_pid).await?;
    let candidate = records::find_candidate(&ctx.db, application.candidate_pid).await?;

    let txn = ctx.db.begin().await?;
    let from = application.stage.clone();
    let mut active: applications::ActiveModel = application.clone().into();
    active.stage = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "application",
        row.pid,
        "application_staged",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "application",
        "application_staged",
        &row.pid.to_string(),
        &candidate.display_name,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;

    let mut employee_pid = None;
    if payload.to == "hired" {
        let person_ref = payload
            .person_ref
            .clone()
            .or_else(|| candidate.person_ref.clone())
            .ok_or_else(|| unprocessable("hiring requires person_ref (on the payload or the candidate)"))?;
        let employee_number = payload
            .employee_number
            .clone()
            .ok_or_else(|| unprocessable("hiring requires employee_number"))?;
        let mut problems = Problems::new();
        problems.require_ref("person_ref", entity_ref::EntityType::Person, &person_ref);
        let employment_type = payload.employment_type.clone().unwrap_or_else(|| "permanent".to_string());
        problems.require_token("employment_type", tokens::EMPLOYMENT_TYPES, &employment_type);
        ensure_valid(&problems.into_vec())?;
        let employee = employees::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            person_ref: ActiveValue::set(person_ref),
            worker_ref: ActiveValue::set(None),
            organization_ref: ActiveValue::set(requisition.organization_ref.clone()),
            employee_number: ActiveValue::set(employee_number),
            display_name: ActiveValue::set(candidate.display_name.clone()),
            status: ActiveValue::set("onboarding".to_string()),
            employment_type: ActiveValue::set(employment_type),
            fte_percent: ActiveValue::set(payload.fte_percent.unwrap_or(100)),
            department: ActiveValue::set(requisition.department.clone()),
            job_title: ActiveValue::set(requisition.job_title.clone()),
            manager_pid: ActiveValue::set(None),
            salary_minor: ActiveValue::set(payload.salary_minor),
            salary_currency: ActiveValue::set(payload.salary_currency.clone()),
            hired_on: ActiveValue::set(payload.hired_on.unwrap_or_else(|| chrono::Utc::now().date_naive())),
            terminated_on: ActiveValue::set(None),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        Audit::record(
            &txn,
            "employee",
            employee.pid,
            "employee_hired",
            caller.actor(),
            Some(serde_json::json!({
                "application_pid": row.pid, "department": employee.department,
            })),
        )
        .await?;
        streaming::emit_on(
            &txn,
            "employee",
            "employee_hired",
            &employee.pid.to_string(),
            &employee.employee_number,
            caller.actor(),
            None,
        )
        .await?;
        employee_pid = Some(employee.pid);
    }
    txn.commit().await?;
    if employee_pid.is_some() {
        Metrics::global().employee_hired_total.inc();
    }
    format::json(serde_json::json!({
        "pid": row.pid, "stage": row.stage, "employee_pid": employee_pid,
    }))
}

/// `POST /api/applications/{pid}/interviews`.
#[debug_handler]
async fn create_interview(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<InterviewPayload>,
) -> Result<Response> {
    let application = records::find_application(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_ref("interviewer_ref", entity_ref::EntityType::Worker, &payload.interviewer_ref);
    problems.cap_opt("notes", payload.notes.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = interviews::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        application_pid: ActiveValue::set(application.pid),
        scheduled_at: ActiveValue::set(payload.scheduled_at),
        interviewer_ref: ActiveValue::set(payload.interviewer_ref.clone()),
        outcome: ActiveValue::set("pending".to_string()),
        notes: ActiveValue::set(payload.notes.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "interview", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/applications/{pid}/interviews`.
#[debug_handler]
async fn list_interviews(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let application = records::find_application(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = interviews::Entity::find()
        .filter(interviews::Column::ApplicationPid.eq(application.pid))
        .filter(interviews::Column::DeletedAt.is_null())
        .order_by_asc(interviews::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `PUT /api/interviews/{pid}` — record the outcome.
#[debug_handler]
async fn interview_outcome(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<OutcomePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("outcome", tokens::INTERVIEW_OUTCOMES, &payload.outcome);
    problems.cap_opt("notes", payload.notes.as_deref());
    ensure_valid(&problems.into_vec())?;
    let pid = records::parse_pid(&pid)?;
    let row = interviews::Entity::find()
        .filter(interviews::Column::Pid.eq(pid))
        .filter(interviews::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let txn = ctx.db.begin().await?;
    let mut active: interviews::ActiveModel = row.into();
    active.outcome = ActiveValue::set(payload.outcome.clone());
    if let Some(notes) = payload.notes {
        active.notes = ActiveValue::set(Some(notes));
    }
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "interview",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "outcome": row.outcome })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/employees/{pid}/onboarding` — add checklist items.
#[debug_handler]
async fn add_onboarding(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<OnboardingPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    if payload.items.is_empty() {
        problems.push("items must be non-empty".to_string());
    }
    if payload.items.len() > 64 {
        problems.push("items exceeds 64 entries".to_string());
    }
    for item in &payload.items {
        problems.require_text("items[].name", &item.name);
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let mut pids = Vec::with_capacity(payload.items.len());
    for item in &payload.items {
        let row = onboarding_items::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            employee_pid: ActiveValue::set(employee.pid),
            name: ActiveValue::set(item.name.clone()),
            mandatory: ActiveValue::set(item.mandatory),
            status: ActiveValue::set("pending".to_string()),
            waived_reason: ActiveValue::set(None),
            completed_at: ActiveValue::set(None),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        pids.push(row.pid.to_string());
    }
    Audit::record(
        &txn,
        "employee",
        employee.pid,
        "onboarding_items_added",
        caller.actor(),
        Some(serde_json::json!({ "count": pids.len() })),
    )
    .await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pids": pids }))
}

/// `GET /api/employees/{pid}/onboarding`.
#[debug_handler]
async fn list_onboarding(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = onboarding_items::Entity::find()
        .filter(onboarding_items::Column::EmployeePid.eq(employee.pid))
        .filter(onboarding_items::Column::DeletedAt.is_null())
        .order_by_asc(onboarding_items::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// Shared item-transition body for complete/waive.
async fn finish_item(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: &str,
    status: &str,
    reason: Option<String>,
) -> Result<Response> {
    let item = records::find_onboarding_item(&ctx.db, records::parse_pid(pid)?).await?;
    if item.status != "pending" {
        return Err(unprocessable(&format!("item is already {}", item.status)));
    }
    let txn = ctx.db.begin().await?;
    let mut active: onboarding_items::ActiveModel = item.into();
    active.status = ActiveValue::set(status.to_string());
    active.waived_reason = ActiveValue::set(reason.clone());
    active.completed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "onboarding_item",
        row.pid,
        &format!("onboarding_item_{status}"),
        caller.actor(),
        reason.map(|r| serde_json::json!({ "reason": r })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "onboarding_item",
        "onboarding_item_completed",
        &row.pid.to_string(),
        &row.name,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/onboarding-items/{pid}/complete`.
#[debug_handler]
async fn complete_item(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    finish_item(&ctx, &caller, &pid, "complete", None).await
}

/// `POST /api/onboarding-items/{pid}/waive` — waive with a recorded
/// reason (HCM-R3).
#[debug_handler]
async fn waive_item(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<WaivePayload>,
) -> Result<Response> {
    if payload.reason.trim().is_empty() {
        return Err(unprocessable("a waive requires a reason"));
    }
    finish_item(&ctx, &caller, &pid, "waived", Some(payload.reason)).await
}

/// The acquisition routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/requisitions", post(create_requisition))
        .add("/requisitions", get(list_requisitions))
        .add("/requisitions/{pid}", get(get_requisition))
        .add("/requisitions/{pid}/status", post(requisition_status))
        .add("/requisitions/{pid}/applications", post(create_application))
        .add("/requisitions/{pid}/applications", get(list_applications))
        .add("/candidates", post(create_candidate))
        .add("/candidates", get(list_candidates))
        .add("/applications/{pid}/stage", post(application_stage))
        .add("/applications/{pid}/interviews", post(create_interview))
        .add("/applications/{pid}/interviews", get(list_interviews))
        .add("/interviews/{pid}", put(interview_outcome))
        .add("/employees/{pid}/onboarding", post(add_onboarding))
        .add("/employees/{pid}/onboarding", get(list_onboarding))
        .add("/onboarding-items/{pid}/complete", post(complete_item))
        .add("/onboarding-items/{pid}/waive", post(waive_item))
}
