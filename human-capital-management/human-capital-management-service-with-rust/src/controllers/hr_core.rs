//! HR core (HCM-R7–R9): employee CRUD + the status state machine +
//! the derived org chart, and benefits administration.
//!
//! Every mutation runs on one transaction: the row change, its audit
//! entry, and (under the `outbox` transport) its event share a commit
//! boundary (HCM-D9). Employee reads run the record-level ABAC pass
//! and honour the `mask` obligation (salary redaction, HCM-R15).

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::metrics::Metrics;
use crate::models::_entities::{benefit_enrollments, benefit_plans, employees, onboarding_items};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{lifecycle, org, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/employees` body.
#[derive(Debug, Deserialize)]
struct EmployeePayload {
    person_ref: String,
    #[serde(default)]
    worker_ref: Option<String>,
    organization_ref: String,
    employee_number: String,
    display_name: String,
    employment_type: String,
    #[serde(default = "default_fte")]
    fte_percent: i32,
    department: String,
    job_title: String,
    #[serde(default)]
    manager_pid: Option<Uuid>,
    #[serde(default)]
    salary_minor: Option<i64>,
    #[serde(default)]
    salary_currency: Option<String>,
    hired_on: chrono::NaiveDate,
}

/// `PUT /api/employees/{pid}` body — the mutable employment facts.
#[derive(Debug, Deserialize)]
struct EmployeeUpdate {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    department: Option<String>,
    #[serde(default)]
    job_title: Option<String>,
    #[serde(default)]
    fte_percent: Option<i32>,
    #[serde(default)]
    manager_pid: Option<Uuid>,
    #[serde(default)]
    clear_manager: bool,
    #[serde(default)]
    salary_minor: Option<i64>,
    #[serde(default)]
    salary_currency: Option<String>,
    #[serde(default)]
    worker_ref: Option<String>,
}

/// `POST /api/employees/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct StatusPayload {
    to: String,
    #[serde(default)]
    reason: Option<String>,
}

/// `POST /api/benefit-plans` body.
#[derive(Debug, Deserialize)]
struct BenefitPlanPayload {
    name: String,
    kind: String,
    provider: String,
    employee_cost_minor: i64,
    employer_cost_minor: i64,
    currency: String,
}

/// `POST /api/employees/{pid}/benefit-enrollments` body.
#[derive(Debug, Deserialize)]
struct EnrollmentPayload {
    plan_pid: Uuid,
    starts_on: chrono::NaiveDate,
    #[serde(default)]
    ends_on: Option<chrono::NaiveDate>,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

const fn default_fte() -> i32 {
    100
}

fn validate_employee(p: &EmployeePayload) -> Vec<String> {
    let mut problems = Problems::new();
    problems.require_ref("person_ref", entity_ref::EntityType::Person, &p.person_ref);
    problems.ref_opt("worker_ref", entity_ref::EntityType::Worker, p.worker_ref.as_deref());
    problems.require_ref(
        "organization_ref",
        entity_ref::EntityType::Organization,
        &p.organization_ref,
    );
    problems.require_text("employee_number", &p.employee_number);
    problems.require_text("display_name", &p.display_name);
    problems.require_token("employment_type", tokens::EMPLOYMENT_TYPES, &p.employment_type);
    problems.require_text("department", &p.department);
    problems.require_text("job_title", &p.job_title);
    if !(1..=100).contains(&p.fte_percent) {
        problems.push(format!("fte_percent {} out of range 1-100", p.fte_percent));
    }
    if p.salary_minor.is_some_and(|s| s < 0) {
        problems.push("salary_minor must be non-negative".to_string());
    }
    if p.salary_minor.is_some() && p.salary_currency.as_deref().unwrap_or("").trim().is_empty() {
        problems.push("salary_currency is required with salary_minor".to_string());
    }
    problems.into_vec()
}

/// The live `manager_of` map for the cycle check (employee pid →
/// manager pid).
async fn manager_map(db: &DatabaseConnection) -> Result<HashMap<Uuid, Uuid>> {
    let rows = employees::Entity::find()
        .filter(employees::Column::DeletedAt.is_null())
        .filter(employees::Column::ManagerPid.is_not_null())
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|e| e.manager_pid.map(|m| (e.pid, m)))
        .collect())
}

