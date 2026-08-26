//! **Realized gains** and **strategic performance** — the surface over
//! [`crate::value`] (entity spec §9.2c / FR-33, FR-34, FR-36).
//!
//! Every figure is derived on read. Nothing here is stored, so
//! recording a value point corrects every number resting on it rather
//! than leaving a stale one behind.
//!
//! The rule that runs through all of it: **absent evidence reports
//! `null` with a reason and sorts last, never `0`**. A plan with no
//! value points has not failed to deliver — it has not been measured,
//! and those are different findings.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use sea_orm::QuerySelect;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{
    adoption_snapshots, budget_lines, business_case_targets, plans, satisfaction_responses,
    value_points,
};
use crate::models::audit_logs::Model as AuditModel;
use crate::value as rules;

const MAX_ROWS: u64 = 5000;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

async fn find_plan(ctx: &AppContext, raw: &str) -> Result<plans::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    plans::Entity::find()
        .filter(plans::Column::Pid.eq(pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// `POST /api/plans/{pid}/business-case` body.
#[derive(Debug, Deserialize)]
struct TargetPayload {
    metric: String,
    baseline_value: i64,
    target_value: i64,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

/// `POST /api/plans/{pid}/business-case` — record a promised target.
///
/// `approved_at` is stamped here and has no update path: it is the
/// Time-to-Value clock start, and a clock start that can move is not a
/// measurement.
#[debug_handler]
async fn create_target(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TargetPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let source = payload.source.unwrap_or_else(|| "charter".to_string());
    if !["charter", "gate_review"].contains(&source.as_str()) {
        return Err(unprocessable("source must be charter or gate_review"));
    }
    if payload.metric.trim().is_empty() {
        return Err(unprocessable("metric is required"));
    }
    let row = business_case_targets::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        metric: ActiveValue::set(payload.metric.trim().to_string()),
        baseline_value: ActiveValue::set(payload.baseline_value),
        target_value: ActiveValue::set(payload.target_value),
        unit: ActiveValue::set(payload.unit.clone()),
        currency: ActiveValue::set(payload.currency.clone()),
        promised_by: ActiveValue::set(None),
        source: ActiveValue::set(source),
        approved_by_ref: ActiveValue::set(caller.actor().map(ToString::to_string)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        plan.pid,
        "business_case_target_set",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `POST /api/plans/{pid}/value-points` body.
#[derive(Debug, Deserialize)]
struct ValuePayload {
    value: i64,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    is_first_measurable: bool,
    #[serde(default)]
    evidence_ref: Option<String>,
}

/// `POST /api/plans/{pid}/value-points` — record observed value.
#[debug_handler]
async fn create_value_point(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<ValuePayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let row = value_points::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        benefit_pid: ActiveValue::set(None),
        value: ActiveValue::set(payload.value),
        currency: ActiveValue::set(payload.currency.clone()),
        is_first_measurable: ActiveValue::set(payload.is_first_measurable),
        method: ActiveValue::set(
            rules::Method::parse(payload.method.as_deref())
                .token()
                .to_string(),
        ),
        evidence_ref: ActiveValue::set(payload.evidence_ref.clone()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(|e| {
        // The partial unique index refuses a second first-measurable
        // point: the clock stops once.
        if e.to_string().contains("value_points_first") {
            unprocessable(
                "this plan already has a first-measurable value point: the \
                 Time-to-Value clock stops once",
            )
        } else {
            db_err(e)
        }
    })?;
    AuditModel::record(&ctx.db, plan.pid, "value_observed", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `POST /api/plans/{pid}/adoption` body.
#[derive(Debug, Deserialize)]
struct AdoptionPayload {
    active_users: i64,
    target_users: i64,
    window_days: i32,
    definition: String,
}

/// `POST /api/plans/{pid}/adoption` — record an adoption snapshot.
#[debug_handler]
async fn create_adoption(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<AdoptionPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    if payload.target_users <= 0 {
        return Err(unprocessable(
            "target_users must be positive: a rate with a zero denominator is \
             refused here rather than divided at read",
        ));
    }
    if payload.active_users < 0 || payload.window_days <= 0 {
        return Err(unprocessable(
            "active_users must not be negative and window_days must be positive",
        ));
    }
    if payload.definition.trim().is_empty() {
        return Err(unprocessable(
            "definition is required: `active user` is the term most easily \
             redefined between two readings, so it is stored beside the rate",
        ));
    }
    let row = adoption_snapshots::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        active_users: ActiveValue::set(payload.active_users),
        target_users: ActiveValue::set(payload.target_users),
        window_days: ActiveValue::set(payload.window_days),
        definition: ActiveValue::set(payload.definition.trim().to_string()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(&ctx.db, plan.pid, "adoption_recorded", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `POST /api/plans/{pid}/satisfaction` body.
#[derive(Debug, Deserialize)]
struct SatisfactionPayload {
    instrument: String,
    score: i16,
    respondent_role: String,
    #[serde(default)]
    comment: Option<String>,
}

/// `POST /api/plans/{pid}/satisfaction` — record a response.
#[debug_handler]
async fn create_satisfaction(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<SatisfactionPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    if !["nps", "csat"].contains(&payload.instrument.as_str()) {
        return Err(unprocessable("instrument must be nps or csat"));
    }
    if !(0..=10).contains(&payload.score) {
        return Err(unprocessable("score must be between 0 and 10"));
    }
    if !["sponsor", "user", "team", "other"].contains(&payload.respondent_role.as_str()) {
        return Err(unprocessable(
            "respondent_role must be sponsor, user, team or other",
        ));
    }
    let row = satisfaction_responses::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        instrument: ActiveValue::set(payload.instrument.clone()),
        score: ActiveValue::set(payload.score),
        respondent_role: ActiveValue::set(payload.respondent_role.clone()),
        comment: ActiveValue::set(payload.comment.clone()),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;
    AuditModel::record(
        &ctx.db,
        plan.pid,
        "satisfaction_recorded",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/plans/{pid}/value-realization` — the realized-gains view.
#[debug_handler]
async fn value_realization(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;

    let points = value_points::Entity::find()
        .filter(value_points::Column::PlanPid.eq(plan.pid))
        .filter(value_points::Column::DeletedAt.is_null())
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let targets = business_case_targets::Entity::find()
        .filter(business_case_targets::Column::PlanPid.eq(plan.pid))
        .filter(business_case_targets::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let budgets = budget_lines::Entity::find()
        .filter(budget_lines::Column::PlanPid.eq(plan.pid))
        .filter(budget_lines::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let adoption = adoption_snapshots::Entity::find()
        .filter(adoption_snapshots::Column::PlanPid.eq(plan.pid))
        .filter(adoption_snapshots::Column::DeletedAt.is_null())
        .order_by_desc(adoption_snapshots::Column::ObservedAt)
        .one(&ctx.db)
        .await
        .map_err(db_err)?;

    let approved_at = targets.iter().map(|t| t.approved_at).min();
    let facts: Vec<rules::ValueFact> = points
        .iter()
        .map(|row| rules::ValueFact {
            value: row.value,
            method: rules::Method::parse(Some(&row.method)),
            is_first_measurable: row.is_first_measurable,
            days_from_approval: approved_at.map(|start| {
                (row.observed_at.with_timezone(&chrono::Utc) - start.with_timezone(&chrono::Utc))
                    .num_days()
            }),
        })
        .collect();

    // Investment is **actual** cost, not planned: ROI on money not yet
    // spent is a forecast, and this view reports what happened.
    //
    // Budget lines carry a currency each, so a mixed-currency plan has
    // no single investment figure. This service converts nowhere, so
    // the ROI is withheld with `mixed_currency` rather than silently
    // adding pounds to euros — the restriction Smart Score already
    // carries.
    let currencies: std::collections::BTreeSet<&str> =
        budgets.iter().map(|line| line.currency.as_str()).collect();
    let mixed = currencies.len() > 1;
    let investment: i64 = budgets
        .iter()
        .fold(0_i64, |acc, line| acc.saturating_add(line.actual_minor));
    let roi = if mixed {
        rules::Roi {
            basis_points: None,
            absent: Some(rules::Absent::MixedCurrency),
            realized_minor: facts.iter().fold(0_i64, |a, f| a.saturating_add(f.value)),
            investment_minor: investment,
            measured_share_basis_points: None,
        }
    } else {
        rules::roi(&facts, investment)
    };

    let adoption_view = adoption.as_ref().map_or_else(
        || serde_json::json!({ "measured": false, "reason": "no adoption snapshot recorded" }),
        |snap| {
            let rate = rules::adoption_rate(snap.active_users, snap.target_users);
            serde_json::json!({
                "measured": rate.is_ok(),
                "basis_points": rate.ok(),
                "absent": rate.err(),
                "active_users": snap.active_users,
                "target_users": snap.target_users,
                // Returned with the rate, because a rate whose
                // definition is not visible cannot be compared.
                "definition": snap.definition,
                "window_days": snap.window_days,
            })
        },
    );

    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "transformation_roi": roi,
        "time_to_value": rules::time_to_value(&facts),
        "adoption": adoption_view,
        "performance_to_business_case": targets.iter().map(|t| serde_json::json!({
            "metric": t.metric,
            "baseline_value": t.baseline_value,
            "target_value": t.target_value,
            "promised_by": t.promised_by,
            "source": t.source,
            "approved_at": t.approved_at,
        })).collect::<Vec<_>>(),
        "as_of": chrono::Utc::now(),
        "asserted": true,
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// `GET /api/plans/{pid}/performance` — the six-dimension view.
#[debug_handler]
async fn performance(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let responses = satisfaction_responses::Entity::find()
        .filter(satisfaction_responses::Column::PlanPid.eq(plan.pid))
        .filter(satisfaction_responses::Column::DeletedAt.is_null())
        .filter(satisfaction_responses::Column::Instrument.eq("nps"))
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let scores: Vec<u8> = responses
        .iter()
        .filter_map(|r| u8::try_from(r.score).ok())
        .collect();

    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "stakeholder": { "nps": rules::nps(&scores) },
        // SPI and CPI need a phased budget baseline this service does
        // not yet hold. Reported as unmeasured **with the reason**,
        // rather than omitted (which would look like nothing to say) or
        // defaulted to 1.0 (which would say "exactly on plan").
        "schedule": {
            "spi": serde_json::Value::Null,
            "absent": "no_baseline",
            "reason": "no phased budget baseline: a plan without one is unmeasured, not on track",
        },
        "financial": {
            "cpi": serde_json::Value::Null,
            "absent": "no_baseline",
            "reason": "no phased budget baseline",
        },
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The value and performance routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/business-case", post(create_target))
        .add("/plans/{pid}/value-points", post(create_value_point))
        .add("/plans/{pid}/adoption", post(create_adoption))
        .add("/plans/{pid}/satisfaction", post(create_satisfaction))
        .add("/plans/{pid}/value-realization", get(value_realization))
        .add("/plans/{pid}/performance", get(performance))
}
