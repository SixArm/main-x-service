//! Postgres-backed, per-key sliding-window rate limiter for magic-link
//! issuance.
//!
//! The unauthenticated magic-link endpoints (`POST /api/auth/signup` and
//! `POST /api/auth/magic-link`) can be abused to email-bomb a victim or to
//! probe for the existence of accounts. This module throttles issuance by a
//! **normalised email** key: at most [`MAX_REQUESTS`] requests per
//! [`WINDOW`].
//!
//! Design notes:
//!
//! - **Durable, multi-instance.** The window log lives in the
//!   `auth_rate_limits` table, so the quota is shared across
//!   horizontally-scaled instances (the previous implementation was an
//!   in-memory `OnceLock` map, correct only for a single process). One row
//!   is recorded per *allowed* request; rejected requests are never
//!   recorded, so a throttled caller cannot push the window forward.
//! - **Wall-clock window.** A distributed limiter must compare against a
//!   shared clock, so the window is measured with `requested_at`
//!   (`TIMESTAMPTZ`) rather than a per-process monotonic `Instant`. Tests
//!   inject a synthetic "now" through [`check_at`] for determinism.
//! - **Exact under concurrency.** Each check serialises on a per-key
//!   transaction-scoped advisory lock (`pg_advisory_xact_lock(hashtext(key))`),
//!   so two simultaneous requests for the same email cannot both slip past
//!   the cap. Different emails hash to different lock slots, so there is no
//!   cross-email contention.
//! - **Fail-open on DB error.** If the limiter's own query fails, the
//!   request is allowed (logged at WARN). A DB outage already breaks the
//!   surrounding handler (it must write the magic-link token), so failing
//!   *closed* here would only lock out legitimate sign-ins on a transient
//!   blip without preventing any abuse.
//! - **Anti-enumeration preserved.** The limiter keys on request *volume*,
//!   never on whether an account exists; the handlers keep their always-`200`
//!   success shape and only swap in `429` once the volume cap is exceeded.

use std::time::Duration;

use chrono::{DateTime, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};

/// Maximum number of issuance requests allowed per key within [`WINDOW`].
/// The N+1th request inside the window is rejected. Typed `i64` to compare
/// directly against the SQL `count(*)`.
pub const MAX_REQUESTS: i64 = 5;

/// Length of the sliding window over which [`MAX_REQUESTS`] is counted.
/// Five minutes balances legitimate retries (a user who mistypes, or whose
/// first email is slow) against abuse.
pub const WINDOW: Duration = Duration::from_mins(5);

/// How long a request is rejected for: the time until the oldest in-window
/// request ages out. Returned to callers so they can surface `Retry-After`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryAfter(pub Duration);

/// Normalise an email for keying: trim surrounding whitespace and lowercase.
/// This collapses trivially-distinct spellings (`Alice@X.com `,
/// `alice@x.com`) onto a single bucket so they share one quota.
#[must_use]
pub fn normalize_key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// Record a request for `key` and decide whether it is allowed, using the
/// real wall clock. Returns `Ok(())` when the request is within quota, or
/// `Err(RetryAfter)` when the cap is exceeded (in which case the request is
/// *not* recorded). A database error is treated as allow (fail-open; see the
/// module docs) and never surfaces as `Err(RetryAfter)`.
///
/// # Errors
///
/// Returns [`RetryAfter`] when `key` has already reached [`MAX_REQUESTS`]
/// within the current [`WINDOW`]. Database failures fail open (the request is
/// allowed) and are never returned as an error.
pub async fn check(db: &DatabaseConnection, key: &str) -> Result<(), RetryAfter> {
    check_at(db, key, Utc::now()).await
}

