//! Collaborative review, the notification inbox, and assignee
//! management.
//!
//! - **Collaborative review** — delegate one idea / proposal / plan to
//!   the right internal or external expert, track whether they accepted,
//!   collect their verdict, and read the consensus
//!   ([`crate::collaboration`] holds the state machine and the
//!   aggregation).
//! - **Assignees** — assign or unassign a task, and read the open
//!   workload per assignee (including the unassigned pile).
//! - **Notifications** — the in-app inbox written by automations and by
//!   the scheduled-action sweep. In-app only: this service sends no
//!   email and no push.
//!
//! Reviewers are named by `EntityRef` URN (`person:` / `worker:` /
//! `organization:`), never by raw email — an outside expert is still a
//! record in the person registry, so no contact detail is duplicated
//! here. `reviewer_scope` records the internal/external distinction
//! explicitly, because sharing something with an outsider is a
//! disclosure decision and must never be inferred.
//!
//! Every mutation writes an `audit_logs` row.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::governance::valid_ref;
use crate::auth::MaybeAuthUser;
use crate::collaboration as rules;
use crate::models::_entities::{ideas, notifications, plans, proposals, reviews, tasks};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::capabilities as cap_models;
use crate::validation::MAX_TEXT_LEN;

/// Highest number of rows any list endpoint here returns.
pub const LIST_CAP: u64 = 200;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// A reviewer / recipient reference: an `EntityRef` over the family's
/// person-bearing services.
fn valid_actor_ref(value: &str) -> bool {
    valid_ref(value, &["person", "worker", "organization"])
}

