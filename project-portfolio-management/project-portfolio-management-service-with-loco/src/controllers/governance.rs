//! PPM Phase-A governance controllers (spec/15-roadmap PPM-1/3/10/12):
//! the `proposals` work-intake pipeline, per-work-item phase-gate
//! reviews, risks, and budget lines, plus the per-item governance
//! summary. Every mutation writes an audit row; the pure rules live in
//! [`crate::governance`]; nothing here ever feeds the matcher (the §8
//! partition rule).

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use project_portfolio_management_matcher::{MatchConfig, MatchingEngine, WorkItem};
use sea_orm::QueryOrder;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::work_items::Collection;
use crate::auth::{self, MaybeAuthUser};
use crate::governance as rules;
use crate::models::_entities::{budget_lines, gate_reviews, proposals, risks, work_items};
use crate::models::audit_logs::Model as AuditModel;
use crate::models::governance as gov;
use crate::models::work_items::Model as WorkItemModel;
use crate::streaming;
use crate::validation::MAX_TEXT_LEN;

/// `422` with a joined problem list (family convention).
fn unprocessable(problems: &[String]) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("validation", &problems.join("; ")),
    )
}

/// One-problem `422` for state-machine refusals.
fn refuse(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

/// A permitted sponsor/approver/owner reference: an EntityRef-shaped
/// URN (`person:` / `worker:` / `organization:` + UUID). Format check
/// only — resolution is the reader's concern.
fn valid_ref(value: &str, schemes: &[&str]) -> bool {
    schemes.iter().any(|scheme| {
        value
            .strip_prefix(scheme)
            .and_then(|rest| rest.strip_prefix(':'))
            .is_some_and(|id| Uuid::parse_str(id).is_ok())
    })
}

fn cap(problems: &mut Vec<String>, field: &str, value: &str) {
    if value.len() > MAX_TEXT_LEN {
        problems.push(format!("{field} exceeds {MAX_TEXT_LEN} characters"));
    }
}

fn cap_opt(problems: &mut Vec<String>, field: &str, value: Option<&str>) {
    if let Some(v) = value {
        cap(problems, field, v);
    }
}

/// `POST /api/proposals` body (also the draft-update shape).
#[derive(Debug, Deserialize)]
struct ProposalPayload {
    title: String,
    #[serde(default)]
    summary: Option<String>,
    /// The collection an approved proposal becomes (`portfolios` /
    /// `projects` / `products` / `programs`).
    kind_target: String,
    #[serde(default)]
    sponsor_ref: Option<String>,
    #[serde(default)]
    strategic_rationale: Option<String>,
    #[serde(default)]
    requested_minor: Option<i64>,
    #[serde(default)]
    currency: Option<String>,
}

fn validate_proposal(p: &ProposalPayload) -> Vec<String> {
    let mut problems = Vec::new();
    if p.title.trim().is_empty() {
        problems.push("title is required".to_string());
    }
    cap(&mut problems, "title", &p.title);
    cap_opt(&mut problems, "summary", p.summary.as_deref());
    cap_opt(&mut problems, "strategic_rationale", p.strategic_rationale.as_deref());
    if Collection::from_segment(&p.kind_target).is_none() {
        problems.push(format!(
            "kind_target must be one of portfolios/projects/products/programs, got {:?}",
            p.kind_target
        ));
    }
    if let Some(sponsor) = p.sponsor_ref.as_deref()
        && !valid_ref(sponsor, &["person", "worker", "organization"]) {
            problems.push("sponsor_ref must be a person:/worker:/organization: URN".to_string());
        }
    if p.requested_minor.is_some() {
        match p.currency.as_deref() {
            Some(code) if rules::valid_currency(code) => {}
            _ => problems.push("currency (ISO 4217, e.g. GBP) is required with requested_minor".to_string()),
        }
    }
    problems
}

/// `POST /api/proposals` — open a demand record (status `draft`).
#[debug_handler]
async fn create_proposal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ProposalPayload>,
) -> Result<Response> {
    let problems = validate_proposal(&payload);
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let row = proposals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(payload.title.clone()),
        summary: ActiveValue::set(payload.summary.clone()),
        kind_target: ActiveValue::set(payload.kind_target.clone()),
        sponsor_ref: ActiveValue::set(payload.sponsor_ref.clone()),
        strategic_rationale: ActiveValue::set(payload.strategic_rationale.clone()),
        requested_minor: ActiveValue::set(payload.requested_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        status: ActiveValue::set("draft".to_string()),
        promoted_work_item_pid: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "proposal_created", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string(), "status": row.status }))
}

