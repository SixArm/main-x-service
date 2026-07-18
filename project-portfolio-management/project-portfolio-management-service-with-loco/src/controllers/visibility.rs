//! PPM Phase-B visibility controllers (spec/15-roadmap PPM-6/7/8/9).
//!
//! - cross-item dependencies + the portfolio schedule view
//!   (violations + critical path)
//! - milestones, resource allocations + the capacity rollup
//! - saved reports, and the ETag-conditional at-a-glance dashboard
//!
//! Pure rules live in [`crate::visibility`]; nothing here feeds the
//! matcher (§8 partition rule).

use axum::http::{HeaderMap, StatusCode, header};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use super::work_items::Collection;
use crate::auth::MaybeAuthUser;
use crate::models::_entities::{
    allocations, budget_lines, milestones, report_definitions, risks, work_item_dependencies,
    work_items,
};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::visibility as vis_models;
use crate::validation::MAX_TEXT_LEN;
use crate::visibility as rules;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// Find an active work item by pid across **any** collection (edges
/// may span kinds).
async fn find_any_item(ctx: &AppContext, pid: Uuid) -> Result<work_items::Model> {
    work_items::Entity::find()
        .filter(work_items::Column::Pid.eq(pid))
        .filter(work_items::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?
        .ok_or(Error::NotFound)
}

/// The schedule facts (parsed flexible dates) of one stored item.
fn schedule_item(model: &work_items::Model) -> rules::ScheduleItem {
    let start = model.data.get("start_date").and_then(|v| v.as_str());
    let end = model.data.get("target_date").and_then(|v| v.as_str());
    rules::ScheduleItem {
        pid: model.pid,
        start: start.and_then(|s| rules::parse_flex_date(s, false)),
        end: end.and_then(|s| rules::parse_flex_date(s, true)),
    }
}

/// Whether the stored item is finished (Completed / Cancelled).
fn is_finished(model: &work_items::Model) -> bool {
    matches!(
        model.data.get("status").and_then(|v| v.as_str()),
        Some("Completed" | "Cancelled")
    )
}

/// `POST /api/dependencies` body: a finish-start edge.
#[derive(Debug, Deserialize)]
struct DependencyPayload {
    predecessor_pid: Uuid,
    successor_pid: Uuid,
    #[serde(default)]
    lag_days: i32,
}

/// `POST /api/dependencies` — add an edge (both items must exist,
/// any collection; self-edges, duplicates, and cycles refuse).
#[debug_handler]
async fn create_dependency(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<DependencyPayload>,
) -> Result<Response> {
    if payload.lag_days < 0 {
        return Err(unprocessable("lag_days must be non-negative"));
    }
    find_any_item(&ctx, payload.predecessor_pid).await?;
    find_any_item(&ctx, payload.successor_pid).await?;
    let existing = vis_models::all_dependencies(&ctx.db).await?;
    if existing
        .iter()
        .any(|e| e.predecessor_pid == payload.predecessor_pid && e.successor_pid == payload.successor_pid)
    {
        return Err(unprocessable("this dependency already exists"));
    }
    let edges: Vec<(Uuid, Uuid)> = existing
        .iter()
        .map(|e| (e.predecessor_pid, e.successor_pid))
        .collect();
    if rules::would_create_cycle(&edges, payload.predecessor_pid, payload.successor_pid) {
        return Err(unprocessable("this dependency would create a cycle"));
    }
    let row = work_item_dependencies::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        predecessor_pid: ActiveValue::set(payload.predecessor_pid),
        successor_pid: ActiveValue::set(payload.successor_pid),
        lag_days: ActiveValue::set(payload.lag_days),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "dependency_added", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/dependencies?item={pid}` — edges touching one item (or
/// all, capped 500).
#[derive(Debug, Deserialize)]
struct DependencyParams {
    #[serde(default)]
    item: Option<Uuid>,
}

#[debug_handler]
async fn list_dependencies(
    State(ctx): State<AppContext>,
    Query(params): Query<DependencyParams>,
) -> Result<Response> {
    let rows = vis_models::all_dependencies(&ctx.db).await?;
    let rows: Vec<_> = rows
        .into_iter()
        .filter(|e| {
            params
                .item
                .is_none_or(|pid| e.predecessor_pid == pid || e.successor_pid == pid)
        })
        .take(500)
        .collect();
    format::json(rows)
}

/// `DELETE /api/dependencies/{pid}`.
#[debug_handler]
async fn delete_dependency(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let row = vis_models::find_dependency(&ctx.db, pid).await?;
    let row_pid = row.pid;
    work_item_dependencies::Entity::delete_by_id(row.id)
        .exec(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, "dependency_removed", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// `GET /api/portfolios/{pid}/schedule` — the roadmap view for one
/// portfolio umbrella: its children's timeframes, the dependency
/// edges among them, finish-start violations, and the critical path.
#[debug_handler]
async fn portfolio_schedule(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let portfolio_pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let portfolio = find_any_item(&ctx, portfolio_pid).await?;
    if portfolio.kind != "Portfolio" {
        return Err(Error::NotFound);
    }
    let mut members = work_items::Entity::find()
        .filter(work_items::Column::PortfolioPid.eq(portfolio_pid))
        .filter(work_items::Column::DeletedAt.is_null())
        .order_by_asc(work_items::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    members.insert(0, portfolio);
    let facts: Vec<rules::ScheduleItem> = members.iter().map(schedule_item).collect();
    let member_pids: std::collections::HashSet<Uuid> = facts.iter().map(|f| f.pid).collect();
    let edges: Vec<rules::ScheduleEdge> = vis_models::all_dependencies(&ctx.db)
        .await?
        .into_iter()
        .filter(|e| member_pids.contains(&e.predecessor_pid) && member_pids.contains(&e.successor_pid))
        .map(|e| rules::ScheduleEdge {
            pid: e.pid,
            predecessor: e.predecessor_pid,
            successor: e.successor_pid,
            lag_days: e.lag_days,
        })
        .collect();
    let violations = rules::violations(&facts, &edges);
    let critical = rules::critical_path(&facts, &edges);
    let items: Vec<_> = members
        .iter()
        .zip(&facts)
        .map(|(m, f)| {
            serde_json::json!({
                "pid": m.pid.to_string(), "kind": m.kind, "name": m.name,
                "stage": m.stage, "start": f.start, "end": f.end,
                "on_critical_path": critical.contains(&m.pid),
            })
        })
        .collect();
    format::json(serde_json::json!({
        "portfolio_pid": portfolio_pid.to_string(),
        "items": items,
        "edges": edges.iter().map(|e| serde_json::json!({
            "pid": e.pid.to_string(),
            "predecessor_pid": e.predecessor.to_string(),
            "successor_pid": e.successor.to_string(),
            "lag_days": e.lag_days,
        })).collect::<Vec<_>>(),
        "violations": violations,
        "critical_path": critical.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "undated": facts.iter().filter(|f| f.start.is_none() || f.end.is_none())
            .map(|f| f.pid.to_string()).collect::<Vec<_>>(),
    }))
}

/// `POST /api/{collection}/{pid}/milestones` body.
#[derive(Debug, Deserialize)]
struct MilestonePayload {
    name: String,
    due: chrono::NaiveDate,
}

#[debug_handler]
async fn create_milestone(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<MilestonePayload>,
) -> Result<Response> {
    if payload.name.trim().is_empty() || payload.name.len() > MAX_TEXT_LEN {
        return Err(unprocessable("name is required (and capped)"));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let row = milestones::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        name: ActiveValue::set(payload.name.clone()),
        due: ActiveValue::set(payload.due),
        done: ActiveValue::set(false),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "milestone_added", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/{collection}/{pid}/milestones` — due-date order, with
/// overdue flags.
#[debug_handler]
async fn list_milestones(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let today = chrono::Utc::now().date_naive();
    let rows = vis_models::milestones_for(&ctx.db, item.pid).await?;
    let views: Vec<_> = rows
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "pid": m.pid.to_string(), "name": m.name, "due": m.due,
                "done": m.done, "overdue": !m.done && m.due < today,
            })
        })
        .collect();
    format::json(views)
}

