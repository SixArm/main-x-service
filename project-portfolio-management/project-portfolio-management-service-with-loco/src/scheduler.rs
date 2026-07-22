//! The **set-and-forget** ticker: an optional in-process loop that
//! sweeps due [scheduled actions](crate::controllers::automation) so a
//! deadline configured once fires without anyone poking an endpoint.
//!
//! Gated by `PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES` (unset /
//! `0` ⇒ off, the default; parse failure ⇒ off with a warning) —
//! the same posture as the estate-snapshot ticker in
//! [`crate::snapshots`]. With the ticker off, `POST
//! /api/scheduled-actions/sweep` still works, so a deployment can
//! drive the sweep from its own cron instead.
//!
//! Firing is claim-based (see
//! [`crate::controllers::automation::sweep_due`]), so running the
//! ticker *and* an external cron cannot double-fire a deadline.

use loco_rs::prelude::AppContext;

/// Smallest sweep period, in minutes. A tighter loop would poll the
/// database far more often than deadlines actually arrive.
pub const MIN_PERIOD_MINUTES: u64 = 1;

/// Read the configured sweep period in minutes; `None` ⇒ ticker off.
#[must_use]
pub fn configured_period_minutes(raw: Option<&str>) -> Option<u64> {
    let raw = raw?.trim();
    if raw.is_empty() {
        return None;
    }
    let minutes = raw.parse::<u64>().ok()?;
    if minutes < MIN_PERIOD_MINUTES {
        return None;
    }
    Some(minutes)
}

/// Start the sweep loop when configured. A no-op by default.
pub fn spawn(ctx: AppContext) {
    let raw = std::env::var("PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES").ok();
    let Some(minutes) = configured_period_minutes(raw.as_deref()) else {
        if raw.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            tracing::warn!(
                "PROJECT_PORTFOLIO_MANAGEMENT_SCHEDULER_MINUTES is not a positive whole \
                 number of minutes; the scheduled-action ticker stays off"
            );
        }
        return;
    };
    tokio::spawn(async move {
        let period = std::time::Duration::from_secs(minutes.saturating_mul(60));
        loop {
            tokio::time::sleep(period).await;
            match crate::controllers::automation::sweep_due(&ctx).await {
                Ok((fired, skipped, capped)) if fired > 0 || capped => {
                    tracing::info!(
                        "scheduled-action sweep fired {fired} action(s) \
                         (skipped {skipped}, capped {capped})"
                    );
                }
                Ok(_) => {}
                Err(err) => tracing::warn!("scheduled-action sweep failed: {err}"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ticker_is_off_by_default() {
        assert_eq!(configured_period_minutes(None), None);
        assert_eq!(configured_period_minutes(Some("")), None);
        assert_eq!(configured_period_minutes(Some("   ")), None);
        assert_eq!(configured_period_minutes(Some("0")), None);
    }

    #[test]
    fn junk_configuration_leaves_the_ticker_off_rather_than_guessing() {
        assert_eq!(configured_period_minutes(Some("hourly")), None);
        assert_eq!(configured_period_minutes(Some("-5")), None);
        assert_eq!(configured_period_minutes(Some("1.5")), None);
    }

    #[test]
    fn a_positive_period_is_accepted() {
        assert_eq!(configured_period_minutes(Some("5")), Some(5));
        assert_eq!(configured_period_minutes(Some(" 60 ")), Some(60));
        assert_eq!(
            configured_period_minutes(Some("1")),
            Some(MIN_PERIOD_MINUTES)
        );
    }
}
