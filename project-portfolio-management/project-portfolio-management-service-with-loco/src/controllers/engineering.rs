//! Engineering-team endpoints — the spec-§13 operational **tasks**
//! sub-resource (Kanban board: CRUD + a PATCH status move with honest
//! flow stamps), **sprints** + the honest **burndown** (real `done_at`
//! completions only), the per-item **standup digest** (audit-derived
//! last-24h changes), and the estate views: **blocked-work aging**,
//! the **`MoSCoW`** scope cut (`moscow:<band>` tags), the
//! **delivery-links** panel (external tracker identifiers), and the
//! **milestone calendar** (demo / release / checkpoint kinds).
//!
//! Tasks and sprints are operational data: deliberately **not** part
//! of the matcher payload (the spec's partition rule) — nothing here
//! feeds matching.

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use super::insights::item_ref;
use crate::engineering as rules;
use crate::models::_entities::{audit_logs, milestones, sprints, tasks, work_items};
use crate::models::audit_logs::Model as AuditModel;
use crate::auth::MaybeAuthUser;

/// Load all live rows of one entity (soft-deleted excluded).
macro_rules! live {
    ($module:ident, $db:expr) => {
        $module::Entity::find()
            .filter($module::Column::DeletedAt.is_null())
            .all($db)
            .await
            .map_err(|e| Error::Model(ModelError::from(e)))?
    };
}

/// `422` with a reason.
fn refuse(reason: &str) -> Error {
    Error::CustomError(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        loco_rs::controller::ErrorDetail::new("unprocessable_entity", reason),
    )
}

/// A `worker:`/`person:` URN check (mirrors governance's rule).
fn valid_person_ref(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(scheme, id)| {
            matches!(scheme, "worker" | "person") && Uuid::parse_str(id).is_ok()
        })
}