/// `POST /api/employees` — create in `onboarding` status.
#[debug_handler]
async fn create_employee(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<EmployeePayload>,
) -> Result<Response> {
    ensure_valid(&validate_employee(&payload))?;
    if let Some(manager) = payload.manager_pid {
        records::find_employee(&ctx.db, manager).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = employees::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        worker_ref: ActiveValue::set(payload.worker_ref.clone()),
        organization_ref: ActiveValue::set(payload.organization_ref.clone()),
        employee_number: ActiveValue::set(payload.employee_number.clone()),
        display_name: ActiveValue::set(payload.display_name.clone()),
        status: ActiveValue::set("onboarding".to_string()),
        employment_type: ActiveValue::set(payload.employment_type.clone()),
        fte_percent: ActiveValue::set(payload.fte_percent),
        department: ActiveValue::set(payload.department.clone()),
        job_title: ActiveValue::set(payload.job_title.clone()),
        manager_pid: ActiveValue::set(payload.manager_pid),
        salary_minor: ActiveValue::set(payload.salary_minor),
        salary_currency: ActiveValue::set(payload.salary_currency.clone()),
        hired_on: ActiveValue::set(payload.hired_on),
        terminated_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "employee",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "department": row.department })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "employee",
        "created",
        &row.pid.to_string(),
        &row.employee_number,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/employees` — active employees, filterable by
/// `?department=` and `?status=`. Record-level masking applies per
/// row (a list must never reveal more than the single read).
#[derive(Debug, Deserialize)]
struct EmployeeListParams {
    #[serde(default)]
    department: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_employees(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Query(params): Query<EmployeeListParams>,
) -> Result<Response> {
    let mut query = employees::Entity::find().filter(employees::Column::DeletedAt.is_null());
    if let Some(department) = &params.department {
        query = query.filter(employees::Column::Department.eq(department));
    }
    if let Some(status) = &params.status {
        query = query.filter(employees::Column::Status.eq(status));
    }
    let rows = query
        .order_by_asc(employees::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for employee in rows {
        let obligations = auth::authorize_record(
            &caller,
            authentication_verifier::Action::Read,
            &auth::employee_resource_attrs(&employee),
        )
        .map_err(record_rejection)?;
        out.push(if obligations.iter().any(|o| o == "mask") {
            auth::mask_employee(employee)
        } else {
            employee
        });
    }
    format::json(out)
}

/// `GET /api/employees/{pid}` — one employee; a salary-bearing read
/// is audited (HCM-D7); the `mask` obligation redacts the salary.
#[debug_handler]
async fn get_employee(
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
    if employee.salary_minor.is_some() && !masked {
        Audit::record(
            &ctx.db,
            "employee",
            employee.pid,
            "salary_read",
            caller.actor(),
            Some(serde_json::json!({ "department": employee.department })),
        )
        .await?;
    }
    format::json(if masked {
        auth::mask_employee(employee)
    } else {
        employee
    })
}

/// `PUT /api/employees/{pid}` — update mutable employment facts.
/// A manager change runs the org-chart cycle check (HCM-R7).
#[debug_handler]
async fn update_employee(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<EmployeeUpdate>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let mut problems = Problems::new();
    if let Some(name) = &payload.display_name {
        problems.require_text("display_name", name);
    }
    if let Some(dept) = &payload.department {
        problems.require_text("department", dept);
    }
    if let Some(fte) = payload.fte_percent
        && !(1..=100).contains(&fte) {
            problems.push(format!("fte_percent {fte} out of range 1-100"));
        }
    if payload.salary_minor.is_some_and(|s| s < 0) {
        problems.push("salary_minor must be non-negative".to_string());
    }
    problems.ref_opt("worker_ref", entity_ref::EntityType::Worker, payload.worker_ref.as_deref());
    ensure_valid(&problems.into_vec())?;
    if let Some(manager) = payload.manager_pid {
        records::find_employee(&ctx.db, manager).await?;
        let map = manager_map(&ctx.db).await?;
        if org::would_create_cycle(employee.pid, manager, &map) {
            return Err(unprocessable("manager assignment would create a cycle"));
        }
    }
    let txn = ctx.db.begin().await?;
    let mut active: employees::ActiveModel = employee.clone().into();
    if let Some(v) = payload.display_name {
        active.display_name = ActiveValue::set(v);
    }
    if let Some(v) = payload.department {
        active.department = ActiveValue::set(v);
    }
    if let Some(v) = payload.job_title {
        active.job_title = ActiveValue::set(v);
    }
    if let Some(v) = payload.fte_percent {
        active.fte_percent = ActiveValue::set(v);
    }
    if payload.clear_manager {
        active.manager_pid = ActiveValue::set(None);
    } else if let Some(v) = payload.manager_pid {
        active.manager_pid = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.salary_minor {
        active.salary_minor = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.salary_currency {
        active.salary_currency = ActiveValue::set(Some(v));
    }
    if let Some(v) = payload.worker_ref {
        active.worker_ref = ActiveValue::set(Some(v));
    }
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "employee",
        row.pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "department": row.department })),
    )
    .await?;
    streaming::emit_on(&txn, "employee", "updated", &row.pid.to_string(), &row.employee_number, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(auth::mask_employee(row))
}

/// `POST /api/employees/{pid}/status` — one lifecycle transition.
/// `onboarding → active` requires every mandatory onboarding item
/// complete or waived (HCM-R3).
#[debug_handler]
async fn change_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::EMPLOYEE_STATUSES, &payload.to);
    problems.cap_opt("reason", payload.reason.as_deref());
    ensure_valid(&problems.into_vec())?;
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    lifecycle::check("employee", lifecycle::EMPLOYEE, &employee.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    if employee.status == "onboarding" && payload.to == "active" {
        let open = onboarding_items::Entity::find()
            .filter(onboarding_items::Column::EmployeePid.eq(employee.pid))
            .filter(onboarding_items::Column::DeletedAt.is_null())
            .filter(onboarding_items::Column::Mandatory.eq(true))
            .filter(onboarding_items::Column::Status.eq("pending"))
            .count(&ctx.db)
            .await?;
        if open > 0 {
            return Err(unprocessable(&format!(
                "{open} mandatory onboarding item(s) still pending (complete or waive them first)"
            )));
        }
    }
    let txn = ctx.db.begin().await?;
    let from = employee.status.clone();
    let number = employee.employee_number.clone();
    let department = employee.department.clone();
    let mut active: employees::ActiveModel = employee.clone().into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "terminated" || payload.to == "retired" {
        active.terminated_on = ActiveValue::set(Some(chrono::Utc::now().date_naive()));
    }
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "active" if from == "onboarding" => "employee_activated",
        "terminated" => "employee_terminated",
        "retired" => "employee_retired",
        _ => "employee_status_changed",
    };
    Audit::record(
        &txn,
        "employee",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({
            "from": from, "to": payload.to, "reason": payload.reason,
            "department": department,
        })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "employee",
        kind,
        &row.pid.to_string(),
        &number,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    txn.commit().await?;
    match kind {
        "employee_activated" => Metrics::global().employee_activated_total.inc(),
        "employee_terminated" => Metrics::global().employee_terminated_total.inc(),
        _ => {}
    }
    format::json(auth::mask_employee(row))
}

/// `DELETE /api/employees/{pid}` — soft delete.
#[debug_handler]
async fn delete_employee(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Destructive,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;
    let txn = ctx.db.begin().await?;
    let pid = employee.pid;
    let number = employee.employee_number.clone();
    let mut active: employees::ActiveModel = employee.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(&txn, "employee", pid, "deleted", caller.actor(), None).await?;
    streaming::emit_on(&txn, "employee", "deleted", &pid.to_string(), &number, caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// One org-chart node.
#[derive(Debug, Serialize)]
struct OrgNode {
    pid: String,
    display_name: String,
    job_title: String,
    department: String,
    reports: Vec<OrgNode>,
}

/// Build one org-chart node (bounded depth; the write path
/// prevents cycles).
fn build_org_node(
    node: &employees::Model,
    children: &HashMap<Option<Uuid>, Vec<&employees::Model>>,
    depth: usize,
) -> OrgNode {
    let reports = if depth > 32 {
        Vec::new() // corrupt-cycle guard; the write path prevents cycles
    } else {
        children
            .get(&Some(node.pid))
            .map(|kids| kids.iter().map(|k| build_org_node(k, children, depth + 1)).collect())
            .unwrap_or_default()
    };
    OrgNode {
        pid: node.pid.to_string(),
        display_name: node.display_name.clone(),
        job_title: node.job_title.clone(),
        department: node.department.clone(),
        reports,
    }
}

/// `GET /api/org-chart?organization=<ref>` — the manager forest for
/// one organization (roots = employees with no manager).
#[derive(Debug, Deserialize)]
struct OrgChartParams {
    organization: String,
}

#[debug_handler]
async fn org_chart(
    State(ctx): State<AppContext>,
    Query(params): Query<OrgChartParams>,
) -> Result<Response> {
    let rows = employees::Entity::find()
        .filter(employees::Column::DeletedAt.is_null())
        .filter(employees::Column::OrganizationRef.eq(&params.organization))
        .order_by_asc(employees::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut children: HashMap<Option<Uuid>, Vec<&employees::Model>> = HashMap::new();
    for e in &rows {
        children.entry(e.manager_pid).or_default().push(e);
    }
    let roots: Vec<OrgNode> = children
        .get(&None)
        .map(|top| top.iter().map(|n| build_org_node(n, &children, 0)).collect())
        .unwrap_or_default();
    format::json(roots)
}

/// `POST /api/benefit-plans`.
#[debug_handler]
async fn create_plan(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<BenefitPlanPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_token("kind", tokens::BENEFIT_KINDS, &payload.kind);
    problems.require_text("provider", &payload.provider);
    problems.require_text("currency", &payload.currency);
    if payload.employee_cost_minor < 0 || payload.employer_cost_minor < 0 {
        problems.push("benefit costs must be non-negative".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = benefit_plans::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        kind: ActiveValue::set(payload.kind.clone()),
        provider: ActiveValue::set(payload.provider.clone()),
        employee_cost_minor: ActiveValue::set(payload.employee_cost_minor),
        employer_cost_minor: ActiveValue::set(payload.employer_cost_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "benefit_plan", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "benefit_plan", "created", &row.pid.to_string(), &row.name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/benefit-plans`.
