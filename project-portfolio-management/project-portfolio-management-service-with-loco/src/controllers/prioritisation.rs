//! Data-driven prioritisation (**Smart Score**) and bird's-eye
//! **lifecycle visibility** — read-only derived views over rows the
//! service already stores. No new tables: a score that ranks somebody's
//! work must never be able to drift from the evidence behind it.
//!
//! - `GET /api/plans/{pid}/smart-score` — one plan's score **with its
//!   full breakdown**: every component's weight, normalised value, and
//!   points, plus the components that had no evidence and the
//!   `coverage` those gaps leave. A plan with no evidence scores
//!   `null`, never a confident-looking zero.
//! - `GET /api/prioritisation` — the ranked list: what deserves
//!   attention first, highest score first.
//! - `GET /api/lifecycle` — the challenge funnel across every phase,
//!   with the stalled counts.
//! - `GET /api/plans/{pid}/lifecycle` — one plan's phase, its next
//!   gate, and the readiness checklist blocking it.
//!
//! **Currency honesty.** ROI is only computed when a plan's benefit and
//! budget lines are all in **one** currency: the family forbids FX
//! conversion ([`crate::insights`]), so mixed-currency money is
//! reported as *no ROI evidence* rather than silently added up.
//!
//! Every response carries `as_of` and is ETag-conditional, mirroring
//! `/api/at-a-glance`.

use std::collections::BTreeSet;

use axum::http::HeaderMap;
use loco_rs::prelude::*;
use serde::Deserialize;

use crate::engineering::parse_moscow_tag;
use crate::lifecycle as life_rules;
use crate::models::_entities::{
    benefits, budget_lines, ideas, objective_links, plans, proposals, reviews, risks,
    scheduled_actions, tasks,
};
use crate::prioritisation as rules;
use crate::strategy::roi_basis_points;

/// Highest number of plans the ranked list scores in one request.
pub const RANK_CAP: usize = 500;

/// Days in a phase after which an item counts as stalled, unless
/// `PROJECT_PORTFOLIO_MANAGEMENT_STALL_DAYS` says otherwise.
pub const DEFAULT_STALL_DAYS: i64 = 30;

/// Risk exposure (probability × impact) at or above which an open risk
/// blocks phase readiness.
pub const SEVERE_RISK_EXPOSURE: i32 = 15;

fn db_err(e: sea_orm::DbErr) -> Error {
    Error::Model(ModelError::from(e))
}

/// Load all live rows of one entity.
macro_rules! live {
    ($module:ident, $db:expr) => {
        $module::Entity::find()
            .filter($module::Column::DeletedAt.is_null())
            .all($db)
            .await
            .map_err(db_err)?
    };
}

/// The configured Smart Score weights, or the documented defaults when
/// the environment holds nothing usable. A malformed weight map is
/// warned about and ignored wholesale rather than half-applied (the
/// ABAC-policy posture).
fn weights() -> rules::Weights {
    let raw = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS").ok();
    match (raw.as_deref(), rules::parse_weights(raw.as_deref())) {
        (_, Some(parsed)) => parsed,
        (Some(raw), None) if !raw.trim().is_empty() => {
            tracing::warn!(
                "PROJECT_PORTFOLIO_MANAGEMENT_SMART_SCORE_WEIGHTS is not a complete, \
                 10000-basis-point weight map; using the default weights"
            );
            rules::Weights::default()
        }
        _ => rules::Weights::default(),
    }
}

/// The stall threshold in days.
fn stall_days() -> i64 {
    std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_STALL_DAYS")
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|days| *days > 0)
        .unwrap_or(DEFAULT_STALL_DAYS)
}

/// Whole days between `then` and now, never negative.
fn days_since(then: chrono::DateTime<chrono::FixedOffset>) -> i64 {
    (chrono::Utc::now() - then.with_timezone(&chrono::Utc))
        .num_days()
        .max(0)
}

/// Everything the service knows, loaded once so a ranked list of N
/// plans costs a fixed number of queries rather than N × queries.
struct Estate {
    plans: Vec<plans::Model>,
    benefits: Vec<benefits::Model>,
    budget_lines: Vec<budget_lines::Model>,
    objective_links: Vec<objective_links::Model>,
    risks: Vec<risks::Model>,
    reviews: Vec<reviews::Model>,
}

impl Estate {
    async fn load(ctx: &AppContext) -> Result<Self> {
        Ok(Self {
            plans: live!(plans, &ctx.db),
            benefits: live!(benefits, &ctx.db),
            budget_lines: live!(budget_lines, &ctx.db),
            objective_links: objective_links::Entity::find()
                .all(&ctx.db)
                .await
                .map_err(db_err)?,
            risks: live!(risks, &ctx.db),
            reviews: live!(reviews, &ctx.db),
        })
    }