/// `GET /api/proposals?status=` — the intake board (default: every
/// non-promoted, most recent first, capped 200).
#[derive(Debug, Deserialize)]
struct ListParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_proposals(
    State(ctx): State<AppContext>,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let mut query = proposals::Entity::find().filter(proposals::Column::DeletedAt.is_null());
    if let Some(status) = &params.status {
        query = query.filter(proposals::Column::Status.eq(status.clone()));
    } else {
        query = query.filter(proposals::Column::Status.ne("promoted"));
    }
    let rows = query
        .order_by_desc(proposals::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    format::json(rows.into_iter().take(200).collect::<Vec<_>>())
}

/// `GET /api/proposals/{pid}`.
#[debug_handler]
async fn get_proposal(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    format::json(gov::find_proposal(&ctx.db, gov::parse_pid(&pid)?).await?)
}

/// `PUT /api/proposals/{pid}` — edit; drafts only (the pipeline owns
/// everything after submission).
#[debug_handler]
async fn update_proposal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ProposalPayload>,
) -> Result<Response> {
    let problems = validate_proposal(&payload);
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let row = gov::find_proposal(&ctx.db, gov::parse_pid(&pid)?).await?;
    if !rules::proposal_editable(&row.status) {
        return Err(refuse(&format!("a {} proposal is no longer editable", row.status)));
    }
    let row_pid = row.pid;
    let mut active: proposals::ActiveModel = row.into();
    active.title = ActiveValue::set(payload.title.clone());
    active.summary = ActiveValue::set(payload.summary.clone());
    active.kind_target = ActiveValue::set(payload.kind_target.clone());
    active.sponsor_ref = ActiveValue::set(payload.sponsor_ref.clone());
    active.strategic_rationale = ActiveValue::set(payload.strategic_rationale.clone());
    active.requested_minor = ActiveValue::set(payload.requested_minor);
    active.currency = ActiveValue::set(payload.currency.clone());
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, "proposal_updated", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// Run one pipeline action and audit it.
async fn transition_proposal(
    ctx: &AppContext,
    caller: &MaybeAuthUser,
    pid: &str,
    action: rules::ProposalAction,
) -> Result<proposals::Model> {
    let row = gov::find_proposal(&ctx.db, gov::parse_pid(pid)?).await?;
    let next = rules::proposal_transition(&row.status, action).map_err(|e| refuse(&e))?;
    let row_pid = row.pid;
    let mut active: proposals::ActiveModel = row.into();
    active.status = ActiveValue::set(next.to_string());
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row_pid, action.token(), caller.actor(), None)
        .await
        .ok();
    Ok(row)
}

macro_rules! pipeline_handler {
    ($fn_name:ident, $action:expr) => {
        #[debug_handler]
        async fn $fn_name(
            State(ctx): State<AppContext>,
            caller: MaybeAuthUser,
            Path(pid): Path<String>,
        ) -> Result<Response> {
            format::json(transition_proposal(&ctx, &caller, &pid, $action).await?)
        }
    };
}

pipeline_handler!(submit_proposal, rules::ProposalAction::Submit);
pipeline_handler!(review_proposal, rules::ProposalAction::Review);
pipeline_handler!(approve_proposal, rules::ProposalAction::Approve);
pipeline_handler!(reject_proposal, rules::ProposalAction::Reject);

