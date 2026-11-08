//! `auth_rate_limits` — the statements behind the magic-link issuance
//! rate limiter ([`crate::rate_limit`]).
//!
//! One row per *allowed* request, keyed by the normalised email. The
//! limiter runs [`lock_key`], [`prune`], [`count_and_oldest`] and then
//! maybe [`insert`] in one transaction it owns, so each of those takes the
//! transaction's connection.

use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};

/// Serialise concurrent checks for this exact key until the caller's
/// transaction ends (different keys hash to different lock slots, so
/// unrelated emails never block each other).
///
/// # Errors
///
/// When the statement fails.
pub async fn lock_key(conn: &mut PgConnection, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query!("SELECT pg_advisory_xact_lock(hashtext($1))", key)
        .execute(conn)
        .await?;
    Ok(())
}

/// Drop this key's requests older than `cutoff`, which have aged out of
/// the window.
///
/// # Errors
///
/// When the statement fails.
pub async fn prune(
    conn: &mut PgConnection,
    key: &str,
    cutoff: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM auth_rate_limits WHERE email_key = $1 AND requested_at < $2",
        key,
        cutoff,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// How many requests this key has recorded, and the oldest of them.
/// `count(*)` always returns one row: `(0, None)` for an unseen key.
///
/// # Errors
///
/// When the statement fails.
pub async fn count_and_oldest(
    conn: &mut PgConnection,
    key: &str,
) -> Result<(i64, Option<DateTime<Utc>>), sqlx::Error> {
    let row = sqlx::query!(
        r#"SELECT count(*) AS "cnt!", min(requested_at) AS oldest
           FROM auth_rate_limits WHERE email_key = $1"#,
        key,
    )
    .fetch_one(conn)
    .await?;
    Ok((row.cnt, row.oldest))
}

/// Record one allowed request for `key` at `now`.
///
/// # Errors
///
/// When the statement fails.
pub async fn insert(
    conn: &mut PgConnection,
    key: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO auth_rate_limits (email_key, requested_at) VALUES ($1, $2)",
        key,
        now,
    )
    .execute(conn)
    .await?;
    Ok(())
}

/// Empty the table, for tests that need a known-empty slate.
///
/// # Errors
///
/// When the statement fails.
pub async fn delete_all(db: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM auth_rate_limits")
        .execute(db)
        .await?;
    Ok(())
}
