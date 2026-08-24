//! The time-based-analysis **flow gauges** refresh loop (spec §15
//! TBA-10).
//!
//! Prometheus gauges are pull-based: something has to compute the value
//! before a scrape reads it. Of the three options — compute on scrape,
//! update on write, refresh periodically — periodic is the only one
//! that works here.
//!
//! Computing on scrape puts a rollup over every plan on the scrape
//! path, so a 15-second scrape interval quietly becomes a 15-second
//! full-estate query, and an unauthenticated endpoint that does real
//! database work is a denial-of-service lever. Updating on write cannot
//! work at all: these are *derived* figures, and an item's flow
//! efficiency changes as it sits in a column — precisely the case with
//! no write to hang an update on.
//!
//! The loop is **off unless configured**:
//!
//! | Variable | Default | Effect |
//! |---|---|---|
//! | `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_SECS` | unset | Refresh interval. Unset or `0` ⇒ the loop never starts and the gauges never appear. |
//! | `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_MAX_PLANS` | 50 | Cap on per-plan series. |
//! | `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_MIN_TASKS` | 5 | Boards below this are counted, never labelled. |
//!
//! See [`crate::metrics`] for why the cap and the suppression floor are
//! not optional, and [`crate::tba::flow_metric_rows`] for the selection.

use std::time::Duration;

use loco_rs::prelude::*;
use sea_orm::QuerySelect;

use crate::metrics::Metrics;
use crate::models::_entities::plans;
use crate::tba;

/// Plans scanned per pass, matching the insight views' own cap.
const MAX_PLANS_SCANNED: u64 = 1000;

