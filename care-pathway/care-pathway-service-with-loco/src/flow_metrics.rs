//! The time-based-analysis **flow gauges** refresh loop (spec §15
//! TBA-11).
//!
//! Prometheus gauges are pull-based: something has to compute the value
//! before a scrape reads it. The three options were to compute on
//! scrape, to update on write, or to refresh periodically.
//!
//! **Periodic wins here, and the other two are worse in specific ways.**
//! Computing on scrape puts a cohort analysis over every pathway on the
//! scrape path, so a 15-second scrape interval quietly becomes a
//! 15-second full-estate query — and an unauthenticated endpoint that
//! does real database work is a denial-of-service lever. Updating on
//! write cannot work at all: these are *derived* figures, and a
//! journey's value-adding ratio changes when time passes and nothing is
//! recorded, which is precisely the case with no write to hang an
//! update on.
//!
//! The loop is **off unless configured**, like every other control in
//! this crate:
//!
//! | Variable | Default | Effect |
//! |---|---|---|
//! | `CARE_PATHWAY_FLOW_METRICS_SECS` | unset | Refresh interval. Unset or `0` ⇒ the loop never starts and the gauges never appear. |
//! | `CARE_PATHWAY_FLOW_METRICS_MAX_PATHWAYS` | 50 | Cap on per-pathway series. |
//! | `CARE_PATHWAY_FLOW_METRICS_MIN_COHORT` | 5 | Cohorts below this are counted, never labelled. |
//!
//! See [`crate::metrics`] for why the cap and the suppression floor are
//! not optional, and [`crate::tba::flow_metric_rows`] for the selection
//! itself.

use std::time::Duration;

use loco_rs::prelude::*;

use crate::metrics::Metrics;
use crate::models::care_pathways::Model as PathwayModel;
use crate::tba;

/// The default cohort floor: below this, a pathway is counted but never
/// labelled (spec §12.2).
pub const DEFAULT_MIN_COHORT: usize = 5;

/// Parse a refresh interval, or `None` when the loop is switched off.
///
/// A zero or unparsable value reads as **off**, not as "every instant":
/// a typo in a deployment variable must not turn into a busy loop
/// against the database. Pure, so the whole matrix is testable without
/// touching the process environment — which this crate could not do in
/// a unit test anyway, since it forbids `unsafe`.
#[must_use]
pub fn parse_interval(raw: Option<&str>) -> Option<Duration> {
    let secs: u64 = raw?.trim().parse().ok()?;
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Parse a positive bound, falling back to `default`.
///
/// Zero falls back rather than applying: a cap of zero would export
/// nothing while looking configured, which is the worst of both.
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
        std::env::var("CARE_PATHWAY_FLOW_METRICS_SECS")
            .ok()
            .as_deref(),
    )
}

/// The per-pathway series cap.
#[must_use]
pub fn max_pathways_from_env() -> usize {
    parse_bound(
        std::env::var("CARE_PATHWAY_FLOW_METRICS_MAX_PATHWAYS")
            .ok()
            .as_deref(),
        tba::DEFAULT_METRIC_MAX_PATHWAYS,
    )
}

/// The cohort size below which a pathway is counted but not labelled.
#[must_use]
pub fn min_cohort_from_env() -> usize {
    parse_bound(
        std::env::var("CARE_PATHWAY_FLOW_METRICS_MIN_COHORT")
            .ok()
            .as_deref(),
        DEFAULT_MIN_COHORT,
    )
}

/// Run one refresh pass: analyse every pathway's cohort, choose what may
/// be published, and write the gauges.
///
/// # Errors
///
/// Propagates a database error. The caller logs and retries on the next
/// tick rather than aborting the loop — a metrics refresh must never be
/// able to take the service down.
pub async fn refresh_once(ctx: &AppContext) -> Result<tba::FlowMetricSet> {
    let as_of = chrono::Utc::now();
    let as_of_ms = as_of.timestamp_millis();

    // The same cap the insight views use, so a large registry cannot
    // turn a refresh into an unbounded scan.
    let pathways = PathwayModel::list(&ctx.db, 1000).await?;
    let mut per_pathway = Vec::with_capacity(pathways.len());
    for pathway in &pathways {
        let instances = crate::controllers::tba::load_cohort(ctx, pathway.pid, None).await?;
        if instances.is_empty() {
            continue;
        }
        let analyses = crate::controllers::tba::analyze_cohort(ctx, &instances, as_of_ms).await?;
        per_pathway.push((pathway.pid.to_string(), tba::cohort(&analyses)));
    }

    let set = tba::flow_metric_rows(&per_pathway, max_pathways_from_env(), min_cohort_from_env());
    #[allow(clippy::cast_precision_loss)] // seconds since the epoch
    Metrics::global().publish_flow(&set, as_of.timestamp() as f64);
    Ok(set)
}

/// Spawn the refresh loop. A no-op unless
/// `CARE_PATHWAY_FLOW_METRICS_SECS` is set to a positive number.
pub fn spawn(ctx: &AppContext) {
    let Some(period) = interval_from_env() else {
        tracing::debug!("flow metrics: CARE_PATHWAY_FLOW_METRICS_SECS unset; gauges not exported");
        return;
    };
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(period);
        // A missed tick is skipped rather than queued: falling behind
        // must not produce a burst of catch-up passes against the
        // database.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match refresh_once(&ctx).await {
                Ok(set) => tracing::debug!(
                    exported = set.rows.len(),
                    suppressed = set.suppressed_pathways,
                    dropped = set.dropped_pathways,
                    "flow metrics refreshed"
                ),
                // Logged and retried on the next tick. The stale
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
        assert_eq!(parse_interval(Some("30")), Some(Duration::from_secs(30)));
        assert_eq!(
            parse_interval(Some("  45  ")),
            Some(Duration::from_secs(45))
        );
    }

    /// A bound falls back to its documented default rather than to zero
    /// — a cap of zero would export nothing while looking configured.
    #[test]
    fn a_bound_falls_back_rather_than_disabling_itself() {
        assert_eq!(parse_bound(None, 50), 50);
        assert_eq!(parse_bound(Some("0"), 50), 50, "zero falls back");
        assert_eq!(parse_bound(Some("oops"), 50), 50);
        assert_eq!(parse_bound(Some(""), 50), 50);
        assert_eq!(parse_bound(Some("-1"), 50), 50);
        assert_eq!(parse_bound(Some("10"), 50), 10);
        assert_eq!(parse_bound(Some(" 7 "), 50), 7);
    }

    /// The defaults are the ones the module docs promise.
    #[test]
    fn the_documented_defaults_hold() {
        assert_eq!(tba::DEFAULT_METRIC_MAX_PATHWAYS, 50);
        assert_eq!(DEFAULT_MIN_COHORT, 5);
    }
}
