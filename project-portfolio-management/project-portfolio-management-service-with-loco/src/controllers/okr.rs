//! The **OKR engine** surface over [`crate::okr`] (entity spec §9.2b /
//! FR-27).
//!
//! Key results hang off an **objective** (which has a `pid`, a `period`
//! and weighted plan alignment), not off a plan's `goals[]` — those are
//! bare structs in the JSONB payload with no identifier, so a key result
//! bound to one would be orphaned by any reordering.
//!
//! **Every score is derived on read.** There is no stored progress
//! column, so recording a check-in corrects every figure that rests on
//! it rather than leaving a stale one behind.
//!
//! Three refusals worth knowing, because each is a number that would
//! otherwise look measured and be wrong:
//!
//! - An objective with no measurable key result is `unmeasured` and
//!   sorts **last**, never `0`.
//! - `start_value` cannot be updated: progress from a moving baseline
//!   is not progress.
//! - Confidence is recorded and **never** blended into a score.

use axum::http::{HeaderMap, StatusCode};
use loco_rs::controller::ErrorDetail;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::MaybeAuthUser;
use crate::models::_entities::{
    key_result_check_ins, key_results, objective_links, objectives, plans,
};
use crate::models::audit_logs::Model as AuditModel;
use crate::okr as rules;

fn unprocessable(message: &str) -> Error {
    Error::CustomError(
        StatusCode::UNPROCESSABLE_ENTITY,
        ErrorDetail::new("unprocessable", message),
    )
}

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

fn parse_metric(raw: &str) -> Option<rules::Metric> {
    match raw {
        "number" => Some(rules::Metric::Number),
        "percent" => Some(rules::Metric::Percent),
        "currency" => Some(rules::Metric::Currency),
        "boolean" => Some(rules::Metric::Boolean),
        _ => None,
    }
}

fn parse_direction(raw: &str) -> Option<rules::Direction> {
    match raw {
        "increase" => Some(rules::Direction::Increase),
        "decrease" => Some(rules::Direction::Decrease),
        "maintain" => Some(rules::Direction::Maintain),
        _ => None,
    }
}

/// Turn a stored row into the pure layer's facts.
fn fact_of(row: &key_results::Model) -> Option<rules::KeyResultFact> {
    Some(rules::KeyResultFact {
        metric: parse_metric(&row.metric)?,
        direction: parse_direction(&row.direction)?,
        start_value: row.start_value,
        target_value: row.target_value,
        current_value: row.current_value,
        tolerance: row.tolerance,
        currency: row.currency.clone(),
    })
}

