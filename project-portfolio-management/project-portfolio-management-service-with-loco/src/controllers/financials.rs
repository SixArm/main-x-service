//! The phased budget baseline and the EAC/ETC forecast (T-28b, spec
//! `13-tasks.md`). Pure logic in [`crate::financials`].
//!
//! **Frozen at approval, append-only.** A baseline is never edited; a
//! re-baseline is a new row at `version + 1`, so a forecast that named
//! a version stays reproducible from that version forever, and a
//! version `> 1` requires a `reason` — the same distinction
//! `phase_transitions` draws between a plan's first phase and a later
//! regression.
//!
//! **This task does not build a finance connector.** Actuals still
//! arrive by hand (`POST /plans/{pid}/budget-lines`, PPM-10) or by a
//! future bulk import (T-8); this module only reads what is already
//! there.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect, TransactionTrait};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::financials as rules;
use crate::models::_entities::{
    budget_baseline_periods, budget_baselines, budget_lines, plans, total_project_control,
};
use crate::models::audit_logs::Model as AuditModel;
use crate::validation::{MAX_ARRAY_LEN, MAX_TEXT_LEN};

/// Plans scanned to build the containment map for the portfolio-wide
/// rollup (mirrors `controllers::tba::rollup`'s own cap).
const MAX_PLANS_SCANNED: u64 = 1000;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// Find one live plan by public id, or `404`.
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

fn valid_currency(raw: &str) -> Option<String> {
    let currency = raw.trim().to_uppercase();
    (currency.len() == 3 && currency.chars().all(|c| c.is_ascii_alphabetic())).then_some(currency)
}

/// `POST /api/plans/{pid}/budget-baselines` body.
#[derive(Debug, Deserialize)]
struct BaselinePayload {
    currency: String,
    periods: Vec<PeriodPayload>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PeriodPayload {
    period_start: chrono::NaiveDate,
    period_end: chrono::NaiveDate,
    planned_minor: i64,
}

/// The next version number for a plan's baseline sequence: `1` for the
/// first, else one past the highest existing version.
async fn next_version(ctx: &AppContext, plan_pid: Uuid) -> Result<i32> {
    let newest = budget_baselines::Entity::find()
        .filter(budget_baselines::Column::PlanPid.eq(plan_pid))
        .order_by_desc(budget_baselines::Column::Version)
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(newest.map_or(1, |row| row.version.saturating_add(1)))
}

/// `POST /api/plans/{pid}/budget-baselines` — approve a new baseline
/// version.
///
/// The **first** baseline needs no reason; every one after it does —
/// re-baselining without saying why is indistinguishable from a
/// mistake, the same rule a backward phase move already carries.
#[debug_handler]
async fn create(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<BaselinePayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;

    let Some(currency) = valid_currency(&payload.currency) else {
        return Err(unprocessable(
            "currency must be a three-letter ISO 4217 code",
        ));
    };
    if payload.periods.is_empty() {
        return Err(unprocessable("at least one period is required"));
    }
    if payload.periods.len() > MAX_ARRAY_LEN {
        return Err(unprocessable("too many periods"));
    }
    for period in &payload.periods {
        if period.period_end < period.period_start {
            return Err(unprocessable("a period's end must not precede its start"));
        }
        if period.planned_minor < 0 {
            return Err(unprocessable("planned_minor must not be negative"));
        }
    }

    let version = next_version(&ctx, plan.pid).await?;
    let reason = payload
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|r| !r.is_empty());
    if version > 1 && reason.is_none() {
        return Err(unprocessable(
            "a re-baseline (version > 1) requires a reason",
        ));
    }
    if reason.is_some_and(|r| r.len() > MAX_TEXT_LEN) {
        return Err(unprocessable("reason is too long"));
    }