/// Confirm the review subject actually exists and is live, so an
/// invitation can never point at nothing.
async fn subject_exists(ctx: &AppContext, subject_kind: &str, subject_pid: Uuid) -> Result<bool> {
    let found = match subject_kind {
        "idea" => ideas::Entity::find()
            .filter(ideas::Column::Pid.eq(subject_pid))
            .filter(ideas::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .is_some(),
        "proposal" => proposals::Entity::find()
            .filter(proposals::Column::Pid.eq(subject_pid))
            .filter(proposals::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .is_some(),
        "plan" => plans::Entity::find()
            .filter(plans::Column::Pid.eq(subject_pid))
            .filter(plans::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .is_some(),
        _ => false,
    };
    Ok(found)
}

/// `POST /api/reviews` body — delegate a subject to one expert.
#[derive(Debug, Deserialize)]
struct InvitePayload {
    subject_kind: String,
    subject_pid: Uuid,
    reviewer_ref: String,
    #[serde(default)]
    reviewer_scope: Option<String>,
    #[serde(default)]
    expertise: Option<String>,
    #[serde(default)]
    due_on: Option<chrono::NaiveDate>,
}

/// `POST /api/reviews` — invite one expert to assess one subject.
///
/// A second live invitation for the same (subject, reviewer) is
/// refused rather than stacked: re-delegating the same item to the same
/// person is almost always a mistake, and the partial unique index
/// would reject it anyway.
/// Validate an invitation payload, reporting every problem at once.
fn invite_problems(payload: &InvitePayload, scope: &str) -> Vec<String> {
    let mut problems = Vec::new();
    if !rules::is_token(rules::REVIEW_SUBJECT_KINDS, &payload.subject_kind) {
        problems.push(format!(
            "subject_kind must be one of {:?}",
            rules::REVIEW_SUBJECT_KINDS
        ));
    }
    if !valid_actor_ref(&payload.reviewer_ref) {
        problems.push(
            "reviewer_ref must be a person:/worker:/organization: URN (never a raw email)"
                .to_string(),
        );
    }
    if !rules::is_token(rules::REVIEWER_SCOPES, scope) {
        problems.push(format!(
            "reviewer_scope must be one of {:?}",
            rules::REVIEWER_SCOPES
        ));
    }
    if let Some(expertise) = payload.expertise.as_deref()
        && (expertise.trim().is_empty() || expertise.len() > MAX_TEXT_LEN)
    {
        problems.push(format!(
            "expertise, when set, must be non-blank and at most {MAX_TEXT_LEN} characters"
        ));
    }
    problems
}

#[debug_handler]
async fn invite_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<InvitePayload>,
) -> Result<Response> {
    let scope = payload
        .reviewer_scope
        .clone()
        .unwrap_or_else(|| "internal".to_string());
    let problems = invite_problems(&payload, &scope);
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    if !subject_exists(&ctx, &payload.subject_kind, payload.subject_pid).await? {
        return Err(unprocessable(&format!(
            "no live {} with pid {}",
            payload.subject_kind, payload.subject_pid
        )));
    }
    let live = cap_models::reviews_for_subject(&ctx.db, &payload.subject_kind, payload.subject_pid)
        .await?
        .into_iter()
        .any(|r| {
            r.reviewer_ref == payload.reviewer_ref
                && rules::LIVE_REVIEW_STATUSES.contains(&r.status.as_str())
        });
    if live {
        return Err(unprocessable(
            "this reviewer already has a live invitation for this subject",
        ));
    }
    let row = reviews::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        subject_kind: ActiveValue::set(payload.subject_kind.clone()),
        subject_pid: ActiveValue::set(payload.subject_pid),
        reviewer_ref: ActiveValue::set(payload.reviewer_ref.clone()),
        reviewer_scope: ActiveValue::set(scope.clone()),
        expertise: ActiveValue::set(payload.expertise.clone()),
        status: ActiveValue::set("invited".to_string()),
        due_on: ActiveValue::set(payload.due_on),
        score: ActiveValue::set(None),
        recommendation: ActiveValue::set(None),
        comment: ActiveValue::set(None),
        invited_by: ActiveValue::set(caller.actor().map(std::string::ToString::to_string)),
        responded_at: ActiveValue::set(None),
        submitted_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        row.pid,
        "review_invited",
        caller.actor(),
        Some(serde_json::json!({
            "subject_kind": payload.subject_kind,
            "subject_pid": payload.subject_pid.to_string(),
            "reviewer_ref": payload.reviewer_ref,
            "reviewer_scope": scope,
        })),
    )
    .await
    .ok();
    // The reviewer learns they have been asked.
    cap_models::notify(
        &ctx.db,
        &payload.reviewer_ref,
        &payload.subject_kind,
        payload.subject_pid,
        "review_invited",
        &format!(
            "You have been asked to review a {}{}",
            payload.subject_kind,
            payload
                .due_on
                .map(|d| format!(" by {d}"))
                .unwrap_or_default()
        ),
    )
    .await
    .ok();
    format::json(row)
}

/// `GET /api/reviews` filter.
#[derive(Debug, Deserialize)]
struct ReviewParams {
    #[serde(default)]
    subject_kind: Option<String>,
    #[serde(default)]
    subject_pid: Option<Uuid>,
    #[serde(default)]
    reviewer: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

/// `GET /api/reviews?subject_kind=&subject_pid=&reviewer=&status=` —
/// the delegation list, newest first (cap [`LIST_CAP`]).
#[debug_handler]
async fn list_reviews(
    State(ctx): State<AppContext>,
    Query(params): Query<ReviewParams>,
) -> Result<Response> {
    let mut query = reviews::Entity::find().filter(reviews::Column::DeletedAt.is_null());
    if let Some(kind) = params.subject_kind.as_deref() {
        query = query.filter(reviews::Column::SubjectKind.eq(kind));
    }
    if let Some(pid) = params.subject_pid {
        query = query.filter(reviews::Column::SubjectPid.eq(pid));
    }
    if let Some(reviewer) = params.reviewer.as_deref() {
        query = query.filter(reviews::Column::ReviewerRef.eq(reviewer));
    }
    if let Some(status) = params.status.as_deref() {
        query = query.filter(reviews::Column::Status.eq(status));
    }
    let rows = query
        .order_by_desc(reviews::Column::Id)
        .limit(LIST_CAP)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(rows)
}

/// `POST /api/reviews/{pid}/respond` body.
#[derive(Debug, Deserialize)]
struct RespondPayload {
    /// `accept` or `decline`.
    response: String,
}

/// `POST /api/reviews/{pid}/respond` — the reviewer accepts or
/// declines the invitation.
#[debug_handler]
async fn respond_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<RespondPayload>,
) -> Result<Response> {
    let target = match payload.response.trim() {
        "accept" => "accepted",
        "decline" => "declined",
        other => {
            return Err(unprocessable(&format!(
                "response must be `accept` or `decline`, got `{other}`"
            )));
        }
    };
    let review =
        cap_models::find_review(&ctx.db, crate::models::governance::parse_pid(&pid)?).await?;
    rules::review_transition(&review.status, target).map_err(|e| unprocessable(&e))?;
    let review_pid = review.pid;
    let mut active: reviews::ActiveModel = review.into();
    active.status = ActiveValue::set(target.to_string());
    active.responded_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        review_pid,
        &format!("review_{target}"),
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(row)
}

/// `POST /api/reviews/{pid}/submit` body — the expert's verdict.
#[derive(Debug, Deserialize)]
struct VerdictPayload {
    #[serde(default)]
    score: Option<i32>,
    recommendation: String,
    #[serde(default)]
    comment: Option<String>,
}

/// `POST /api/reviews/{pid}/submit` — record the verdict. Only a
/// reviewer who accepted may submit, so an unanswered invitation never
/// becomes evidence. Submitting fires any `review_submitted`
/// automations.
#[debug_handler]
async fn submit_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<VerdictPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if !rules::is_token(rules::RECOMMENDATIONS, payload.recommendation.trim()) {
        problems.push(format!(
            "recommendation must be one of {:?}",
            rules::RECOMMENDATIONS
        ));
    }
    if let Some(score) = payload.score
        && !rules::valid_review_score(score)
    {
        problems.push("score, when given, must be between 0 and 100".to_string());
    }
    if let Some(comment) = payload.comment.as_deref()
        && comment.len() > MAX_TEXT_LEN
    {
        problems.push(format!("comment exceeds {MAX_TEXT_LEN} characters"));
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let review =
        cap_models::find_review(&ctx.db, crate::models::governance::parse_pid(&pid)?).await?;
    rules::review_transition(&review.status, "submitted").map_err(|e| unprocessable(&e))?;
    let (review_pid, subject_kind, subject_pid, invited_by) = (
        review.pid,
        review.subject_kind.clone(),
        review.subject_pid,
        review.invited_by.clone(),
    );
    let mut active: reviews::ActiveModel = review.into();
    active.status = ActiveValue::set("submitted".to_string());
    active.score = ActiveValue::set(payload.score);
    active.recommendation = ActiveValue::set(Some(payload.recommendation.trim().to_string()));
    active.comment = ActiveValue::set(payload.comment.clone());
    active.submitted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        review_pid,
        "review_submitted",
        caller.actor(),
        Some(serde_json::json!({
            "recommendation": row.recommendation,
            "score": row.score,
        })),
    )
    .await
    .ok();
    // Tell whoever delegated it that the verdict has landed.
    if let Some(invited_by) = invited_by.as_deref()
        && valid_actor_ref(invited_by)
    {
        cap_models::notify(
            &ctx.db,
            invited_by,
            &subject_kind,
            subject_pid,
            "review_submitted",
            &format!(
                "A review you delegated came back: {}",
                row.recommendation.as_deref().unwrap_or("submitted")
            ),
        )
        .await
        .ok();
    }
    // A review verdict is a trigger like a board move is.
    if subject_kind == "plan" {
        super::automation::fire(
            &ctx,
            &crate::automation::TriggerFact {
                kind: "review_submitted".to_string(),
                plan_pid: subject_pid,
                from_status: None,
                to_status: None,
            },
            "review",
            review_pid,
            caller.actor(),
        )
        .await;
    }
    format::json(row)
}

