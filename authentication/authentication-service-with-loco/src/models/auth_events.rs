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

use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, Condition, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::auth_events::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::auth_events::ActiveModel {}

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
    /// `expired_token`). Pass no token or secret here.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: &DatabaseConnection,
        event: &str,
        email: Option<&str>,
        user_pid: Option<Uuid>,
        detail: Option<&str>,
    ) -> ModelResult<Self> {
        // The timestamp is part of the MAC pre-image, so it is minted here
        // rather than defaulted by the database: the alternative —
        // insert, then stamp — leaves a window in which the row exists
        // unprotected.
        let created_at: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
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
        let entry = auth_events::ActiveModel {
            event: ActiveValue::set(event.to_string()),
            email: ActiveValue::set(email),
            user_pid: ActiveValue::set(user_pid),
            detail: ActiveValue::set(detail),
            created_at: ActiveValue::set(created_at),
            hash: ActiveValue::set(Some(digests.sha256)),
            hash_sha3: ActiveValue::set(Some(digests.sha3)),
            mac: ActiveValue::set(digests.mac),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(entry)
    }

    /// Record one authentication event, swallowing any error after
    /// logging it. Auditing must never break the request path, so
    /// handlers call this rather than `record` + `?`.
    pub async fn record_best_effort(
        db: &DatabaseConnection,
        event: &str,
        email: Option<&str>,
        user_pid: Option<Uuid>,
        detail: Option<&str>,
    ) {
        if let Err(err) = Self::record(db, event, email, user_pid, detail).await {
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
        db: &DatabaseConnection,
        target_email: Option<&str>,
        target_pid: Uuid,
        op: &str,
        key: Option<&str>,
        actor: &str,
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
        db: &DatabaseConnection,
        target_email: Option<&str>,
        target_pid: Uuid,
        actor: &str,
    ) {
        let detail = format!("actor={actor}");
        Self::record_best_effort(
            db,
            ATTRIBUTES_VIEWED,
            target_email,
            Some(target_pid),
            Some(&detail),
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
    pub async fn for_subject(
        db: &DatabaseConnection,
        user_pid: Uuid,
        email: &str,
    ) -> ModelResult<Vec<Self>> {
        let rows = auth_events::Entity::find()
            .filter(
                Condition::any()
                    .add(auth_events::Column::UserPid.eq(user_pid))
                    .add(auth_events::Column::Email.eq(normalise_email(email))),
            )
            .order_by_desc(auth_events::Column::Id)
            .all(db)
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
        db: &DatabaseConnection,
        user_pid: Uuid,
        email: &str,
    ) -> ModelResult<u64> {
        let res = auth_events::Entity::update_many()
            .col_expr(
                auth_events::Column::Email,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .filter(
                Condition::any()
                    .add(auth_events::Column::UserPid.eq(user_pid))
                    .add(auth_events::Column::Email.eq(normalise_email(email))),
            )
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }

    /// Most-recent authentication events, newest first, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &DatabaseConnection, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = auth_events::Entity::find()
            .order_by_desc(auth_events::Column::Id)
            .limit(limit)
            .all(db)
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
