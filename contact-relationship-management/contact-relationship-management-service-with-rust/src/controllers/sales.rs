//! Sales automation (CRM-R3–R5): leads with deterministic scoring +
//! conversion, pipelines + stages, deals with Kanban stage moves and
//! terminal closes, and the stage-weighted forecast.

use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{ensure_valid, unprocessable};
use crate::auth::MaybeAuthUser;
use crate::metrics::Metrics;
use crate::models::_entities::{
    activities, contacts, deals, forecast_snapshots, leads, pipeline_stages, pipelines,
};
use crate::models::audit_logs::Model as Audit;
use crate::models::records;
use crate::rules::{analytics, lifecycle, scoring, tokens};
use crate::streaming;
use crate::validation::Problems;

/// `POST /api/leads` body.
#[derive(Debug, Deserialize)]
struct LeadPayload {
    display_name: String,
    source: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    campaign_pid: Option<Uuid>,
    #[serde(default)]
    contact_pid: Option<Uuid>,
}

/// `POST /api/leads/{pid}/status` body. `converted` may open a deal.
#[derive(Debug, Deserialize)]
struct LeadStatusPayload {
    to: String,
    /// On `converted`: the person URN for the new contact (when the
    /// lead is not yet linked to one).
    #[serde(default)]
    person_ref: Option<String>,
    /// On `converted`: optionally open a deal in this pipeline.
    #[serde(default)]
    deal: Option<DealPayload>,
}

/// `POST /api/pipelines` body: the named, ordered stage list.
#[derive(Debug, Deserialize)]
struct PipelinePayload {
    name: String,
    stages: Vec<StagePayload>,
}

/// One stage in a pipeline payload.
#[derive(Debug, Deserialize)]
struct StagePayload {
    name: String,
    probability_percent: i32,
    #[serde(default)]
    is_won: bool,
    #[serde(default)]
    is_lost: bool,
}

/// `POST /api/deals` body.
#[derive(Debug, Deserialize)]
struct DealPayload {
    name: String,
    pipeline_pid: Uuid,
    amount_minor: i64,
    currency: String,
    #[serde(default)]
    account_pid: Option<Uuid>,
    #[serde(default)]
    primary_contact_pid: Option<Uuid>,
    #[serde(default)]
    owner_ref: Option<String>,
    #[serde(default)]
    expected_close_on: Option<chrono::NaiveDate>,
    #[serde(default)]
    source_campaign_pid: Option<Uuid>,
}

/// `POST /api/deals/{pid}/stage` body — the Kanban move.
#[derive(Debug, Deserialize)]
struct DealStagePayload {
    stage_pid: Uuid,
    #[serde(default)]
    kanban_position: Option<i32>,
    /// Required when the target stage `is_lost`.
    #[serde(default)]
    lost_reason: Option<String>,
}

/// `POST /api/deals/{pid}/reopen` body.
#[derive(Debug, Deserialize)]
struct ReopenPayload {
    reason: String,
}

/// A `{pid}` reference response.
#[derive(Debug, Serialize)]
struct PidRef {
    pid: String,
}

/// Derive the scoring facts for a lead from its rows (activities +
/// linkage), then run the pure scorer.
async fn score_lead<C: sea_orm::ConnectionTrait>(
    db: &C,
    lead: &leads::Model,
) -> Result<scoring::ScoreBreakdown> {
    let activity_rows = activities::Entity::find()
        .filter(activities::Column::SubjectKind.eq("lead"))
        .filter(activities::Column::SubjectPid.eq(lead.pid))
        .filter(activities::Column::DeletedAt.is_null())
        .order_by_desc(activities::Column::OccurredAt)
        .all(db)
        .await?;
    let now = chrono::Utc::now();
    let days_since_last = activity_rows
        .first()
        .map(|a| (now - a.occurred_at.with_timezone(&chrono::Utc)).num_days());
    let facts = scoring::LeadFacts {
        source: lead.source.clone(),
        campaign_attributed: lead.campaign_pid.is_some(),
        known_contact: lead.contact_pid.is_some(),
        email_domain: lead.email_domain.clone(),
        days_since_last_activity: days_since_last,
        activity_count: i64::try_from(activity_rows.len()).unwrap_or(i64::MAX),
        campaign_click: lead.campaign_click,
        unsubscribed: lead.unsubscribed,
    };
    Ok(scoring::score(&facts, &scoring::Weights::default()))
}