/// `POST /api/proposals/{pid}/promote` — mint the work item from an
/// approved proposal (`provenance` recorded in the audit trail) and
/// mark the proposal `promoted`.
#[debug_handler]
async fn promote_proposal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = gov::find_proposal(&ctx.db, gov::parse_pid(&pid)?).await?;
    let next = rules::proposal_transition(&row.status, rules::ProposalAction::Promote)
        .map_err(|e| refuse(&e))?;
    let collection = Collection::from_segment(&row.kind_target)
        .ok_or_else(|| refuse("proposal has an unknown kind_target"))?;
    let mut work_item = WorkItem {
        kind: collection.kind(),
        name: row.title.clone(),
        ..WorkItem::default()
    };
    if let Some(summary) = &row.summary {
        work_item.keywords = vec![summary.clone()];
    }
    let model = streaming::create_and_emit(&ctx.db, collection.kind_str(), &work_item, caller.actor())
        .await?;
    let proposal_pid = row.pid;
    let mut active: proposals::ActiveModel = row.into();
    active.status = ActiveValue::set(next.to_string());
    active.promoted_work_item_pid = ActiveValue::set(Some(model.pid));
    let updated = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let snapshot = serde_json::json!({
        "work_item_pid": model.pid.to_string(),
        "collection": updated.kind_target,
        "provenance": "intake",
    });
    AuditModel::record(
        &ctx.db,
        proposal_pid,
        rules::ProposalAction::Promote.token(),
        caller.actor(),
        Some(snapshot),
    )
    .await
    .ok();
    format::json(serde_json::json!({
        "pid": updated.pid.to_string(),
        "status": updated.status,
        "work_item_pid": model.pid.to_string(),
        "collection": updated.kind_target,
    }))
}

/// A scored duplicate-demand hit.
#[derive(Debug, Serialize)]
struct DemandHit {
    source: &'static str,
    pid: String,
    name: String,
    score: f64,
}

/// `GET /api/proposals/{pid}/duplicates` — duplicate-demand check
/// (the registry heritage applied at intake): match the proposal's
/// title against the live work items of its target collection **and**
/// the other open proposals.
#[debug_handler]
async fn proposal_duplicates(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let row = gov::find_proposal(&ctx.db, gov::parse_pid(&pid)?).await?;
    let collection = Collection::from_segment(&row.kind_target)
        .ok_or_else(|| refuse("proposal has an unknown kind_target"))?;
    let engine = MatchingEngine::new(MatchConfig::default());
    let query = WorkItem {
        kind: collection.kind(),
        name: row.title.clone(),
        ..WorkItem::default()
    };
    let mut hits: Vec<DemandHit> = Vec::new();
    // Live work items of the target collection.
    let items = WorkItemModel::list(
        &ctx.db,
        collection.kind_str(),
        super::work_items::CHECK_DUPLICATES_SCAN_CAP,
    )
    .await?;
    for item in &items {
        let candidate = item.to_work_item()?;
        let result = engine.match_work_items(&query, &candidate);
        if result.is_match {
            hits.push(DemandHit {
                source: "work_item",
                pid: item.pid.to_string(),
                name: item.name.clone(),
                score: result.score,
            });
        }
    }
    // Other open proposals targeting the same collection.
    let siblings = proposals::Entity::find()
        .filter(proposals::Column::DeletedAt.is_null())
        .filter(proposals::Column::KindTarget.eq(row.kind_target.clone()))
        .filter(proposals::Column::Status.ne("promoted"))
        .filter(proposals::Column::Pid.ne(row.pid))
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    for sibling in &siblings {
        let candidate = WorkItem {
            kind: collection.kind(),
            name: sibling.title.clone(),
            ..WorkItem::default()
        };
        let result = engine.match_work_items(&query, &candidate);
        if result.is_match {
            hits.push(DemandHit {
                source: "proposal",
                pid: sibling.pid.to_string(),
                name: sibling.title.clone(),
                score: result.score,
            });
        }
    }
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    format::json(hits)
}

