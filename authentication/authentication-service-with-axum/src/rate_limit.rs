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

/// Normalise an email into a **throttle bucket** key (SEC-A6).
///
/// The point of the key is to stop an attacker email-bombing one victim (or
/// spawning quota) by dressing the same inbox up in trivially-distinct
/// spellings. It therefore folds aggressively — this only ever *tightens*
/// the quota (lookalikes share it), never loosens it, and it does **not**
/// decide account identity (that stays case-only; see `users::find_by_email`):
///
/// - trim + lowercase (`Alice@X.com ` → `alice@x.com`);
/// - strip a `+tag` sub-address from the local part
///   (`victim+1@x` → `victim@x`) — the sub-addressing convention every
///   provider that honours it routes to the same inbox;
/// - for Gmail (`gmail.com` / `googlemail.com`), fold `.` in the local part
///   (`v.ictim@gmail.com` → `victim@gmail.com`) — Gmail ignores dots, so a
///   dotted spelling reaches the same victim.
///
/// So `victim+1@gmail.com`, `v.ictim@gmail.com`, and `Victim@gmail.com` all
/// key to `victim@gmail.com`.
#[must_use]
pub fn normalize_key(email: &str) -> String {
    let email = email.trim().to_lowercase();
    let Some((local, domain)) = email.rsplit_once('@') else {
        // Not an `local@domain` shape — key on the whole trimmed/lowered
        // string rather than inventing structure.
        return email;
    };
    // Drop a `+tag` sub-address.
    let mut local = local.split('+').next().unwrap_or(local).to_string();
    // Gmail folds dots in the local part to the same inbox.
    if matches!(domain, "gmail.com" | "googlemail.com") {
        local = local.replace('.', "");
    }
    format!("{local}@{domain}")
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
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtext($1))",
        [key.clone().into()],
    ))
    .await?;

    // Drop this key's requests that have aged out of the window — keeps the
    // table bounded and makes the count below a pure in-window count.
    txn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "DELETE FROM auth_rate_limits WHERE email_key = $1 AND requested_at < $2",
        [key.clone().into(), cutoff.into()],
    ))
    .await?;

    // Count what remains (all in-window) and find the oldest, used to tell a
    // throttled caller when a slot will free.
    let row = txn
        .query_one_raw(Statement::from_sql_and_values(
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
    txn.execute_raw(Statement::from_sql_and_values(
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
    db.execute_raw(Statement::from_string(
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

    /// SEC-A6: plus-address, Gmail dot, and case variants of the same inbox
    /// collapse to a single throttle bucket, so an attacker cannot bypass
    /// the per-email cap to email-bomb one victim.
    #[test]
    fn normalize_key_collapses_plus_dot_and_case_variants() {
        let canonical = "victim@gmail.com";
        for variant in [
            "victim+1@gmail.com",
            "victim+anything@gmail.com",
            "v.ictim@gmail.com",
            "v.i.c.t.i.m@gmail.com",
            "Victim@Gmail.com",
            "  victim+tag@googlemail.com  ", // googlemail folds too (→ gmail-style local)
        ] {
            let got = normalize_key(variant);
            // googlemail keeps its own domain; assert the local part folds.
            let want = if variant.contains("googlemail") {
                "victim@googlemail.com"
            } else {
                canonical
            };
            assert_eq!(got, want, "variant {variant:?} should fold to {want}");
        }
    }

    /// Non-Gmail providers keep their dots (dots are significant there), and
    /// a non-email-shaped key is passed through trimmed/lowered.
    #[test]
    fn normalize_key_keeps_dots_for_non_gmail_and_passes_through_non_email() {
        assert_eq!(normalize_key("v.ictim@example.com"), "v.ictim@example.com");
        // Plus-tag stripping still applies at every provider.
        assert_eq!(
            normalize_key("v.ictim+x@example.com"),
            "v.ictim@example.com"
        );
        assert_eq!(normalize_key("  NotAnEmail "), "notanemail");
    }
}
