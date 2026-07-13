//! `sessions` model — issuance and revocation of access tokens.

use std::collections::BTreeMap;

use chrono::offset::Local;
use loco_rs::prelude::*;
use sea_orm::QueryOrder;
use uuid::Uuid;

pub use super::_entities::sessions::{self, ActiveModel, Entity, Model};

/// Build the `sessions.data` JSONB payload for a new session: the user's
/// ABAC `attributes` under the `attrs` member (shared
/// `authorization-attributes.md` §6 — session establishment copies the
/// user's attributes so token minting needs no users read) plus the
/// per-session **CSRF** synchroniser token under `csrf` (shared
/// `authentication-sessions.md` §4). Pure, so the copy shape is
/// unit-testable DB-free.
#[must_use]
pub fn session_data(attributes: &serde_json::Value, csrf: &str) -> serde_json::Value {
    serde_json::json!({ "attrs": attributes, "csrf": csrf })
}

impl ActiveModelBehavior for super::_entities::sessions::ActiveModel {}

/// Default **idle** session TTL (seconds): 30 min. The idle window slides
/// on each use (`/me`); a session expires once idle. Override with
/// `AUTH_SESSION_IDLE_TTL_SECS`.
const DEFAULT_IDLE_TTL_SECS: i64 = 1800;
/// Default **absolute** session TTL (seconds): 12 h — a hard ceiling set
/// at issuance and never extended. Override with
/// `AUTH_SESSION_ABSOLUTE_TTL_SECS`.
const DEFAULT_ABSOLUTE_TTL_SECS: i64 = 43200;

/// Read a positive TTL (seconds) from `var`, else `default`.
fn ttl_secs(var: &str, default: i64) -> i64 {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .filter(|s| *s > 0)
        .unwrap_or(default)
}

/// Pure session-validity check (shared `authentication-sessions.md` §3):
/// a session is valid iff it is not revoked **and** `now` is before both
/// the idle and absolute expiries. A `None` bound is treated as
/// unbounded (a row predating the TTL reshape — legacy grace). Split out
/// so the boundary logic is unit-testable without a clock or database.
#[must_use]
fn is_active_at(
    revoked: bool,
    idle_expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    absolute_expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    now: chrono::DateTime<chrono::FixedOffset>,
) -> bool {
    if revoked {
        return false;
    }
    if idle_expires_at.is_some_and(|idle| now >= idle) {
        return false;
    }
    if absolute_expires_at.is_some_and(|absolute| now >= absolute) {
        return false;
    }
    true
}

