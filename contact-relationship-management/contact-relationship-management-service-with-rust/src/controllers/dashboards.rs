//! Analytics & reporting (CRM-R13, CRM-R14): every number is a
//! pure-core derivation from recorded facts (CRM-D4) — per-currency,
//! honest ratios, `as_of`-stamped, ETag-conditional.

use std::hash::{DefaultHasher, Hash, Hasher};

use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, QueryOrder, QuerySelect};
use serde::Deserialize;

use super::sales;
use crate::metrics::Metrics;
use crate::models::_entities::{accounts, activities, deals, pipeline_stages, tickets};
use crate::models::records;
use crate::rules::{analytics, sla};

/// Weak-ETag helper over the payload **without** `as_of` (the tag
/// must be stable across reads of unchanged data).
fn etag_of(value: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    value.to_string().hash(&mut hasher);
    format!("W/\"{:x}\"", hasher.finish())
}

/// Wrap a dashboard payload: 304 on a matching `If-None-Match`, else
/// the payload + `as_of` with the `ETag` header.
fn conditional(headers: &HeaderMap, payload: serde_json::Value) -> Response {
    let tag = etag_of(&payload);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        == Some(tag.as_str())
    {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, tag)]).into_response();
    }
    let mut body = payload;
    if let Some(object) = body.as_object_mut() {
        object.insert("as_of".into(), serde_json::json!(chrono::Utc::now()));
    }
    (
        StatusCode::OK,
        [(header::ETAG, tag)],
        axum::Json(body),
    )
        .into_response()
}

/// `GET /api/dashboards/sales` — win rate + pipeline value by stage
/// (per currency) over live data.
#[debug_handler]
async fn sales_dashboard(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let (won, lost) = sales::closed_counts(&ctx.db).await?;
    let rate = analytics::win_rate(
        i64::try_from(won).unwrap_or(i64::MAX),
        i64::try_from(lost).unwrap_or(i64::MAX),
    );
    // Pipeline value by stage, per currency.
    let open = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::ClosedAt.is_null())
        .all(&ctx.db)
        .await?;
    let mut by_stage: std::collections::BTreeMap<String, serde_json::Value> =
        std::collections::BTreeMap::new();
    for deal in &open {
        let stage = records::find_stage(&ctx.db, deal.stage_pid).await?;
        let entry = by_stage
            .entry(stage.name.clone())
            .or_insert_with(|| serde_json::json!({ "count": 0, "totals_minor": {} }));
        entry["count"] = serde_json::json!(entry["count"].as_i64().unwrap_or(0) + 1);
        let totals = entry["totals_minor"].as_object_mut().expect("object");
        let current = totals.get(&deal.currency).and_then(serde_json::Value::as_i64).unwrap_or(0);
        totals.insert(
            deal.currency.clone(),
            serde_json::json!(current.saturating_add(deal.amount_minor)),
        );
    }
    Metrics::global()
        .deals_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    Ok(conditional(
        &headers,
        serde_json::json!({ "win_rate": rate, "open_deals": open.len(), "pipeline_by_stage": by_stage }),
    ))
}

/// `GET /api/dashboards/sla` — open tickets by priority × breach
/// state; live breach truth (CRM-R13).
#[debug_handler]
async fn sla_dashboard(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let open = tickets::Entity::find()
        .filter(tickets::Column::DeletedAt.is_null())
        .filter(tickets::Column::Status.is_in(["open", "pending"]))
        .all(&ctx.db)
        .await?;
    let mut by_priority: std::collections::BTreeMap<String, (i64, i64)> =
        std::collections::BTreeMap::new();
    for ticket in &open {
        let breached = ticket
            .first_response_due_at
            .is_some_and(|due| sla::first_response_breached(now, due, ticket.first_responded_at))
            || ticket
                .resolution_due_at
                .is_some_and(|due| sla::resolution_breached(now, due, ticket.resolved_at));
        let slot = by_priority.entry(ticket.priority.clone()).or_default();
        slot.0 += 1;
        if breached {
            slot.1 += 1;
        }
    }
    Metrics::global()
        .tickets_open
        .set(i64::try_from(open.len()).unwrap_or(i64::MAX));
    let rows: Vec<serde_json::Value> = by_priority
        .into_iter()
        .map(|(priority, (count, breached))| {
            serde_json::json!({ "priority": priority, "open": count, "breached": breached })
        })
        .collect();
    Ok(conditional(
        &headers,
        serde_json::json!({ "open_tickets": open.len(), "by_priority": rows }),
    ))
}

/// `GET /api/accounts/{pid}/clv` — CLV per currency over won deals
/// (CRM-R13; the v1 revenue-sum definition).
#[debug_handler]
async fn account_clv(State(ctx): State<AppContext>, Path(pid): Path<String>) -> Result<Response> {
    let account = records::find_account(&ctx.db, records::parse_pid(&pid)?).await?;
    let won = deals::Entity::find()
        .filter(deals::Column::DeletedAt.is_null())
        .filter(deals::Column::AccountPid.eq(account.pid))
        .filter(deals::Column::Won.eq(true))
        .all(&ctx.db)
        .await?;
    let inputs: Vec<(i64, String)> = won
        .iter()
        .map(|deal| (deal.amount_minor, deal.currency.clone()))
        .collect();
    let totals = analytics::clv_by_currency(&inputs)
        .map_err(|e| super::unprocessable(&e))?;
    format::json(serde_json::json!({
        "account_pid": account.pid,
        "won_deals": won.len(),
        "clv_minor": totals,
        "as_of": chrono::Utc::now(),
    }))
}

/// `GET /api/dashboards/activity?days=` — per-kind activity counts
/// over the window (CRM-R14).
#[derive(Debug, Deserialize)]
struct ActivityParams {
    #[serde(default = "default_days")]
    days: i64,
}

const fn default_days() -> i64 {
    30
}

#[debug_handler]
async fn activity_dashboard(
    State(ctx): State<AppContext>,
    Query(params): Query<ActivityParams>,
) -> Result<Response> {
    let since = chrono::Utc::now() - chrono::Duration::days(params.days.clamp(1, 365));
    let rows = activities::Entity::find()
        .filter(activities::Column::DeletedAt.is_null())
        .filter(activities::Column::OccurredAt.gte(since))
        .order_by_desc(activities::Column::OccurredAt)
        .limit(5000)
        .all(&ctx.db)
        .await?;
    let mut by_kind: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
    for activity in &rows {
        *by_kind.entry(activity.kind.clone()).or_insert(0) += 1;
    }
    format::json(serde_json::json!({
        "window_days": params.days,
        "total": rows.len(),
        "by_kind": by_kind,
        "as_of": chrono::Utc::now(),
    }))
}

/// Keep the helper imports honest (`accounts` + `pipeline_stages` are
/// used through the finders).
#[allow(dead_code)]
async fn _touch(db: &DatabaseConnection) -> Result<u64> {
    let a = accounts::Entity::find().count(db).await?;
    let s = pipeline_stages::Entity::find().count(db).await?;
    Ok(a + s)
}

/// The dashboard routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/dashboards/sales", get(sales_dashboard))
        .add("/dashboards/sla", get(sla_dashboard))
        .add("/dashboards/activity", get(activity_dashboard))
        .add("/accounts/{pid}/clv", get(account_clv))
}
