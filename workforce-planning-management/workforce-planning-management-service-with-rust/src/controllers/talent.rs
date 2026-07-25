//! Talent strategy — **development plans** (upskilling and
//! reskilling), **talent pipelines**, and **early-career programmes**
//! (apprenticeships, internships, graduate schemes). WPM-R21–R23.
//!
//! Declared / recorded data plus derived views; the pure rules live in
//! [`crate::rules::talent`] and this module only wires them, persists,
//! audits, and emits.
//!
//! Three things this module is deliberate about:
//!
//! - **Upskilling vs reskilling is a real distinction.** An `upskill`
//!   plan deepens the current role and must not name a target role; a
//!   `reskill` plan builds toward a *different* role and must name it.
//!   Anything else is a `422` rather than a plan whose meaning depends
//!   on who reads it.
//! - **Progress is evidence, not a claim.** A plan reports both its
//!   declared progress (items marked `achieved`) *and* its verified
//!   progress (the employee's declared proficiency actually reaching
//!   the target), so the two can disagree in the open.
//! - **An apprenticeship cannot be completed below its off-the-job
//!   training minimum** — completing one that has not met its hours
//!   would be a false record of a regulated programme.

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, QueryOrder};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

use super::{ensure_valid, record_rejection, unprocessable};
use crate::auth::{self, MaybeAuthUser};
use crate::models::_entities::{
    candidates, development_plan_items, development_plans, early_career_programs, employee_skills,
    employees, pipeline_members, program_placements, skills, talent_pipelines,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::talent as rules;
use crate::streaming;
use crate::validation::Problems;

/// A `{pid}` reference response.
#[derive(serde::Serialize)]
struct PidRef {
    pid: String,
}

impl PidRef {
    fn of(pid: Uuid) -> Self {
        Self {
            pid: pid.to_string(),
        }
    }
}

// ─── Development plans (upskilling / reskilling) ─────────────────────────────

/// `POST /api/employees/{pid}/development-plans` body.
#[derive(Debug, Deserialize)]
struct PlanPayload {
    /// `upskill` (deepen the current role) or `reskill` (build toward a
    /// different one).
    kind: String,
    #[serde(default)]
    target_job_title: Option<String>,
    #[serde(default)]
    target_department: Option<String>,
    #[serde(default)]
    rationale: Option<String>,
    #[serde(default)]
    target_on: Option<chrono::NaiveDate>,
    /// The skill steps, declared inline.
    #[serde(default)]
    items: Vec<PlanItemPayload>,
}

/// One skill step of a development plan.
#[derive(Debug, Deserialize)]
struct PlanItemPayload {
    skill_pid: Uuid,
    current_level: i32,
    target_level: i32,
    method: String,
    #[serde(default)]
    course_ref: Option<String>,
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
}

/// `POST /api/employees/{pid}/development-plans` — open a plan
/// (`draft`) with its skill steps.
///
/// The kind and the target role must agree
/// ([`rules::target_matches_kind`]); a reskill toward the employee's
/// *current* job title is refused, since that is an upskill by another
/// name. Every step must raise the level on the 1–5 scale, and no skill
/// may appear twice.
#[debug_handler]
#[allow(clippy::too_many_lines)] // validation + the plan and its items in one transaction
async fn create_plan(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<PlanPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;

    let mut problems = Problems::new();
    problems.require_token("kind", rules::DEVELOPMENT_PLAN_KINDS, &payload.kind);
    problems.cap_opt("rationale", payload.rationale.as_deref());
    problems.cap_opt("target_job_title", payload.target_job_title.as_deref());
    problems.cap_opt("target_department", payload.target_department.as_deref());
    if rules::DEVELOPMENT_PLAN_KINDS.contains(&payload.kind.as_str())
        && let Err(reason) = rules::target_matches_kind(
            &payload.kind,
            payload.target_job_title.as_deref(),
            payload.target_department.as_deref(),
        )
    {
        problems.push(reason);
    }
    if payload.kind == "reskill"
        && payload
            .target_job_title
            .as_deref()
            .is_some_and(|t| t == employee.job_title)
        && payload
            .target_department
            .as_deref()
            .is_none_or(|d| d == employee.department)
    {
        problems.push("the target role is the employee's current role; use kind `upskill` instead");
    }
    let mut seen: Vec<Uuid> = Vec::new();
    for (index, item) in payload.items.iter().enumerate() {
        problems.require_token(
            &format!("items[{index}].method"),
            rules::DEVELOPMENT_METHODS,
            &item.method,
        );
        if let Err(reason) = rules::valid_step(item.current_level, item.target_level) {
            problems.push(format!("items[{index}]: {reason}"));
        }
        if seen.contains(&item.skill_pid) {
            problems.push(format!("items[{index}]: skill appears more than once"));
        } else {
            seen.push(item.skill_pid);
        }
    }
    ensure_valid(&problems.into_vec())?;

    let txn = ctx.db.begin().await?;
    let plan = development_plans::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        target_job_title: ActiveValue::set(payload.target_job_title.clone()),
        target_department: ActiveValue::set(payload.target_department.clone()),
        rationale: ActiveValue::set(payload.rationale.clone()),
        status: ActiveValue::set("draft".to_string()),
        started_on: ActiveValue::set(None),
        target_on: ActiveValue::set(payload.target_on),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    for item in &payload.items {
        // The skill must exist — a plan step against an unknown skill
        // could never be verified against declared proficiency.
        let skill = skills::Entity::find()
            .filter(skills::Column::Pid.eq(item.skill_pid))
            .filter(skills::Column::DeletedAt.is_null())
            .one(&txn)
            .await?
            .ok_or(Error::NotFound)?;
        development_plan_items::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            plan_pid: ActiveValue::set(plan.pid),
            skill_pid: ActiveValue::set(skill.pid),
            current_level: ActiveValue::set(item.current_level),
            target_level: ActiveValue::set(item.target_level),
            method: ActiveValue::set(item.method.clone()),
            course_ref: ActiveValue::set(item.course_ref.clone()),
            due_on: ActiveValue::set(item.due_on),
            status: ActiveValue::set("planned".to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
    }
    Audit::record(
        &txn,
        "development_plan",
        plan.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.kind, "items": payload.items.len() })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "development_plan",
        "created",
        &plan.pid.to_string(),
        &employee.display_name,
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.kind })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef::of(plan.pid))
}