    /// Assemble one plan's score evidence.
    fn facts(&self, plan: &plans::Model) -> rules::ScoreFacts {
        // --- ROI, single-currency only (no FX conversion, ever) ------
        let plan_benefits: Vec<&benefits::Model> = self
            .benefits
            .iter()
            .filter(|b| b.plan_pid == plan.pid)
            .collect();
        let plan_budgets: Vec<&budget_lines::Model> = self
            .budget_lines
            .iter()
            .filter(|b| b.plan_pid == plan.pid)
            .collect();
        let currencies: BTreeSet<&str> = plan_benefits
            .iter()
            .filter_map(|b| b.currency.as_deref())
            .chain(plan_budgets.iter().map(|b| b.currency.as_str()))
            .collect();
        let roi = if currencies.len() == 1 && !plan_budgets.is_empty() {
            let realized: i64 = plan_benefits.iter().map(|b| b.realized_minor).sum();
            // Spend to date is the honest cost basis; a plan that has
            // not spent anything yet has no ROI evidence.
            let spent: i64 = plan_budgets.iter().map(|b| b.actual_minor).sum();
            roi_basis_points(realized, spent)
        } else {
            None
        };

        // --- strategic alignment: strongest objective link ----------
        let objective_weight = self
            .objective_links
            .iter()
            .filter(|l| l.plan_pid == plan.pid)
            .map(|l| i64::from(l.weight))
            .max();

        // --- expert review: mean submitted score --------------------
        let scores: Vec<i64> = self
            .reviews
            .iter()
            .filter(|r| {
                r.subject_kind == "plan" && r.subject_pid == plan.pid && r.status == "submitted"
            })
            .filter_map(|r| r.score.map(i64::from))
            .collect();
        let review_score = if scores.is_empty() {
            None
        } else {
            let len = i64::try_from(scores.len()).unwrap_or(1).max(1);
            Some(scores.iter().sum::<i64>() / len)
        };

        // --- risk: the worst open exposure --------------------------
        let risk_exposure = self
            .risks
            .iter()
            .filter(|r| {
                r.plan_pid == plan.pid && matches!(r.status.as_str(), "open" | "mitigating")
            })
            .map(|r| i64::from(r.probability) * i64::from(r.impact))
            .max();

        // --- priority: the MoSCoW tag on the payload ----------------
        let moscow_band = plan
            .data
            .get("tags")
            .and_then(serde_json::Value::as_array)
            .and_then(|tags| {
                tags.iter()
                    .filter_map(serde_json::Value::as_str)
                    .find_map(parse_moscow_tag)
            })
            .map(std::string::ToString::to_string);

        rules::ScoreFacts {
            roi_basis_points: roi,
            objective_weight,
            review_score,
            risk_exposure,
            // Votes live on the originating idea, which the plan does
            // not link back to; left as no evidence rather than a guess.
            votes: None,
            moscow_band,
            days_since_update: Some(days_since(plan.updated_at)),
        }
    }
}