impl Model {
    /// Record a freshly issued session so it can be revoked later, with an
    /// **idle** window (`now + idle TTL`, slides on use) and an
    /// **absolute** ceiling (`now + absolute TTL`, never extended). `data`
    /// is the session payload JSONB — build it with [`session_data`] so
    /// the user's ABAC attributes and CSRF token ride along. The legacy
    /// `expires_at` column is set to the absolute ceiling for coherence.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn issue(
        db: &DatabaseConnection,
        jid: &str,
        user_pid: Uuid,
        user_agent: Option<String>,
        data: serde_json::Value,
    ) -> ModelResult<Self> {
        let now = Local::now().fixed_offset();
        let idle = now
            + chrono::Duration::seconds(ttl_secs(
                "AUTH_SESSION_IDLE_TTL_SECS",
                DEFAULT_IDLE_TTL_SECS,
            ));
        let absolute = now
            + chrono::Duration::seconds(ttl_secs(
                "AUTH_SESSION_ABSOLUTE_TTL_SECS",
                DEFAULT_ABSOLUTE_TTL_SECS,
            ));
        let session = sessions::ActiveModel {
            jid: ActiveValue::set(jid.to_string()),
            user_pid: ActiveValue::set(user_pid),
            expires_at: ActiveValue::set(absolute),
            revoked_at: ActiveValue::set(None),
            user_agent: ActiveValue::set(user_agent),
            data: ActiveValue::set(data),
            last_seen_at: ActiveValue::set(Some(now)),
            idle_expires_at: ActiveValue::set(Some(idle)),
            absolute_expires_at: ActiveValue::set(Some(absolute)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(session)
    }

    /// Slide the **idle** window on use: stamp `last_seen_at = now` and
    /// `idle_expires_at = now + idle TTL`, **capped** at the absolute
    /// ceiling (never extends the session past its hard expiry). Called on
    /// `/me`. A no-op-shaped best-effort at the call site — a failure here
    /// must not break the read.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn touch(self, db: &DatabaseConnection) -> ModelResult<Model> {
        let now = Local::now().fixed_offset();
        let mut idle = now
            + chrono::Duration::seconds(ttl_secs(
                "AUTH_SESSION_IDLE_TTL_SECS",
                DEFAULT_IDLE_TTL_SECS,
            ));
        if let Some(absolute) = self.absolute_expires_at
            && idle > absolute
        {
            idle = absolute;
        }
        let mut active = self.into_active_model();
        active.last_seen_at = ActiveValue::set(Some(now));
        active.idle_expires_at = ActiveValue::set(Some(idle));
        active.update(db).await.map_err(ModelError::from)
    }

    /// Find a session by its JWT id.
    ///
    /// # Errors
    ///
    /// When the session does not exist or the query fails.
    pub async fn find_by_jid(db: &DatabaseConnection, jid: &str) -> ModelResult<Self> {
        let session = sessions::Entity::find()
            .filter(
                model::query::condition()
                    .eq(sessions::Column::Jid, jid)
                    .build(),
            )
            .one(db)
            .await?;
        session.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// All sessions for a user, newest first. Used by the GDPR account
    /// export (right of access) — issuance/expiry/revocation timestamps
    /// and the captured `user_agent`, never any token or secret.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_all_by_user_pid(
        db: &DatabaseConnection,
        user_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let rows = sessions::Entity::find()
            .filter(
                model::query::condition()
                    .eq(sessions::Column::UserPid, user_pid)
                    .build(),
            )
            .order_by_desc(sessions::Column::Id)
            .all(db)
            .await?;
        Ok(rows)
    }

    /// Revoke every still-active session for a user (GDPR erasure stamps
    /// `revoked_at` on each). Already-revoked sessions are left as-is so
    /// their original revocation timestamp survives. Returns the number
    /// of sessions newly revoked.
    ///
    /// # Errors
    ///
    /// When a query or update fails.
    pub async fn revoke_all_for_user(
        db: &DatabaseConnection,
        user_pid: Uuid,
    ) -> ModelResult<usize> {
        let sessions = Self::find_all_by_user_pid(db, user_pid).await?;
        let mut revoked = 0;
        for session in sessions {
            if session.is_active() {
                session.into_active_model().revoke(db).await?;
                revoked += 1;
            }
        }
        Ok(revoked)
    }

    /// GDPR erasure (SEC-A7): NULL out the `user_agent` (a retained fragment
    /// of PII) on **every** session of `user_pid`. Revocation alone only sets
    /// `revoked_at` and leaves the user-agent string behind. Returns the
    /// number of rows scrubbed.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn scrub_user_agent_for_user(
        db: &DatabaseConnection,
        user_pid: Uuid,
    ) -> ModelResult<u64> {
        let res = sessions::Entity::update_many()
            .col_expr(
                sessions::Column::UserAgent,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .filter(sessions::Column::UserPid.eq(user_pid))
            .exec(db)
            .await?;
        Ok(res.rows_affected)
    }

    /// True when the session is valid: not revoked, and `now` is before
    /// both its idle and absolute expiries (shared
    /// `authentication-sessions.md` §3). A row predating the TTL reshape
    /// (nullable bounds) is valid until revoked (legacy grace).
    #[must_use]
    pub fn is_active(&self) -> bool {
        is_active_at(
            self.revoked_at.is_some(),
            self.idle_expires_at,
            self.absolute_expires_at,
            Local::now().fixed_offset(),
        )
    }

    /// The ABAC subject attributes copied into this session at
    /// establishment (`data.attrs`), parsed with the same tolerant rules
    /// as the users column ([`super::users::attributes_map`]). A session
    /// predating the copy (or holding no `attrs`) yields an empty map —
    /// read-only under the family's default policy.
    #[must_use]
    pub fn attrs(&self) -> BTreeMap<String, Vec<String>> {
        self.data
            .get("attrs")
            .map(super::users::attributes_map)
            .unwrap_or_default()
    }

    /// This session's CSRF synchroniser token (`data.csrf`), if any. A
    /// session predating CSRF has none — the `POST /token` guard then
    /// skips the CSRF check for it (graceful; new sessions always carry
    /// one). See [`crate::csrf`].
    #[must_use]
    pub fn csrf(&self) -> Option<&str> {
        self.data.get("csrf").and_then(serde_json::Value::as_str)
    }
}

