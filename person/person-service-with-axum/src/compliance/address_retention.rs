//! How long an audit row keeps the address of the caller.
//!
//! An address is personal data, so it is kept for
//! `PERSON_AUDIT_ADDRESS_RETENTION_DAYS`, a week unless set. After that
//! [`scrub`] removes the address and its salt. The row stays, and stays
//! verifiable, because the chain binds a commitment to the address rather
//! than the address ([`super::audit_chain::CHAIN_VERSION_COMMITTED`]).
//!
//! This is not `redacted_at`. A redacted row's content is no longer checked,
//! which is right for an erasure and would be wrong here: it would turn off
//! tamper evidence for every row older than a week.
//!
//! **What cannot be scrubbed.** A chained `p1` row with an address binds the
//! address itself, so removing it would be a content break on a row nothing
//! is wrong with. [`unscrubbable`] counts them and the sweep warns. Rows with
//! no chain hash, or already redacted, bind nothing and are scrubbed.
//!
//! **What this does not prove.** Removing an address early looks the same as
//! removing it on time. The chain shows an address was not altered, not how
//! long it was held.
//!
//! It runs in every process, not only a job worker, because a retention limit
//! must not depend on a deployment running a worker. One statement, so
//! replicas running it together do no harm.

use std::time::Duration;

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement, Value};

/// The setting, in whole days. `0` keeps nothing past the next sweep.
pub const RETENTION_ENV: &str = "PERSON_AUDIT_ADDRESS_RETENTION_DAYS";

/// A week, unless set.
pub const DEFAULT_RETENTION_DAYS: i32 = 7;

/// How often a process sweeps, so that an address goes within this of its limit.
const SWEEP_INTERVAL: Duration = Duration::from_hours(1);

/// `$1` is the retention in days.
const SCRUB: &str = "UPDATE audit_log SET ip_address = NULL, ip_salt = NULL
    WHERE ip_address IS NOT NULL
      AND \"timestamp\" < now() - make_interval(days => $1)
      AND (ip_commitment IS NOT NULL OR hash IS NULL OR redacted_at IS NOT NULL)";

/// `$1` is the retention in days.
const UNSCRUBBABLE: &str = "SELECT count(*) AS n FROM audit_log
    WHERE ip_address IS NOT NULL
      AND \"timestamp\" < now() - make_interval(days => $1)
      AND ip_commitment IS NULL AND hash IS NOT NULL AND redacted_at IS NULL";

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
pub async fn scrub<C: ConnectionTrait>(conn: &C, days: i32) -> Result<u64, DbErr> {
    let statement = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        SCRUB,
        [Value::from(days)],
    );
    Ok(conn.execute_raw(statement).await?.rows_affected())
}

/// How many rows past `days` still hold an address the chain binds.
///
/// # Errors
///
/// Returns the database's error.
pub async fn unscrubbable<C: ConnectionTrait>(conn: &C, days: i32) -> Result<i64, DbErr> {
    let statement = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        UNSCRUBBABLE,
        [Value::from(days)],
    );
    let row = conn.query_one_raw(statement).await?;
    row.map_or(Ok(0), |row| row.try_get("", "n"))
}

async fn run(db: DatabaseConnection, days: i32) {
    let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
    loop {
        ticker.tick().await;
        match scrub(&db, days).await {
            Ok(0) => {}
            Ok(rows) => tracing::info!(rows, days, "audit addresses removed"),
            Err(error) => tracing::error!(%error, "audit address sweep failed"),
        }
        match unscrubbable(&db, days).await {
            Ok(0) => {}
            Ok(rows) => tracing::warn!(
                rows,
                "audit rows past retention hold an address their chain hash binds"
            ),
            Err(error) => tracing::error!(%error, "audit address count failed"),
        }
    }
}

/// Start sweeping.
///
/// # Errors
///
/// A bad [`RETENTION_ENV`].
pub fn spawn(db: DatabaseConnection) -> axa::Result<()> {
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