    let txn = ctx.db.begin().await.map_err(db_err)?;
    let row = budget_baselines::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        version: ActiveValue::set(version),
        currency: ActiveValue::set(currency),
        approved_at: ActiveValue::set(chrono::Utc::now().into()),
        reason: ActiveValue::set(reason.map(ToString::to_string)),
        ..Default::default()
    }
    .insert(&txn)
    .await
    .map_err(db_err)?;
    for period in &payload.periods {
        budget_baseline_periods::ActiveModel {
            pid: ActiveValue::set(Uuid::new_v4()),
            baseline_pid: ActiveValue::set(row.pid),
            period_start: ActiveValue::set(period.period_start),
            period_end: ActiveValue::set(period.period_end),
            planned_minor: ActiveValue::set(period.planned_minor),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(db_err)?;
    }
    txn.commit().await.map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        plan.pid,
        "budget_baseline_approved",
        caller.actor(),
        Some(serde_json::json!({ "version": version })),
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string(), "version": version }))
}

/// `GET /api/plans/{pid}/budget-baselines` — every version, newest
/// first, with its periods — so an older figure stays reproducible
/// from the version it named.
#[debug_handler]
async fn list(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let baselines = budget_baselines::Entity::find()
        .filter(budget_baselines::Column::PlanPid.eq(plan.pid))
        .order_by_desc(budget_baselines::Column::Version)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let mut out = Vec::with_capacity(baselines.len());
    for baseline in baselines {
        let periods = budget_baseline_periods::Entity::find()
            .filter(budget_baseline_periods::Column::BaselinePid.eq(baseline.pid))
            .order_by_asc(budget_baseline_periods::Column::PeriodStart)
            .all(&ctx.db)
            .await
            .map_err(db_err)?;
        out.push(serde_json::json!({
            "pid": baseline.pid.to_string(),
            "version": baseline.version,
            "currency": baseline.currency,
            "approved_at": baseline.approved_at,
            "reason": baseline.reason,
            "periods": periods.into_iter().map(|p| serde_json::json!({
                "period_start": p.period_start,
                "period_end": p.period_end,
                "planned_minor": p.planned_minor,
            })).collect::<Vec<_>>(),
        }));
    }
    format::json(out)
}

