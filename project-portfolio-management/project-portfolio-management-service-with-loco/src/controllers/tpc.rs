//! **Total Project Control** (TPC) — the read/write surface over
//! [`crate::tpc`]. Entity spec §5.9.7 / FR-37; field set from
//! `spec/total-project-control/index.md`.
//!
//! DIPP asks the question earned value does not: *is the value still to
//! come worth the money still to spend?* Sunk cost appears nowhere in
//! it, which is the whole point — SPI and CPI measure conformance to a
//! baseline, DIPP measures whether continuing is rational.
//!
//! Three conventions, shared with the rest of this crate:
//!
//! - **Money is minor units, ratios are basis points, no float.**
//! - **Every ratio ships its own inputs**, so a reader can check the
//!   arithmetic rather than trust it (§9.2c response conventions).
//! - **Undefined is not zero.** `CEC = 0` reports `null` with a reason;
//!   a plan whose DIPP cannot be computed is set aside from the triage
//!   ranking rather than sorted last as though measured and bad.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::{QueryOrder, QuerySelect};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{plans, total_project_control as tpc_rows};
use crate::models::audit_logs::Model as AuditModel;
use crate::tpc as rules;

/// Plans scanned for the portfolio triage read.
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

/// A stored `NUMERIC` as `i64` minor units / basis points.
///
/// Out-of-range reads as absent rather than saturating: a silently
/// clamped money figure is worse than a missing one, because it still
/// looks like an answer.
fn to_i64(value: Option<sea_orm::prelude::Decimal>) -> Option<i64> {
    use std::str::FromStr;
    value.and_then(|d| i64::from_str(&d.trunc().to_string()).ok())
}

/// Turn a stored row into the pure layer's facts.
fn facts_of(row: &tpc_rows::Model) -> rules::TpcFacts {
    rules::TpcFacts {
        currency: row.currency.clone(),
        dipp: to_i64(row.total_project_control_dipp),
        expected_monetary_value: to_i64(Some(row.total_project_control_expected_monetary_value))
            .unwrap_or(0),
        cost_estimate_to_complete: to_i64(Some(
            row.total_project_control_cost_estimate_to_complete,
        ))
        .unwrap_or(0),
        dipp_progress_index_numerator: to_i64(
            row.total_project_control_dipp_progress_index_numerator,
        ),
        dipp_progress_index_denominator: to_i64(
            row.total_project_control_dipp_progress_index_denominator,
        ),
    }
}

/// `POST /api/plans/{pid}/tpc` body. Money in minor units of
/// `currency`; ratios in basis points.
#[derive(Debug, Deserialize)]
struct TpcPayload {
    currency: String,
    expected_monetary_value: i64,
    cost_estimate_to_complete: i64,
    #[serde(default)]
    dipp: Option<i64>,
    #[serde(default)]
    dipp_progress_index_numerator: Option<i64>,
    #[serde(default)]
    dipp_progress_index_denominator: Option<i64>,
}

/// `POST /api/plans/{pid}/tpc` — record one TPC observation.
///
/// A **negative expected monetary value is accepted deliberately**: a
/// project can be worth less than nothing to finish, and refusing to
/// record that would hide the one case the metric exists to expose. A
/// negative cost-estimate-to-complete is refused, because no such
/// estimate exists.
#[debug_handler]
async fn record(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<TpcPayload>,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;

    let currency = payload.currency.trim().to_uppercase();
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(unprocessable(
            "currency must be a three-letter ISO 4217 code",
        ));
    }
    if payload.cost_estimate_to_complete < 0 {
        return Err(unprocessable(
            "cost_estimate_to_complete must not be negative",
        ));
    }

    let row = tpc_rows::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        plan_pid: ActiveValue::set(plan.pid),
        currency: ActiveValue::set(currency),
        total_project_control_dipp: ActiveValue::set(payload.dipp.map(Into::into)),
        total_project_control_dipp_progress_index_numerator: ActiveValue::set(
            payload.dipp_progress_index_numerator.map(Into::into),
        ),
        total_project_control_dipp_progress_index_denominator: ActiveValue::set(
            payload.dipp_progress_index_denominator.map(Into::into),
        ),
        total_project_control_expected_monetary_value: ActiveValue::set(
            payload.expected_monetary_value.into(),
        ),
        total_project_control_cost_estimate_to_complete: ActiveValue::set(
            payload.cost_estimate_to_complete.into(),
        ),
        deleted_at: ActiveValue::set(None),
        // `..._ratio` is GENERATED ALWAYS in Postgres and is never
        // written here — that is what stops it disagreeing with the two
        // numbers beside it.
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(&ctx.db, plan.pid, "tpc_recorded", caller.actor(), None)
        .await
        .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// Newest-first observations for one plan.
