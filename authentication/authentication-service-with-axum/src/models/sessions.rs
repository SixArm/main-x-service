//! `sessions` model — issuance and revocation of cookie sessions.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset, offset::Local};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{ModelError, ModelResult};

/// A `sessions` row: one server-side cookie session, keyed by `jid` (the
/// hash of the opaque session id the client presents) so the auth service
/// can revoke it on signout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation time.
    pub created_at: DateTime<FixedOffset>,
    /// Last update time.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal autoincrement primary key.
    pub id: i64,
    /// SHA-256 of the opaque session id (SEC-A9); unique.
    pub jid: String,
    /// Owning user's `pid`.
    pub user_pid: Uuid,
    /// Token expiry.
    pub expires_at: DateTime<FixedOffset>,
    /// Set when the session is revoked via signout.
    pub revoked_at: Option<DateTime<FixedOffset>>,
    /// Best-effort user agent captured at issuance.
    pub user_agent: Option<String>,
    /// Session payload JSONB (shared authentication-sessions design §3).
    /// Holds the user's ABAC attributes under `attrs`, copied at session
    /// establishment so token minting reads only the session.
    pub data: serde_json::Value,
    /// When the session was last used (bumped on each `/me`). Nullable
    /// for rows predating the TTL reshape.
    pub last_seen_at: Option<DateTime<FixedOffset>>,
    /// Sliding **idle** expiry: `now + idle TTL`, bumped on each use. A
    /// session is expired once `now >= idle_expires_at`. Nullable (legacy
    /// rows ⇒ no idle bound).
    pub idle_expires_at: Option<DateTime<FixedOffset>>,
    /// Hard **absolute** expiry: `now + absolute TTL` at issuance, never
    /// extended. Nullable (legacy rows ⇒ no absolute bound).
    pub absolute_expires_at: Option<DateTime<FixedOffset>>,
    /// Best-effort connecting-peer address captured at issuance (T-14).
    /// Nullable for rows predating this column.
    pub source_ip: Option<String>,
}