/// `GET /api/plans/{pid}/smart-score` — one plan's score and the full
/// derivation behind it.
#[debug_handler]
async fn plan_smart_score(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(pid): Path<String>,
) -> Result<Response> {
    let plan = super::governance::find_item(&ctx, &pid).await?;
    let estate = Estate::load(&ctx).await?;
    let score = rules::smart_score(&estate.facts(&plan), &weights());
    let body = serde_json::json!({
        "pid": plan.pid.to_string(),
        "name": plan.name,
        "smart_score": score,
    });
    let etag = super::etag_of(&body);
    let mut with_as_of = body;
    with_as_of["as_of"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    super::conditional_json(&headers, &etag, &with_as_of)
}

/// `GET /api/prioritisation` filter.
#[derive(Debug, Deserialize)]
struct RankParams {
    /// How many rows to return (default 50, capped at [`RANK_CAP`]).
    #[serde(default)]
    limit: Option<usize>,
    /// Only rows in this band (`high` / `medium` / `low` / `unscored`).
    #[serde(default)]
    band: Option<String>,
}

/// `GET /api/prioritisation?limit=&band=` — the ranked queue: what
/// deserves attention first.
///
/// Unscored plans sort last and keep their `unscored` band, so "we have
/// no evidence about this" never reads as "this is low value".
#[debug_handler]
async fn prioritisation(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(params): Query<RankParams>,
) -> Result<Response> {
    let limit = params.limit.unwrap_or(50).min(RANK_CAP);
    let estate = Estate::load(&ctx).await?;
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let weights = weights();
    let mut scored: Vec<(Option<f64>, serde_json::Value)> = estate
        .plans
        .iter()
        .take(RANK_CAP)
        .map(|plan| {
            let score = rules::smart_score(&estate.facts(plan), &weights);
            (
                score.score,
                serde_json::json!({
                    "pid": plan.pid.to_string(),
                    "name": plan.name,
                    "kind": plan.kind,
                    "stage": plan.stage,
                    "score": score.score,
                    "band": score.band,
                    "coverage": score.coverage,
                    "missing_evidence": score.missing,
                }),
            )
        })
        .collect();
    // Highest first; unscored last, never mixed in as zeros.
    scored.sort_by(|a, b| match (a.0, b.0) {
        (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    for (_, row) in scored {
        if let Some(want) = params.band.as_deref()
            && row["band"] != serde_json::json!(want)
        {
            continue;
        }
        rows.push(row);
        if rows.len() >= limit {
            break;
        }
    }
    let body = serde_json::json!({ "plans": rows, "scored_of": estate.plans.len() });
    let etag = super::etag_of(&body);
    let mut with_as_of = body;
    with_as_of["as_of"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    super::conditional_json(&headers, &etag, &with_as_of)
}

/// `GET /api/lifecycle` — the bird's-eye funnel: how much sits in each
/// phase of the challenge lifecycle and how much of it has stalled.
#[debug_handler]
async fn lifecycle(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    let stall = stall_days();
    let mut items: Vec<life_rules::PhaseItem> = Vec::new();

    for idea in live!(ideas, &ctx.db) {
        if idea.status == "open" {
            items.push(life_rules::PhaseItem {
                phase: "idea".to_string(),
                days_in_phase: Some(days_since(idea.updated_at)),
            });
        }
    }
    for proposal in live!(proposals, &ctx.db) {
        if matches!(
            proposal.status.as_str(),
            "draft" | "submitted" | "in_review"
        ) {
            items.push(life_rules::PhaseItem {
                phase: "proposal".to_string(),
                days_in_phase: Some(days_since(proposal.updated_at)),
            });
        }
    }
    let plan_rows = plans::Entity::find()
        .filter(plans::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    for plan in &plan_rows {
        items.push(life_rules::PhaseItem {
            phase: life_rules::phase_of_plan(plan.stage.as_deref(), plan.active).to_string(),
            days_in_phase: Some(days_since(plan.updated_at)),
        });
    }
    let (phases, unknown_phase) = life_rules::funnel(&items, stall);
    let body = serde_json::json!({
        "phases": phases,
        "unknown_phase": unknown_phase,
        "stall_days": stall,
    });
    let etag = super::etag_of(&body);
    let mut with_as_of = body;
    with_as_of["as_of"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    super::conditional_json(&headers, &etag, &with_as_of)
}

/// `GET /api/plans/{pid}/lifecycle` — where one plan sits and what
/// stands between it and the next gate.
#[debug_handler]
async fn plan_lifecycle(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(pid): Path<String>,
) -> Result<Response> {
    let plan = super::governance::find_item(&ctx, &pid).await?;
    let plan_risks = risks::Entity::find()
        .filter(risks::Column::PlanPid.eq(plan.pid))
        .filter(risks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let plan_tasks = tasks::Entity::find()
        .filter(tasks::Column::PlanPid.eq(plan.pid))
        .filter(tasks::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let plan_reviews =
        crate::models::capabilities::reviews_for_subject(&ctx.db, "plan", plan.pid).await?;
    let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
    let overdue = scheduled_actions::Entity::find()
        .filter(scheduled_actions::Column::SubjectPid.eq(plan.pid))
        .filter(scheduled_actions::Column::Status.eq("pending"))
        .filter(scheduled_actions::Column::DueAt.lte(now))
        .filter(scheduled_actions::Column::DeletedAt.is_null())
        .all(&ctx.db)
        .await
        .map_err(db_err)?;
    let facts = life_rules::ReadinessFacts {
        stage: plan.stage.clone(),
        severe_open_risks: plan_risks
            .iter()
            .filter(|r| {
                matches!(r.status.as_str(), "open" | "mitigating")
                    && r.probability * r.impact >= SEVERE_RISK_EXPOSURE
            })
            .count(),
        outstanding_reviews: plan_reviews
            .iter()
            .filter(|r| crate::collaboration::LIVE_REVIEW_STATUSES.contains(&r.status.as_str()))
            .count(),
        overdue_actions: overdue.len(),
        blocked_tasks: plan_tasks.iter().filter(|t| t.status == "blocked").count(),
        open_tasks: plan_tasks.iter().filter(|t| t.status != "done").count(),
    };
    let body = serde_json::json!({
        "pid": plan.pid.to_string(),
        "name": plan.name,
        "phase": life_rules::phase_of_plan(plan.stage.as_deref(), plan.active),
        "readiness": life_rules::readiness(&facts),
        "review_consensus": super::collaboration::consensus_of(&plan_reviews),
    });
    let etag = super::etag_of(&body);
    let mut with_as_of = body;
    with_as_of["as_of"] = serde_json::json!(chrono::Utc::now().to_rfc3339());
    super::conditional_json(&headers, &etag, &with_as_of)
}

/// The prioritisation + lifecycle routes.
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/prioritisation", get(prioritisation))
        .add("/lifecycle", get(lifecycle))
        .add("/plans/{pid}/smart-score", get(plan_smart_score))
        .add("/plans/{pid}/lifecycle", get(plan_lifecycle))
}