/// Find one live task of one work item, or 404.
async fn find_task(
    ctx: &AppContext,
    work_item_pid: Uuid,
    t_pid: &str,
) -> Result<tasks::Model> {
    let t_pid = Uuid::parse_str(t_pid).map_err(|_| Error::NotFound)?;
    tasks::Entity::find()
        .filter(tasks::Column::Pid.eq(t_pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .filter(|task| task.work_item_pid == work_item_pid)
        .ok_or(Error::NotFound)
}

/// The task wire view (adds blocked-age days to the stored row).
fn task_view(task: &tasks::Model) -> serde_json::Value {
    let blocked_days = (task.status == "blocked")
        .then(|| (chrono::Utc::now() - task.status_changed_at.to_utc()).num_days());
    serde_json::json!({
        "pid": task.pid,
        "title": task.title,
        "description": task.description,
        "status": task.status,
        "assignee_ref": task.assignee_ref,
        "sprint_pid": task.sprint_pid,
        "created_at": task.created_at.to_utc(),
        "status_changed_at": task.status_changed_at.to_utc(),
        "done_at": task.done_at.map(|at| at.to_utc()),
        "blocked_days": blocked_days,
    })
}

/// `POST /api/{collection}/{pid}/tasks` body.
#[derive(Debug, Deserialize)]
struct TaskPayload {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    assignee_ref: Option<String>,
    #[serde(default)]
    sprint_pid: Option<Uuid>,
}

/// Validate the shared task fields; returns the resolved status.
async fn validate_task(
    ctx: &AppContext,
    work_item_pid: Uuid,
    payload: &TaskPayload,
) -> Result<String> {
    if payload.title.trim().is_empty() {
        return Err(refuse("title is required"));
    }
    if payload.title.len() > crate::validation::MAX_TEXT_LEN {
        return Err(refuse("title exceeds the text cap"));
    }
    let status = payload.status.clone().unwrap_or_else(|| "todo".to_string());
    if !rules::TASK_STATUSES.contains(&status.as_str()) {
        return Err(refuse(&format!("status must be one of {:?}", rules::TASK_STATUSES)));
    }
    if let Some(assignee) = payload.assignee_ref.as_deref()
        && !valid_person_ref(assignee)
    {
        return Err(refuse("assignee_ref must be a worker:/person: URN"));
    }
    if let Some(sprint_pid) = payload.sprint_pid {
        let sprint = sprints::Entity::find()
            .filter(sprints::Column::Pid.eq(sprint_pid))
            .filter(sprints::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(|e| Error::Model(ModelError::from(e)))?;
        if sprint.is_none_or(|s| s.work_item_pid != work_item_pid) {
            return Err(refuse("sprint_pid must name a sprint of this work item"));
        }
    }
    Ok(status)
}

/// `POST /api/{collection}/{pid}/tasks` — create a task (default
/// status `todo`; `done` on create stamps `done_at`).
#[debug_handler]
async fn create_task(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<TaskPayload>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let status = validate_task(&ctx, item.pid, &payload).await?;
    let now = chrono::Utc::now();
    let row = tasks::ActiveModel {
        pid: sea_orm::ActiveValue::set(Uuid::new_v4()),
        work_item_pid: sea_orm::ActiveValue::set(item.pid),
        sprint_pid: sea_orm::ActiveValue::set(payload.sprint_pid),
        title: sea_orm::ActiveValue::set(payload.title.clone()),
        description: sea_orm::ActiveValue::set(payload.description.clone()),
        status: sea_orm::ActiveValue::set(status.clone()),
        assignee_ref: sea_orm::ActiveValue::set(payload.assignee_ref.clone()),
        status_changed_at: sea_orm::ActiveValue::set(now.into()),
        done_at: sea_orm::ActiveValue::set((status == "done").then(|| now.into())),
        deleted_at: sea_orm::ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "task_created", caller.actor(), None)
        .await
        .ok();
    format::json(task_view(&row))
}

/// `GET /api/{collection}/{pid}/tasks` — the item's live tasks plus
/// per-status counts (board columns).
#[debug_handler]
async fn list_tasks(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let rows = tasks::Entity::find()
        .filter(tasks::Column::WorkItemPid.eq(item.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .order_by_asc(tasks::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let mut counts: std::collections::BTreeMap<&str, usize> = rules::TASK_STATUSES
        .iter()
        .map(|status| (*status, 0))
        .collect();
    for task in &rows {
        if let Some(entry) = counts.get_mut(task.status.as_str()) {
            *entry += 1;
        }
    }
    format::json(serde_json::json!({
        "tasks": rows.iter().map(task_view).collect::<Vec<_>>(),
        "counts": counts,
    }))
}

/// `PUT /api/{collection}/{pid}/tasks/{t_pid}` — update fields
/// (status changes route through PATCH so the flow stamps stay true).
#[debug_handler]
async fn update_task(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, t_pid)): Path<(String, String, String)>,
    Json(payload): Json<TaskPayload>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let task = find_task(&ctx, item.pid, &t_pid).await?;
    if payload.status.as_deref().is_some_and(|s| s != task.status) {
        return Err(refuse("status changes go through PATCH (the board move)"));
    }
    validate_task(&ctx, item.pid, &payload).await?;
    let task_pid = task.pid;
    let mut active: tasks::ActiveModel = task.into();
    active.title = sea_orm::ActiveValue::set(payload.title.clone());
    active.description = sea_orm::ActiveValue::set(payload.description.clone());
    active.assignee_ref = sea_orm::ActiveValue::set(payload.assignee_ref.clone());
    active.sprint_pid = sea_orm::ActiveValue::set(payload.sprint_pid);
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, task_pid, "task_updated", caller.actor(), None)
        .await
        .ok();
    format::json(task_view(&row))
}

/// `PATCH /api/{collection}/{pid}/tasks/{t_pid}` body: the move.
#[derive(Debug, Deserialize)]
struct MovePayload {
    status: String,
}

/// `PATCH /api/{collection}/{pid}/tasks/{t_pid}` — the board move.
/// Stamps `status_changed_at`; first entry into `done` stamps
/// `done_at` (kept thereafter — the completion history stays true).
#[debug_handler]
async fn move_task(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, t_pid)): Path<(String, String, String)>,
    Json(payload): Json<MovePayload>,
) -> Result<Response> {
    if !rules::TASK_STATUSES.contains(&payload.status.as_str()) {
        return Err(refuse(&format!("status must be one of {:?}", rules::TASK_STATUSES)));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let task = find_task(&ctx, item.pid, &t_pid).await?;
    if task.status == payload.status {
        return format::json(task_view(&task));
    }
    let from = task.status.clone();
    let task_pid = task.pid;
    let first_done = task.done_at.is_none() && payload.status == "done";
    let mut active: tasks::ActiveModel = task.into();
    active.status = sea_orm::ActiveValue::set(payload.status.clone());
    active.status_changed_at = sea_orm::ActiveValue::set(chrono::Utc::now().into());
    if first_done {
        active.done_at = sea_orm::ActiveValue::set(Some(chrono::Utc::now().into()));
    }
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(
        &ctx.db,
        task_pid,
        "task_moved",
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.status })),
    )
    .await
    .ok();
    format::json(task_view(&row))
}

