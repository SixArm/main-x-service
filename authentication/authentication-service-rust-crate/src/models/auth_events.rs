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
use sea_orm::{QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::auth_events::{self, ActiveModel, Entity, Model};

impl ActiveModelBehavior for super::_entities::auth_events::ActiveModel {}

impl Model {
    /// Record one authentication event. **Best-effort**: callers use
    /// [`record`](Self::record) for its side effect and must never fail
    /// the request on an audit error (see [`record_best_effort`]).
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
        let entry = auth_events::ActiveModel {
            event: ActiveValue::set(event.to_string()),
            email: ActiveValue::set(email.map(normalise_email)),
            user_pid: ActiveValue::set(user_pid),
            detail: ActiveValue::set(detail.map(ToString::to_string)),
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