/// Build the `sessions.data` JSONB payload for a new session: the user's
/// ABAC `attributes` under the `attrs` member (shared
/// `authorization-attributes.md` §6 — session establishment copies the
/// user's attributes so token minting needs no users read) plus the
/// per-session **CSRF** synchroniser token under `csrf` (shared
/// `authentication-sessions.md` §4). Pure, so the copy shape is
/// unit-testable DB-free.
///
/// SEC-A9: `csrf` is the **plaintext** synchroniser token (the value put
/// in the readable `__Host-mxi_csrf` cookie); only its **hash** is stored
/// here, so the database never holds the usable CSRF token. Validation
/// therefore compares `hash(presented)` against this stored hash
/// (`crate::secret_hash`, [`Model::csrf`]).
#[must_use]
pub fn session_data(attributes: &serde_json::Value, csrf: &str) -> serde_json::Value {
    serde_json::json!({ "attrs": attributes, "csrf": crate::secret_hash::hash(csrf) })
}

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
    /// `source_ip` is the client's address (T-14), from
    /// `axa::client::ClientAddr`. Like `user_agent` it is advisory audit
    /// context, never trusted for a security decision.
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn issue(
        db: &PgPool,
        jid: &str,
        user_pid: Uuid,
        user_agent: Option<String>,
        source_ip: Option<String>,
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
        // SEC-A9: `jid` is the opaque session id the client presents (the
        // cookie value + PASETO `sid` claim); the DB stores only its hash,
        // so a read at rest yields nothing replayable. Lookups hash the
        // presented id the same way (`find_by_jid`).
        let session = sqlx::query_as!(
            Self,
            r#"INSERT INTO sessions
                   (jid, user_pid, expires_at, revoked_at, user_agent, source_ip, data,
                    last_seen_at, idle_expires_at, absolute_expires_at)
               VALUES ($1, $2, $3, NULL, $4, $5, $6, $7, $8, $9)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, jid, user_pid, expires_at AS "expires_at: _",
                         revoked_at AS "revoked_at: _", user_agent, data,
                         last_seen_at AS "last_seen_at: _",
                         idle_expires_at AS "idle_expires_at: _",
                         absolute_expires_at AS "absolute_expires_at: _", source_ip"#,
            crate::secret_hash::hash(jid),
            user_pid,
            absolute,
            user_agent,
            source_ip,
            data,
            now,
            idle,
            absolute,
        )
        .fetch_one(db)
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
    /// When the update fails, or matches no row.
    pub async fn touch(self, db: &PgPool) -> ModelResult<Model> {
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
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE sessions
               SET last_seen_at = $2, idle_expires_at = $3
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, jid, user_pid, expires_at AS "expires_at: _",
                         revoked_at AS "revoked_at: _", user_agent, data,
                         last_seen_at AS "last_seen_at: _",
                         idle_expires_at AS "idle_expires_at: _",
                         absolute_expires_at AS "absolute_expires_at: _", source_ip"#,
            self.id,
            now,
            idle,
        )
        .fetch_optional(db)
        .await?;
        row.ok_or(ModelError::RecordNotUpdated)
    }

    /// Find a session by its opaque id.
    ///
    /// # Errors
    ///
    /// When the session does not exist or the query fails.
    pub async fn find_by_jid(db: &PgPool, jid: &str) -> ModelResult<Self> {
        // SEC-A9: the DB stores the hash of the session id, so match on the
        // hash of the presented (cookie / `sid`-claim) value.
        let session = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, jid, user_pid, expires_at AS "expires_at: _",
                      revoked_at AS "revoked_at: _", user_agent, data,
                      last_seen_at AS "last_seen_at: _",
                      idle_expires_at AS "idle_expires_at: _",
                      absolute_expires_at AS "absolute_expires_at: _", source_ip
               FROM sessions
               WHERE jid = $1
               LIMIT 1"#,
            crate::secret_hash::hash(jid),
        )
        .fetch_optional(db)
        .await?;
        session.ok_or(ModelError::EntityNotFound)
    }

    /// All sessions for a user, newest first. Used by the GDPR account
    /// export (right of access) — issuance/expiry/revocation timestamps
    /// and the captured `user_agent`, never any token or secret.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_all_by_user_pid(
        db: impl Into<axa::sqlx::Db<'_>>,
        user_pid: Uuid,
    ) -> ModelResult<Vec<Self>> {
        let mut conn = db.into().acquire().await?;
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, jid, user_pid, expires_at AS "expires_at: _",
                      revoked_at AS "revoked_at: _", user_agent, data,
                      last_seen_at AS "last_seen_at: _",
                      idle_expires_at AS "idle_expires_at: _",
                      absolute_expires_at AS "absolute_expires_at: _", source_ip
               FROM sessions
               WHERE user_pid = $1
               ORDER BY id DESC"#,
            user_pid,
        )
        .fetch_all(&mut *conn)
        .await?;
        Ok(rows)
    }

    /// Revoke every still-active session for a user (GDPR erasure stamps
    /// `revoked_at` on each). Already-revoked sessions are left as-is so
    /// their original revocation timestamp survives. Returns the number
    /// of sessions newly revoked.
    ///
    /// Separate statements on one connection, not a transaction, as
    /// before: each revocation commits on its own.
    ///
    /// # Errors
    ///
    /// When a query or update fails.
    pub async fn revoke_all_for_user(
        db: impl Into<axa::sqlx::Db<'_>>,
        user_pid: Uuid,
    ) -> ModelResult<usize> {
        let mut conn = db.into().acquire().await?;
        let sessions = Self::find_all_by_user_pid(&mut *conn, user_pid).await?;
        let mut revoked = 0;
        for session in sessions {
            if session.is_active() {
                session.revoke(&mut *conn).await?;
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
        db: impl Into<axa::sqlx::Db<'_>>,
        user_pid: Uuid,
    ) -> ModelResult<u64> {
        let mut conn = db.into().acquire().await?;
        let res = sqlx::query!(
            "UPDATE sessions SET user_agent = NULL WHERE user_pid = $1",
            user_pid,
        )
        .execute(&mut *conn)
        .await?;
        Ok(res.rows_affected())
    }

    /// Empty `source_ip` on every session older than `days` (the address
    /// retention sweep, `crate::compliance::address_retention`). Returns
    /// the number of rows emptied.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn scrub_source_ip_older_than(db: &PgPool, days: i32) -> Result<u64, sqlx::Error> {
        let res = sqlx::query!(
            "UPDATE sessions SET source_ip = NULL
             WHERE source_ip IS NOT NULL AND created_at < now() - make_interval(days => $1)",
            days,
        )
        .execute(db)
        .await?;
        Ok(res.rows_affected())
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

    /// Mark this session revoked (signout).
    ///
    /// # Errors
    ///
    /// When the update fails, or matches no row.
    pub async fn revoke(self, db: impl Into<axa::sqlx::Db<'_>>) -> ModelResult<Model> {
        let mut conn = db.into().acquire().await?;
        let row = sqlx::query_as!(
            Self,
            r#"UPDATE sessions
               SET revoked_at = $2
               WHERE id = $1
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, jid, user_pid, expires_at AS "expires_at: _",
                         revoked_at AS "revoked_at: _", user_agent, data,
                         last_seen_at AS "last_seen_at: _",
                         idle_expires_at AS "idle_expires_at: _",
                         absolute_expires_at AS "absolute_expires_at: _", source_ip"#,
            self.id,
            Local::now().fixed_offset(),
        )
        .fetch_optional(&mut *conn)
        .await?;
        row.ok_or(ModelError::RecordNotUpdated)
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
            source_ip: None,
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
        // SEC-A9: `session_data` stores the *hash* of the CSRF token, not the
        // plaintext — the DB never holds the usable synchroniser token.
        let stored = session.csrf().expect("csrf stored");
        assert_eq!(stored, crate::secret_hash::hash("csrf-tok"));
        assert_ne!(stored, "csrf-tok", "the plaintext csrf must not be stored");
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
