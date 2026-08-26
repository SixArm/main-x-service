//! **Flow Distribution** — the read surface over
//! [`crate::distribution`] (entity spec §9.2b / FR-31).
//!
//! The fifth Flow Framework metric, and the only one that was not
//! already computed under time-based-analysis vocabulary. The other
//! four are served by `/api/plans/{pid}/{time-analysis,flow}` and are
//! deliberately **not** re-exposed here under Flow-Framework names: the
//! same number behind two names is exactly what §1.6's mapping exists
//! to prevent.
//!
//! Counted over **completed** work in the window, because a mix of
//! what was *started* measures intent rather than delivery.

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::QuerySelect;
use serde::Deserialize;
use uuid::Uuid;

use crate::distribution as rules;
use crate::models::_entities::{plans, risks, tasks};
use crate::tba;

/// Default window, matching the flow-analysis default.
const DEFAULT_WINDOW_DAYS: i64 = 90;

/// Rows scanned per read, so an unbounded estate cannot become an
/// unbounded query.
const MAX_ROWS: u64 = 5000;

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

/// The deployment's declared intended mix, or `None`.
///
/// A malformed value is warned about and ignored **wholesale**, never
/// half-applied — the Smart Score weights posture.
fn intent() -> Vec<(rules::FlowType, i64)> {
    let raw = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_FLOW_INTENT").ok();
    match (raw.as_deref(), rules::parse_intent(raw.as_deref())) {
        (_, Some(parsed)) => parsed,
        (Some(raw), None) if !raw.trim().is_empty() => {
            tracing::warn!(
                "PROJECT_PORTFOLIO_MANAGEMENT_FLOW_INTENT is not a valid basis-point mix \
                 (e.g. `feature=6000,debt=2000`); reporting the mix without an intent"
            );
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Classified completed work for one plan within the window.
async fn items_for(
    ctx: &AppContext,
    plan_pid: Uuid,
    since: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<rules::FlowType>> {
    let task_rows = tasks::Entity::find()
        .filter(tasks::Column::PlanPid.eq(plan_pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .filter(tasks::Column::DoneAt.is_not_null())
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;

    let mut items: Vec<rules::FlowType> = task_rows
        .iter()
        .filter(|row| {
            row.done_at
                .is_some_and(|done| done.with_timezone(&chrono::Utc) >= since)
        })
        .map(|row| rules::FlowType::parse(row.flow_type.as_deref()))
        .collect();

    // Closed risk-register rows contribute the risk and debt this
    // service already tracks, so those two types are not left empty
    // merely because nobody declared a task for them.
    let risk_rows = risks::Entity::find()
        .filter(risks::Column::PlanPid.eq(plan_pid))
        .filter(risks::Column::DeletedAt.is_null())
        .filter(risks::Column::Status.eq("closed"))
        .limit(MAX_ROWS)
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    items.extend(
        risk_rows
            .iter()
            .filter_map(|row| rules::FlowType::from_risk_category(row.category.as_deref())),
    );

    Ok(items)
}

/// `GET /api/plans/{pid}/flow-distribution` query.
#[derive(Debug, Deserialize)]
struct Params {
    #[serde(default)]
    window_days: Option<i64>,
    #[serde(default)]
    rollup: Option<bool>,
    #[serde(default)]
    depth: Option<usize>,
}

/// `GET /api/plans/{pid}/flow-distribution` — the feature / defect /
/// risk / debt mix of completed work, optionally across the plan's
/// containment subtree.
#[debug_handler]
async fn flow_distribution(
    Path(pid): Path<String>,
    State(ctx): State<AppContext>,
    Query(params): Query<Params>,
    headers: HeaderMap,
) -> Result<Response> {
    let plan = find_plan(&ctx, &pid).await?;
    let window_days = params
        .window_days
        .filter(|d| *d > 0 && *d <= 3650)
        .unwrap_or(DEFAULT_WINDOW_DAYS);
    let since = chrono::Utc::now() - chrono::Duration::days(window_days);
    let declared = intent();

    let (items, scanned_plans, capped) = if params.rollup.unwrap_or(false) {
        // One query for the containment map, then the walk costs none —
        // the same N+1 avoidance the TBA rollup already uses.
        let all = plans::Entity::find()
            .filter(plans::Column::DeletedAt.is_null())
            .limit(MAX_ROWS)
            .all(&ctx.db)
            .await
            .map_err(db_err)?;
        let mut children: std::collections::BTreeMap<Uuid, Vec<Uuid>> =
            std::collections::BTreeMap::new();
        for row in &all {
            if let Some(parent) = row.parent_pid {
                children.entry(parent).or_default().push(row.pid);
            }
        }
        let depth = params.depth.unwrap_or(tba::MAX_ROLLUP_DEPTH);
        let walk = tba::walk_descendants(&children, plan.pid, tba::MAX_ROLLUP_NODES, depth);
        let mut all_items = Vec::new();
        for node in &walk.nodes {
            all_items.extend(items_for(&ctx, node.pid, since).await?);
        }
        (all_items, walk.nodes.len(), walk.truncated)
    } else {
        (items_for(&ctx, plan.pid, since).await?, 1, false)
    };

    let mix = rules::distribution(&items, &declared);
    let body = serde_json::json!({
        "plan_pid": plan.pid.to_string(),
        "window_days": window_days,
        "rollup": params.rollup.unwrap_or(false),
        "plans_counted": scanned_plans,
        // Reported rather than silent: a truncated walk is a smaller
        // answer than the caller asked for.
        "truncated": capped,
        "distribution": mix,
        "as_of": chrono::Utc::now(),
        "note": "Flow Time, Velocity, Efficiency and Load are served by \
                 /api/plans/{pid}/time-analysis and /flow — the same number \
                 is deliberately not published under two names.",
    });
    let etag = super::etag_of(&body);
    super::conditional_json(&headers, &etag, &body)
}

/// The Flow Distribution route.
#[must_use]
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/plans/{pid}/flow-distribution", get(flow_distribution))
}