/// Resolve `{collection}/{pid}` to the stored work item (404 on
/// either being unknown).
async fn find_item(ctx: &AppContext, collection: &str, pid: &str) -> Result<work_items::Model> {
    let collection = Collection::from_segment(collection).ok_or(Error::NotFound)?;
    WorkItemModel::find_by_pid(&ctx.db, collection.kind_str(), pid)
        .await
        .map_err(super::model_not_found)
}

/// `POST /api/{collection}/{pid}/gate-reviews` body.
#[derive(Debug, Deserialize)]
struct GateReviewPayload {
    gate: String,
    decision: String,
    #[serde(default)]
    conditions: Option<String>,
    #[serde(default)]
    approver_ref: Option<String>,
}

/// `POST /api/{collection}/{pid}/gate-reviews` — record a phase-gate
/// decision; an approving decision advances the item's `stage`
/// (strictly in gate order). Record-level ABAC applies
/// (`resource.stage`), so gate-locking is policy.
#[debug_handler]
async fn create_gate_review(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<GateReviewPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    cap_opt(&mut problems, "conditions", payload.conditions.as_deref());
    if let Some(approver) = payload.approver_ref.as_deref()
        && !valid_ref(approver, &["worker", "person"]) {
            problems.push("approver_ref must be a worker:/person: URN".to_string());
        }
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let item = find_item(&ctx, &collection, &pid).await?;
    auth::authorize_record(
        &caller,
        authentication_verifier::Action::Write,
        &auth::work_item_resource_attrs(item.stage.as_deref()),
    )
    .map_err(super::record_rejection)?;
    let new_stage = rules::apply_gate_review(item.stage.as_deref(), &payload.gate, &payload.decision)
        .map_err(|e| refuse(&e))?;
    let review = gate_reviews::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        gate: ActiveValue::set(payload.gate.clone()),
        decision: ActiveValue::set(payload.decision.clone()),
        conditions: ActiveValue::set(payload.conditions.clone()),
        approver_ref: ActiveValue::set(payload.approver_ref.clone()),
        decided_at: ActiveValue::set(chrono::Utc::now().into()),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    let item_pid = item.pid;
    let old_stage = item.stage.clone();
    let mut active: work_items::ActiveModel = item.into();
    active.stage = ActiveValue::set(new_stage.clone());
    active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let snapshot = serde_json::json!({
        "gate": payload.gate,
        "decision": payload.decision,
        "from_stage": old_stage,
        "to_stage": new_stage,
        "approver_ref": payload.approver_ref,
    });
    AuditModel::record(&ctx.db, item_pid, "gate_reviewed", caller.actor(), Some(snapshot))
        .await
        .ok();
    format::json(serde_json::json!({
        "pid": review.pid.to_string(),
        "gate": review.gate,
        "decision": review.decision,
        "stage": new_stage,
    }))
}

/// `GET /api/{collection}/{pid}/gate-reviews`.
#[debug_handler]
async fn list_gate_reviews(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let reviews = gov::gate_reviews_for(&ctx.db, item.pid).await?;
    format::json(serde_json::json!({
        "stage": item.stage,
        "next_gate": rules::next_gate(item.stage.as_deref()),
        "reviews": reviews,
    }))
}

/// `POST /api/{collection}/{pid}/risks` body.
#[derive(Debug, Deserialize)]
struct RiskPayload {
    title: String,
    #[serde(default)]
    description: Option<String>,
    probability: i32,
    impact: i32,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    mitigation: Option<String>,
    #[serde(default)]
    review_date: Option<chrono::NaiveDate>,
}