/// `POST /api/leads` — capture (scored immediately).
#[debug_handler]
async fn create_lead(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<LeadPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("display_name", &payload.display_name);
    problems.require_token("source", tokens::LEAD_SOURCES, &payload.source);
    problems.cap_opt("email", payload.email.as_deref());
    ensure_valid(&problems.into_vec())?;
    if let Some(contact) = payload.contact_pid {
        records::find_contact(&ctx.db, contact).await?;
    }
    let email_domain = payload
        .email
        .as_deref()
        .and_then(|e| e.rsplit_once('@'))
        .map(|(_, domain)| domain.to_ascii_lowercase());
    let txn = ctx.db.begin().await?;
    let mut row = leads::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        source: ActiveValue::set(payload.source.clone()),
        campaign_pid: ActiveValue::set(payload.campaign_pid),
        contact_pid: ActiveValue::set(payload.contact_pid),
        display_name: ActiveValue::set(payload.display_name.clone()),
        email: ActiveValue::set(payload.email.clone()),
        email_domain: ActiveValue::set(email_domain),
        score: ActiveValue::set(0),
        campaign_click: ActiveValue::set(false),
        unsubscribed: ActiveValue::set(false),
        status: ActiveValue::set("new".to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let breakdown = score_lead(&txn, &row).await?;
    let mut active: leads::ActiveModel = row.clone().into();
    active.score = ActiveValue::set(breakdown.score);
    row = active.update(&txn).await?;
    Audit::record(&txn, "lead", row.pid, "lead_captured", caller.actor(), None).await?;
    streaming::emit_on(&txn, "lead", "lead_captured", &row.pid.to_string(), &row.display_name, caller.actor(), None).await?;
    txn.commit().await?;
    Metrics::global().lead_captured_total.inc();
    format::json(serde_json::json!({ "pid": row.pid, "score": breakdown }))
}

/// `GET /api/leads` — the queue, score-sorted (CRM-R3).
#[debug_handler]
async fn list_leads(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = leads::Entity::find()
        .filter(leads::Column::DeletedAt.is_null())
        .order_by_desc(leads::Column::Score)
        .limit(1000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `GET /api/leads/{pid}` — the lead + its live score breakdown.
#[debug_handler]
async fn get_lead(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let lead = records::find_lead(&ctx.db, records::parse_pid(&pid)?).await?;
    let breakdown = score_lead(&ctx.db, &lead).await?;
    format::json(serde_json::json!({ "lead": lead, "score": breakdown }))
}

/// `POST /api/leads/{pid}/status` — one lifecycle transition;
/// `converted` creates/links the Contact (+ optional deal) in one
/// transaction (CRM-R3, CRM-D9).
#[allow(clippy::too_many_lines)] // one linear walk incl. the in-tx conversion
#[debug_handler]
async fn lead_status(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<LeadStatusPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_token("to", tokens::LEAD_STATUSES, &payload.to);
    ensure_valid(&problems.into_vec())?;
    let lead = records::find_lead(&ctx.db, records::parse_pid(&pid)?).await?;
    lifecycle::check("lead", lifecycle::LEAD, &lead.status, &payload.to)
        .map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let from = lead.status.clone();
    let mut contact_pid = lead.contact_pid;
    let mut deal_pid = None;
    if payload.to == "converted" {
        // Create or link the contact.
        if contact_pid.is_none() {
            let person_ref = payload
                .person_ref
                .clone()
                .ok_or_else(|| unprocessable("conversion requires person_ref (no linked contact)"))?;
            let mut problems = Problems::new();
            problems.require_ref("person_ref", entity_ref::EntityType::Person, &person_ref);
            ensure_valid(&problems.into_vec())?;
            let contact = contacts::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                person_ref: ActiveValue::set(person_ref),
                account_pid: ActiveValue::set(None),
                owner_ref: ActiveValue::set(None),
                display_name: ActiveValue::set(lead.display_name.clone()),
                status: ActiveValue::set("active".to_string()),
                job_title: ActiveValue::set(None),
                preferred_channel: ActiveValue::set("email".to_string()),
                marketing_consent: ActiveValue::set("never".to_string()),
                consent_changed_at: ActiveValue::set(None),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            contact_pid = Some(contact.pid);
        }
        // Optionally open the deal (attributed to the lead's campaign).
        if let Some(deal) = &payload.deal {
            let pipeline = records::find_pipeline(&txn, deal.pipeline_pid).await?;
            let first_stage = pipeline_stages::Entity::find()
                .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
                .filter(pipeline_stages::Column::DeletedAt.is_null())
                .order_by_asc(pipeline_stages::Column::Position)
                .one(&txn)
                .await?
                .ok_or_else(|| unprocessable("pipeline has no stages"))?;
            if deal.amount_minor < 0 {
                return Err(unprocessable("amount_minor must be non-negative"));
            }
            let row = deals::ActiveModel {
                pid: ActiveValue::set(Uuid::new_v4()),
                account_pid: ActiveValue::set(deal.account_pid),
                primary_contact_pid: ActiveValue::set(contact_pid),
                owner_ref: ActiveValue::set(deal.owner_ref.clone()),
                pipeline_pid: ActiveValue::set(pipeline.pid),
                stage_pid: ActiveValue::set(first_stage.pid),
                name: ActiveValue::set(deal.name.clone()),
                amount_minor: ActiveValue::set(deal.amount_minor),
                currency: ActiveValue::set(deal.currency.clone()),
                expected_close_on: ActiveValue::set(deal.expected_close_on),
                kanban_position: ActiveValue::set(0),
                source_campaign_pid: ActiveValue::set(deal.source_campaign_pid.or(lead.campaign_pid)),
                closed_at: ActiveValue::set(None),
                won: ActiveValue::set(false),
                lost_reason: ActiveValue::set(None),
                deleted_at: ActiveValue::set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
            streaming::emit_on(&txn, "deal", "deal_opened", &row.pid.to_string(), &row.name, caller.actor(), None).await?;
            deal_pid = Some(row.pid);
        }
    }
    let mut active: leads::ActiveModel = lead.into();
    active.status = ActiveValue::set(payload.to.clone());
    active.contact_pid = ActiveValue::set(contact_pid);
    let row = active.update(&txn).await?;
    let kind = if payload.to == "converted" { "lead_converted" } else { "lead_status_changed" };
    Audit::record(
        &txn,
        "lead",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({ "from": from, "to": payload.to })),
    )
    .await?;
    streaming::emit_on(&txn, "lead", kind, &row.pid.to_string(), &row.display_name, caller.actor(), None).await?;
    txn.commit().await?;
    if payload.to == "converted" {
        Metrics::global().lead_converted_total.inc();
    }
    format::json(serde_json::json!({
        "pid": row.pid, "status": row.status,
        "contact_pid": contact_pid, "deal_pid": deal_pid,
    }))
}

/// `POST /api/pipelines` — create with its ordered stages.
#[debug_handler]
async fn create_pipeline(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<PipelinePayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    if payload.stages.len() < 2 || payload.stages.len() > 20 {
        problems.push("a pipeline needs 2-20 stages".to_string());
    }
    for stage in &payload.stages {
        problems.require_text("stages[].name", &stage.name);
        if !(0..=100).contains(&stage.probability_percent) {
            problems.push(format!("probability {} out of range 0-100", stage.probability_percent));
        }
        if stage.is_won && stage.is_lost {
            problems.push("a stage cannot be both won and lost".to_string());
        }
    }
    ensure_valid(&problems.into_vec())?;
    let txn = ctx.db.begin().await?;
    let pipeline = pipelines::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        name: ActiveValue::set(payload.name.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    let mut stage_pids = Vec::with_capacity(payload.stages.len());
    for (position, stage) in payload.stages.iter().enumerate() {
        let row = pipeline_stages::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            pipeline_pid: ActiveValue::set(pipeline.pid),
            name: ActiveValue::set(stage.name.clone()),
            position: ActiveValue::set(i32::try_from(position).unwrap_or(i32::MAX)),
            probability_percent: ActiveValue::set(stage.probability_percent),
            is_won: ActiveValue::set(stage.is_won),
            is_lost: ActiveValue::set(stage.is_lost),
            deleted_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        stage_pids.push(row.pid.to_string());
    }
    Audit::record(&txn, "pipeline", pipeline.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(serde_json::json!({ "pid": pipeline.pid, "stage_pids": stage_pids }))
}

/// `GET /api/pipelines` — pipelines with their stages.
#[debug_handler]
async fn list_pipelines(State(ctx): State<AppContext>) -> Result<Response> {
    let rows = pipelines::Entity::find()
        .filter(pipelines::Column::DeletedAt.is_null())
        .order_by_asc(pipelines::Column::Id)
        .all(&ctx.db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for pipeline in rows {
        let stages = pipeline_stages::Entity::find()
            .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
            .filter(pipeline_stages::Column::DeletedAt.is_null())
            .order_by_asc(pipeline_stages::Column::Position)
            .all(&ctx.db)
            .await?;
        out.push(serde_json::json!({ "pipeline": pipeline, "stages": stages }));
    }
    format::json(out)
}

/// `POST /api/deals` — open a deal in its pipeline's first stage.
#[debug_handler]
async fn create_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<DealPayload>,
) -> Result<Response> {
    let mut problems = Problems::new();
    problems.require_text("name", &payload.name);
    problems.require_text("currency", &payload.currency);
    problems.ref_opt("owner_ref", entity_ref::EntityType::Worker, payload.owner_ref.as_deref());
    if payload.amount_minor < 0 {
        problems.push("amount_minor must be non-negative".to_string());
    }
    ensure_valid(&problems.into_vec())?;
    let pipeline = records::find_pipeline(&ctx.db, payload.pipeline_pid).await?;
    let first_stage = pipeline_stages::Entity::find()
        .filter(pipeline_stages::Column::PipelinePid.eq(pipeline.pid))
        .filter(pipeline_stages::Column::DeletedAt.is_null())
        .order_by_asc(pipeline_stages::Column::Position)
        .one(&ctx.db)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no stages"))?;
    if let Some(account) = payload.account_pid {
        records::find_account(&ctx.db, account).await?;
    }
    let txn = ctx.db.begin().await?;
    let row = deals::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        account_pid: ActiveValue::set(payload.account_pid),
        primary_contact_pid: ActiveValue::set(payload.primary_contact_pid),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        pipeline_pid: ActiveValue::set(pipeline.pid),
        stage_pid: ActiveValue::set(first_stage.pid),
        name: ActiveValue::set(payload.name.clone()),
        amount_minor: ActiveValue::set(payload.amount_minor),
        currency: ActiveValue::set(payload.currency.clone()),
        expected_close_on: ActiveValue::set(payload.expected_close_on),
        kanban_position: ActiveValue::set(0),
        source_campaign_pid: ActiveValue::set(payload.source_campaign_pid),
        closed_at: ActiveValue::set(None),
        won: ActiveValue::set(false),
        lost_reason: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "deal", row.pid, "deal_opened", caller.actor(), None).await?;
    streaming::emit_on(&txn, "deal", "deal_opened", &row.pid.to_string(), &row.name, caller.actor(), None).await?;
    txn.commit().await?;
    format::json(PidRef { pid: row.pid.to_string() })
}

/// `GET /api/deals?pipeline=<pid>` — the board rows (open + closed).
#[derive(Debug, Deserialize)]
struct DealListParams {
    #[serde(default)]
    pipeline: Option<Uuid>,
}

#[debug_handler]
async fn list_deals(
    State(ctx): State<AppContext>,
    Query(params): Query<DealListParams>,
) -> Result<Response> {
    let mut query = deals::Entity::find().filter(deals::Column::DeletedAt.is_null());
    if let Some(pipeline) = params.pipeline {
        query = query.filter(deals::Column::PipelinePid.eq(pipeline));
    }
    let rows = query
        .order_by_asc(deals::Column::KanbanPosition)
        .limit(2000)
        .all(&ctx.db)
        .await?;
    format::json(rows)
}

/// `POST /api/deals/{pid}/stage` — the Kanban stage move: validates
/// pipeline membership, serializes on the locked deal row, closes on
/// a terminal stage (lost requires a reason) (CRM-R4, CRM-D9).
#[debug_handler]
async fn deal_stage(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<DealStagePayload>,
) -> Result<Response> {
    let pid = records::parse_pid(&pid)?;
    let txn = ctx.db.begin().await?;
    let deal = deals::Entity::find()
        .filter(deals::Column::Pid.eq(pid))
        .filter(deals::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    if deal.closed_at.is_some() {
        return Err(unprocessable("deal is closed — reopen it first"));
    }
    let stage = records::find_stage(&txn, payload.stage_pid).await?;
    if stage.pipeline_pid != deal.pipeline_pid {
        return Err(unprocessable("stage belongs to a different pipeline"));
    }
    if stage.is_lost && payload.lost_reason.as_deref().unwrap_or("").trim().is_empty() {
        return Err(unprocessable("a lost close requires lost_reason"));
    }
    let from_stage = deal.stage_pid;
    let name = deal.name.clone();
    let mut active: deals::ActiveModel = deal.into();
    active.stage_pid = ActiveValue::set(stage.pid);
    if let Some(position) = payload.kanban_position {
        active.kanban_position = ActiveValue::set(position);
    }
    let terminal = stage.is_won || stage.is_lost;
    if terminal {
        active.closed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        active.won = ActiveValue::set(stage.is_won);
        active.lost_reason = ActiveValue::set(payload.lost_reason.clone());
    }
    let row = active.update(&txn).await?;
    let kind = if stage.is_won {
        "deal_won"
    } else if stage.is_lost {
        "deal_lost"
    } else {
        "deal_stage_changed"
    };
    Audit::record(
        &txn,
        "deal",
        row.pid,
        kind,
        caller.actor(),
        Some(serde_json::json!({
            "from_stage": from_stage, "to_stage": stage.pid,
            "lost_reason": payload.lost_reason,
        })),
    )
    .await?;
    streaming::emit_on(&txn, "deal", kind, &row.pid.to_string(), &name, caller.actor(), None).await?;
    txn.commit().await?;
    match kind {
        "deal_won" => Metrics::global().deal_won_total.inc(),
        "deal_lost" => Metrics::global().deal_lost_total.inc(),
        _ => {}
    }
    format::json(row)
}

/// `POST /api/deals/{pid}/reopen` — reasoned reopen to the prior
/// (first non-terminal) stage (CRM-R4).
#[debug_handler]
async fn reopen_deal(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Path(pid): Path<String>,
    Json(payload): Json<ReopenPayload>,
) -> Result<Response> {
    if payload.reason.trim().is_empty() {
        return Err(unprocessable("a reopen requires a reason"));
    }
    let pid = records::parse_pid(&pid)?;
    let txn = ctx.db.begin().await?;
    let deal = deals::Entity::find()
        .filter(deals::Column::Pid.eq(pid))
        .filter(deals::Column::DeletedAt.is_null())
        .lock_exclusive()
        .one(&txn)
        .await?
        .ok_or(Error::NotFound)?;
    if deal.closed_at.is_none() {
        return Err(unprocessable("deal is not closed"));
    }
    // Reopen into the pipeline's last non-terminal stage.
    let target = pipeline_stages::Entity::find()
        .filter(pipeline_stages::Column::PipelinePid.eq(deal.pipeline_pid))
        .filter(pipeline_stages::Column::DeletedAt.is_null())
        .filter(pipeline_stages::Column::IsWon.eq(false))
        .filter(pipeline_stages::Column::IsLost.eq(false))
        .order_by_desc(pipeline_stages::Column::Position)
        .one(&txn)
        .await?
        .ok_or_else(|| unprocessable("pipeline has no open stage"))?;
    let mut active: deals::ActiveModel = deal.into();
    active.stage_pid = ActiveValue::set(target.pid);
    active.closed_at = ActiveValue::set(None);
    active.won = ActiveValue::set(false);
    active.lost_reason = ActiveValue::set(None);
    let row = active.update(&txn).await?;
    Audit::record(
        &txn,
        "deal",
        row.pid,
        "deal_reopened",
        caller.actor(),
        Some(serde_json::json!({ "reason": payload.reason })),
    )
    .await?;
    txn.commit().await?;
    format::json(row)
}

/// `GET /api/forecast` — the live stage-weighted forecast per
/// currency over open deals (CRM-R5); stamped `as_of`.
#[debug_handler]
async fn forecast(State(ctx): State<AppContext>) -> Result<Response> {
    let open = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut inputs = Vec::with_capacity(open.len());
    for deal in &open {
        let stage = records::find_stage(&ctx.db, deal.stage_pid).await?;
        inputs.push(analytics::OpenDeal {
            amount_minor: deal.amount_minor,
            currency: deal.currency.clone(),
            probability_percent: stage.probability_percent,
        });
    }
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    Metrics::global()
        .deals_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    format::json(serde_json::json!({
        "as_of": chrono::Utc::now(),
        "open_deals": open.len(),
        "totals_minor": totals,
    }))
}

/// `POST /api/forecast/snapshot` — freeze the current roll-up
/// (CRM-R5: an output, never an input).
#[debug_handler]
async fn forecast_snapshot(
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
) -> Result<Response> {
    let open = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut inputs = Vec::with_capacity(open.len());
    for deal in &open {
        let stage = records::find_stage(&ctx.db, deal.stage_pid).await?;
        inputs.push(analytics::OpenDeal {
            amount_minor: deal.amount_minor,
            currency: deal.currency.clone(),
            probability_percent: stage.probability_percent,
        });
    }
    let totals = analytics::forecast_by_currency(&inputs).map_err(|e| unprocessable(&e))?;
    let txn = ctx.db.begin().await?;
    let row = forecast_snapshots::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        taken_on: ActiveValue::set(chrono::Utc::now().date_naive()),
        totals: ActiveValue::set(serde_json::to_value(&totals).unwrap_or_default()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    Audit::record(&txn, "forecast_snapshot", row.pid, "created", caller.actor(), None).await?;
    txn.commit().await?;
    format::json(row)
}

/// Count helper for the win-rate dashboard (exposed here so sales
/// owns the deal queries).
pub(crate) async fn closed_counts(db: &DatabaseConnection) -> Result<(u64, u64)> {
    let won = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::Won.eq(true))
        .count(db)
        .await?;
    let lost = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_not_null())
        .filter(deals::Column::Won.eq(false))
        .count(db)
        .await?;
    Ok((won, lost))
}

/// The sales routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/leads", post(create_lead))
        .add("/leads", get(list_leads))
        .add("/leads/{pid}", get(get_lead))
        .add("/leads/{pid}/status", post(lead_status))
        .add("/pipelines", post(create_pipeline))
        .add("/pipelines", get(list_pipelines))
        .add("/deals", post(create_deal))
        .add("/deals", get(list_deals))
        .add("/deals/{pid}/stage", post(deal_stage))
        .add("/deals/{pid}/reopen", post(reopen_deal))
        .add("/forecast", get(forecast))
        .add("/forecast/snapshot", post(forecast_snapshot))
}
