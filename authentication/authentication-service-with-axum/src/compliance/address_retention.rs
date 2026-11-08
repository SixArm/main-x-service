//! How long an audit row and a session keep the client's address.
//!
//! An address is personal data, so it is kept for
//! `AUTH_AUDIT_ADDRESS_RETENTION_DAYS`, a week unless set. After that [`scrub`]
//! empties `auth_events.source_ip` and `sessions.source_ip`. Unlike the
//! person and worker audit chains, neither is hashed
//! ([`super::audit_integrity`] leaves `source_ip` out of its pre-image), so
//! emptying one breaks nothing.
//!
//! One statement per table, so replicas running it together do no harm.

use std::time::Duration;

use sqlx::PgPool;

use crate::models::{auth_events, sessions};

/// The setting, in whole days. `0` keeps nothing past the next sweep.
pub const RETENTION_ENV: &str = "AUTH_AUDIT_ADDRESS_RETENTION_DAYS";

/// A week, unless set.
pub const DEFAULT_RETENTION_DAYS: i32 = 7;

/// How often a process sweeps, so that an address goes within this of its limit.
const SWEEP_INTERVAL: Duration = Duration::from_hours(1);

/// The retention from its setting. Unset or empty is the default.
///
/// # Errors
///
/// The text to stop the service with, for anything but a whole number of
/// days. Guessing at a privacy limit would be worse than not starting.
pub fn retention_days(raw: Option<&str>) -> Result<i32, String> {
    let raw = raw.map_or("", str::trim);
    if raw.is_empty() {
        return Ok(DEFAULT_RETENTION_DAYS);
    }
    match raw.parse::<i32>() {
        Ok(days) if days >= 0 => Ok(days),
        _ => Err(format!(
            "{RETENTION_ENV} must be a whole number of days, not {raw:?}"
        )),
    }
}

/// Remove addresses older than `days`, returning how many went.
///
/// # Errors
///
/// Returns the database's error.
pub async fn scrub(db: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
    let mut rows = auth_events::Model::scrub_source_ip_older_than(db, days).await?;
    rows += sessions::Model::scrub_source_ip_older_than(db, days).await?;
    Ok(rows)
}

async fn run(db: PgPool, days: i32) {
    let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
    loop {
        ticker.tick().await;
        match scrub(&db, days).await {
            Ok(0) => {}
            Ok(rows) => tracing::info!(rows, days, "client addresses removed"),
            Err(error) => tracing::error!(%error, "client address sweep failed"),
        }
    }
}

/// Start sweeping.
///
/// # Errors
///
/// A bad [`RETENTION_ENV`].
pub fn spawn(db: PgPool) -> axa::Result<()> {
    let raw = std::env::var(RETENTION_ENV).ok();
    let days = retention_days(raw.as_deref()).map_err(|message| axa::Error::string(&message))?;
    tokio::spawn(run(db, days));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_a_week() {
        assert_eq!(retention_days(None), Ok(7));
        assert_eq!(retention_days(Some(" ")), Ok(7));
    }

    #[test]
    fn the_retention_can_be_set_and_zero_is_allowed() {
        assert_eq!(retention_days(Some("30")), Ok(30));
        assert_eq!(retention_days(Some("0")), Ok(0));
    }

    #[test]
    fn a_bad_setting_stops_the_service() {
        for raw in ["-1", "seven", "1.5", "7d"] {
            assert!(retention_days(Some(raw)).is_err(), "{raw}");
        }
    }
}