/// `POST /api/{collection}/{pid}/milestones/{m_pid}/complete`.
#[debug_handler]
async fn complete_milestone(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, m_pid)): Path<(String, String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let m_pid = Uuid::parse_str(&m_pid).map_err(|_| Error::NotFound)?;
    let milestone = vis_models::find_milestone(&ctx.db, m_pid).await?;
    if milestone.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    let row_pid = milestone.pid;
    let mut active: milestones::ActiveModel = milestone.into();
    active.done = ActiveValue::set(true);
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, "milestone_completed", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/{collection}/{pid}/allocations` body.
#[derive(Debug, Deserialize)]
struct AllocationPayload {
    /// `person:` / `worker:` `EntityRef` URN.
    person_ref: String,
    #[serde(default)]
    role: Option<String>,
    /// Percent of the person's capacity (1–100).
    percent: i32,
    #[serde(default)]
    start_date: Option<chrono::NaiveDate>,
    #[serde(default)]
    end_date: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn create_allocation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<AllocationPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if !super::governance::valid_ref(&payload.person_ref, &["person", "worker"]) {
        problems.push("person_ref must be a person:/worker: URN".to_string());
    }
    if !(1..=100).contains(&payload.percent) {
        problems.push(format!("percent must be 1–100, got {}", payload.percent));
    }
    if let (Some(start), Some(end)) = (payload.start_date, payload.end_date)
        && end < start {
            problems.push("end_date is before start_date".to_string());
        }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let row = allocations::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        person_ref: ActiveValue::set(payload.person_ref.clone()),
        role: ActiveValue::set(payload.role.clone()),
        percent: ActiveValue::set(payload.percent),
        start_date: ActiveValue::set(payload.start_date),
        end_date: ActiveValue::set(payload.end_date),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "allocation_added", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/{collection}/{pid}/allocations`.
#[debug_handler]
async fn list_allocations(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    format::json(vis_models::allocations_for(&ctx.db, item.pid).await?)
}

/// `DELETE /api/{collection}/{pid}/allocations/{a_pid}` — soft-delete.
#[debug_handler]
async fn delete_allocation(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, a_pid)): Path<(String, String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let a_pid = Uuid::parse_str(&a_pid).map_err(|_| Error::NotFound)?;
    let allocation = vis_models::find_allocation(&ctx.db, a_pid).await?;
    if allocation.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    let row_pid = allocation.pid;
    let mut active: allocations::ActiveModel = allocation.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, "allocation_removed", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// `GET /api/capacity?from=&to=` — the per-person capacity rollup
/// over a window (default: today → today + 30 days). Summed percent
/// over 100 ⇒ over-allocated (PPM-8).
#[derive(Debug, Deserialize)]
struct CapacityParams {
    #[serde(default)]
    from: Option<chrono::NaiveDate>,
    #[serde(default)]
    to: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn capacity(
    State(ctx): State<AppContext>,
    Query(params): Query<CapacityParams>,
) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let from = params.from.unwrap_or(today);
    let to = params.to.unwrap_or(from + chrono::Days::new(30));
    if to < from {
        return Err(unprocessable("to is before from"));
    }
    let rows = vis_models::all_allocations(&ctx.db).await?;
    let mut people: Vec<&str> = rows.iter().map(|a| a.person_ref.as_str()).collect();
    people.sort_unstable();
    people.dedup();
    let mut out = Vec::with_capacity(people.len());
    for person in people {
        let theirs: Vec<(i32, Option<chrono::NaiveDate>, Option<chrono::NaiveDate>)> = rows
            .iter()
            .filter(|a| a.person_ref == person)
            .map(|a| (a.percent, a.start_date, a.end_date))
            .collect();
        let total = rules::summed_percent(&theirs, from, to);
        out.push(serde_json::json!({
            "person_ref": person,
            "allocated_percent": total,
            "over_allocated": total > 100,
            "allocations": theirs.len(),
        }));
    }
    out.sort_by_key(|v| -v["allocated_percent"].as_i64().unwrap_or(0));
    format::json(serde_json::json!({ "from": from, "to": to, "people": out }))
}

/// `POST /api/reports` body (PPM-9).
#[derive(Debug, Deserialize)]
struct ReportPayload {
    name: String,
    collection: String,
    #[serde(default)]
    filters: serde_json::Value,
    fields: Vec<String>,
}

/// Columns a report may project.
const REPORT_FIELDS: &[&str] = &[
    "pid", "kind", "name", "stage", "status", "start_date", "target_date", "created_at",
];

#[debug_handler]
async fn create_report(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ReportPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if payload.name.trim().is_empty() || payload.name.len() > MAX_TEXT_LEN {
        problems.push("name is required (and capped)".to_string());
    }
    if Collection::from_segment(&payload.collection).is_none() {
        problems.push("collection must be portfolios/projects/products/programs".to_string());
    }
    if payload.fields.is_empty() {
        problems.push("fields must name at least one column".to_string());
    }
    for field in &payload.fields {
        if !REPORT_FIELDS.contains(&field.as_str()) {
            problems.push(format!("unknown field {field:?} (allowed: {REPORT_FIELDS:?})"));
        }
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let row = report_definitions::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        collection: ActiveValue::set(payload.collection.clone()),
        filters: ActiveValue::set(payload.filters.clone()),
        fields: ActiveValue::set(serde_json::json!(payload.fields)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "report_saved", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/reports` — saved definitions.
#[debug_handler]
async fn list_reports(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = report_definitions::Entity::find()
        .filter(report_definitions::Column::DeletedAt.is_null())
        .order_by_asc(report_definitions::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    format::json(rows)
}

/// `DELETE /api/reports/{pid}` — soft-delete a definition.
#[debug_handler]
async fn delete_report(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let row = vis_models::find_report(&ctx.db, pid).await?;
    let row_pid = row.pid;
    let mut active: report_definitions::ActiveModel = row.into();
    active.deleted_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, "report_deleted", caller.actor(), None)
        .await
        .ok();
    format::empty_json()
}

/// One projected report cell.
fn report_cell(model: &work_items::Model, field: &str) -> String {
    match field {
        "pid" => model.pid.to_string(),
        "kind" => model.kind.clone(),
        "name" => model.name.clone(),
        "stage" => model.stage.clone().unwrap_or_default(),
        "created_at" => model.created_at.to_rfc3339(),
        "status" | "start_date" | "target_date" => model
            .data
            .get(field)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        _ => String::new(),
    }
}

/// `GET /api/reports/{pid}/run?format=json|csv` — execute a saved
/// report synchronously (row cap 1000; scheduled/bulk-artifact runs
/// are the family bulk machinery, roadmap).
#[derive(Debug, Deserialize)]
struct RunParams {
    #[serde(default)]
    format: Option<String>,
}

#[debug_handler]
async fn run_report(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
    Query(params): Query<RunParams>,
) -> Result<Response> {
    let pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let def = vis_models::find_report(&ctx.db, pid).await?;
    let collection = Collection::from_segment(&def.collection).ok_or(Error::NotFound)?;
    let fields: Vec<String> = serde_json::from_value(def.fields.clone()).unwrap_or_default();
    let stage = def.filters.get("stage").and_then(|v| v.as_str());
    let status = def.filters.get("status").and_then(|v| v.as_str());
    let name_like = def
        .filters
        .get("name_like")
        .and_then(|v| v.as_str())
        .map(str::to_lowercase);
    let rows = crate::models::work_items::Model::list(&ctx.db, collection.kind_str(), 1000).await?;
    let selected: Vec<&work_items::Model> = rows
        .iter()
        .filter(|m| stage.is_none_or(|want| m.stage.as_deref() == Some(want)))
        .filter(|m| {
            status.is_none_or(|want| m.data.get("status").and_then(|v| v.as_str()) == Some(want))
        })
        .filter(|m| {
            name_like
                .as_deref()
                .is_none_or(|want| m.name.to_lowercase().contains(want))
        })
        .collect();
    if params.format.as_deref() == Some("csv") {
        let mut csv = fields
            .iter()
            .map(|f| rules::csv_field(f))
            .collect::<Vec<_>>()
            .join(",");
        csv.push('\n');
        for model in &selected {
            let line = fields
                .iter()
                .map(|f| rules::csv_field(&report_cell(model, f)))
                .collect::<Vec<_>>()
                .join(",");
            csv.push_str(&line);
            csv.push('\n');
        }
        return Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, header::HeaderValue::from_static("text/csv"))],
            csv,
        )
            .into_response());
    }
    let objects: Vec<serde_json::Value> = selected
        .iter()
        .map(|model| {
            fields
                .iter()
                .map(|f| (f.clone(), serde_json::json!(report_cell(model, f))))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into()
        })
        .collect();
    format::json(serde_json::json!({
        "name": def.name,
        "collection": def.collection,
        "rows": objects.len(),
        "data": objects,
    }))
}

/// `GET /api/at-a-glance` — the portfolio dashboard (PPM-7): per
/// collection, counts by RAG / stage / status plus risk exposure and
/// per-currency budget variance; site tiles headline the whole
/// estate. ETag-conditional (the tag excludes `as_of`).
#[debug_handler]
#[allow(clippy::too_many_lines)] // one pass over the whole estate
async fn at_a_glance(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let today = chrono::Utc::now().date_naive();
    let items = work_items::Entity::find()
        .filter(work_items::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let risk_rows = risks::Entity::find()
        .filter(risks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let budget_rows = budget_lines::Entity::find()
        .filter(budget_lines::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let proposal_rows = crate::models::_entities::proposals::Entity::find()
        .filter(crate::models::_entities::proposals::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let dependency_rows = vis_models::all_dependencies(&ctx.db).await?;
    let allocation_rows = vis_models::all_allocations(&ctx.db).await?;

    // Global schedule violations (per successor item).
    let facts: Vec<rules::ScheduleItem> = items.iter().map(schedule_item).collect();
    let edges: Vec<rules::ScheduleEdge> = dependency_rows
        .iter()
        .map(|e| rules::ScheduleEdge {
            pid: e.pid,
            predecessor: e.predecessor_pid,
            successor: e.successor_pid,
            lag_days: e.lag_days,
        })
        .collect();
    let all_violations = rules::violations(&facts, &edges);
    let violated: std::collections::HashSet<Uuid> =
        all_violations.iter().map(|v| v.successor).collect();

    // Per-item RAG.
    let mut per_collection: std::collections::BTreeMap<&str, Vec<&work_items::Model>> =
        std::collections::BTreeMap::new();
    for item in &items {
        per_collection.entry(item.kind.as_str()).or_default().push(item);
    }
    let mut collections = Vec::new();
    for (kind, members) in &per_collection {
        let mut rag_counts = std::collections::BTreeMap::from([("red", 0), ("amber", 0), ("green", 0)]);
        let mut stages: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
        for member in members {
            let member_risks: Vec<_> = risk_rows
                .iter()
                .filter(|r| r.work_item_pid == member.pid)
                .collect();
            let materialised = member_risks.iter().filter(|r| r.status == "materialised").count();
            let max_exposure = member_risks
                .iter()
                .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
                .map(|r| r.probability * r.impact)
                .max()
                .unwrap_or(0);
            let member_budgets: Vec<_> = budget_rows
                .iter()
                .filter(|b| b.work_item_pid == member.pid)
                .collect();
            let overrun = {
                let mut currencies: Vec<&str> =
                    member_budgets.iter().map(|b| b.currency.as_str()).collect();
                currencies.sort_unstable();
                currencies.dedup();
                currencies.into_iter().any(|c| {
                    let planned: i64 = member_budgets.iter().filter(|b| b.currency == c).map(|b| b.planned_minor).sum();
                    let actual: i64 = member_budgets.iter().filter(|b| b.currency == c).map(|b| b.actual_minor).sum();
                    actual > planned
                })
            };
            let target = member
                .data
                .get("target_date")
                .and_then(|v| v.as_str())
                .and_then(|s| rules::parse_flex_date(s, true));
            let colour = rules::rag(
                is_finished(member),
                target,
                today,
                materialised,
                max_exposure,
                overrun,
                violated.contains(&member.pid),
            );
            *rag_counts.get_mut(colour).expect("fixed keys") += 1;
            *stages
                .entry(member.stage.clone().unwrap_or_else(|| "pre_gate".to_string()))
                .or_default() += 1;
        }
        collections.push(serde_json::json!({
            "collection": kind,
            "total": members.len(),
            "rag": rag_counts,
            "stages": stages,
        }));
    }

    // Site tiles.
    let open_risk_exposure: i32 = risk_rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .map(|r| r.probability * r.impact)
        .sum();
    let proposals_open = proposal_rows
        .iter()
        .filter(|p| matches!(p.status.as_str(), "draft" | "submitted" | "in_review"))
        .count();
    let over_allocated = {
        let mut people: Vec<&str> = allocation_rows.iter().map(|a| a.person_ref.as_str()).collect();
        people.sort_unstable();
        people.dedup();
        people
            .into_iter()
            .filter(|person| {
                let theirs: Vec<_> = allocation_rows
                    .iter()
                    .filter(|a| a.person_ref == *person)
                    .map(|a| (a.percent, a.start_date, a.end_date))
                    .collect();
                rules::summed_percent(&theirs, today, today + chrono::Days::new(30)) > 100
            })
            .count()
    };
    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "collections": collections,
        "site_tiles": {
            "work_items": items.len(),
            "proposals_open": proposals_open,
            "materialised_risks": risk_rows.iter().filter(|r| r.status == "materialised").count(),
            "open_risk_exposure": open_risk_exposure,
            "schedule_violations": all_violations.len(),
            "over_allocated_people": over_allocated,
        },
    });
    let mut fingerprint = body.clone();
    if let Some(map) = fingerprint.as_object_mut() {
        map.remove("as_of");
    }
    super::conditional_json(&headers, &super::etag_of(&fingerprint), &body)
}

/// The visibility routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/dependencies", post(create_dependency))
        .add("/dependencies", get(list_dependencies))
        .add("/dependencies/{pid}", delete(delete_dependency))
        .add("/portfolios/{pid}/schedule", get(portfolio_schedule))
        .add("/{collection}/{pid}/milestones", post(create_milestone))
        .add("/{collection}/{pid}/milestones", get(list_milestones))
        .add(
            "/{collection}/{pid}/milestones/{m_pid}/complete",
            post(complete_milestone),
        )
        .add("/{collection}/{pid}/allocations", post(create_allocation))
        .add("/{collection}/{pid}/allocations", get(list_allocations))
        .add(
            "/{collection}/{pid}/allocations/{a_pid}",
            delete(delete_allocation),
        )
        .add("/capacity", get(capacity))
        .add("/reports", post(create_report))
        .add("/reports", get(list_reports))
        .add("/reports/{pid}", delete(delete_report))
        .add("/reports/{pid}/run", get(run_report))
        .add("/at-a-glance", get(at_a_glance))
}