/// `DELETE /api/{collection}/{pid}/tasks/{t_pid}` — soft delete.
#[debug_handler]
async fn delete_task(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, t_pid)): Path<(String, String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let task = find_task(&ctx, item.pid, &t_pid).await?;
    let task_pid = task.pid;
    let mut active: tasks::ActiveModel = task.into();
    active.deleted_at = sea_orm::ActiveValue::set(Some(chrono::Utc::now().into()));
    active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, task_pid, "task_deleted", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// `POST /api/{collection}/{pid}/sprints` body.
#[derive(Debug, Deserialize)]
struct SprintPayload {
    name: String,
    starts_on: chrono::NaiveDate,
    ends_on: chrono::NaiveDate,
}

/// `POST /api/{collection}/{pid}/sprints` — create a time-boxed
/// sprint (`ends_on` must not precede `starts_on`).
#[debug_handler]
async fn create_sprint(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<SprintPayload>,
) -> Result<Response> {
    if payload.name.trim().is_empty() || payload.name.len() > crate::validation::MAX_TEXT_LEN {
        return Err(refuse("name is required (and capped)"));
    }
    if payload.ends_on < payload.starts_on {
        return Err(refuse("ends_on is before starts_on"));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let row = sprints::ActiveModel {
        pid: sea_orm::ActiveValue::set(Uuid::new_v4()),
        work_item_pid: sea_orm::ActiveValue::set(item.pid),
        name: sea_orm::ActiveValue::set(payload.name.clone()),
        starts_on: sea_orm::ActiveValue::set(payload.starts_on),
        ends_on: sea_orm::ActiveValue::set(payload.ends_on),
        deleted_at: sea_orm::ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "sprint_created", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `GET /api/{collection}/{pid}/sprints` — the item's sprints, newest
/// first.
#[debug_handler]
async fn list_sprints(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let rows = sprints::Entity::find()
        .filter(sprints::Column::WorkItemPid.eq(item.pid))
        .filter(sprints::Column::DeletedAt.is_null())
        .order_by_desc(sprints::Column::StartsOn)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    format::json(rows)
}

/// Query for the burndown: the sprint to derive over.
#[derive(Debug, Deserialize)]
struct BurndownQuery {
    sprint: Uuid,
}

/// `GET /api/{collection}/{pid}/burndown?sprint=` — the honest sprint
/// burndown: remaining task count per day of the sprint window, from
/// real `done_at` stamps only. The response says so; an ideal line is
/// the client's to draw and label.
#[debug_handler]
async fn burndown(
    axum::extract::Query(query): axum::extract::Query<BurndownQuery>,
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let sprint = sprints::Entity::find()
        .filter(sprints::Column::Pid.eq(query.sprint))
        .filter(sprints::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .filter(|s| s.work_item_pid == item.pid)
        .ok_or(Error::NotFound)?;
    let sprint_tasks = tasks::Entity::find()
        .filter(tasks::Column::SprintPid.eq(sprint.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let done_dates: Vec<chrono::NaiveDate> = sprint_tasks
        .iter()
        .filter_map(|task| task.done_at.map(|at| at.to_utc().date_naive()))
        .collect();
    let points = rules::burndown(sprint_tasks.len(), &done_dates, sprint.starts_on, sprint.ends_on);
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "sprint": { "pid": sprint.pid, "name": sprint.name,
                     "starts_on": sprint.starts_on, "ends_on": sprint.ends_on },
        "total_tasks": sprint_tasks.len(),
        "derivation": "remaining = assigned tasks minus real done_at stamps on or \
                       before each day; no ideal line, no interpolation",
        "points": points,
    });
    conditional(&headers, &body)
}

/// `GET /api/{collection}/{pid}/standup` — the last-24h digest,
/// audit-derived: tasks created / moved / completed, current blockers,
/// risks raised. What a standup reads out, from what was recorded.
#[debug_handler]
async fn standup(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let since = chrono::Utc::now() - chrono::Days::new(1);
    let item_tasks = tasks::Entity::find()
        .filter(tasks::Column::WorkItemPid.eq(item.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let task_pids: std::collections::HashSet<Uuid> =
        item_tasks.iter().map(|task| task.pid).collect();
    let titles: std::collections::BTreeMap<Uuid, &str> =
        item_tasks.iter().map(|task| (task.pid, task.title.as_str())).collect();

    let recent = audit_logs::Entity::find()
        .filter(audit_logs::Column::CreatedAt.gte(since))
        .order_by_desc(audit_logs::Column::CreatedAt)
        .limit(500)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let mut moves = Vec::new();
    let mut created = Vec::new();
    let mut risks_raised = 0usize;
    for row in &recent {
        match row.action.as_str() {
            "task_moved" if task_pids.contains(&row.entity_pid) => {
                moves.push(serde_json::json!({
                    "at": row.created_at.to_utc(),
                    "task": titles.get(&row.entity_pid),
                    "actor": row.actor,
                    "move": row.snapshot,
                }));
            }
            "task_created" if task_pids.contains(&row.entity_pid) => {
                created.push(serde_json::json!({
                    "at": row.created_at.to_utc(),
                    "task": titles.get(&row.entity_pid),
                    "actor": row.actor,
                }));
            }
            "risk_raised" => {
                // Risk audits key the risk pid, not the item pid; a
                // per-item filter needs the risk rows — count only.
                risks_raised += 1;
            }
            _ => {}
        }
    }
    let blocked: Vec<serde_json::Value> = item_tasks
        .iter()
        .filter(|task| task.status == "blocked")
        .map(task_view)
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "since": since,
        "item": item_ref(&item),
        "tasks_created": created,
        "tasks_moved": moves,
        "blocked_now": blocked,
        "risks_raised_estate_wide": risks_raised,
    });
    conditional(&headers, &body)
}

// ─── Estate views ───────────────────────────────────────────────────────────

/// `GET /api/engineering/blocked` — every blocked task estate-wide,
/// oldest blockage first, with age in days (from `status_changed_at`).
#[debug_handler]
async fn blocked(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let mut rows: Vec<&tasks::Model> = Vec::new();
    let all_tasks = tasks::Entity::find()
        .filter(tasks::Column::Status.eq("blocked"))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    for task in &all_tasks {
        rows.push(task);
    }
    rows.sort_by_key(|task| task.status_changed_at);
    let blocked: Vec<serde_json::Value> = rows
        .iter()
        .map(|task| {
            let mut view = task_view(task);
            if let Some(map) = view.as_object_mut() {
                map.insert(
                    "item".to_string(),
                    by_pid
                        .get(&task.work_item_pid)
                        .map_or(serde_json::Value::Null, |i| item_ref(i)),
                );
            }
            view
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "derivation": "age = days since the task entered blocked (status_changed_at)",
        "blocked": blocked,
    });
    conditional(&headers, &body)
}

/// `GET /api/engineering/moscow` — the `MoSCoW` scope cut from
/// `moscow:<band>` tags (convention disclosed; untagged items listed
/// separately, never guessed into a band).
#[debug_handler]
async fn moscow(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let items = live!(work_items, &ctx.db);
    let mut bands: std::collections::BTreeMap<&str, Vec<serde_json::Value>> =
        rules::MOSCOW_BANDS.iter().map(|band| (*band, Vec::new())).collect();
    let mut untagged = 0usize;
    for item in &items {
        let tags = item
            .data
            .get("tags")
            .and_then(|v| v.as_array())
            .map_or(&[][..], Vec::as_slice);
        let band = tags
            .iter()
            .filter_map(|tag| tag.as_str())
            .find_map(rules::parse_moscow_tag);
        match band {
            Some(band) => bands.get_mut(band).expect("known band").push(item_ref(item)),
            None => untagged += 1,
        }
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "convention": "tag `moscow:<must|should|could|wont>`; the first moscow tag wins; \
                       untagged items are counted, never guessed into a band",
        "bands": bands,
        "untagged": untagged,
    });
    conditional(&headers, &body)
}

/// `GET /api/engineering/delivery-links` — which items are tracked in
/// which external delivery tool (the `identifiers` schemes the matcher
/// already short-circuits on), plus the untracked list.
#[debug_handler]
async fn delivery_links(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    const TRACKER_SCHEMES: [&str; 6] = [
        "JiraProjectKey",
        "AsanaGid",
        "TrelloBoardId",
        "MsProjectId",
        "GitHubProjectId",
        "LinearId",
    ];
    let items = live!(work_items, &ctx.db);
    let mut tracked = Vec::new();
    let mut untracked = Vec::new();
    for item in &items {
        let identifiers = item
            .data
            .get("identifiers")
            .and_then(|v| v.as_array())
            .map_or(&[][..], Vec::as_slice);
        let links: Vec<serde_json::Value> = identifiers
            .iter()
            .filter_map(|identifier| {
                let scheme = identifier.get("scheme").and_then(|v| v.as_str())?;
                TRACKER_SCHEMES.contains(&scheme).then(|| {
                    serde_json::json!({
                        "scheme": scheme,
                        "value": identifier.get("value"),
                    })
                })
            })
            .collect();
        if links.is_empty() {
            untracked.push(item_ref(item));
        } else {
            tracked.push(serde_json::json!({ "item": item_ref(item), "links": links }));
        }
    }
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "schemes": TRACKER_SCHEMES,
        "tracked": tracked,
        "untracked": untracked,
    });
    conditional(&headers, &body)
}

/// Query for the milestone calendar: optional kind filter.
#[derive(Debug, Deserialize)]
struct CalendarQuery {
    kind: Option<String>,
}

/// `GET /api/engineering/milestone-calendar?kind=` — estate milestones
/// with their kinds (absent reads `milestone`), due-date order, for
/// the demo / release calendar.
#[debug_handler]
async fn milestone_calendar(
    axum::extract::Query(query): axum::extract::Query<CalendarQuery>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(kind) = query.kind.as_deref()
        && !rules::MILESTONE_KINDS.contains(&kind)
    {
        return Err(refuse(&format!("kind must be one of {:?}", rules::MILESTONE_KINDS)));
    }
    let items = live!(work_items, &ctx.db);
    let by_pid: std::collections::BTreeMap<Uuid, &work_items::Model> =
        items.iter().map(|i| (i.pid, i)).collect();
    let mut rows = live!(milestones, &ctx.db);
    rows.sort_by_key(|m| m.due);
    let entries: Vec<serde_json::Value> = rows
        .iter()
        .filter(|m| {
            query.kind.as_deref().is_none_or(|kind| {
                m.kind.as_deref().unwrap_or("milestone") == kind
            })
        })
        .map(|m| {
            serde_json::json!({
                "pid": m.pid,
                "name": m.name,
                "kind": m.kind.as_deref().unwrap_or("milestone"),
                "due": m.due,
                "done": m.done,
                "item": by_pid.get(&m.work_item_pid).map(|i| item_ref(i)),
            })
        })
        .collect();
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "kinds": rules::MILESTONE_KINDS,
        "milestones": entries,
    });
    conditional(&headers, &body)
}