impl ActiveModel {
    /// Mark this session revoked (signout).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn revoke(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.revoked_at = ActiveValue::set(Some(Local::now().into()));
        self.update(db).await.map_err(ModelError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, is_active_at, session_data};
    use chrono::{Duration, FixedOffset, TimeZone};
    use uuid::Uuid;

    #[test]
    fn is_active_at_enforces_revocation_and_idle_and_absolute() {
        let base = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 4, 12, 0, 0)
            .unwrap();
        let future = base + Duration::hours(1);
        let past = base - Duration::hours(1);

        // Not revoked, both bounds in the future ⇒ active.
        assert!(is_active_at(false, Some(future), Some(future), base));
        // Revoked ⇒ inactive regardless of bounds.
        assert!(!is_active_at(true, Some(future), Some(future), base));
        // Idle window elapsed ⇒ inactive (the sliding bound bit).
        assert!(!is_active_at(false, Some(past), Some(future), base));
        // Absolute ceiling reached ⇒ inactive even if idle is fresh.
        assert!(!is_active_at(false, Some(future), Some(past), base));
        // Legacy row (nullable bounds) ⇒ active until revoked.
        assert!(is_active_at(false, None, None, base));
        assert!(!is_active_at(true, None, None, base));
    }

    fn session_with_data(data: serde_json::Value) -> Model {
        let ts = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 4, 12, 0, 0)
            .unwrap();
        Model {
            created_at: ts,
            updated_at: ts,
            id: 1,
            jid: "sid-1".to_string(),
            user_pid: Uuid::new_v4(),
            expires_at: ts,
            revoked_at: None,
            user_agent: None,
            data,
            last_seen_at: None,
            idle_expires_at: None,
            absolute_expires_at: None,
        }
    }

    #[test]
    fn attributes_copy_round_trips_user_column_to_session_attrs() {
        // The §6 copy path: users.attributes → session_data(...) at
        // establishment → Model::attrs() at token minting.
        let attributes = serde_json::json!({ "access": ["write"], "svc": ["true"] });
        let session = session_with_data(session_data(&attributes, "csrf-tok"));
        let attrs = session.attrs();
        assert_eq!(attrs["access"], vec!["write"]);
        assert_eq!(attrs["svc"], vec!["true"]);
        assert_eq!(attrs.len(), 2);
        // The CSRF token also round-trips through the session payload.
        assert_eq!(session.csrf(), Some("csrf-tok"));
    }

    #[test]
    fn session_without_attrs_yields_an_empty_map() {
        // A pre-ABAC session ({} default) or a data payload with no
        // `attrs` member grants nothing — read-only under the family's
        // default policy.
        assert!(session_with_data(serde_json::json!({})).attrs().is_empty());
        assert!(
            session_with_data(serde_json::json!({ "mfa": "done" }))
                .attrs()
                .is_empty()
        );
    }
}