async fn find_objective(ctx: &AppContext, raw: &str) -> Result<objectives::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    objectives::Entity::find()
        .filter(objectives::Column::Pid.eq(pid))
        .filter(objectives::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

async fn find_key_result(ctx: &AppContext, raw: &str) -> Result<key_results::Model> {
    let pid = Uuid::parse_str(raw).map_err(|_| Error::NotFound)?;
    key_results::Entity::find()
        .filter(key_results::Column::Pid.eq(pid))
        .filter(key_results::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)
}

/// Live key results for one objective.
async fn key_results_of(ctx: &AppContext, objective_pid: Uuid) -> Result<Vec<key_results::Model>> {
    key_results::Entity::find()
        .filter(key_results::Column::ObjectivePid.eq(objective_pid))
        .filter(key_results::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)
}

/// `POST /api/objectives/{pid}/key-results` body.
#[derive(Debug, Deserialize)]
struct KeyResultPayload {
    title: String,
    metric: String,
    direction: String,
    start_value: i64,
    target_value: i64,
    #[serde(default)]
    tolerance: Option<i64>,
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    owner_ref: Option<String>,
}

/// `POST /api/objectives/{pid}/key-results` — declare a key result.
///
/// `current_value` starts **at the baseline**, not at zero: a key result
/// running from 100 defects down to 0 is 0% done at 100, and seeding it
/// at zero would report it complete on the day it was created.
#[debug_handler]
async fn create_key_result(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<KeyResultPayload>,
) -> Result<Response> {
    let objective = find_objective(&ctx, &pid).await?;

    let Some(metric) = parse_metric(&payload.metric) else {
        return Err(unprocessable(
            "metric must be number, percent, currency or boolean",
        ));
    };
    let Some(direction) = parse_direction(&payload.direction) else {
        return Err(unprocessable(
            "direction must be increase, decrease or maintain",
        ));
    };
    if payload.title.trim().is_empty() || payload.title.len() > crate::validation::MAX_TEXT_LEN {
        return Err(unprocessable("title is required (and capped)"));
    }
    if direction == rules::Direction::Maintain && payload.tolerance.is_none() {
        return Err(unprocessable(
            "a `maintain` key result needs a `tolerance`: a band is what the \
             direction means, and without one it is unmeasurable",
        ));
    }
    if payload.tolerance.is_some_and(|t| t < 0) {
        return Err(unprocessable("tolerance must not be negative"));
    }
    if metric == rules::Metric::Currency && payload.currency.is_none() {
        return Err(unprocessable(
            "a currency-valued key result must name its currency: this service \
             never converts between them",
        ));
    }
    // A key result with nowhere to travel cannot report progress. Say so
    // at write, rather than letting it read `unmeasured` for a quarter.
    if direction != rules::Direction::Maintain && payload.start_value == payload.target_value {
        return Err(unprocessable(
            "start_value equals target_value, so there is no distance to travel \
             and progress can never be computed",
        ));
    }

    let row = key_results::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        objective_pid: ActiveValue::set(objective.pid),
        title: ActiveValue::set(payload.title.trim().to_string()),
        metric: ActiveValue::set(payload.metric.clone()),
        direction: ActiveValue::set(payload.direction.clone()),
        start_value: ActiveValue::set(payload.start_value),
        target_value: ActiveValue::set(payload.target_value),
        current_value: ActiveValue::set(payload.start_value),
        tolerance: ActiveValue::set(payload.tolerance),
        unit: ActiveValue::set(payload.unit.clone()),
        currency: ActiveValue::set(payload.currency.clone()),
        owner_ref: ActiveValue::set(payload.owner_ref.clone()),
        due_date: ActiveValue::set(None),
        deleted_at: ActiveValue::set(None),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        objective.pid,
        "key_result_declared",
        caller.actor(),
        None,
    )
    .await
    .ok();
    format::json(serde_json::json!({ "pid": row.pid.to_string() }))
}

/// `GET /api/objectives/{pid}/key-results` — with each one's derived
/// progress and, where absent, the reason.
#[debug_handler]
async fn list_key_results(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let objective = find_objective(&ctx, &pid).await?;
    let rows = key_results_of(&ctx, objective.pid).await?;
    let view: Vec<serde_json::Value> = rows
        .iter()
        .map(|row| {
            let (progress, reason) =
                fact_of(row).map_or((None, None), |fact| match rules::progress(&fact) {
                    Ok(value) => (Some(value), None),
                    Err(why) => (None, Some(why)),
                });
            serde_json::json!({
                "key_result": row,
                "progress_basis_points": progress,
                "unmeasurable": reason,
            })
        })
        .collect();
    format::json(serde_json::json!(view))
}

/// `POST /api/key-results/{pid}/check-ins` body.
#[derive(Debug, Deserialize)]
struct CheckInPayload {
    value: i64,
    #[serde(default)]
    confidence: Option<i16>,
    #[serde(default)]
    note: Option<String>,
}