/// ETag-conditional JSON response; the tag excludes `as_of`.
fn conditional(headers: &HeaderMap, body: &serde_json::Value) -> Result<Response> {
    let mut fingerprint = body.clone();
    if let Some(map) = fingerprint.as_object_mut() {
        map.remove("as_of");
    }
    super::conditional_json(headers, &super::etag_of(&fingerprint), body)
}

/// The engineering routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/{collection}/{pid}/tasks", post(create_task))
        .add("/{collection}/{pid}/tasks", get(list_tasks))
        .add("/{collection}/{pid}/tasks/{t_pid}", put(update_task))
        .add("/{collection}/{pid}/tasks/{t_pid}", patch(move_task))
        .add("/{collection}/{pid}/tasks/{t_pid}", delete(delete_task))
        .add("/{collection}/{pid}/sprints", post(create_sprint))
        .add("/{collection}/{pid}/sprints", get(list_sprints))
        .add("/{collection}/{pid}/burndown", get(burndown))
        .add("/{collection}/{pid}/standup", get(standup))
        .add("/engineering/blocked", get(blocked))
        .add("/engineering/moscow", get(moscow))
        .add("/engineering/delivery-links", get(delivery_links))
        .add("/engineering/milestone-calendar", get(milestone_calendar))
}