/// `GET /api/employees/{pid}/development-plans` — one employee's plans
/// with both progress readings (declared and verified).
#[debug_handler]
async fn list_plans(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;

    let plans = development_plans::Entity::find()
        .filter(development_plans::Column::EmployeePid.eq(employee.pid))
        .filter(development_plans::Column::DeletedAt.is_null())
        .order_by_asc(development_plans::Column::Id)
        .all(&ctx.db)
        .await?;
    let declared = declared_levels(&ctx, employee.pid).await?;

    let mut view = Vec::with_capacity(plans.len());
    for plan in &plans {
        let items = development_plan_items::Entity::find()
            .filter(development_plan_items::Column::PlanPid.eq(plan.pid))
            .order_by_asc(development_plan_items::Column::Id)
            .all(&ctx.db)
            .await?;
        view.push(plan_view(plan, &items, &declared));
    }
    format::json(serde_json::json!({
        "employee_pid": employee.pid,
        "derivation": PROGRESS_DERIVATION,
        "plans": view,
    }))
}

/// How the two progress readings are derived, echoed in every payload
/// that carries them.
const PROGRESS_DERIVATION: &str = "declared progress = items marked achieved / all items (abandoned items stay in the \
     denominator); verified progress = items whose skill's declared proficiency has actually \
     reached the target / all items";

/// Project one plan with its items and both progress readings.
fn plan_view(
    plan: &development_plans::Model,
    items: &[development_plan_items::Model],
    declared: &BTreeMap<Uuid, i32>,
) -> serde_json::Value {
    let statuses: Vec<String> = items.iter().map(|i| i.status.clone()).collect();
    let (achieved, total) = rules::plan_progress(&statuses);
    let targets: Vec<(Uuid, i32)> = items
        .iter()
        .map(|i| (i.skill_pid, i.target_level))
        .collect();
    let (verified, _) = rules::verified_progress(&targets, declared);
    serde_json::json!({
        "pid": plan.pid,
        "kind": plan.kind,
        "status": plan.status,
        "target_job_title": plan.target_job_title,
        "target_department": plan.target_department,
        "rationale": plan.rationale,
        "started_on": plan.started_on,
        "target_on": plan.target_on,
        "declared_progress": { "numerator": achieved, "denominator": total },
        "verified_progress": { "numerator": verified, "denominator": total },
        "items": items,
    })
}

/// One employee's declared skill levels, keyed by skill.
async fn declared_levels(ctx: &AppContext, employee_pid: Uuid) -> Result<BTreeMap<Uuid, i32>> {
    let rows = employee_skills::Entity::find()
        .filter(employee_skills::Column::EmployeePid.eq(employee_pid))
        .filter(employee_skills::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| (r.skill_pid, r.proficiency))
        .collect())
}

