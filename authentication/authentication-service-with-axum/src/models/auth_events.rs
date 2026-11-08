//! `auth_events` model — record and query the authentication audit trail.
//!
//! A durable, security/compliance audit log of authentication events:
//! signup, magic-link request, magic-link redemption, signout, and `me`.
//! Each row captures *what happened* (`event`), the *subject* when known
//! (normalised `email` and/or `user_pid`), and an *outcome* (`detail`,
//! e.g. `rate_limited` / `unknown_email` / `expired_token`). It never
//! stores tokens or secrets — the audit row may distinguish outcomes
//! internally, but it must not let a reader of the HTTP response infer
//! account existence (anti-enumeration is preserved at the wire).

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelResult, to_i64};

/// An `auth_events` row. One row per authentication event (signup /
/// magic-link request / redemption / signout / me) for the security +
/// compliance audit trail. Never stores tokens or secrets. Serialized as
/// is by `GET /api/auth/audit/recent`, so the field order and names are
/// the wire shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation time; part of the integrity pre-image.
    pub created_at: DateTime<FixedOffset>,
    /// Last update time.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal autoincrement primary key.
    pub id: i64,
    /// What happened.
    pub event: String,
    /// Normalised email when applicable.
    pub email: Option<String>,
    /// Subject user pid when known.
    pub user_pid: Option<Uuid>,
    /// Outcome detail. Never a token or secret.
    pub detail: Option<String>,
    /// SHA-256 (FIPS 180-4) over this audit row's pre-image.
    ///
    /// Unkeyed, so anyone holding the database can recompute it — what it
    /// catches is careless or unaware modification. Written
    /// unconditionally, unlike the MAC, which needs a key: with no key
    /// configured these two digests are the row's only integrity.
    pub hash: Option<String>,
    /// SHA3-256 (FIPS 202) over the same pre-image. A sponge, unrelated
    /// to SHA-256's Merkle-Damgard chaining, so a cryptanalytic advance
    /// against one design family does not transfer.
    pub hash_sha3: Option<String>,
    /// HMAC-SHA256 over this row's pre-image, as
    /// `"<scheme>.<key id>:<hex>"`.
    ///
    /// Detects a row whose content was altered — notably an
    /// `attributes_assigned` row rewritten to hide a privilege grant. It
    /// does **not** detect a row deleted wholesale (see
    /// `crate::compliance`).
    pub mac: Option<String>,
    /// Best-effort connecting-peer address captured when the row was
    /// written (T-14). Nullable for rows predating this column.
    /// Deliberately **not** part of the MAC/hash pre-image (see
    /// `crate::compliance::audit_integrity`) — widening that fixed-format
    /// pre-image would break verification of every pre-existing row.
    pub source_ip: Option<String>,
}

/// The `event` value for an ABAC attribute-assignment audit row, written
/// whenever an operator changes a user's `users.attributes` (via the
/// `user_attributes` CLI task or the admin HTTP API). Distinct from the
/// authentication events so a compliance reviewer can filter for
/// authorization changes.
pub const ATTRIBUTES_ASSIGNED: &str = "attributes_assigned";

/// The `event` value for an ABAC attribute-**read** audit row (T-15),
/// written when an admin views another user's `users.attributes` via
/// `GET /api/auth/admin/users/{pid}/attributes`. Recording *mutations
/// only* does not satisfy HIPAA §164.312(b) — reads are activity — and
/// an admin viewing a different user's privilege attributes is exactly
/// the read that provision exists for
/// (`agents/share/compliance-for-healthcare.md` §2.1).
pub const ATTRIBUTES_VIEWED: &str = "attributes_viewed";

