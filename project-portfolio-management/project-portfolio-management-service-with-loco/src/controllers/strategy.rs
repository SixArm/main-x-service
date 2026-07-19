//! PPM Phase-C strategy controllers (spec/15-roadmap PPM-2/4/5/11):
//! the idea funnel, what-if scenarios, OKR objectives + weighted
//! work-item mappings, and value-realization benefits. Pure rules
//! live in [`crate::strategy`]; every mutation audits; nothing here
//! feeds the matcher (§8 partition rule).

use axum::http::StatusCode;
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use super::governance::valid_ref;
use super::work_items::Collection;
use crate::auth::MaybeAuthUser;
use crate::governance as gov_rules;
use crate::models::_entities::{
    benefits, budget_lines, ideas, objective_links, objectives, proposals, risks, scenarios,
    work_items,
};
use crate::models::audit_logs::Model as AuditModel;
use crate::strategy as rules;
use crate::validation::MAX_TEXT_LEN;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// Generic active-row-by-pid finder over a strategy entity.
macro_rules! find_active {
    ($fn_name:ident, $module:ident) => {
        async fn $fn_name(ctx: &AppContext, pid: &str) -> Result<$module::Model> {
            let pid = Uuid::parse_str(pid).map_err(|_| Error::NotFound)?;
            $module::Entity::find()
                .filter($module::Column::Pid.eq(pid))
                .filter($module::Column::DeletedAt.is_null())
                .one(&ctx.db)
                .await
                .map_err(db_err)?
                .ok_or(Error::NotFound)
        }
    };
}

find_active!(find_idea, ideas);
find_active!(find_scenario, scenarios);
find_active!(find_objective, objectives);
find_active!(find_benefit, benefits);