/// `POST /api/development-plans/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct StatusPayload {
    to: String,
}

/// `POST /api/development-plans/{pid}/status` — the lifecycle move
/// (`draft → active → completed`, cancel from either open state).
/// Activating stamps `started_on`. Completing requires every item to
/// have been resolved (`achieved` or `abandoned`) — a plan is not
/// complete while work on it is still open.
#[debug_handler]
async fn plan_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StatusPayload>,
) -> Result<Response> {
    let plan = records::find_development_plan(&ctx.db, records::parse_pid(&pid)?).await?;
    rules::plan_transition(&plan.status, &payload.to).map_err(|reason| unprocessable(&reason))?;

    if payload.to == "completed" {
        let open = development_plan_items::Entity::find()
            .filter(development_plan_items::Column::PlanPid.eq(plan.pid))
            .all(&ctx.db)
            .await?
            .into_iter()
            .filter(|i| !matches!(i.status.as_str(), "achieved" | "abandoned"))
            .count();
        if open > 0 {
            return Err(unprocessable(&format!(
                "{open} plan item(s) are still open; resolve them before completing the plan"
            )));
        }
    }

    let from = plan.status.clone();
    let plan_pid = plan.pid;
    let today = chrono::Utc::now().date_naive();
    let txn = ctx.db.begin().await?;
    let mut active: development_plans::ActiveModel = plan.into();
    active.status = ActiveValue::set(payload.to.clone());
    if payload.to == "active" {
        active.started_on = ActiveValue::set(Some(today));
    }
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "development_plan",
        plan_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// `PUT /api/development-plan-items/{pid}` body.
#[derive(Debug, Deserialize)]
struct ItemUpdate {
    status: String,
}

/// `PUT /api/development-plan-items/{pid}` — move one step's status.
///
/// Marking a step `achieved` does **not** change the employee's
/// declared proficiency: that is a separate, evidenced act
/// (`PUT /api/employees/{pid}/skills`). The plan view reports both, so
/// a claimed achievement with no proficiency behind it is visible
/// rather than hidden.
#[debug_handler]
async fn update_item(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ItemUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("status", rules::DEVELOPMENT_ITEM_STATUSES, &payload.status);
    ensure_valid(&problems.into_vec())?;

    let item = development_plan_items::Entity::find()
        .filter(development_plan_items::Column::Pid.eq(records::parse_pid(&pid)?))
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let item_pid = item.pid;
    let mut active: development_plan_items::ActiveModel = item.into();
    active.status = ActiveValue::set(payload.status.clone());
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "development_plan_item",
        item_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "to": payload.status })),
    )
    .await?;
    format::json(updated)
}

// ─── Talent pipelines ────────────────────────────────────────────────────────

/// `POST /api/talent-pipelines` body.
#[derive(Debug, Deserialize)]
struct PipelinePayload {
    name: String,
    /// `succession` | `hiring` | `early_careers` | `internal_mobility`.
    purpose: String,
    #[serde(default)]
    target_job_title: Option<String>,
    #[serde(default)]
    target_department: Option<String>,
}