/// Clock-injectable core of [`check`]. `now` is the synthetic instant the
/// window is evaluated against; production callers pass `Utc::now()`. Exposed
/// for deterministic (DB-gated) tests.
///
/// On a database error the request is allowed (fail-open) and a WARN is
/// logged; only an actual quota breach yields `Err(RetryAfter)`.
///
/// # Errors
///
/// Returns [`RetryAfter`] when `key` has already reached [`MAX_REQUESTS`]
/// within the [`WINDOW`] ending at `now`. Database failures fail open and are
/// never returned as an error.
pub async fn check_at(
    db: &DatabaseConnection,
    key: &str,
    now: DateTime<Utc>,
) -> Result<(), RetryAfter> {
    match check_at_inner(db, key, now).await {
        Ok(decision) => decision,
        Err(error) => {
            tracing::warn!(%error, "magic-link rate limiter DB error; allowing request (fail-open)");
            Ok(())
        }
    }
}

/// The fallible core: returns the throttle decision (`Ok`/`Err(RetryAfter)`)
/// wrapped in a `Result` whose outer error is any database failure.
///
/// Runs in one transaction: take a per-key advisory lock so concurrent
/// same-key checks serialise, prune rows older than the window, count what
/// remains, then insert iff under the cap.
async fn check_at_inner(
    db: &DatabaseConnection,
    key: &str,
    now: DateTime<Utc>,
) -> Result<Result<(), RetryAfter>, sea_orm::DbErr> {
    let key = normalize_key(key);
    // Window start: rows older than this have aged out.
    let cutoff =
        now - chrono::Duration::seconds(i64::try_from(WINDOW.as_secs()).unwrap_or(i64::MAX));

    let txn = db.begin().await?;

    // Serialise concurrent checks for this exact key (different keys hash to
    // different lock slots, so unrelated emails never block each other). The
    // lock is held until the transaction commits.
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtext($1))",
        [key.clone().into()],
    ))
    .await?;

    // Drop this key's requests that have aged out of the window — keeps the
    // table bounded and makes the count below a pure in-window count.
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM auth_rate_limits WHERE email_key = $1 AND requested_at < $2",
        [key.clone().into(), cutoff.into()],
    ))
    .await?;

    // Count what remains (all in-window) and find the oldest, used to tell a
    // throttled caller when a slot will free.
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT count(*) AS cnt, min(requested_at) AS oldest FROM auth_rate_limits WHERE email_key = $1",
            [key.clone().into()],
        ))
        .await?;
    // `count(*)` always returns exactly one row (cnt = 0, oldest = NULL when
    // empty), so `row` is `Some`.
    let (count, oldest) = match row {
        Some(r) => (
            r.try_get::<i64>("", "cnt")?,
            r.try_get::<Option<DateTime<Utc>>>("", "oldest")?,
        ),
        None => (0, None),
    };

    if count >= MAX_REQUESTS {
        // Over quota: do not record this request. Releasing the lock (commit)
        // is enough; the prune above is worth keeping.
        txn.commit().await?;
        let retry = oldest.map_or(WINDOW, |oldest| {
            let elapsed = (now - oldest).to_std().unwrap_or(Duration::ZERO);
            WINDOW.saturating_sub(elapsed)
        });
        return Ok(Err(RetryAfter(retry)));
    }

    // Under quota: record this request and allow it.
    txn.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO auth_rate_limits (email_key, requested_at) VALUES ($1, $2)",
        [key.into(), now.into()],
    ))
    .await?;
    txn.commit().await?;
    Ok(Ok(()))
}

/// Clear all rate-limit state (empty the table). A test-support helper so
/// DB-gated suites that share a database can start from a known-empty slate.
/// Not used on any production code path.
///
/// # Errors
///
/// Propagates any database error from the `DELETE`.
pub async fn reset(db: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.execute(Statement::from_string(
        DbBackend::Postgres,
        "DELETE FROM auth_rate_limits",
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The sliding-window behaviour is exercised against a real database in
    // `tests/requests/rate_limit.rs` (DB-gated), where a `now` can be
    // injected via `check_at`. Here we only unit-test the pure key
    // normalisation, so the default `cargo test` stays DB-free.
    #[test]
    fn normalize_key_trims_and_lowercases() {
        assert_eq!(normalize_key("  Alice@Example.COM "), "alice@example.com");
    }
}