/// Parse a refresh interval, or `None` when the loop is switched off.
///
/// A zero or unparsable value reads as **off**, not as "every instant":
/// a typo in a deployment variable must not become a busy loop against
/// the database. Pure, so the matrix is testable without touching the
/// process environment.
#[must_use]
pub fn parse_interval(raw: Option<&str>) -> Option<Duration> {
    let secs: u64 = raw?.trim().parse().ok()?;
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Parse a positive bound, falling back to `default`.
///
/// Zero falls back rather than applying: a cap of zero would export
/// nothing while looking configured.
#[must_use]
pub fn parse_bound(raw: Option<&str>, default: usize) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// The refresh interval from the environment.
#[must_use]
pub fn interval_from_env() -> Option<Duration> {
    parse_interval(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_SECS")
            .ok()
            .as_deref(),
    )
}

/// The per-plan series cap.
#[must_use]
pub fn max_plans_from_env() -> usize {
    parse_bound(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_MAX_PLANS")
            .ok()
            .as_deref(),
        tba::DEFAULT_METRIC_MAX_PLANS,
    )
}

/// The board size below which a plan is counted but not labelled.
#[must_use]
pub fn min_tasks_from_env() -> usize {
    parse_bound(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_MIN_TASKS")
            .ok()
            .as_deref(),
        tba::DEFAULT_METRIC_MIN_TASKS,
    )
}

/// Run one refresh pass: roll every plan up, choose what may be
/// published, and write the gauges.
///
/// # Errors
///
/// Propagates a database error. The caller logs and retries on the next
/// tick rather than aborting the loop — a metrics refresh must never be
/// able to take the service down.
pub async fn refresh_once(ctx: &AppContext) -> Result<tba::FlowMetricSet> {
    let as_of = chrono::Utc::now();
    let as_of_ms = as_of.timestamp_millis();
    let classes = tba::classes_in_force();
    let limits = crate::engineering::parse_wip_limits(
        std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_WIP_LIMITS")
            .ok()
            .as_deref(),
    );

    let plans = plans::Entity::find()
        .filter(plans::Column::DeletedAt.is_null())
        .limit(MAX_PLANS_SCANNED)
        .all(&ctx.db)
        .await
        .map_err(|e| Error::Model(ModelError::from(e)))?;

    let mut samples = Vec::with_capacity(plans.len());
    for plan in &plans {
        let (rows, transitions) = crate::controllers::tba::load_board(ctx, plan.pid, None).await?;
        if rows.is_empty() {
            continue;
        }
        let paired =
            crate::controllers::tba::analyze_board(&rows, &transitions, &classes, as_of_ms);
        let analyses: Vec<tba::TaskAnalysis> = paired.into_iter().map(|(_, a)| a).collect();
        let rollup = tba::plan(&analyses);
        let cycle_times: Vec<i64> = analyses
            .iter()
            .filter(|a| a.finished)
            .filter_map(|a| a.cycle_time_ms)
            .collect();
        let sle = tba::service_level_expectation(&cycle_times, 0.85, None);

        // Columns over their cap. The per-column detail is deliberately
        // not exported (see `tba::flow_metric_rows`); the count is the
        // alertable fact.
        let columns_over_limit = limits.as_ref().map_or(0, |limits| {
            crate::engineering::TASK_STATUSES
                .iter()
                .filter(|status| {
                    limits.get(**status).is_some_and(|cap| {
                        rows.iter().filter(|task| task.status == **status).count() > *cap
                    })
                })
                .count()
        });

        samples.push(tba::PlanFlowSample {
            plan_pid: plan.pid.to_string(),
            analysis: rollup,
            sle,
            columns_over_limit,
        });
    }

    let set = tba::flow_metric_rows(&samples, max_plans_from_env(), min_tasks_from_env());
    #[allow(clippy::cast_precision_loss)] // seconds since the epoch
    Metrics::global().publish_flow(&set, as_of.timestamp() as f64);
    Ok(set)
}

/// Spawn the refresh loop. A no-op unless
/// `PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_SECS` is a positive
/// number.
pub fn spawn(ctx: &AppContext) {
    let Some(period) = interval_from_env() else {
        tracing::debug!(
            "flow metrics: PROJECT_PORTFOLIO_MANAGEMENT_FLOW_METRICS_SECS unset; \
             gauges not exported"
        );
        return;
    };
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(period);
        // A missed tick is skipped rather than queued: falling behind
        // must not produce a burst of catch-up passes.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match refresh_once(&ctx).await {
                Ok(set) => tracing::debug!(
                    exported = set.rows.len(),
                    suppressed = set.suppressed_plans,
                    dropped = set.dropped_plans,
                    "flow metrics refreshed"
                ),
                // Logged and retried next tick; the stale
                // `last_refresh_timestamp` is what a scraper alerts on.
                Err(error) => tracing::error!(%error, "flow metrics refresh failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The loop is off unless explicitly configured, and a nonsense
    /// value reads as off rather than as a busy loop.
    #[test]
    fn the_interval_is_off_by_default_and_on_bad_input() {
        assert_eq!(parse_interval(None), None, "unset ⇒ off");
        for raw in ["0", "", "   ", "nonsense", "-5", "1.5"] {
            assert_eq!(parse_interval(Some(raw)), None, "for {raw:?}");
        }
        assert_eq!(parse_interval(Some("60")), Some(Duration::from_mins(1)));
        assert_eq!(parse_interval(Some(" 90 ")), Some(Duration::from_secs(90)));
    }

    /// A bound falls back to its documented default rather than to zero
    /// — a cap of zero would export nothing while looking configured.
    #[test]
    fn a_bound_falls_back_rather_than_disabling_itself() {
        assert_eq!(parse_bound(None, 50), 50);
        assert_eq!(parse_bound(Some("0"), 50), 50, "zero falls back");
        assert_eq!(parse_bound(Some("oops"), 50), 50);
        assert_eq!(parse_bound(Some("-1"), 50), 50);
        assert_eq!(parse_bound(Some("12"), 50), 12);
    }

    /// The defaults are the ones the module docs promise.
    #[test]
    fn the_documented_defaults_hold() {
        assert_eq!(tba::DEFAULT_METRIC_MAX_PLANS, 50);
        assert_eq!(tba::DEFAULT_METRIC_MIN_TASKS, 5);
    }
}