/// `POST /api/ideas` body.
#[derive(Debug, Deserialize)]
struct IdeaPayload {
    title: String,
    #[serde(default)]
    pitch: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// `POST /api/ideas` — capture an idea (status `open`, zero votes).
#[debug_handler]
async fn create_idea(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<IdeaPayload>,
) -> Result<Response> {
    if payload.title.trim().is_empty() || payload.title.len() > MAX_TEXT_LEN {
        return Err(unprocessable("title is required (and capped)"));
    }
    if payload.tags.len() > 32 || payload.tags.iter().any(|t| t.trim().is_empty() || t.len() > 64) {
        return Err(unprocessable("tags: at most 32, non-blank, each ≤ 64 chars"));
    }
    let row = ideas::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(payload.title.clone()),
        pitch: ActiveValue::set(payload.pitch.clone()),
        tags: ActiveValue::set(serde_json::json!(payload.tags)),
        votes: ActiveValue::set(0),
        status: ActiveValue::set("open".to_string()),
        converted_proposal_pid: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(&ctx.db, row.pid, "idea_captured", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/ideas?status=` — most-voted first (default: open).
#[derive(Debug, Deserialize)]
struct IdeaParams {
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn list_ideas(
    State(ctx): State<AppContext>,
    Query(params): Query<IdeaParams>,
) -> Result<Response> {
    let status = params.status.unwrap_or_else(|| "open".to_string());
    let mut rows = ideas::Entity::find()
        .filter(ideas::Column::DeletedAt.is_null())
        .filter(ideas::Column::Status.eq(status))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    rows.sort_by_key(|i| -i.votes);
    format::json(rows.into_iter().take(200).collect::<Vec<_>>())
}

/// `POST /api/ideas/{pid}/vote` — +1.
#[debug_handler]
async fn vote_idea(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let idea = find_idea(&ctx, &pid).await?;
    if idea.status != "open" {
        return Err(unprocessable("only open ideas take votes"));
    }
    let row_pid = idea.pid;
    let next = idea.votes.saturating_add(1);
    let mut active: ideas::ActiveModel = idea.into();
    active.votes = ActiveValue::set(next);
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(&ctx.db, row_pid, "idea_voted", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/ideas/{pid}/dismiss`.
#[debug_handler]
async fn dismiss_idea(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let idea = find_idea(&ctx, &pid).await?;
    if idea.status != "open" {
        return Err(unprocessable(&format!("idea is {}", idea.status)));
    }
    let row_pid = idea.pid;
    let mut active: ideas::ActiveModel = idea.into();
    active.status = ActiveValue::set("dismissed".to_string());
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    AuditModel::record(&ctx.db, row_pid, "idea_dismissed", caller.actor(), None)
        .await
        .ok();
    format::json(row)
}

/// `POST /api/ideas/{pid}/convert` — mint a **draft proposal** from
/// an open idea (the funnel's next stage: idea → proposal → work
/// item; PPM-2).
#[derive(Debug, Deserialize)]
struct ConvertPayload {
    /// The collection the eventual work item belongs to.
    kind_target: String,
    #[serde(default)]
    sponsor_ref: Option<String>,
}

#[debug_handler]
async fn convert_idea(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ConvertPayload>,
) -> Result<Response> {
    if Collection::from_segment(&payload.kind_target).is_none() {
        return Err(unprocessable(
            "kind_target must be portfolios/projects/products/programs",
        ));
    }
    if let Some(sponsor) = payload.sponsor_ref.as_deref()
        && !valid_ref(sponsor, &["person", "worker", "organization"]) {
            return Err(unprocessable("sponsor_ref must be a person:/worker:/organization: URN"));
        }
    let idea = find_idea(&ctx, &pid).await?;
    if idea.status != "open" {
        return Err(unprocessable(&format!("idea is {}", idea.status)));
    }
    let proposal = proposals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(idea.title.clone()),
        summary: ActiveValue::set(idea.pitch.clone()),
        kind_target: ActiveValue::set(payload.kind_target.clone()),
        sponsor_ref: ActiveValue::set(payload.sponsor_ref.clone()),
        strategic_rationale: ActiveValue::set(None),
        requested_minor: ActiveValue::set(None),
        currency: ActiveValue::set(None),
        status: ActiveValue::set("draft".to_string()),
        promoted_work_item_pid: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    let idea_pid = idea.pid;
    let mut active: ideas::ActiveModel = idea.into();
    active.status = ActiveValue::set("converted".to_string());
    active.converted_proposal_pid = ActiveValue::set(Some(proposal.pid));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    let snapshot = serde_json::json!({
        "proposal_pid": proposal.pid.to_string(),
        "kind_target": payload.kind_target,
        "provenance": "idea",
    });
    AuditModel::record(&ctx.db, idea_pid, "idea_converted", caller.actor(), Some(snapshot))
        .await
        .ok();
    format::json(serde_json::json!({
        "pid": row.pid.to_string(),
        "status": row.status,
        "proposal_pid": proposal.pid.to_string(),
    }))
}

/// `POST /api/scenarios` body.
#[derive(Debug, Deserialize)]
struct ScenarioPayload {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    work_item_pids: Vec<Uuid>,
    #[serde(default)]
    proposal_pids: Vec<Uuid>,
    #[serde(default)]
    budget_cap_minor: Option<i64>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    must_include: Vec<Uuid>,
}

/// `POST /api/scenarios` — a named candidate portfolio (draft).
#[debug_handler]
async fn create_scenario(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ScenarioPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if payload.name.trim().is_empty() || payload.name.len() > MAX_TEXT_LEN {
        problems.push("name is required (and capped)".to_string());
    }
    if payload.work_item_pids.len() + payload.proposal_pids.len() > 256 {
        problems.push("at most 256 members".to_string());
    }
    if payload.budget_cap_minor.is_some() {
        match payload.currency.as_deref() {
            Some(code) if gov_rules::valid_currency(code) => {}
            _ => problems.push("currency (ISO 4217) is required with budget_cap_minor".to_string()),
        }
    }
    // Every named member must exist (unknown pids listed at once).
    for pid in &payload.work_item_pids {
        let exists = work_items::Entity::find()
            .filter(work_items::Column::Pid.eq(*pid))
            .filter(work_items::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .is_some();
        if !exists {
            problems.push(format!("unknown work item {pid}"));
        }
    }
    for pid in &payload.proposal_pids {
        let exists = proposals::Entity::find()
            .filter(proposals::Column::Pid.eq(*pid))
            .filter(proposals::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .is_some();
        if !exists {
            problems.push(format!("unknown proposal {pid}"));
        }
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let row = scenarios::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        description: ActiveValue::set(payload.description.clone()),
        members: ActiveValue::set(serde_json::json!({
            "work_item_pids": payload.work_item_pids,
            "proposal_pids": payload.proposal_pids,
        })),
        budget_cap_minor: ActiveValue::set(payload.budget_cap_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        must_include: ActiveValue::set(serde_json::json!(payload.must_include)),
        status: ActiveValue::set("draft".to_string()),
        committed_at: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(&ctx.db, row.pid, "scenario_created", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/scenarios`.
#[debug_handler]
async fn list_scenarios(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = scenarios::Entity::find()
        .filter(scenarios::Column::DeletedAt.is_null())
        .order_by_asc(scenarios::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(rows)
}

/// Prepare a scenario's member facts and run the pure evaluation.
pub(crate) async fn evaluate(ctx: &AppContext, scenario: &scenarios::Model) -> Result<rules::Evaluation> {
    let work_item_pids: Vec<Uuid> = scenario
        .members
        .get("work_item_pids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let proposal_pids: Vec<Uuid> = scenario
        .members
        .get("proposal_pids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let mut members = Vec::new();
    for pid in &work_item_pids {
        let budgets = budget_lines::Entity::find()
            .filter(budget_lines::Column::WorkItemPid.eq(*pid))
            .filter(budget_lines::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await
            .map_err(db_err)?;
        let mut planned: Vec<(String, i64)> = Vec::new();
        for line in &budgets {
            match planned.iter_mut().find(|(c, _)| *c == line.currency) {
                Some((_, total)) => *total = total.saturating_add(line.planned_minor),
                None => planned.push((line.currency.clone(), line.planned_minor)),
            }
        }
        let open_exposure: i32 = risks::Entity::find()
            .filter(risks::Column::WorkItemPid.eq(*pid))
            .filter(risks::Column::DeletedAt.is_null())
            .all(&ctx.db)
            .await
            .map_err(db_err)?
            .iter()
            .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
            .map(|r| r.probability * r.impact)
            .sum();
        let alignment_weight: i32 = objective_links::Entity::find()
            .filter(objective_links::Column::WorkItemPid.eq(*pid))
            .all(&ctx.db)
            .await
            .map_err(db_err)?
            .iter()
            .map(|l| l.weight)
            .sum();
        members.push(rules::MemberFact {
            pid: *pid,
            planned_by_currency: planned,
            open_exposure,
            alignment_weight,
        });
    }
    for pid in &proposal_pids {
        let proposal = proposals::Entity::find()
            .filter(proposals::Column::Pid.eq(*pid))
            .filter(proposals::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?;
        let planned = proposal
            .and_then(|p| Some((p.currency?, p.requested_minor?)))
            .map(|(currency, amount)| vec![(currency, amount)])
            .unwrap_or_default();
        members.push(rules::MemberFact {
            pid: *pid,
            planned_by_currency: planned,
            open_exposure: 0,
            alignment_weight: 0,
        });
    }
    let must_include: Vec<Uuid> =
        serde_json::from_value(scenario.must_include.clone()).unwrap_or_default();
    Ok(rules::evaluate_scenario(
        &members,
        &rules::Constraints {
            budget_cap_minor: scenario.budget_cap_minor,
            currency: scenario.currency.clone(),
            must_include,
        },
    ))
}

/// Query for `GET /api/scenarios/compare`: the two scenario pids.
#[derive(Debug, serde::Deserialize)]
struct CompareQuery {
    a: String,
    b: String,
}

/// `GET /api/scenarios/compare?a=&b=` — evaluate two scenarios live
/// (the same evaluation the single endpoint runs; nothing persisted)
/// and report them side by side with per-currency planned deltas
/// (`b - a`, same-currency only — currencies never merge), exposure
/// and alignment deltas, and both feasibility verdicts.
#[debug_handler]
async fn compare_scenarios(
    axum::extract::Query(query): axum::extract::Query<CompareQuery>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let load = |pid: &str| -> Result<Uuid> {
        Uuid::parse_str(pid).map_err(|_| Error::NotFound)
    };
    let (a_pid, b_pid) = (load(&query.a)?, load(&query.b)?);
    let mut pair = Vec::new();
    for pid in [a_pid, b_pid] {
        let scenario = scenarios::Entity::find()
            .filter(scenarios::Column::Pid.eq(pid))
            .filter(scenarios::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
            .ok_or(Error::NotFound)?;
        let evaluation = evaluate(&ctx, &scenario).await?;
        pair.push((scenario, evaluation));
    }
    let (b_side, b_eval) = pair.pop().expect("two loaded");
    let (a_side, a_eval) = pair.pop().expect("two loaded");
    // Per-currency planned delta (b - a); a currency present on only
    // one side still gets a row (the other side reads 0).
    let mut currencies: Vec<&str> = a_eval
        .planned_by_currency
        .iter()
        .chain(&b_eval.planned_by_currency)
        .map(|(c, _)| c.as_str())
        .collect();
    currencies.sort_unstable();
    currencies.dedup();
    let planned_delta: Vec<serde_json::Value> = currencies
        .into_iter()
        .map(|currency| {
            let of = |eval: &rules::Evaluation| {
                eval.planned_by_currency
                    .iter()
                    .find(|(c, _)| c == currency)
                    .map_or(0, |(_, minor)| *minor)
            };
            let (a_minor, b_minor) = (of(&a_eval), of(&b_eval));
            serde_json::json!({
                "currency": currency,
                "a_minor": a_minor,
                "b_minor": b_minor,
                "delta_minor": b_minor.saturating_sub(a_minor),
            })
        })
        .collect();
    format::json(serde_json::json!({
        "a": { "pid": a_side.pid, "name": a_side.name, "status": a_side.status,
               "feasible": a_eval.violations.is_empty(), "evaluation": a_eval },
        "b": { "pid": b_side.pid, "name": b_side.name, "status": b_side.status,
               "feasible": b_eval.violations.is_empty(), "evaluation": b_eval },
        "deltas": {
            "planned_by_currency": planned_delta,
            "exposure": b_eval.total_exposure - a_eval.total_exposure,
            "alignment": b_eval.total_alignment - a_eval.total_alignment,
        },
        "note": "b minus a; per-currency deltas only — currencies never merge",
    }))
}

/// `GET /api/scenarios/{pid}/evaluate` — what-if arithmetic over live
/// data: per-currency spend vs the cap, summed risk exposure, summed
/// OKR alignment, and named constraint violations.
#[debug_handler]
async fn evaluate_scenario(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let scenario = find_scenario(&ctx, &pid).await?;
    let evaluation = evaluate(&ctx, &scenario).await?;
    format::json(serde_json::json!({
        "pid": scenario.pid.to_string(),
        "name": scenario.name,
        "status": scenario.status,
        "evaluation": evaluation,
        "feasible": evaluation.violations.is_empty(),
    }))
}

/// `POST /api/scenarios/{pid}/commit` — freeze the choice: a feasible
/// draft becomes `committed` and the evaluation snapshot is audited
/// (the committed portfolio of record).
#[debug_handler]
async fn commit_scenario(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
) -> Result<Response> {
    let scenario = find_scenario(&ctx, &pid).await?;
    if scenario.status != "draft" {
        return Err(unprocessable(&format!("scenario is {}", scenario.status)));
    }
    let evaluation = evaluate(&ctx, &scenario).await?;
    if !evaluation.violations.is_empty() {
        return Err(unprocessable(&format!(
            "cannot commit an infeasible scenario: {}",
            evaluation.violations.join("; ")
        )));
    }
    let row_pid = scenario.pid;
    let mut active: scenarios::ActiveModel = scenario.into();
    active.status = ActiveValue::set("committed".to_string());
    active.committed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    let snapshot = serde_json::to_value(&evaluation).unwrap_or_default();
    AuditModel::record(&ctx.db, row_pid, "scenario_committed", caller.actor(), Some(snapshot))
        .await
        .ok();
    format::json(row)
}

/// `POST /api/objectives` body.
#[derive(Debug, Deserialize)]
struct ObjectivePayload {
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    period: Option<String>,
}

/// `POST /api/objectives` — register an OKR-level objective.
#[debug_handler]
async fn create_objective(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ObjectivePayload>,
) -> Result<Response> {
    if payload.title.trim().is_empty() || payload.title.len() > MAX_TEXT_LEN {
        return Err(unprocessable("title is required (and capped)"));
    }
    let row = objectives::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        title: ActiveValue::set(payload.title.clone()),
        description: ActiveValue::set(payload.description.clone()),
        period: ActiveValue::set(payload.period.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(&ctx.db, row.pid, "objective_created", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/objectives`.
#[debug_handler]
async fn list_objectives(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = objectives::Entity::find()
        .filter(objectives::Column::DeletedAt.is_null())
        .order_by_asc(objectives::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(rows)
}

/// `GET /api/objectives/{pid}/alignment` — which items serve the
/// objective, with weights and per-collection totals.
#[debug_handler]
async fn objective_alignment(
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let objective = find_objective(&ctx, &pid).await?;
    let links = objective_links::Entity::find()
        .filter(objective_links::Column::ObjectivePid.eq(objective.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let mut items = Vec::new();
    let mut per_collection: std::collections::BTreeMap<String, i32> =
        std::collections::BTreeMap::new();
    for link in &links {
        let Some(item) = work_items::Entity::find()
            .filter(work_items::Column::Pid.eq(link.work_item_pid))
            .filter(work_items::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
        else {
            continue;
        };
        *per_collection.entry(item.kind.clone()).or_default() += link.weight;
        items.push(serde_json::json!({
            "pid": item.pid.to_string(), "kind": item.kind, "name": item.name,
            "weight": link.weight,
        }));
    }
    format::json(serde_json::json!({
        "objective_pid": objective.pid.to_string(),
        "title": objective.title,
        "period": objective.period,
        "items": items,
        "weight_by_collection": per_collection,
        "total_weight": links.iter().map(|l| l.weight).sum::<i32>(),
    }))
}

/// `POST /api/{collection}/{pid}/objectives` body: a weighted mapping
/// (upserts on the pair — re-linking updates the weight).
#[derive(Debug, Deserialize)]
struct LinkPayload {
    objective_pid: Uuid,
    weight: i32,
}

#[debug_handler]
async fn link_objective(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<LinkPayload>,
) -> Result<Response> {
    if !rules::valid_weight(payload.weight) {
        return Err(unprocessable("weight must be 1–5"));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let objective = find_objective(&ctx, &payload.objective_pid.to_string()).await?;
    let existing = objective_links::Entity::find()
        .filter(objective_links::Column::ObjectivePid.eq(objective.pid))
        .filter(objective_links::Column::WorkItemPid.eq(item.pid))
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    let row = if let Some(existing) = existing {
        let mut active: objective_links::ActiveModel = existing.into();
        active.weight = ActiveValue::set(payload.weight);
        active.update(&ctx.db).await.map_err(db_err)?
    } else {
        objective_links::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            objective_pid: ActiveValue::set(objective.pid),
            work_item_pid: ActiveValue::set(item.pid),
            weight: ActiveValue::set(payload.weight),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .map_err(db_err)?
    };
    AuditModel::record(&ctx.db, row.pid, "objective_linked", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string(), "weight": row.weight }))
}

/// `GET /api/{collection}/{pid}/objectives` — the item's mappings.
#[debug_handler]
async fn item_objectives(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let links = objective_links::Entity::find()
        .filter(objective_links::Column::WorkItemPid.eq(item.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let mut out = Vec::new();
    for link in &links {
        let Some(objective) = objectives::Entity::find()
            .filter(objectives::Column::Pid.eq(link.objective_pid))
            .filter(objectives::Column::DeletedAt.is_null())
            .one(&ctx.db)
            .await
            .map_err(db_err)?
        else {
            continue;
        };
        out.push(serde_json::json!({
            "objective_pid": objective.pid.to_string(),
            "title": objective.title,
            "period": objective.period,
            "weight": link.weight,
        }));
    }
    format::json(out)
}

/// `POST /api/{collection}/{pid}/benefits` body.
#[derive(Debug, Deserialize)]
struct BenefitPayload {
    title: String,
    category: String,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    target_minor: Option<i64>,
    #[serde(default)]
    target_note: Option<String>,
    #[serde(default)]
    expected_on: Option<chrono::NaiveDate>,
}

/// `POST /api/{collection}/{pid}/benefits` — declare an expected
/// benefit (financial via minor units, or non-financial via note).
#[debug_handler]
async fn create_benefit(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid)): Path<(String, String)>,
    Json(payload): Json<BenefitPayload>,
) -> Result<Response> {
    let mut problems = Vec::new();
    if payload.title.trim().is_empty() || payload.title.len() > MAX_TEXT_LEN {
        problems.push("title is required (and capped)".to_string());
    }
    if !rules::BENEFIT_CATEGORIES.contains(&payload.category.as_str()) {
        problems.push(format!("category must be one of {:?}", rules::BENEFIT_CATEGORIES));
    }
    if payload.target_minor.is_some() {
        match payload.currency.as_deref() {
            Some(code) if gov_rules::valid_currency(code) => {}
            _ => problems.push("currency (ISO 4217) is required with target_minor".to_string()),
        }
    }
    if payload.target_minor.is_none() && payload.target_note.is_none() {
        problems.push("a benefit needs target_minor (financial) or target_note".to_string());
    }
    if !problems.is_empty() {
        return Err(unprocessable(&problems.join("; ")));
    }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let row = benefits::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        work_item_pid: ActiveValue::set(item.pid),
        title: ActiveValue::set(payload.title.clone()),
        category: ActiveValue::set(payload.category.clone()),
        currency: ActiveValue::set(payload.currency.clone()),
        target_minor: ActiveValue::set(payload.target_minor),
        realized_minor: ActiveValue::set(0),
        target_note: ActiveValue::set(payload.target_note.clone()),
        realized_note: ActiveValue::set(None),
        expected_on: ActiveValue::set(payload.expected_on),
        status: ActiveValue::set("planned".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(&ctx.db, row.pid, "benefit_declared", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/{collection}/{pid}/benefits` — the benefits plus
/// per-currency target/realized totals and a simple ROI against the
/// item's recorded budget actuals (basis points; absent when spend
/// is zero or currencies differ).
#[debug_handler]
async fn list_benefits(
    State(ctx): State<AppContext>,
    Path((collection, pid)): Path<(String, String)>,
) -> Result<Response> {
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let rows = benefits::Entity::find()
        .filter(benefits::Column::WorkItemPid.eq(item.pid))
        .filter(benefits::Column::DeletedAt.is_null())
        .order_by_asc(benefits::Column::Id)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let budget_rows = budget_lines::Entity::find()
        .filter(budget_lines::Column::WorkItemPid.eq(item.pid))
        .filter(budget_lines::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let mut currencies: Vec<&str> = rows.iter().filter_map(|b| b.currency.as_deref()).collect();
    currencies.sort_unstable();
    currencies.dedup();
    let totals: Vec<_> = currencies
        .into_iter()
        .map(|currency| {
            let target: i64 = rows
                .iter()
                .filter(|b| b.currency.as_deref() == Some(currency))
                .filter_map(|b| b.target_minor)
                .sum();
            let realized: i64 = rows
                .iter()
                .filter(|b| b.currency.as_deref() == Some(currency))
                .map(|b| b.realized_minor)
                .sum();
            let spend: i64 = budget_rows
                .iter()
                .filter(|l| l.currency == currency)
                .map(|l| l.actual_minor)
                .sum();
            serde_json::json!({
                "currency": currency,
                "target_minor": target,
                "realized_minor": realized,
                "spend_minor": spend,
                "roi_basis_points": rules::roi_basis_points(realized, spend),
            })
        })
        .collect();
    format::json(serde_json::json!({ "benefits": rows, "totals": totals }))
}

/// `POST /api/{collection}/{pid}/benefits/{b_pid}/realize` — record
/// realized value (accumulates; overflow refused) and optionally move
/// the status.
#[derive(Debug, Deserialize)]
struct RealizePayload {
    #[serde(default)]
    amount_minor: Option<i64>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    status: Option<String>,
}

#[debug_handler]
async fn realize_benefit(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path((collection, pid, b_pid)): Path<(String, String, String)>,
    Json(payload): Json<RealizePayload>,
) -> Result<Response> {
    if let Some(status) = payload.status.as_deref()
        && !rules::BENEFIT_STATUSES.contains(&status) {
            return Err(unprocessable(&format!(
                "status must be one of {:?}",
                rules::BENEFIT_STATUSES
            )));
        }
    let item = super::governance::find_item(&ctx, &collection, &pid).await?;
    let benefit = find_benefit(&ctx, &b_pid).await?;
    if benefit.work_item_pid != item.pid {
        return Err(Error::NotFound);
    }
    let next = match payload.amount_minor {
        Some(amount) => gov_rules::accumulate_actual(benefit.realized_minor, amount)
            .map_err(|e| unprocessable(&e))?,
        None => benefit.realized_minor,
    };
    let row_pid = benefit.pid;
    let mut active: benefits::ActiveModel = benefit.into();
    active.realized_minor = ActiveValue::set(next);
    if let Some(note) = payload.note.clone() {
        active.realized_note = ActiveValue::set(Some(note));
    }
    if let Some(status) = payload.status.clone() {
        active.status = ActiveValue::set(status);
    }
    let row = active.update(&ctx.db).await.map_err(db_err)?;
    let snapshot = serde_json::json!({
        "amount_minor": payload.amount_minor,
        "realized_minor": next,
        "note": payload.note,
        "status": payload.status,
    });
    AuditModel::record(&ctx.db, row_pid, "benefit_realized", caller.actor(), Some(snapshot))
        .await
        .ok();
    format::json(row)
}

/// The strategy routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/ideas", post(create_idea))
        .add("/ideas", get(list_ideas))
        .add("/ideas/{pid}/vote", post(vote_idea))
        .add("/ideas/{pid}/dismiss", post(dismiss_idea))
        .add("/ideas/{pid}/convert", post(convert_idea))
        .add("/scenarios", post(create_scenario))
        .add("/scenarios", get(list_scenarios))
        .add("/scenarios/compare", get(compare_scenarios))
        .add("/scenarios/{pid}/evaluate", get(evaluate_scenario))
        .add("/scenarios/{pid}/commit", post(commit_scenario))
        .add("/objectives", post(create_objective))
        .add("/objectives", get(list_objectives))
        .add("/objectives/{pid}/alignment", get(objective_alignment))
        .add("/{collection}/{pid}/objectives", post(link_objective))
        .add("/{collection}/{pid}/objectives", get(item_objectives))
        .add("/{collection}/{pid}/benefits", post(create_benefit))
        .add("/{collection}/{pid}/benefits", get(list_benefits))
        .add(
            "/{collection}/{pid}/benefits/{b_pid}/realize",
            post(realize_benefit),
        )
}