/// `POST /api/key-results/{pid}/check-ins` — record an observation.
///
/// Advances `current_value`; **never** `start_value`. Confidence is
/// stored and never blended into any score.
#[debug_handler]
async fn create_check_in(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    caller: MaybeAuthUser,
    Json(payload): Json<CheckInPayload>,
) -> Result<Response> {
    let key_result = find_key_result(&ctx, &pid).await?;
    if payload.confidence.is_some_and(|c| !(0..=100).contains(&c)) {
        return Err(unprocessable("confidence must be between 0 and 100"));
    }

    let row = key_result_check_ins::ActiveModel {
        pid: ActiveValue::set(Uuid::new_v4()),
        key_result_pid: ActiveValue::set(key_result.pid),
        value: ActiveValue::set(payload.value),
        confidence: ActiveValue::set(payload.confidence),
        note: ActiveValue::set(payload.note.clone()),
        actor: ActiveValue::set(caller.actor().map(ToString::to_string)),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .map_err(db_err)?;

    // The observation moves the current value. The baseline is left
    // alone — deliberately, and there is no path here that touches it.
    let objective_pid = key_result.objective_pid;
    let mut active: key_results::ActiveModel = key_result.into();
    active.current_value = ActiveValue::set(payload.value);
    let updated = active.update(&ctx.db).await.map_err(db_err)?;

    AuditModel::record(
        &ctx.db,
        objective_pid,
        "key_result_checked_in",
        caller.actor(),
        None,
    )
    .await
    .ok();

    let progress = fact_of(&updated).and_then(|fact| rules::progress(&fact).ok());
    format::json(serde_json::json!({
        "pid": row.pid.to_string(),
        "current_value": updated.current_value,
        "progress_basis_points": progress,
        // Echoed so a reader can see it was recorded, and see that it
        // did not move the score.
        "confidence": payload.confidence,
    }))
}

/// `GET /api/key-results/{pid}/check-ins` — newest first.
#[debug_handler]
async fn list_check_ins(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let key_result = find_key_result(&ctx, &pid).await?;
    let rows = key_result_check_ins::Entity::find()
        .filter(key_result_check_ins::Column::KeyResultPid.eq(key_result.pid))
        .order_by_desc(key_result_check_ins::Column::ObservedAt)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    format::json(serde_json::json!(rows))
}

/// `GET /api/plans/{pid}/okr` — the plan's alignment-weighted score and
/// the objectives behind it.
///
/// An objective with no measurable key result reports `unmeasured` and
/// is excluded from the weighted mean — it must neither drag the plan
/// down nor silently lift it.
#[debug_handler]
async fn plan_okr(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan_pid = Uuid::parse_str(&pid).map_err(|_| Error::NotFound)?;
    let plan = plans::Entity::find()
        .filter(plans::Column::Pid.eq(plan_pid))
        .filter(plans::Column::DeletedAt.is_null())
        .one(&ctx.db)
        .await
        .map_err(db_err)?
        .ok_or(Error::NotFound)?;

    let links = objective_links::Entity::find()
        .filter(objective_links::Column::PlanPid.eq(plan.pid))
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let mut aligned = Vec::with_capacity(links.len());
    let mut detail = Vec::with_capacity(links.len());
    for link in &links {
        let rows = key_results_of(&ctx, link.objective_pid).await?;
        let facts: Vec<rules::KeyResultFact> = rows.iter().filter_map(fact_of).collect();
        let score = rules::objective_score(&facts);
        aligned.push(rules::AlignedObjective {
            score,
            weight: i64::from(link.weight),
        });
        detail.push(serde_json::json!({
            "objective_pid": link.objective_pid.to_string(),
            "weight": link.weight,
            "score_basis_points": score,
            "measured": score.is_some(),
            "key_results": rows.len(),
            "single_currency": rules::same_currency(&facts),
        }));
    }

    let score = rules::plan_score(&aligned);
    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "score_basis_points": score,
        // `measured: false` is not a zero — a plan whose objectives are
        // all unmeasured sorts last, it does not score bottom.
        "measured": score.is_some(),
        "objectives": detail,
        "as_of": chrono::Utc::now(),
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The OKR routes.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/objectives/{pid}/key-results", post(create_key_result))
        .add("/objectives/{pid}/key-results", get(list_key_results))
        .add("/key-results/{pid}/check-ins", post(create_check_in))
        .add("/key-results/{pid}/check-ins", get(list_check_ins))
        .add("/plans/{pid}/okr", get(plan_okr))
}