/// `POST /api/{collection}/{pid}/risks` — raise a risk (status `open`).
#[debug_handler]
async fn create_risk(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<RiskPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if payload.title.trim().is_empty() {
        problems.push("title is required".to_string());
    }
    cap(&mut problems, "title", &payload.title);
    cap_opt(&mut problems, "description", payload.description.as_deref());
    cap_opt(&mut problems, "mitigation", payload.mitigation.as_deref());
    if let Err(e) = rules::risk_exposure(payload.probability, payload.impact) {
        problems.push(e);
    }
    if let Some(owner) = payload.owner_ref.as_deref()
        && !valid_ref(owner, &["worker", "person"]) {
            problems.push("owner_ref must be a worker:/person: URN".to_string());
        }
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let item = find_item(&ctx, &collection, &pid).await?;
    let row = risks::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        title: ActiveValue::set(payload.title.clone()),
        description: ActiveValue::set(payload.description.clone()),
        probability: ActiveValue::set(payload.probability),
        impact: ActiveValue::set(payload.impact),
        status: ActiveValue::set("open".to_string()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        mitigation: ActiveValue::set(payload.mitigation.clone()),
        review_date: ActiveValue::set(payload.review_date),
        escalated_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "risk_raised", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/{collection}/{pid}/risks` — active risks with derived
/// exposure, highest first.
#[debug_handler]
async fn list_risks(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let mut rows = gov::risks_for(&ctx.db, item.pid).await?;
    rows.sort_by_key(|r| -(r.probability * r.impact));
    let views: Vec<_> = rows
        .into_iter()
        .map(|r| {
            let exposure = r.probability * r.impact;
            serde_json::json!({
                "pid": r.pid.to_string(), "title": r.title, "status": r.status,
                "probability": r.probability, "impact": r.impact, "exposure": exposure,
                "owner_ref": r.owner_ref, "mitigation": r.mitigation,
                "review_date": r.review_date, "escalated_at": r.escalated_at,
            })
        })
        .collect();
    format::json(views)
}

/// `PUT /api/{collection}/{pid}/risks/{risk_pid}` — update scoring /
/// status / mitigation.
#[derive(Debug, Deserialize)]
struct RiskUpdate {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    probability: Option<i32>,
    #[serde(default)]
    impact: Option<i32>,
    #[serde(default)]
    mitigation: Option<String>,
    #[serde(default)]
    review_date: Option<chrono::NaiveDate>,
}

#[debug_handler]
async fn update_risk(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, risk_pid)): Path<(String, String, String)>,
    Json(payload): Json<RiskUpdate>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let risk = gov::find_risk(&ctx.db, gov::parse_pid(&risk_pid)?).await?;
    if risk.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    let mut problems = Vec::new();
    if let Some(status) = payload.status.as_deref()
        && !rules::is_token(rules::RISK_STATUSES, status) {
            problems.push(format!("status must be one of {:?}", rules::RISK_STATUSES));
        }
    let probability = payload.probability.unwrap_or(risk.probability);
    let impact = payload.impact.unwrap_or(risk.impact);
    if let Err(e) = rules::risk_exposure(probability, impact) {
        problems.push(e);
    }
    cap_opt(&mut problems, "mitigation", payload.mitigation.as_deref());
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let risk_row_pid = risk.pid;
    let mut active: risks::ActiveModel = risk.into();
    if let Some(status) = payload.status {
        active.status = ActiveValue::set(status);
    }
    active.probability = ActiveValue::set(probability);
    active.impact = ActiveValue::set(impact);
    if let Some(mitigation) = payload.mitigation {
        active.mitigation = ActiveValue::set(Some(mitigation));
    }
    if let Some(review_date) = payload.review_date {
        active.review_date = ActiveValue::set(Some(review_date));
    }
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, risk_row_pid, "risk_updated", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/{collection}/{pid}/risks/{risk_pid}/escalate` — a
/// materialised risk: status → `materialised`, stamped. (Conversion
/// into a tracked issue arrives with the issues sub-resource.)
#[debug_handler]
async fn escalate_risk(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, risk_pid)): Path<(String, String, String)>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let risk = gov::find_risk(&ctx.db, gov::parse_pid(&risk_pid)?).await?;
    if risk.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    if !matches!(risk.status.as_str(), "open" | "mitigating") {
        return Err(refuse(&format!("cannot escalate a {} risk", risk.status)));
    }
    let risk_row_pid = risk.pid;
    let mut active: risks::ActiveModel = risk.into();
    active.status = ActiveValue::set("materialised".to_string());
    active.escalated_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, risk_row_pid, "risk_escalated", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/{collection}/{pid}/budget-lines` body.
#[derive(Debug, Deserialize)]
struct BudgetPayload {
    category: String,
    description: String,
    currency: String,
    planned_minor: i64,
    #[serde(default)]
    period_start: Option<chrono::NaiveDate>,
    #[serde(default)]
    period_end: Option<chrono::NaiveDate>,
}

/// `POST /api/{collection}/{pid}/budget-lines`.
#[debug_handler]
async fn create_budget_line(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<BudgetPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if !rules::is_token(rules::BUDGET_CATEGORIES, &payload.category) {
        problems.push(format!("category must be one of {:?}", rules::BUDGET_CATEGORIES));
    }
    if payload.description.trim().is_empty() {
        problems.push("description is required".to_string());
    }
    cap(&mut problems, "description", &payload.description);
    if !rules::valid_currency(&payload.currency) {
        problems.push("currency must be an ISO 4217 code (e.g. GBP)".to_string());
    }
    if payload.planned_minor < 0 {
        problems.push("planned_minor must be non-negative".to_string());
    }
    if let (Some(start), Some(end)) = (payload.period_start, payload.period_end)
        && end < start {
            problems.push("period_end is before period_start".to_string());
        }
    if !problems.is_empty() {
        return Err(unprocessable(&problems));
    }
    let item = find_item(&ctx, &collection, &pid).await?;
    let row = budget_lines::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        category: ActiveValue::set(payload.category.clone()),
        description: ActiveValue::set(payload.description.clone()),
        currency: ActiveValue::set(payload.currency.clone()),
        planned_minor: ActiveValue::set(payload.planned_minor),
        actual_minor: ActiveValue::set(0),
        period_start: ActiveValue::set(payload.period_start),
        period_end: ActiveValue::set(payload.period_end),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| Error::Model(ModelError::from(e)))?;
    AuditModel::record(&ctx.db, row.pid, "budget_line_created", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/{collection}/{pid}/budget-lines` — lines + per-currency
/// totals (planned / actual / variance).
#[debug_handler]
async fn list_budget_lines(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let rows = gov::budget_lines_for(&ctx.db, item.pid).await?;
    format::json(serde_json::json!({
        "lines": rows,
        "totals": budget_totals(&rows),
    }))
}

/// Per-currency planned / actual / variance rollup.
fn budget_totals(rows: &[budget_lines::Model]) -> Vec<serde_json::Value> {
    let mut currencies: Vec<&str> = rows.iter().map(|r| r.currency.as_str()).collect();
    currencies.sort_unstable();
    currencies.dedup();
    currencies
        .into_iter()
        .map(|currency| {
            let planned: i64 = rows
                .iter()
                .filter(|r| r.currency == currency)
                .map(|r| r.planned_minor)
                .sum();
            let actual: i64 = rows
                .iter()
                .filter(|r| r.currency == currency)
                .map(|r| r.actual_minor)
                .sum();
            serde_json::json!({
                "currency": currency,
                "planned_minor": planned,
                "actual_minor": actual,
                "variance_minor": planned - actual,
            })
        })
        .collect()
}

/// `POST /api/{collection}/{pid}/budget-lines/{line_pid}/actual` —
/// record spend against a line (accumulates; negative adjustments
/// allowed; overflow refused).
#[derive(Debug, Deserialize)]
struct ActualPayload {
    amount_minor: i64,
    #[serde(default)]
    note: Option<String>,
}

#[debug_handler]
async fn record_actual(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, line_pid)): Path<(String, String, String)>,
    Json(payload): Json<ActualPayload>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let line = gov::find_budget_line(&ctx.db, gov::parse_pid(&line_pid)?).await?;
    if line.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    let next = rules::accumulate_actual(line.actual_minor, payload.amount_minor)
        .map_err(|e| refuse(&e))?;
    let line_pid = line.pid;
    let mut active: budget_lines::ActiveModel = line.into();
    active.actual_minor = ActiveValue::set(next);
    let row = active
        .update(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;
    let snapshot = serde_json::json!({
        "amount_minor": payload.amount_minor,
        "actual_minor": next,
        "note": payload.note,
    });
    AuditModel::record(&ctx.db, line_pid, "budget_actual_recorded", caller.actor(), Some(snapshot))
        .await
        .ok();
    format::json(row)
}

