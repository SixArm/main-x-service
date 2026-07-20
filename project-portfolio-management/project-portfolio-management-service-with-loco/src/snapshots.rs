//! Point-in-time estate snapshots behind the board / CRO **trend**
//! views. A snapshot captures the current portfolio RAG counts, the
//! estate's open risk exposure, and the per-currency money totals into
//! `insight_snapshots`; trends read the stored series back — history
//! is only ever what was actually captured, never re-derived.
//!
//! Capture runs two ways: explicitly (`POST /api/board/snapshots`) or
//! via [`spawn`] — an optional in-process ticker gated by
//! `PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS` (unset/0 ⇒ off, the
//! default; parse failure ⇒ off with a warning).

use loco_rs::prelude::*;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::insights;
use crate::models::_entities::{budget_lines, insight_snapshots, risks, work_items};

/// Capture one `estate` snapshot row from live data and insert it.
///
/// # Errors
///
/// Returns the model error if a query or the insert fails.
pub async fn capture(db: &DatabaseConnection) -> Result<insight_snapshots::Model, ModelError> {
    let items = work_items::Entity::find()
        .filter(work_items::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let risk_rows = risks::Entity::find()
        .filter(risks::Column::DeletedAt.is_null())
        .all(db)
        .await?;
    let budget_rows = budget_lines::Entity::find()
        .filter(budget_lines::Column::DeletedAt.is_null())
        .all(db)
        .await?;

    let open_exposure: i32 = risk_rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "open" | "mitigating"))
        .map(|r| r.probability * r.impact)
        .sum();
    let lines: Vec<insights::MoneyLine> = budget_rows
        .iter()
        .map(|b| insights::MoneyLine {
            currency: b.currency.clone(),
            planned_minor: b.planned_minor,
            actual_minor: b.actual_minor,
        })
        .collect();
    let money = insights::variance_by_currency(&lines);
    let portfolios = items.iter().filter(|i| i.kind == "Portfolio").count();
    let body = serde_json::json!({
        "work_items": items.len(),
        "portfolios": portfolios,
        "open_exposure": open_exposure,
        "money": money,
    });
    insight_snapshots::ActiveModel {
        kind: ActiveValue::set("estate".to_string()),
        body: ActiveValue::set(body),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ModelError::from)
}

/// Start the optional snapshot ticker: every
/// `PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS` hours, capture one
/// estate snapshot. Unset / `0` / unparsable ⇒ no ticker (the
/// default); capture failures are warned and the loop continues.
pub fn spawn(db: DatabaseConnection) {
    let hours = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_SNAPSHOT_HOURS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(0);
    if hours == 0 {
        return;
    }
    tokio::spawn(async move {
        let period = std::time::Duration::from_secs(hours.saturating_mul(3600));
        loop {
            tokio::time::sleep(period).await;
            if let Err(err) = capture(&db).await {
                tracing::warn!("estate snapshot capture failed: {err}");
            }
        }
    });
}