async fn observations(ctx: &AppContext, plan_pid: Uuid) -> Result<Vec<tpc_rows::Model>> {
    tpc_rows::Entity::find()
        .filter(tpc_rows::Column::PlanPid.eq(plan_pid))
        .filter(tpc_rows::Column::DeletedAt.is_null())
        .order_by_desc(tpc_rows::Column::ObservedAt)
        .all(&ctx.db)
        .await
        .map_err(db_err)
}

/// `GET /api/plans/{pid}/tpc` — the observation history, newest first.
#[debug_handler]
async fn list(Path(pid): Path<String>, State(ctx): State<AppContext>) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let rows = observations(&ctx, plan.pid).await?;
    format::json(serde_json::json!(rows))
}

/// `GET /api/plans/{pid}/tpc/report` — the derived view over the newest
/// observation: DIPP, the progress index, the band, and the
/// stored-versus-computed divergence.
///
/// A plan with no observation is **not** an error and **not** a zero:
/// it reports `unmeasured`, the same posture the OKR and Smart Score
/// views take for absent evidence.
#[debug_handler]
async fn report(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let newest = observations(&ctx, plan.pid).await?.into_iter().next();

    let body = match newest {
        Some(row) => serde_json::json!({
            "plan_pid": plan.pid.to_string(),
            "observed_at": row.observed_at,
            "measured": true,
            "report": rules::report(&facts_of(&row)),
        }),
        None => serde_json::json!({
            "plan_pid": plan.pid.to_string(),
            "measured": false,
            "reason": "no TPC observation recorded for this plan",
        }),
    };
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// `GET /api/tpc?currency=GBP` — portfolio triage, highest DIPP first.
#[derive(Debug, Deserialize)]
struct TriageParams {
    #[serde(default)]
    currency: Option<String>,
}

/// `GET /api/tpc` — rank plans by DIPP descending within one currency.
///
/// Two exclusions are **reported rather than hidden**: a plan recorded
/// in another currency (this service never converts) and a plan whose
/// DIPP is undefined. Both would otherwise sort as if measured and bad.
#[debug_handler]
async fn triage(
    State(ctx): State<AppContext>,
    Query(params): Query<TriageParams>,
    headers: HeaderMap,
) -> Result<Response> {
    let currency = params
        .currency
        .unwrap_or_else(|| "GBP".to_string())
        .trim()
        .to_uppercase();

    let rows = tpc_rows::Entity::find()
        .filter(tpc_rows::Column::DeletedAt.is_null())
        .order_by_desc(tpc_rows::Column::ObservedAt)
        .limit(MAX_PLANS_SCANNED)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    // Newest observation per plan wins; the query is already newest
    // first, so the first sighting of a plan is the one to keep.
    let mut seen: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
    let mut entries: Vec<(Uuid, rules::TpcFacts)> = Vec::new();
    for row in &rows {
        if seen.insert(row.plan_pid) {
            entries.push((row.plan_pid, facts_of(row)));
        }
    }

    let (ranked, wrong_currency, undefined) = rules::triage(&entries, &currency);
    let body = serde_json::json!({
        "currency": currency,
        "ranked": ranked
            .iter()
            .map(|(plan_pid, dipp)| serde_json::json!({
                "plan_pid": plan_pid.to_string(),
                "dipp_basis_points": dipp,
            }))
            .collect::<Vec<_>>(),
        "excluded_other_currency": wrong_currency
            .iter().map(ToString::to_string).collect::<Vec<_>>(),
        "excluded_undefined_dipp": undefined
            .iter().map(ToString::to_string).collect::<Vec<_>>(),
        "scanned_cap": MAX_PLANS_SCANNED,
        "asserted": true,
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The TPC routes (entity spec §9.2c).
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/tpc", post(record))
        .add("/plans/{pid}/tpc", get(list))
        .add("/plans/{pid}/tpc/report", get(report))
        .add("/tpc", get(triage))
}
