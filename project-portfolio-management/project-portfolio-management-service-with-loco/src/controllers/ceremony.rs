//! **Sprint ceremonies** and the commitment snapshot (entity spec
//! §9.2b / FR-29).
//!
//! The retrospective already existed as `sprint_notes`. This adds
//! planning, daily and review as records, and the snapshot that makes a
//! mid-sprint scope change legible.
//!
//! # Why the commitment is written once
//!
//! Planning declares a set of tasks. If that set could be rewritten, a
//! sprint that grew by half would look like a sprint that was always
//! that size — the goalpost would move silently. So `commit` refuses a
//! second call, and the sprint's *current* task set is reported beside
//! the committed one so the delta is visible rather than inferred.
//!
//! # Sprint metrics are not flow metrics
//!
//! Velocity and burndown here are sprint-scoped and count-based. The
//! Flow Framework metrics (§1.6) are item-scoped and time-based.
//! Neither is derived from the other, and this module computes none of
//! the latter.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{ceremonies, sprint_commitments, sprint_notes, sprints, tasks};
use crate::models::audit_logs::Model as AuditModel;

/// The four ceremonies.
pub const CEREMONY_KINDS: &[&str] = &["planning", "daily", "review", "retrospective"];

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

async fn find_sprint(ctx: &AppContext, raw: &str) -> Result<sprints::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    sprints::Entity::find()
        .filter(sprints::Column::Pid.eq(pid))
        .filter(sprints::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// `POST /api/sprints/{pid}/ceremonies` body.
#[derive(Debug, Deserialize)]
struct CeremonyPayload {
    kind: String,
    #[serde(default)]
    facilitator_ref: Option<String>,
    #[serde(default)]
    summary: Option<String>,
}

/// `POST /api/sprints/{pid}/ceremonies` — hold a ceremony.
#[debug_handler]
async fn create_ceremony(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CeremonyPayload>,
) -> Result<Response> {
    let sprint = find_sprint(&ctx, &pid).await?;
    if !CEREMONY_KINDS.contains(&payload.kind.as_str()) {
        return Err(unprocessable(&format!(
            "kind must be one of {CEREMONY_KINDS:?}"
        )));
    }
    // A second planning or review is a re-plan, which is a new sprint —
    // and the schema refuses it too.
    if ["planning", "review"].contains(&payload.kind.as_str()) {
        let existing = ceremonies::Entity::find()
            .filter(ceremonies::Column::SprintPid.eq(sprint.pid))
            .filter(ceremonies::Column::Kind.eq(payload.kind.as_str()))
            .filter(ceremonies::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?;
        if existing.is_some() {
            return Err(unprocessable(&format!(
                "this sprint already held its {}: a second one is a re-plan, \
                 which is a new sprint",
                payload.kind
            )));
        }
    }

    let row = ceremonies::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        sprint_pid: ActiveValue::set(sprint.pid),
        kind: ActiveValue::set(payload.kind.clone()),
        facilitator_ref: ActiveValue::set(payload.facilitator_ref.clone()),
        summary: ActiveValue::set(payload.summary.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        sprint.plan_pid,
        "ceremony_held",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/sprints/{pid}/ceremonies` — with the retrospective notes
/// already stored against the sprint.
#[debug_handler]
async fn list_ceremonies(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let sprint = find_sprint(&ctx, &pid).await?;
    let held = ceremonies::Entity::find()
        .filter(ceremonies::Column::SprintPid.eq(sprint.pid))
        .filter(ceremonies::Column::DeletedAt.is_null())
        .order_by_asc(ceremonies::Column::HeldAt)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let notes = sprint_notes::Entity::find()
        .filter(sprint_notes::Column::SprintPid.eq(sprint.pid))
        .filter(sprint_notes::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!({
        "ceremonies": held,
        // Every kind reported, even at zero — a sprint that never held a
        // retrospective is a finding, not a missing row.
        "held": CEREMONY_KINDS.iter().map(|kind| serde_json::json!({
            "kind": kind,
            "count": held.iter().filter(|c| c.kind == *kind).count(),
        })).collect::<Vec<_>>(),
        "retrospective_notes": notes,
    }))
}

/// `POST /api/sprints/{pid}/commit` — snapshot the committed task set.
///
/// Refuses a second call: a rewritable commitment is not a commitment,
/// and a sprint that grew by half would otherwise look like one that was
/// always that size.
#[debug_handler]
async fn commit_sprint(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let sprint = find_sprint(&ctx, &pid).await?;
    let existing = sprint_commitments::Entity::find()
        .filter(sprint_commitments::Column::SprintPid.eq(sprint.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    if !existing.is_empty() {
        return Err(unprocessable(
            "this sprint is already committed: a rewritable commitment would let \
             mid-sprint scope look like scope committed at the outset",
        ));
    }

    let current = tasks::Entity::find()
        .filter(tasks::Column::SprintPid.eq(sprint.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    for task in &current {
        sprint_commitments::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            sprint_pid: ActiveValue::set(sprint.pid),
            task_pid: ActiveValue::set(task.pid),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .map_err(db_err)?;
    }

    AuditModel::record(
        &ctx.db,
        sprint.plan_pid,
        "sprint_committed",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "committed": current.len() }))
}

/// `GET /api/sprints/{pid}/commitment` — the committed set beside the
/// current one, so a scope change reads **as a change**.
#[debug_handler]
async fn commitment(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let sprint = find_sprint(&ctx, &pid).await?;
    let committed = sprint_commitments::Entity::find()
        .filter(sprint_commitments::Column::SprintPid.eq(sprint.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let current = tasks::Entity::find()
        .filter(tasks::Column::SprintPid.eq(sprint.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let committed_pids: std::collections::BTreeSet<Uuid> =
        committed.iter().map(|c| c.task_pid).collect();
    let current_pids: std::collections::BTreeSet<Uuid> = current.iter().map(|t| t.pid).collect();

    let body = serde_json::json!({
        "sprint_pid": sprint.pid.to_string(),
        "committed": committed_pids.len(),
        "current": current_pids.len(),
        // Named, not just counted: "what was added" is the question.
        "added_after_commitment": current_pids.difference(&committed_pids)
            .map(ToString::to_string).collect::<Vec<_>>(),
        "removed_after_commitment": committed_pids.difference(&current_pids)
            .map(ToString::to_string).collect::<Vec<_>>(),
        "was_committed": !committed.is_empty(),
        "note": "Sprint velocity and burndown are sprint-scoped and count-based; \
                 the Flow Framework metrics are item-scoped and time-based. \
                 Neither is derived from the other.",
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The ceremony routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sprints/{pid}/ceremonies", post(create_ceremony))
        .add("/sprints/{pid}/ceremonies", get(list_ceremonies))
        .add("/sprints/{pid}/commit", post(commit_sprint))
        .add("/sprints/{pid}/commitment", get(commitment))
}