/// `GET /api/{collection}/{pid}/governance` — the per-item summary:
/// stage + next gate, risk posture, and budget totals in one read.
#[debug_handler]
async fn governance_summary(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = find_item(&ctx, &collection, &pid).await?;
    let reviews = gov::gate_reviews_for(&ctx.db, item.pid).await?;
    let risks = gov::risks_for(&ctx.db, item.pid).await?;
    let budgets = gov::budget_lines_for(&ctx.db, item.pid).await?;
    let open_risks: Vec<_> = risks
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .collect();
    format::json(serde_json::json!({
        "pid": item.pid.to_string(),
        "name": item.name,
        "stage": item.stage,
        "next_gate": rules::next_gate(item.stage.as_deref()),
        "gate_reviews": reviews.len(),
        "latest_review": reviews.last().map(|r| serde_json::json!({
            "gate": r.gate, "decision": r.decision, "decided_at": r.decided_at,
        })),
        "risks": {
            "open": open_risks.len(),
            "materialised": risks.iter().filter(|r| r.status == "materialised").count(),
            "max_exposure": open_risks.iter().map(|r| r.probability * r.impact).max(),
            "total_exposure": open_risks.iter().map(|r| r.probability * r.impact).sum::<i32>(),
        },
        "budget": budget_totals(&budgets),
    }))
}

