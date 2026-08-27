//! Talent development (WPM-R10–R12): review cycles / reviews / goals
//! / feedback, training enrollments over the course registry, and
//! succession plans with the gap report. Review content is
//! high-sensitivity: reads are audited (WPM-D7).

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{
    feedback_entries, goals, review_cycles, reviews, succession_candidates, succession_plans,
    training_enrollments,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{lifecycle, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/review-cycles` body.
#[derive(Debug, Deserialize)]
struct CyclePayload {
    name: String,
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
}

/// `POST /api/review-cycles/{pid}/reviews` body.
#[derive(Debug, Deserialize)]
struct ReviewPayload {
    employee_pid: Uuid,
    reviewer_ref: String,
}

/// `PUT /api/reviews/{pid}` body — author edits (draft only).
#[derive(Debug, Deserialize)]
struct ReviewUpdate {
    #[serde(default)]
    rating: Option<i32>,
    #[serde(default)]
    content: Option<String>,
}

/// `POST /api/reviews/{pid}/status` body.
#[derive(Debug, Deserialize)]
struct ReviewStatusPayload {
    to: String,
}

/// `POST /api/reviews/{pid}/goals` body.
#[derive(Debug, Deserialize)]
struct GoalPayload {
    title: String,
    #[serde(default = "default_weight")]
    weight_percent: i32,
}

/// `PUT /api/goals/{pid}` body.
#[derive(Debug, Deserialize)]
struct GoalUpdate {
    status: String,
}

/// `POST /api/reviews/{pid}/feedback` body.
#[derive(Debug, Deserialize)]
struct FeedbackPayload {
    author_ref: String,
    content: String,
}

/// `POST /api/employees/{pid}/training-enrollments` body.
#[derive(Debug, Deserialize)]
struct TrainingPayload {
    course_ref: String,
}

/// `PUT /api/training-enrollments/{pid}` body.
#[derive(Debug, Deserialize)]
struct TrainingUpdate {
    status: String,
    #[serde(default)]
    completed_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    certificate_expires_on: Option<chrono::NaiveDate>,
}

/// `POST /api/succession-plans` body.
#[derive(Debug, Deserialize)]
struct SuccessionPayload {
    role_title: String,
    department: String,
    criticality: i32,
    #[serde(default)]
    incumbent_pid: Option<Uuid>,
    /// How likely the incumbent is to leave: `low` | `medium` | `high`.
    /// Together with `criticality` this is what makes a role a single
    /// point of failure (`/api/workforce-intelligence/succession`).
    #[serde(default)]
    risk_of_loss: Option<String>,
    /// When the role is expected to fall vacant, when that is known.
    #[serde(default)]
    vacancy_expected_on: Option<chrono::NaiveDate>,
}

/// `PUT /api/succession-plans/{pid}` body — restate the planning
/// judgements as they change. Every field is optional.
#[derive(Debug, Deserialize)]
struct SuccessionUpdate {
    #[serde(default)]
    criticality: Option<i32>,
    #[serde(default)]
    risk_of_loss: Option<String>,
    #[serde(default)]
    vacancy_expected_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    incumbent_pid: Option<Uuid>,
}

/// `PUT /api/succession-candidates/{pid}` body — readiness moves as a
/// successor develops (or regresses).
#[derive(Debug, Deserialize)]
struct SuccessionCandidateUpdate {
    #[serde(default)]
    readiness: Option<String>,
    #[serde(default)]
    rank: Option<i32>,
}

/// `POST /api/succession-plans/{pid}/candidates` body.
#[derive(Debug, Deserialize)]
struct SuccessionCandidatePayload {
    employee_pid: Uuid,
    readiness: String,
    #[serde(default = "default_rank")]
    rank: i32,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

const fn default_weight() -> i32 {
    100
}
const fn default_rank() -> i32 {
    1
}

/// `POST /api/review-cycles`.
#[debug_handler]
async fn create_cycle(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CyclePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.period_end < payload.period_start {
        problems.push("period_end is before period_start".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = review_cycles::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        period_start: ActiveValue::set(payload.period_start),
        period_end: ActiveValue::set(payload.period_end),
        status: ActiveValue::set("open".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "review_cycle",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/review-cycles`.
#[debug_handler]
async fn list_cycles(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = review_cycles::Entity::find()
        .filter(review_cycles::Column::DeletedAt.is_null())
        .order_by_asc(review_cycles::Column::Id)
        .limit(200)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/review-cycles/{pid}/reviews` — open a draft review.
#[debug_handler]
async fn create_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ReviewPayload>,
) -> Result<Response> {
    let cycle = records::find_review_cycle(&ctx.db, records::parse_pid(&pid)?).await?;
    if cycle.status != "open" {
        return Err(unprocessable("review cycle is closed"));
    }
    let employee = records::find_employee(&ctx.db, payload.employee_pid).await?;
    let mut problems = Problems::new();
    problems.require_ref(
        "reviewer_ref",
        entity_ref::EntityType::Worker,
        &payload.reviewer_ref,
    );
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = reviews::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        cycle_pid: ActiveValue::set(cycle.pid),
        employee_pid: ActiveValue::set(employee.pid),
        reviewer_ref: ActiveValue::set(payload.reviewer_ref.clone()),
        status: ActiveValue::set("draft".to_string()),
        rating: ActiveValue::set(None),
        content: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "review", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/employees/{pid}/reviews` — an employee's reviews.
/// Draft/submitted/calibrated content is redacted; `shared` reviews
/// carry content, and the content read is audited (WPM-R10).
#[debug_handler]
async fn list_reviews(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = reviews::Entity::find()
        .filter(reviews::Column::EmployeePid.eq(employee.pid))
        .filter(reviews::Column::DeletedAt.is_null())
        .order_by_asc(reviews::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut any_shared = false;
    let rows: Vec<reviews::Model> = rows
        .into_iter()
        .map(|mut review| {
            if review.status == "shared" {
                any_shared = true;
            } else {
                review.content = None;
                review.rating = None;
            }
            review
        })
        .collect();
    if any_shared {
        Audit::record(
            &ctx.db,
            "employee",
            employee.pid,
            "review_content_read",
            caller.actor(),
            None,
        )
        .await?;
    }
    format::json(rows)
}

/// `PUT /api/reviews/{pid}` — author edits, drafts only.
#[debug_handler]
async fn update_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ReviewUpdate>,
) -> Result<Response> {
    let review = records::find_review(&ctx.db, records::parse_pid(&pid)?).await?;
    if review.status != "draft" {
        return Err(unprocessable(&format!(
            "review is {} — only drafts are editable",
            review.status
        )));
    }
    let mut problems = Problems::new();
    if let Some(rating) = payload.rating
        && !(1..=5).contains(&rating)
    {
        problems.push(format!("rating {rating} out of range 1-5"));
    }
    problems.cap_opt("content", payload.content.as_deref());
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let mut active: reviews::ActiveModel = review.into();
    if let Some(rating) = payload.rating {
        active.rating = ActiveValue::set(Some(rating));
    }
    if let Some(content) = payload.content {
        active.content = ActiveValue::set(Some(content));
    }
    let row = active.update(&txn).await?;
    Audit::record(&txn, "review", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/reviews/{pid}/status` — one review transition
/// (`draft → submitted → calibrated → shared`).
#[debug_handler]
async fn review_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ReviewStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::REVIEW_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let review = records::find_review(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("review", lifecycle::REVIEW, &review.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let from = review.status.clone();
    let mut active: reviews::ActiveModel = review.into();
    active.status = ActiveValue::set(payload.to.clone());
    let row = active.update(&txn).await?;
    let kind = match payload.to.as_str() {
        "submitted" => "review_submitted",
        "shared" => "review_shared",
        _ => "review_status_changed",
    };
    Audit::record(
        &txn,
        "review",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(
        &txn,
        "review",
        kind,
        &row.pid.to_string(),
        "",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `POST /api/reviews/{pid}/goals`.
#[debug_handler]
async fn create_goal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<GoalPayload>,
) -> Result<Response> {
    let review = records::find_review(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_text("title", &payload.title);
    if !(1..=100).contains(&payload.weight_percent) {
        problems.push(format!(
            "weight_percent {} out of range 1-100",
            payload.weight_percent
        ));
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = goals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        review_pid: ActiveValue::set(review.pid),
        title: ActiveValue::set(payload.title.clone()),
        weight_percent: ActiveValue::set(payload.weight_percent),
        status: ActiveValue::set("open".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "goal", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `PUT /api/goals/{pid}` — set the status.
#[debug_handler]
async fn update_goal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<GoalUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("status", tokens::GOAL_STATUSES, &payload.status);
    ensure_valid(&problems.into_vec())?;
    let pid = records::parse_pid(&pid)?;
    let goal = goals::Entity::find()
        .filter(goals::Column::Pid.eq(pid))
        .filter(goals::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let txn = ctx.db.begin().await?;
    let mut active: goals::ActiveModel = goal.into();
    active.status = ActiveValue::set(payload.status.clone());
    let row = active.update(&txn).await?;
    Audit::record(&txn, "goal", row.pid, "updated", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/reviews/{pid}/goals` (+ feedback list alongside).
#[debug_handler]
async fn review_detail(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let review = records::find_review(&ctx.db, records::parse_pid(&pid)?).await?;
    let goal_rows = goals::Entity::find()
        .filter(goals::Column::ReviewPid.eq(review.pid))
        .filter(goals::Column::DeletedAt.is_null())
        .order_by_asc(goals::Column::Id)
        .all(&ctx.db)
        .await?;
    let feedback = feedback_entries::Entity::find()
        .filter(feedback_entries::Column::ReviewPid.eq(review.pid))
        .filter(feedback_entries::Column::DeletedAt.is_null())
        .order_by_asc(feedback_entries::Column::Id)
        .all(&ctx.db)
        .await?;
    // Reading a review with content is a sensitive read (WPM-D7).
    if review.content.is_some() {
        Audit::record(
            &ctx.db,
            "review",
            review.pid,
            "review_content_read",
            caller.actor(),
            None,
        )
        .await?;
    }
    format::json(serde_json::json!({
        "review": review, "goals": goal_rows, "feedback": feedback,
    }))
}

/// `POST /api/reviews/{pid}/feedback`.
#[debug_handler]
async fn create_feedback(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<FeedbackPayload>,
) -> Result<Response> {
    let review = records::find_review(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    problems.require_ref(
        "author_ref",
        entity_ref::EntityType::Worker,
        &payload.author_ref,
    );
    problems.require_text("content", &payload.content);
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = feedback_entries::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        review_pid: ActiveValue::set(review.pid),
        author_ref: ActiveValue::set(payload.author_ref.clone()),
        content: ActiveValue::set(payload.content.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "feedback_entry",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `POST /api/employees/{pid}/training-enrollments` — enrol against a
/// `course:` / `courseinstance:` URN (WPM-D10).
#[debug_handler]
async fn create_training(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TrainingPayload>,
) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    // Accept either course-family entity type.
    let parsed: std::result::Result<entity_ref::EntityRef, _> = payload.course_ref.parse();
    match parsed {
        Ok(r)
            if r.entity_type == entity_ref::EntityType::Course
                || r.entity_type == entity_ref::EntityType::CourseInstance => {}
        _ => {
            return Err(unprocessable(
                "course_ref must be a course: or courseinstance: URN",
            ));
        }
    }
    let txn = ctx.db.begin().await?;
    let row = training_enrollments::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        employee_pid: ActiveValue::set(employee.pid),
        course_ref: ActiveValue::set(payload.course_ref.clone()),
        status: ActiveValue::set("enrolled".to_string()),
        completed_on: ActiveValue::set(None),
        certificate_expires_on: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "training_enrollment",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/employees/{pid}/training-enrollments`.
#[debug_handler]
async fn list_training(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let employee = records::find_employee(&ctx.db, records::parse_pid(&pid)?).await?;
    let rows = training_enrollments::Entity::find()
        .filter(training_enrollments::Column::EmployeePid.eq(employee.pid))
        .filter(training_enrollments::Column::DeletedAt.is_null())
        .order_by_asc(training_enrollments::Column::Id)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `PUT /api/training-enrollments/{pid}` — progress the enrolment.
#[debug_handler]
async fn update_training(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<TrainingUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("status", tokens::TRAINING_STATUSES, &payload.status);
    ensure_valid(&problems.into_vec())?;
    let enrollment = records::find_training_enrollment(&ctx.db, records::parse_pid(&pid)?).await?;
    let txn = ctx.db.begin().await?;
    let mut active: training_enrollments::ActiveModel = enrollment.into();
    active.status = ActiveValue::set(payload.status.clone());
    if let Some(done) = payload.completed_on {
        active.completed_on = ActiveValue::set(Some(done));
    }
    if let Some(expiry) = payload.certificate_expires_on {
        active.certificate_expires_on = ActiveValue::set(Some(expiry));
    }
    let row = active.update(&txn).await?;
    let kind = if payload.status == "completed" {
        "training_completed"
    } else {
        "updated"
    };
    Audit::record(
        &txn,
        "training_enrollment",
        row.pid,
        kind,
        caller.actor(),
        None,
    )
    .await?;
    streaming::emit_on(
        &txn,
        "training_enrollment",
        kind,
        &row.pid.to_string(),
        &row.course_ref,
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/training/expiring?within_days=` — certificates expiring
/// soon (WPM-R11).
#[derive(Debug, Deserialize)]
struct ExpiringParams {
    #[serde(default = "default_within")]
    within_days: i64,
}

const fn default_within() -> i64 {
    90
}

#[debug_handler]
async fn expiring_training(
    State(ctx): State<AppContext>,
    Query(params): Query<ExpiringParams>,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let horizon = today + chrono::Duration::days(params.within_days.clamp(0, 3650));
    let rows = training_enrollments::Entity::find()
        .filter(training_enrollments::Column::DeletedAt.is_null())
        .filter(training_enrollments::Column::CertificateExpiresOn.is_not_null())
        .filter(training_enrollments::Column::CertificateExpiresOn.lte(horizon))
        .order_by_asc(training_enrollments::Column::CertificateExpiresOn)
        .limit(500)
        .all(&ctx.db)
        .await?;
    format::json(serde_json::json!({ "as_of": today, "horizon": horizon, "expiring": rows }))
}

/// `POST /api/succession-plans`.
#[debug_handler]
async fn create_succession(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SuccessionPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("role_title", &payload.role_title);
    problems.require_text("department", &payload.department);
    if !(1..=5).contains(&payload.criticality) {
        problems.push(format!(
            "criticality {} out of range 1-5",
            payload.criticality
        ));
    }
    problems.token_opt(
        "risk_of_loss",
        crate::rules::talent::RISK_OF_LOSS,
        payload.risk_of_loss.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;
    if let Some(incumbent) = payload.incumbent_pid {
        records::find_employee(&ctx.db, incumbent).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = succession_plans::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        role_title: ActiveValue::set(payload.role_title.clone()),
        department: ActiveValue::set(payload.department.clone()),
        criticality: ActiveValue::set(payload.criticality),
        incumbent_pid: ActiveValue::set(payload.incumbent_pid),
        risk_of_loss: ActiveValue::set(payload.risk_of_loss.clone()),
        vacancy_expected_on: ActiveValue::set(payload.vacancy_expected_on),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "succession_plan",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/succession-plans` — plans + candidates; the read is
/// audited (high-sensitivity, WPM-R12).
#[debug_handler]
async fn list_succession(State(ctx): State<AppContext>, caller: MaybeAuthUser) -> Result<Response> {
    let plans = succession_plans::Entity::find()
        .filter(succession_plans::Column::DeletedAt.is_null())
        .order_by_asc(succession_plans::Column::Id)
        .limit(500)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(plans.len());
    for plan in plans {
        let candidates = succession_candidates::Entity::find()
            .filter(succession_candidates::Column::PlanPid.eq(plan.pid))
            .filter(succession_candidates::Column::DeletedAt.is_null())
            .order_by_asc(succession_candidates::Column::Rank)
            .all(&ctx.db)
            .await?;
        out.push(serde_json::json!({ "plan": plan, "candidates": candidates }));
    }
    if let Some(actor) = caller.actor() {
        Audit::record(
            &ctx.db,
            "succession_plan",
            Uuid::nil(),
            "succession_read",
            Some(actor),
            None,
        )
        .await?;
    }
    format::json(out)
}

/// `POST /api/succession-plans/{pid}/candidates`.
#[debug_handler]
async fn add_succession_candidate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<SuccessionCandidatePayload>,
) -> Result<Response> {
    let plan = records::find_succession_plan(&ctx.db, records::parse_pid(&pid)?).await?;
    let employee = records::find_employee(&ctx.db, payload.employee_pid).await?;
    let mut problems = Problems::new();
    problems.require_token("readiness", tokens::READINESS, &payload.readiness);
    if payload.rank < 1 {
        problems.push("rank must be at least 1".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let row = succession_candidates::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        employee_pid: ActiveValue::set(employee.pid),
        readiness: ActiveValue::set(payload.readiness.clone()),
        rank: ActiveValue::set(payload.rank),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(
        &txn,
        "succession_candidate",
        row.pid,
        "created",
        caller.actor(),
        None,
    )
    .await?;
    txn.commit().await?;
    format::json(PidRef {
        pid: row.pid.to_string(),
    })
}

/// `GET /api/succession-plans/gaps` — critical roles (criticality ≥ 4)
/// with no `ready_now` candidate (WPM-R12).
#[debug_handler]
async fn succession_gaps(State(ctx): State<AppContext>) -> Result<Response> {
    let plans = succession_plans::Entity::find()
        .filter(succession_plans::Column::DeletedAt.is_null())
        .filter(succession_plans::Column::Criticality.gte(4))
        .all(&ctx.db)
        .await?;
    let mut gaps = Vec::new();
    for plan in plans {
        let ready_now = succession_candidates::Entity::find()
            .filter(succession_candidates::Column::PlanPid.eq(plan.pid))
            .filter(succession_candidates::Column::DeletedAt.is_null())
            .filter(succession_candidates::Column::Readiness.eq("ready_now"))
            .count(&ctx.db)
            .await?;
        if ready_now == 0 {
            gaps.push(plan);
        }
    }
    format::json(serde_json::json!({ "gaps": gaps }))
}

/// `PUT /api/succession-plans/{pid}` — restate a plan's criticality,
/// risk of loss, expected vacancy date, or incumbent as the picture
/// changes. Succession judgements go stale faster than anything else in
/// WPM, so they must be updatable without deleting the plan and its
/// bench.
#[debug_handler]
async fn update_succession(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<SuccessionUpdate>,
) -> Result<Response> {
    let plan = records::find_succession_plan(&ctx.db, records::parse_pid(&pid)?).await?;
    let mut problems = Problems::new();
    if let Some(criticality) = payload.criticality
        && !(1..=5).contains(&criticality)
    {
        problems.push(format!("criticality {criticality} out of range 1-5"));
    }
    problems.token_opt(
        "risk_of_loss",
        crate::rules::talent::RISK_OF_LOSS,
        payload.risk_of_loss.as_deref(),
    );
    ensure_valid(&problems.into_vec())?;
    if let Some(incumbent) = payload.incumbent_pid {
        records::find_employee(&ctx.db, incumbent).await?;
    }

    let plan_pid = plan.pid;
    let before = serde_json::json!({
        "criticality": plan.criticality,
        "risk_of_loss": plan.risk_of_loss,
        "vacancy_expected_on": plan.vacancy_expected_on,
        "incumbent_pid": plan.incumbent_pid,
    });
    let txn = ctx.db.begin().await?;
    let mut active: succession_plans::ActiveModel = plan.into();
    if let Some(criticality) = payload.criticality {
        active.criticality = ActiveValue::set(criticality);
    }
    if payload.risk_of_loss.is_some() {
        active.risk_of_loss = ActiveValue::set(payload.risk_of_loss.clone());
    }
    if payload.vacancy_expected_on.is_some() {
        active.vacancy_expected_on = ActiveValue::set(payload.vacancy_expected_on);
    }
    if payload.incumbent_pid.is_some() {
        active.incumbent_pid = ActiveValue::set(payload.incumbent_pid);
    }
    let updated = active.update(&txn).await?;
    Audit::record(
        &txn,
        "succession_plan",
        plan_pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "before": before })),
    )
    .await?;
    txn.commit().await?;
    format::json(updated)
}

/// `PUT /api/succession-candidates/{pid}` — move a successor's
/// readiness or rank. Readiness may go **down** as well as up: a bench
/// that can only improve on paper would overstate the organisation's
/// cover.
#[debug_handler]
async fn update_succession_candidate(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<SuccessionCandidateUpdate>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.token_opt("readiness", tokens::READINESS, payload.readiness.as_deref());
    if payload.rank.is_some_and(|r| r < 1) {
        problems.push("rank must be 1 or greater");
    }
    ensure_valid(&problems.into_vec())?;

    let candidate = succession_candidates::Entity::find()
        .filter(succession_candidates::Column::Pid.eq(records::parse_pid(&pid)?))
        .filter(succession_candidates::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or(Error::NotFound)?;
    let candidate_pid = candidate.pid;
    let from = candidate.readiness.clone();
    let mut active: succession_candidates::ActiveModel = candidate.into();
    if payload.readiness.is_some() {
        active.readiness = ActiveValue::set(payload.readiness.clone().unwrap_or(from.clone()));
    }
    if let Some(rank) = payload.rank {
        active.rank = ActiveValue::set(rank);
    }
    let updated = active.update(&ctx.db).await?;
    Audit::record(
        &ctx.db,
        "succession_candidate",
        candidate_pid,
        "updated",
        caller.actor(),
        Some(serde_json::json!({ "readiness_from": from, "readiness_to": payload.readiness })),
    )
    .await?;
    format::json(updated)
}

/// The development routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/review-cycles", post(create_cycle))
        .add("/review-cycles", get(list_cycles))
        .add("/review-cycles/{pid}/reviews", post(create_review))
        .add("/employees/{pid}/reviews", get(list_reviews))
        .add("/reviews/{pid}", put(update_review))
        .add("/reviews/{pid}", get(review_detail))
        .add("/reviews/{pid}/status", post(review_status))
        .add("/reviews/{pid}/goals", post(create_goal))
        .add("/goals/{pid}", put(update_goal))
        .add("/reviews/{pid}/feedback", post(create_feedback))
        .add(
            "/employees/{pid}/training-enrollments",
            post(create_training),
        )
        .add("/employees/{pid}/training-enrollments", get(list_training))
        .add("/training-enrollments/{pid}", put(update_training))
        .add("/training/expiring", get(expiring_training))
        .add("/succession-plans", post(create_succession))
        .add("/succession-plans", get(list_succession))
        .add("/succession-plans/gaps", get(succession_gaps))
        .add("/succession-plans/{pid}", put(update_succession))
        .add(
            "/succession-plans/{pid}/candidates",
            post(add_succession_candidate),
        )
        .add(
            "/succession-candidates/{pid}",
            put(update_succession_candidate),
        )
}