/// `POST /api/talent-pipelines` — open a pipeline.
#[debug_handler]
async fn create_pipeline(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PipelinePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_token("purpose", rules::PIPELINE_PURPOSES, &payload.purpose);
    problems.cap_opt("target_job_title", payload.target_job_title.as_deref());
    problems.cap_opt("target_department", payload.target_department.as_deref());
    ensure_valid(&problems.into_vec())?;

    let row = talent_pipelines::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        purpose: ActiveValue::set(payload.purpose.clone()),
        target_job_title: ActiveValue::set(payload.target_job_title.clone()),
        target_department: ActiveValue::set(payload.target_department.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        "talent_pipeline",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    format::json(PidRef::of(row.pid))
}

/// `GET /api/talent-pipelines` — every pipeline with its health.
#[debug_handler]
async fn list_pipelines(State(ctx): State<AppContext>) -> Result<Response> {
    let pipelines = talent_pipelines::Entity::find()
        .filter(talent_pipelines::Column::DeletedAt.is_null())
        .order_by_asc(talent_pipelines::Column::Name)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::with_capacity(pipelines.len());
    for pipeline in &pipelines {
        let stages = member_stages(&ctx, pipeline.pid).await?;
        view.push(serde_json::json!({
            "pid": pipeline.pid,
            "name": pipeline.name,
            "purpose": pipeline.purpose,
            "target_job_title": pipeline.target_job_title,
            "target_department": pipeline.target_department,
            "health": rules::pipeline_health(&stages),
        }));
    }
    format::json(serde_json::json!({
        "note": "health counts live members (placed and exited have left the pipeline)",
        "pipelines": view,
    }))
}

/// The stages of one pipeline's live member rows.
async fn member_stages(ctx: &AppContext, pipeline_pid: Uuid) -> Result<Vec<String>> {
    let rows = pipeline_members::Entity::find()
        .filter(pipeline_members::Column::PipelinePid.eq(pipeline_pid))
        .filter(pipeline_members::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await?;
    Ok(rows.into_iter().map(|r| r.stage).collect())
}

/// `POST /api/talent-pipelines/{pid}/members` body.
#[derive(Debug, Deserialize)]
struct MemberPayload {
    /// `candidate` or `employee`.
    subject_kind: String,
    subject_pid: Uuid,
    #[serde(default)]
    readiness: Option<String>,
}

/// `POST /api/talent-pipelines/{pid}/members` — add someone to the
/// pipeline at the `identified` stage (idempotent per subject: a second
/// add is a `422`, not a duplicate row).
#[debug_handler]
async fn add_member(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<MemberPayload>,
) -> Result<Response> {
    let pipeline = records::find_talent_pipeline(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_token(
        "subject_kind",
        rules::PIPELINE_SUBJECTS,
        &payload.subject_kind,
    );
    problems.token_opt("readiness", rules::READINESS, payload.readiness.as_deref());
    ensure_valid(&problems.into_vec())?;

    // The subject must exist.
    if payload.subject_kind == "employee" {
        records::find_employee(&ctx.db, payload.subject_pid).await?;
    } else {
        candidates::Entity::find()
            .filter(candidates::Column::Pid.eq(payload.subject_pid))
            .filter(candidates::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await?
            .ok_or(Error::NotFound)?;
    }

    let existing = pipeline_members::Entity::find()
        .filter(pipeline_members::Column::PipelinePid.eq(pipeline.pid))
        .filter(pipeline_members::Column::SubjectKind.eq(payload.subject_kind.clone()))
        .filter(pipeline_members::Column::SubjectPid.eq(payload.subject_pid))
        .filter(pipeline_members::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?;
    if existing.is_some() {
        return Err(unprocessable("this subject is already in the pipeline"));
    }

    let row = pipeline_members::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        pipeline_pid: ActiveValue::set(pipeline.pid),
        subject_kind: ActiveValue::set(payload.subject_kind.clone()),
        subject_pid: ActiveValue::set(payload.subject_pid),
        stage: ActiveValue::set("identified".to_string()),
        readiness: ActiveValue::set(payload.readiness.clone()),
        added_on: ActiveValue::set(chrono::Utc::now().date_naive()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        "pipeline_member",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "pipeline": pipeline.name })),
    )
    .await?;
    format::json(PidRef::of(row.pid))
}

/// `POST /api/pipeline-members/{pid}/stage` body.
#[derive(Debug, Deserialize)]
struct StagePayload {
    to: String,
    /// Optionally restate readiness alongside the move.
    #[serde(default)]
    readiness: Option<String>,
}

/// `POST /api/pipeline-members/{pid}/stage` — move a member through the
/// pipeline. The machine allows a **step back** from `ready` to
/// `developing`: readiness can regress, and a pipeline that cannot say
/// so would overstate its bench.
#[debug_handler]
async fn move_member(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<StagePayload>,
) -> Result<Response> {
    let member = records::find_pipeline_member(&ctx.db, records::parse_pid(&pid)?).await?;
    rules::pipeline_transition(&member.stage, &payload.to)
        .map_err(|reason| unprocessable(&reason))?;
    let mut problems = Problems::new();
    problems.token_opt("readiness", rules::READINESS, payload.readiness.as_deref());
    ensure_valid(&problems.into_vec())?;

    let from = member.stage.clone();
    let member_pid = member.pid;
    let mut active: pipeline_members::ActiveModel = member.into();
    active.stage = ActiveValue::set(payload.to.clone());
    if payload.readiness.is_some() {
        active.readiness = ActiveValue::set(payload.readiness.clone());
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "pipeline_member",
        member_pid,
        "stage_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    format::json(updated)
}

/// `GET /api/talent-pipelines/{pid}` — one pipeline, its health, and
/// its members with display names.
#[debug_handler]
async fn get_pipeline(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let pipeline = records::find_talent_pipeline(&ctx.db, records::parse_pid(&pid)?).await?;
    let members = pipeline_members::Entity::find()
        .filter(pipeline_members::Column::PipelinePid.eq(pipeline.pid))
        .filter(pipeline_members::Column::DeletedAt.is_null())
        .order_by_asc(pipeline_members::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::with_capacity(members.len());
    for member in &members {
        let display_name = if member.subject_kind == "employee" {
            employees::Entity::find()
                .filter(employees::Column::Pid.eq(member.subject_pid))
                .one(&ctx.db)
                .await?
                .map(|e| e.display_name)
        } else {
            candidates::Entity::find()
                .filter(candidates::Column::Pid.eq(member.subject_pid))
                .one(&ctx.db)
                .await?
                .map(|c| c.display_name)
        };
        view.push(serde_json::json!({
            "pid": member.pid,
            "subject_kind": member.subject_kind,
            "subject_pid": member.subject_pid,
            "display_name": display_name,
            "stage": member.stage,
            "readiness": member.readiness,
            "added_on": member.added_on,
        }));
    }
    let stages: Vec<String> = members.iter().map(|m| m.stage.clone()).collect();
    format::json(serde_json::json!({
        "pipeline": pipeline,
        "health": rules::pipeline_health(&stages),
        "members": view,
    }))
}

// ─── Early-career programmes (apprenticeships / internships) ─────────────────

/// `POST /api/early-career-programs` body.
#[derive(Debug, Deserialize)]
struct ProgramPayload {
    name: String,
    /// `apprenticeship` | `internship` | `graduate`.
    kind: String,
    /// Apprenticeship level (e.g. 3 = advanced, 6 = degree).
    #[serde(default)]
    level: Option<i32>,
    duration_months: i32,
    /// Off-the-job training hours a placement must accrue before it can
    /// be completed. Required for an apprenticeship — the hours are the
    /// substance of the programme, not a nicety.
    #[serde(default)]
    min_off_the_job_hours: Option<i32>,
    /// The training provider, as an `organization:` `EntityRef` URN.
    #[serde(default)]
    provider_ref: Option<String>,
}

/// `POST /api/early-career-programs` — add an apprenticeship,
/// internship, or graduate scheme.
#[debug_handler]
async fn create_program(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ProgramPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_token("kind", rules::PROGRAM_KINDS, &payload.kind);
    if payload.duration_months <= 0 {
        problems.push("duration_months must be positive");
    }
    if payload.min_off_the_job_hours.is_some_and(|h| h <= 0) {
        problems.push("min_off_the_job_hours must be positive when given");
    }
    if payload.kind == "apprenticeship" && payload.min_off_the_job_hours.is_none() {
        problems.push(
            "an apprenticeship must declare min_off_the_job_hours (its off-the-job training \
             requirement)",
        );
    }
    if payload.level.is_some_and(|l| l <= 0) {
        problems.push("level must be positive when given");
    }
    if let Some(provider) = payload.provider_ref.as_deref() {
        problems.require_ref(
            "provider_ref",
            entity_ref::EntityType::Organization,
            provider,
        );
    }
    ensure_valid(&problems.into_vec())?;

    let row = early_career_programs::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        kind: ActiveValue::set(payload.kind.clone()),
        level: ActiveValue::set(payload.level),
        duration_months: ActiveValue::set(payload.duration_months),
        min_off_the_job_hours: ActiveValue::set(payload.min_off_the_job_hours),
        provider_ref: ActiveValue::set(payload.provider_ref.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Audit::record(
        &ctx.db,
        "early_career_program",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "kind": payload.kind })),
    )
    .await?;
    format::json(PidRef::of(row.pid))
}

/// Query for the programme list.
#[derive(Debug, Deserialize)]
struct ProgramQuery {
    kind: Option<String>,
}

/// `GET /api/early-career-programs?kind=` — the catalog with each
/// programme's placement counts and conversion rate.
#[debug_handler]
async fn list_programs(
    axum::extract::Query(query): axum::extract::Query<ProgramQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    if let Some(kind) = &query.kind
        && !rules::PROGRAM_KINDS.contains(&kind.as_str())
    {
        return Err(unprocessable(&format!(
            "unknown kind `{kind}` (kinds: {:?})",
            rules::PROGRAM_KINDS
        )));
    }
    let mut find = early_career_programs::Entity::find()
        .filter(early_career_programs::Column::DeletedAt.is_null());
    if let Some(kind) = &query.kind {
        find = find.filter(early_career_programs::Column::Kind.eq(kind.clone()));
    }
    let programs = find
        .order_by_asc(early_career_programs::Column::Name)
        .all(&ctx.db)
        .await?;

    let mut view = Vec::with_capacity(programs.len());
    for program in &programs {
        let placements = program_placements::Entity::find()
            .filter(program_placements::Column::ProgramPid.eq(program.pid))
            .filter(program_placements::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await?;
        view.push(serde_json::json!({
            "pid": program.pid,
            "name": program.name,
            "kind": program.kind,
            "level": program.level,
            "duration_months": program.duration_months,
            "min_off_the_job_hours": program.min_off_the_job_hours,
            "provider_ref": program.provider_ref,
            "placements": placement_rollup(&placements),
        }));
    }
    format::json(serde_json::json!({
        "note": CONVERSION_NOTE,
        "programs": view,
    }))
}

/// How the conversion rate is derived, echoed wherever it is reported.
const CONVERSION_NOTE: &str = "conversion rate = placements whose outcome is `converted` / placements that have \
     completed; a running placement has not had the chance to convert, so it is excluded \
     from the denominator (null until something completes)";

/// Placement counts + conversion rate for one programme.
fn placement_rollup(placements: &[program_placements::Model]) -> serde_json::Value {
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    for placement in placements {
        *by_status.entry(placement.status.clone()).or_default() += 1;
    }
    let completed_outcomes: Vec<String> = placements
        .iter()
        .filter(|p| p.status == "completed")
        .map(|p| p.outcome.clone())
        .collect();
    let conversion = rules::conversion_rate(&completed_outcomes).map_or(
        serde_json::Value::Null,
        |(converted, completed, ratio)| {
            serde_json::json!({
                "numerator": converted, "denominator": completed, "value": ratio,
            })
        },
    );
    serde_json::json!({
        "total": placements.len(),
        "by_status": by_status,
        "conversion_rate": conversion,
    })
}

/// `POST /api/early-career-programs/{pid}/placements` body.
#[derive(Debug, Deserialize)]
struct PlacementPayload {
    employee_pid: Uuid,
    #[serde(default)]
    supervisor_pid: Option<Uuid>,
    started_on: chrono::NaiveDate,
    #[serde(default)]
    ends_on: Option<chrono::NaiveDate>,
}

/// `POST /api/early-career-programs/{pid}/placements` — place an
/// apprentice, intern, or graduate on the programme (`offered`).
#[debug_handler]
async fn create_placement(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<PlacementPayload>,
) -> Result<Response> {
    let program = records::find_early_career_program(&ctx.db, records::parse_pid(&pid)?).await?;
    let employee = records::find_employee(&ctx.db, payload.employee_pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;

    let mut problems = Problems::new();
    if let Some(ends_on) = payload.ends_on
        && ends_on < payload.started_on
    {
        problems.push("ends_on cannot be before started_on");
    }
    if let Some(supervisor_pid) = payload.supervisor_pid
        && supervisor_pid == employee.pid
    {
        problems.push("a placement's supervisor must be someone else");
    }
    ensure_valid(&problems.into_vec())?;
    if let Some(supervisor_pid) = payload.supervisor_pid {
        records::find_employee(&ctx.db, supervisor_pid).await?;
    }

    let txn = ctx.db.begin().await?;
    let row = program_placements::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        program_pid: ActiveValue::set(program.pid),
        employee_pid: ActiveValue::set(employee.pid),
        supervisor_pid: ActiveValue::set(payload.supervisor_pid),
        started_on: ActiveValue::set(payload.started_on),
        ends_on: ActiveValue::set(payload.ends_on),
        status: ActiveValue::set("offered".to_string()),
        off_the_job_hours: ActiveValue::set(0),
        outcome: ActiveValue::set("pending".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "program_placement",
        row.pid,
        "created",
        caller.actor(),
        Some(serde_json::json!({ "program": program.name, "kind": program.kind })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "program_placement",
        "created",
        &row.pid.to_string(),
        &program.name,
        caller.actor(),
        Some(serde_json::json!({ "kind": program.kind })),
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef::of(row.pid))
}

/// `POST /api/program-placements/{pid}/hours` body.
#[derive(Debug, Deserialize)]
struct HoursPayload {
    /// Off-the-job training hours to add (positive).
    hours: i32,
}

/// `POST /api/program-placements/{pid}/hours` — log off-the-job
/// training hours against a placement. Only an `active` placement
/// accrues hours, and the total is capped at `i32` by a checked add
/// (an overflow is a `422`, never a panic).
#[debug_handler]
async fn log_hours(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<HoursPayload>,
) -> Result<Response> {
    let placement = records::find_program_placement(&ctx.db, records::parse_pid(&pid)?).await?;
    if payload.hours <= 0 {
        return Err(unprocessable("hours must be positive"));
    }
    if placement.status != "active" {
        return Err(unprocessable(&format!(
            "only an active placement accrues off-the-job hours (this one is `{}`)",
            placement.status
        )));
    }
    let total = placement
        .off_the_job_hours
        .checked_add(payload.hours)
        .ok_or_else(|| unprocessable("recorded hours would overflow"))?;

    let placement_pid = placement.pid;
    let mut active: program_placements::ActiveModel = placement.into();
    active.off_the_job_hours = ActiveValue::set(total);
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "program_placement",
        placement_pid,
        "hours_logged",
        caller.actor(),
        Some(serde_json::json!({ "added": payload.hours, "total": total })),
    )
    .await?;
    format::json(updated)
}

/// `POST /api/program-placements/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct PlacementStatusPayload {
    to: String,
    /// The outcome to record when completing or withdrawing.
    #[serde(default)]
    outcome: Option<String>,
}

/// `POST /api/program-placements/{pid}/status` — the placement
/// lifecycle (`offered → active → completed`, or withdraw).
///
/// **Completing an apprenticeship requires its off-the-job training
/// hours** ([`rules::may_complete_placement`]): the refusal names the
/// hours recorded and the hours required. Withdrawing forces the
/// `withdrawn` outcome, so a withdrawn placement can never be counted
/// as a conversion.
#[debug_handler]
async fn placement_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<PlacementStatusPayload>,
) -> Result<Response> {
    let placement = records::find_program_placement(&ctx.db, records::parse_pid(&pid)?).await?;
    let program = records::find_early_career_program(&ctx.db, placement.program_pid).await?;
    rules::placement_transition(&placement.status, &payload.to)
        .map_err(|reason| unprocessable(&reason))?;

    let mut problems = Problems::new();
    problems.token_opt(
        "outcome",
        rules::PLACEMENT_OUTCOMES,
        payload.outcome.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;

    if payload.to == "completed" {
        rules::may_complete_placement(
            &program.kind,
            placement.off_the_job_hours,
            program.min_off_the_job_hours,
        )
        .map_err(|reason| unprocessable(&reason))?;
    }

    let from = placement.status.clone();
    let placement_pid = placement.pid;
    let outcome = if payload.to == "withdrawn" {
        Some("withdrawn".to_string())
    } else {
        payload.outcome.clone()
    };

    let txn = ctx.db.begin().await?;
    let mut active: program_placements::ActiveModel = placement.into();
    active.status = ActiveValue::set(payload.to.clone());
    if let Some(outcome) = &outcome {
        active.outcome = ActiveValue::set(outcome.clone());
    }
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "program_placement",
        placement_pid,
        "status_changed",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to, "outcome": outcome })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "program_placement",
        &format!("placement_{}", payload.to),
        &placement_pid.to_string(),
        &program.name,
        caller.actor(),
        Some(serde_json::json!({ "outcome": outcome })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// `GET /api/employees/{pid}/placements` — one person's early-career
/// placements, with the off-the-job hours against the requirement.
#[debug_handler]
async fn list_employee_placements(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Read,
        &auth::employee_resource_attrs(&employee),
    )
    .map_err(record_rejection)?;

    let placements = program_placements::Entity::find()
        .filter(program_placements::Column::EmployeePid.eq(employee.pid))
        .filter(program_placements::Column::DeletedAt.is_null())
        .order_by_asc(program_placements::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut view = Vec::with_capacity(placements.len());
    for placement in &placements {
        let program = records::find_early_career_program(&ctx.db, placement.program_pid).await?;
        view.push(serde_json::json!({
            "pid": placement.pid,
            "program": { "pid": program.pid, "name": program.name, "kind": program.kind, "level": program.level },
            "status": placement.status,
            "outcome": placement.outcome,
            "started_on": placement.started_on,
            "ends_on": placement.ends_on,
            "supervisor_pid": placement.supervisor_pid,
            "off_the_job": {
                "hours": placement.off_the_job_hours,
                "required": program.min_off_the_job_hours,
                "met": rules::may_complete_placement(
                    &program.kind, placement.off_the_job_hours, program.min_off_the_job_hours,
                ).is_ok(),
            },
        }));
    }
    format::json(serde_json::json!({
        "employee_pid": employee.pid,
        "placements": view,
    }))
}

/// The talent routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        // Development plans (upskilling / reskilling)
        .add("/employees/{pid}/development-plans", post(create_plan))
        .add("/employees/{pid}/development-plans", get(list_plans))
        .add("/development-plans/{pid}/status", post(plan_status))
        .add("/development-plan-items/{pid}", put(update_item))
        // Talent pipelines
        .add("/talent-pipelines", post(create_pipeline))
        .add("/talent-pipelines", get(list_pipelines))
        .add("/talent-pipelines/{pid}", get(get_pipeline))
        .add("/talent-pipelines/{pid}/members", post(add_member))
        .add("/pipeline-members/{pid}/stage", post(move_member))
        // Early careers: apprenticeships / internships / graduate schemes
        .add("/early-career-programs", post(create_program))
        .add("/early-career-programs", get(list_programs))
        .add(
            "/early-career-programs/{pid}/placements",
            post(create_placement),
        )
        .add("/program-placements/{pid}/hours", post(log_hours))
        .add("/program-placements/{pid}/status", post(placement_status))
        .add("/employees/{pid}/placements", get(list_employee_placements))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stored plan item.
    fn an_item(skill: Uuid, target: i32, status: &str) -> development_plan_items::Model {
        development_plan_items::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            plan_pid: Uuid::new_v4(),
            skill_pid: skill,
            current_level: 2,
            target_level: target,
            method: "course".to_string(),
            course_ref: None,
            due_on: None,
            status: status.to_string(),
        }
    }

    /// A stored plan.
    fn a_plan(kind: &str) -> development_plans::Model {
        development_plans::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            employee_pid: Uuid::new_v4(),
            kind: kind.to_string(),
            target_job_title: None,
            target_department: None,
            rationale: None,
            status: "active".to_string(),
            started_on: None,
            target_on: None,
            deleted_at: None,
        }
    }

    /// A stored placement.
    fn a_placement(status: &str, outcome: &str) -> program_placements::Model {
        program_placements::Model {
            created_at: chrono::Utc::now().into(),
            updated_at: chrono::Utc::now().into(),
            id: 1,
            pid: Uuid::new_v4(),
            program_pid: Uuid::new_v4(),
            employee_pid: Uuid::new_v4(),
            supervisor_pid: None,
            started_on: chrono::NaiveDate::from_ymd_opt(2026, 1, 6).expect("date"),
            ends_on: None,
            status: status.to_string(),
            off_the_job_hours: 0,
            outcome: outcome.to_string(),
            deleted_at: None,
        }
    }

    /// The plan view reports declared and verified progress separately,
    /// so a claimed achievement with no proficiency behind it shows up.
    #[test]
    fn plan_view_separates_claimed_from_verified_progress() {
        let rust = Uuid::new_v4();
        let sql = Uuid::new_v4();
        let items = vec![
            an_item(rust, 4, "achieved"),
            an_item(sql, 4, "achieved"),
            an_item(Uuid::new_v4(), 3, "in_progress"),
        ];
        // Only the Rust target has actually been reached.
        let declared = BTreeMap::from([(rust, 4), (sql, 2)]);

        let view = plan_view(&a_plan("upskill"), &items, &declared);
        assert_eq!(view["declared_progress"]["numerator"], 2);
        assert_eq!(view["declared_progress"]["denominator"], 3);
        assert_eq!(
            view["verified_progress"]["numerator"], 1,
            "only the reached target counts as verified"
        );
        assert_eq!(view["verified_progress"]["denominator"], 3);
    }

    /// The programme rollup counts by status and divides the conversion
    /// rate by completed placements only.
    #[test]
    fn placement_rollup_divides_by_completed() {
        let placements = vec![
            a_placement("completed", "converted"),
            a_placement("completed", "not_converted"),
            a_placement("active", "pending"),
        ];
        let rollup = placement_rollup(&placements);
        assert_eq!(rollup["total"], 3);
        assert_eq!(rollup["by_status"]["completed"], 2);
        assert_eq!(rollup["by_status"]["active"], 1);
        assert_eq!(rollup["conversion_rate"]["numerator"], 1);
        assert_eq!(
            rollup["conversion_rate"]["denominator"], 2,
            "the running placement is not in the denominator"
        );

        // Nothing completed yet ⇒ no rate at all, rather than 0%.
        let running = vec![a_placement("active", "pending")];
        assert!(placement_rollup(&running)["conversion_rate"].is_null());
    }

    /// A plan with no items reports 0 of 0 rather than dividing.
    #[test]
    fn empty_plan_reports_zero_of_zero() {
        let view = plan_view(&a_plan("reskill"), &[], &BTreeMap::new());
        assert_eq!(view["declared_progress"]["denominator"], 0);
        assert_eq!(view["verified_progress"]["numerator"], 0);
    }
}