/// `DELETE /api/reviews/{pid}` — withdraw an invitation (soft delete).
/// A submitted verdict cannot be withdrawn: the record of what an
/// expert said stands.
#[debug_handler]
async fn withdraw_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let review =
        cap_models::find_review(&ctx.db, crate::models::governance::parse_pid(&pid)?).await?;
    rules::review_transition(&review.status, "withdrawn").map_err(|e| unprocessable(&e))?;
    let review_pid = review.pid;
    let mut active: reviews::ActiveModel = review.into();
    active.status = ActiveValue::set("withdrawn".to_string());
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        review_pid,
        "review_withdrawn",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::empty_json()
}

/// `GET /api/reviews/consensus` filter.
#[derive(Debug, Deserialize)]
struct ConsensusParams {
    subject_kind: String,
    subject_pid: Uuid,
}

/// `GET /api/reviews/consensus?subject_kind=&subject_pid=` — the
/// aggregate verdict on one subject. Outstanding invitations are
/// reported, and a tie is reported as a tie.
#[debug_handler]
async fn review_consensus(
    State(ctx): State<AppContext>,
    Query(params): Query<ConsensusParams>,
) -> Result<Response> {
    if !rules::is_token(rules::REVIEW_SUBJECT_KINDS, &params.subject_kind) {
        return Err(unprocessable(&format!(
            "subject_kind must be one of {:?}",
            rules::REVIEW_SUBJECT_KINDS
        )));
    }
    let rows =
        cap_models::reviews_for_subject(&ctx.db, &params.subject_kind, params.subject_pid).await?;
    format::json(consensus_of(&rows))
}