/// The governance routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/proposals", post(create_proposal))
        .add("/proposals", get(list_proposals))
        .add("/proposals/{pid}", get(get_proposal))
        .add("/proposals/{pid}", put(update_proposal))
        .add("/proposals/{pid}/submit", post(submit_proposal))
        .add("/proposals/{pid}/review", post(review_proposal))
        .add("/proposals/{pid}/approve", post(approve_proposal))
        .add("/proposals/{pid}/reject", post(reject_proposal))
        .add("/proposals/{pid}/promote", post(promote_proposal))
        .add("/proposals/{pid}/duplicates", get(proposal_duplicates))
        .add("/{collection}/{pid}/gate-reviews", post(create_gate_review))
        .add("/{collection}/{pid}/gate-reviews", get(list_gate_reviews))
        .add("/{collection}/{pid}/risks", post(create_risk))
        .add("/{collection}/{pid}/risks", get(list_risks))
        .add("/{collection}/{pid}/risks/{risk_pid}", put(update_risk))
        .add("/{collection}/{pid}/risks/{risk_pid}/escalate", post(escalate_risk))
        .add("/{collection}/{pid}/budget-lines", post(create_budget_line))
        .add("/{collection}/{pid}/budget-lines", get(list_budget_lines))
        .add(
            "/{collection}/{pid}/budget-lines/{line_pid}/actual",
            post(record_actual),
        )
        .add("/{collection}/{pid}/governance", get(governance_summary))
}