#[debug_handler]
async fn list_plans(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = benefit_plans::Entity::find()
        .filter(benefit_plans::Column::DeletedAt.is_null())
        .order_by_asc(benefit_plans::Column::Id)
        .limit(200)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/employees/{pid}/benefit-enrollments` — enrol; the
/// partial unique index refuses a double enrolment (HCM-R9).
#[debug_handler]
async fn enroll(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<EnrollmentPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let plan = records::find_benefit_plan(&ctx.db, payload.plan_pid).await?;
    if payload.ends_on.is_some_and(|end| end < payload.starts_on) {
        return Err(unprocessable("ends_on is before starts_on"));
    }
    let existing = benefit_enrollments::Entity::find()
        .filter(benefit_enrollments::Column::PlanPid.eq(plan.pid))
        .filter(benefit_enrollments::Column::EmployeePid.eq(employee.pid))
        .filter(benefit_enrollments::Column::DeletedAt.is_null())
        .count(&ctx.db)
        .await?;
    if existing > 0 {
        return Err(unprocessable("employee is already enrolled in this plan"));
    }
    let txn = ctx.db.begin().await?;
    let row = benefit_enrollments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        employee_pid: ActiveValue::set(employee.pid),
        starts_on: ActiveValue::set(payload.starts_on),
        ends_on: ActiveValue::set(payload.ends_on),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "benefit_enrollment", row.pid, "created", caller.actor(), None).await?;
    streaming::emit_on(&txn, "benefit_enrollment", "benefit_enrolled", &row.pid.to_string(), &plan.name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/employees/{pid}/benefit-enrollments`.
#[debug_handler]
async fn list_enrollments(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = benefit_enrollments::Entity::find()
        .filter(benefit_enrollments::Column::EmployeePid.eq(employee.pid))
        .filter(benefit_enrollments::Column::DeletedAt.is_null())
        .order_by_asc(benefit_enrollments::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `DELETE /api/benefit-enrollments/{pid}` — soft delete (unenrol).
#[debug_handler]
async fn unenroll(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = records::find_benefit_enrollment(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let pid = row.pid;
    let mut active: benefit_enrollments::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&txn).await?;
    Audit::record(&txn, "benefit_enrollment", pid, "deleted", caller.actor(), None).await?;
    streaming::emit_on(&txn, "benefit_enrollment", "deleted", &pid.to_string(), "", caller.actor(), None).await?;
    txn.commit().await?;
    format::empty_json()
}

/// The HR-core routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/employees", post(create_employee))
        .add("/employees", get(list_employees))
        .add("/employees/{pid}", get(get_employee))
        .add("/employees/{pid}", put(update_employee))
        .add("/employees/{pid}", delete(delete_employee))
        .add("/employees/{pid}/status", post(change_status))
        .add("/org-chart", get(org_chart))
        .add("/benefit-plans", post(create_plan))
        .add("/benefit-plans", get(list_plans))
        .add("/employees/{pid}/benefit-enrollments", post(enroll))
        .add("/employees/{pid}/benefit-enrollments", get(list_enrollments))
        .add("/benefit-enrollments/{pid}", delete(unenroll))
}