/// Load the plan's latest baseline (with periods), if any.
async fn latest_baseline(ctx: &AppContext, plan_pid: Uuid) -> Result<Option<rules::Baseline>> {
    let Some(row) = budget_baselines::Entity::find()
        .filter(budget_baselines::Column::PlanPid.eq(plan_pid))
        .order_by_desc(budget_baselines::Column::Version)
        .one(&ctx.db)
        .await
        .map_err(db_err)?
    else {
        return Ok(None);
    };
    let periods = budget_baseline_periods::Entity::find()
        .filter(budget_baseline_periods::Column::BaselinePid.eq(row.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?
        .into_iter()
        .map(|p| rules::BaselinePeriod {
            period_start: p.period_start,
            period_end: p.period_end,
            planned_minor: p.planned_minor,
        })
        .collect();
    Ok(Some(rules::Baseline {
        version: row.version,
        currency: row.currency,
        periods,
    }))
}

/// A stored `NUMERIC` as `i64` minor units, matching `controllers::tpc`.
fn to_i64(value: Option<sea_orm::prelude::Decimal>) -> Option<i64> {
    use std::str::FromStr;
    value.and_then(|d| i64::from_str(&d.trunc().to_string()).ok())
}

/// The plan's latest TPC observation's own `(currency,
/// cost_estimate_to_complete)`, if any.
async fn latest_tpc_cec(ctx: &AppContext, plan_pid: Uuid) -> Result<Option<(String, i64)>> {
    let row = total_project_control::Entity::find()
        .filter(total_project_control::Column::PlanPid.eq(plan_pid))
        .filter(total_project_control::Column::DeletedAt.is_null())
        .order_by_desc(total_project_control::Column::ObservedAt)
        .one(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(row.and_then(|row| {
        to_i64(Some(row.total_project_control_cost_estimate_to_complete))
            .map(|cec| (row.currency, cec))
    }))
}

/// Every `(currency, actual_minor)` line recorded against the plan.
async fn actual_lines(ctx: &AppContext, plan_pid: Uuid) -> Result<Vec<(String, i64)>> {
    let rows = budget_lines::Entity::find()
        .filter(budget_lines::Column::PlanPid.eq(plan_pid))
        .filter(budget_lines::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    Ok(rows
        .into_iter()
        .map(|r| (r.currency, r.actual_minor))
        .collect())
}

/// Build one plan's forecast from its live inputs.
async fn forecast_of(ctx: &AppContext, plan_pid: Uuid) -> Result<rules::Forecast> {
    let baseline = latest_baseline(ctx, plan_pid).await?;
    let tpc_cec = latest_tpc_cec(ctx, plan_pid).await?;
    let lines = actual_lines(ctx, plan_pid).await?;
    let today = chrono::Utc::now().date_naive();
    Ok(rules::forecast(
        &lines,
        tpc_cec.as_ref().map(|(c, v)| (c.as_str(), *v)),
        baseline.as_ref(),
        today,
    ))
}

/// `GET /api/plans/{pid}/financials/forecast` — one plan's EAC/ETC.
///
/// **A plan without a baseline reports `null` with a reason,
/// unchanged** by this endpoint existing — the acceptance wording,
/// unless a TPC observation alone can still resolve a currency and an
/// ETC (see `crate::financials::forecast`'s own precedence).
#[debug_handler]
async fn forecast(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let forecast = forecast_of(&ctx, plan.pid).await?;
    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "forecast": forecast,
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// `GET /api/financials/forecast?plan=&depth=` query.
#[derive(Debug, Deserialize)]
struct RollupQuery {
    plan: String,
    #[serde(default)]
    depth: Option<usize>,
}

/// `GET /api/financials/forecast?plan=<pid>` — the EAC/ETC forecast
/// rolled over `parent_ref` from `plan`, one row **per currency**.
///
/// Reuses `tba::walk_descendants` — the same bounded, cycle-safe
/// containment walk `GET /plans/{pid}/rollup` already uses — so a
/// depth/node cap or a revisit is disclosed exactly the same way.
#[debug_handler]
async fn rollup(
    State(ctx): State<AppContext>,
    Query(query): Query<RollupQuery>,
    headers: HeaderMap,
) -> Result<Response> {
    let depth = query.depth.unwrap_or(crate::tba::MAX_ROLLUP_DEPTH);
    if depth == 0 || depth > crate::tba::MAX_ROLLUP_DEPTH {
        return Err(unprocessable(&format!(
            "depth must be between 1 and {}",
            crate::tba::MAX_ROLLUP_DEPTH
        )));
    }
    let root = find_plan(&ctx, &query.plan).await?;

    let all = plans::Entity::find()
        .filter(plans::Column::DeletedAt.is_null())
        .limit(MAX_PLANS_SCANNED)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let mut children: std::collections::BTreeMap<Uuid, Vec<Uuid>> =
        std::collections::BTreeMap::new();
    for plan in &all {
        if let Some(parent) = plan.parent_pid {
            children.entry(parent).or_default().push(plan.pid);
        }
    }

    let walk =
        crate::tba::walk_descendants(&children, root.pid, crate::tba::MAX_ROLLUP_NODES, depth);
    let mut forecasts = Vec::with_capacity(walk.nodes.len());
    for node in &walk.nodes {
        forecasts.push(forecast_of(&ctx, node.pid).await?);
    }
    let rollup = rules::rollup_forecast(&forecasts);

    let body = serde_json::json!({
        "as_of": chrono::Utc::now(),
        "root": { "pid": root.pid.to_string(), "name": root.name },
        "note": "one row per currency; currencies are never merged into one sum",
        "tree": {
            "plans": walk.nodes.len(),
            "depth_limit": depth,
            "truncated": walk.truncated,
            "revisits": walk.revisits,
        },
        "rollup": rollup,
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The financials routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/budget-baselines", post(create))
        .add("/plans/{pid}/budget-baselines", get(list))
        .add("/plans/{pid}/financials/forecast", get(forecast))
        .add("/financials/forecast", get(rollup))
}