impl Model {
    /// Record one authentication event. **Best-effort**: callers use
    /// [`record`](Self::record) for its side effect and must never fail
    /// the request on an audit error (see [`Self::record_best_effort`]).
    ///
    /// `email` is the normalised (trimmed, lowercased) address where
    /// applicable; `user_pid` the subject when known; `detail` an
    /// outcome marker (e.g. `rate_limited`, `unknown_email`,
    /// `expired_token`). Pass no token or secret here. `source_ip` is the
    /// client's address (T-14), from `axa::client::ClientAddr` — stored as a plain
    /// column, deliberately **not** part of the MAC/hash pre-image (see
    /// `crate::compliance::audit_integrity`, whose fixed-format pre-image
    /// has no per-row version marker; widening it would make every
    /// pre-existing row's stored digest unrecomputable).
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: &PgPool,
        event: &str,
        email: Option<&str>,
        user_pid: Option<Uuid>,
        detail: Option<&str>,
        source_ip: Option<&str>,
    ) -> ModelResult<Self> {
        // The timestamp is part of the MAC pre-image, so it is read before
        // the insert rather than defaulted by it: the alternative — insert,
        // then stamp — leaves a window in which the row exists unprotected.
        // It is read from the database, not this process, because retention
        // (`scrub_source_ip`) and every reviewer's query compare it with
        // Postgres `now()`. An application clock that drifts from the
        // database's would move the retention edge by the drift.
        let created_at: DateTime<FixedOffset> =
            sqlx::query_scalar!(r#"SELECT clock_timestamp() AS "now!: DateTime<FixedOffset>""#)
                .fetch_one(db)
                .await?;
        // The stored address is the normalised one, so the MAC covers
        // what the row actually holds rather than what was passed in.
        let email = email.map(normalise_email);
        let detail = detail.map(ToString::to_string);
        // All three digests from one call, so none can be stamped
        // without the others. The two unkeyed ones are written
        // unconditionally; only the MAC depends on a configured key.
        let digests = crate::compliance::audit_integrity::digests(
            &crate::compliance::audit_integrity::AuditInput {
                event,
                email: email.as_deref(),
                user_pid,
                detail: detail.as_deref(),
                created_at_micros: created_at.timestamp_micros(),
            },
        );
        let entry = sqlx::query_as!(
            Self,
            r#"INSERT INTO auth_events
                   (event, email, user_pid, detail, created_at, hash, hash_sha3, mac, source_ip)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, event, email, user_pid, detail, hash, hash_sha3, mac,
                         source_ip"#,
            event,
            email,
            user_pid,
            detail,
            created_at,
            digests.sha256,
            digests.sha3,
            digests.mac,
            source_ip,
        )
        .fetch_one(db)
        .await?;
        Ok(entry)
    }

    /// Record one authentication event, swallowing any error after
    /// logging it. Auditing must never break the request path, so
    /// handlers call this rather than `record` + `?`.
    pub async fn record_best_effort(
        db: &PgPool,
        event: &str,
        email: Option<&str>,
        user_pid: Option<Uuid>,
        detail: Option<&str>,
        source_ip: Option<&str>,
    ) {
        if let Err(err) = Self::record(db, event, email, user_pid, detail, source_ip).await {
            tracing::warn!(error = %err, event, "failed to write auth event");
        }
    }

    /// Record an ABAC **attribute-assignment** audit row (best-effort).
    /// The row's subject is the **target** user (`email` / `user_pid`);
    /// the `detail` captures the operation (`op`), the affected `key`
    /// where applicable, and the **actor** who made the change (`cli`
    /// for the CLI task, or the admin's `pid:<uuid>` for the HTTP API) —
    /// the fixed `auth_events` shape has no actor column, so the actor
    /// rides in `detail`. Attribute **values** are deliberately omitted
    /// (a value like a department can itself be sensitive; the value set
    /// lives in `users.attributes`, not the audit trail).
    pub async fn record_attribute_assignment_best_effort(
        db: &PgPool,
        target_email: Option<&str>,
        target_pid: Uuid,
        op: &str,
        key: Option<&str>,
        actor: &str,
        source_ip: Option<&str>,
    ) {
        let detail = match key {
            Some(key) => format!("op={op} key={key} actor={actor}"),
            None => format!("op={op} actor={actor}"),
        };
        Self::record_best_effort(
            db,
            ATTRIBUTES_ASSIGNED,
            target_email,
            Some(target_pid),
            Some(&detail),
            source_ip,
        )
        .await;
    }

    /// Record an ABAC **attribute-read** audit row (best-effort, T-15).
    /// The row's subject is the **target** user, mirroring
    /// [`Self::record_attribute_assignment_best_effort`]'s shape; the
    /// `detail` carries only the admin's identity (`actor=pid:<uuid>`) —
    /// attribute **values** stay out of the audit detail here too, same
    /// reasoning as the assignment row.
    pub async fn record_attribute_view_best_effort(
        db: &PgPool,
        target_email: Option<&str>,
        target_pid: Uuid,
        actor: &str,
        source_ip: Option<&str>,
    ) {
        let detail = format!("actor={actor}");
        Self::record_best_effort(
            db,
            ATTRIBUTES_VIEWED,
            target_email,
            Some(target_pid),
            Some(&detail),
            source_ip,
        )
        .await;
    }

    /// All audit rows for one subject, newest first. A subject is matched
    /// by `user_pid` **or** by their normalised `email` — the early
    /// events of a flow (e.g. a `signup` before the pid is known, or an
    /// `unknown_email` probe) carry only the email, while later ones
    /// carry the pid, so the GDPR right-of-access export must union both.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_subject(db: &PgPool, user_pid: Uuid, email: &str) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, event, email, user_pid, detail, hash, hash_sha3, mac, source_ip
               FROM auth_events
               WHERE user_pid = $1 OR email = $2
               ORDER BY id DESC"#,
            user_pid,
            normalise_email(email),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// GDPR erasure (SEC-A7): NULL out the `email` column on **every** audit
    /// row for this subject — matched exactly as [`for_subject`](Self::for_subject)
    /// (by `user_pid` OR the normalised `email`) so pre-account
    /// `unknown_email`-style rows are scrubbed too. The event names, pids,
    /// and outcome markers stay (a non-PII audit trail); only the address is
    /// removed. Returns the number of rows scrubbed.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn scrub_subject_email(
        db: impl Into<axa::sqlx::Db<'_>>,
        user_pid: Uuid,
        email: &str,
    ) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        let res = sqlx::query!(
            "UPDATE auth_events SET email = NULL WHERE user_pid = $1 OR email = $2",
            user_pid,
            normalise_email(email),
        )
        .execute(&mut *conn)
        .await?;
        Ok(res.rows_affected())
    }

    /// Empty `source_ip` on every row older than `days` (the address
    /// retention sweep, `crate::compliance::address_retention`). The
    /// column is outside the integrity pre-image, so the rows still
    /// verify. Returns the number of rows emptied.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn scrub_source_ip_older_than(db: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
        let res = sqlx::query!(
            "UPDATE auth_events SET source_ip = NULL
             WHERE source_ip IS NOT NULL AND created_at < now() - make_interval(days => $1)",
            days,
        )
        .execute(db)
        .await?;
        Ok(res.rows_affected())
    }

    /// Most-recent authentication events, newest first, capped at `limit`.
    /// Also the sample `GET /api/compliance/audit/verify` checks.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, event, email, user_pid, detail, hash, hash_sha3, mac, source_ip
               FROM auth_events
               ORDER BY id DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }
}

/// Normalise an email the same way the rate limiter keys it: trim and
/// lowercase, so audit rows and throttle buckets agree on identity.
fn normalise_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::normalise_email;

    #[test]
    fn normalise_email_trims_and_lowercases() {
        assert_eq!(normalise_email("  Alice@Example.COM "), "alice@example.com");
        assert_eq!(normalise_email("bob@example.com"), "bob@example.com");
    }
}