/// Aggregate stored review rows into the consensus view.
pub(crate) fn consensus_of(rows: &[reviews::Model]) -> rules::Consensus {
    let verdicts: Vec<rules::Verdict> = rows
        .iter()
        .filter(|r| r.status == "submitted")
        .map(|r| rules::Verdict {
            score: r.score,
            recommendation: r
                .recommendation
                .clone()
                .unwrap_or_else(|| "hold".to_string()),
            scope: r.reviewer_scope.clone(),
        })
        .collect();
    let outstanding = rows
        .iter()
        .filter(|r| rules::LIVE_REVIEW_STATUSES.contains(&r.status.as_str()))
        .count();
    let declined = rows.iter().filter(|r| r.status == "declined").count();
    rules::consensus(&verdicts, outstanding, declined)
}

/// `POST /api/plans/{pid}/tasks/{t_pid}/assign` body. A `null` /
/// absent `assignee_ref` unassigns.
#[derive(Debug, Deserialize)]
struct AssignPayload {
    #[serde(default)]
    assignee_ref: Option<String>,
}

/// `POST /api/plans/{pid}/tasks/{t_pid}/assign` — assign or unassign
/// one task.
#[debug_handler]
async fn assign_task(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((pid, t_pid)): Path<(String, String)>,
    Json(payload): Json<AssignPayload>,
) -> Result<Response> {
    let assignee = payload
        .assignee_ref
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(assignee) = assignee
        && !valid_ref(assignee, &["person", "worker"])
    {
        return Err(unprocessable(
            "assignee_ref must be a person:/worker: URN, or null to unassign",
        ));
    }
    let plan = super::governance::find_item(&ctx, &pid).await?;
    let task_pid = crate::models::governance::parse_pid(&t_pid)?;
    let task = tasks::Entity::find()
        .filter(tasks::Column::Pid.eq(task_pid))
        .filter(tasks::Column::PlanPid.eq(plan.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;
    let previous = task.assignee_ref.clone();
    let mut active: tasks::ActiveModel = task.into();
    active.assignee_ref = ActiveValue::set(assignee.map(std::string::ToString::to_string));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        task_pid,
        "task_assigned",
        caller.actor(),
        Some(serde_json::json!({ "from": previous, "to": row.assignee_ref })),
    )
    .await
    .ok();
    if let Some(assignee) = row.assignee_ref.as_deref() {
        cap_models::notify(
            &ctx.db,
            assignee,
            "task",
            task_pid,
            "task_assigned",
            &format!("You were assigned: {}", row.title),
        )
        .await
        .ok();
    }
    format::json(row)
}

/// `GET /api/assignees/workload` filter.
#[derive(Debug, Deserialize)]
struct WorkloadParams {
    /// Restrict to one plan's board; absent = every plan.
    #[serde(default)]
    plan: Option<Uuid>,
}

/// `GET /api/assignees/workload?plan=` — open work per assignee,
/// busiest first, including the `unassigned` pile. ETag-conditional.
#[debug_handler]
async fn assignee_workload(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(params): Query<WorkloadParams>,
) -> Result<Response> {
    let mut query = tasks::Entity::find().filter(tasks::Column::DeletedAt.is_null());
    if let Some(plan) = params.plan {
        query = query.filter(tasks::Column::PlanPid.eq(plan));
    }
    let rows = query.all(&ctx.db).await.map_err(db_err)?;
    let facts: Vec<rules::TaskLoadFact> = rows
        .iter()
        .map(|t| rules::TaskLoadFact {
            assignee_ref: t.assignee_ref.clone(),
            status: t.status.clone(),
            points: t.points,
        })
        .collect();
    let body = serde_json::json!({ "assignees": rules::workload(&facts) });
    let etag = super::etag_of(&body);
    super::conditional_json(
        &headers,
        &etag,
        &serde_json::json!({
            "assignees": body["assignees"],
            "as_of": chrono::Utc::now().to_rfc3339(),
        }),
    )
}

/// `GET /api/notifications` filter.
#[derive(Debug, Deserialize)]
struct NotificationParams {
    /// Whose inbox to read.
    recipient: String,
    /// `true` ⇒ only unread.
    #[serde(default)]
    unread: Option<bool>,
}

/// `GET /api/notifications?recipient=&unread=` — one inbox, newest
/// first (cap [`LIST_CAP`]).
#[debug_handler]
async fn list_notifications(
    State(ctx): State<AppContext>,
    Query(params): Query<NotificationParams>,
) -> Result<Response> {
    if !valid_actor_ref(&params.recipient) {
        return Err(unprocessable(
            "recipient must be a person:/worker:/organization: URN",
        ));
    }
    let mut query = notifications::Entity::find()
        .filter(notifications::Column::RecipientRef.eq(params.recipient.as_str()))
        .filter(notifications::Column::DeletedAt.is_null());
    if params.unread.unwrap_or(false) {
        query = query.filter(notifications::Column::ReadAt.is_null());
    }
    let rows = query
        .order_by_desc(notifications::Column::Id)
        .limit(LIST_CAP)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(rows)
}

/// `POST /api/notifications/{pid}/read` — mark one notification read
/// (idempotent: the first stamp stands).
#[debug_handler]
async fn read_notification(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row =
        cap_models::find_notification(&ctx.db, crate::models::governance::parse_pid(&pid)?).await?;
    if row.read_at.is_some() {
        return format::json(row);
    }
    let mut active: notifications::ActiveModel = row.into();
    active.read_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    format::json(row)
}

/// The collaboration routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/reviews", post(invite_review))
        .add("/reviews", get(list_reviews))
        .add("/reviews/consensus", get(review_consensus))
        .add("/reviews/{pid}/respond", post(respond_review))
        .add("/reviews/{pid}/submit", post(submit_review))
        .add("/reviews/{pid}", delete(withdraw_review))
        .add("/plans/{pid}/tasks/{t_pid}/assign", post(assign_task))
        .add("/assignees/workload", get(assignee_workload))
        .add("/notifications", get(list_notifications))
        .add("/notifications/{pid}/read", post(read_notification))
}
