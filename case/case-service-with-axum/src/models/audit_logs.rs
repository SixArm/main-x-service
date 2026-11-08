//! `audit_logs` model — record and query the CRUD audit trail, and
//! maintain its **tamper-evident hash chain**
//! ([`crate::compliance::audit_chain`]).
//!
//! Every row written through [`Model::record_with_context`] binds its own
//! content and the current chain head, so the trail can prove it was not
//! rewritten (HIPAA §164.312(c)). The trail stays **append-only**: the
//! statements that ever modify an existing row are
//! [`Model::redact_for_entity`], the GDPR Art. 17 erasure path, which
//! destroys content while preserving the `hash` / `prev_hash` linkage so
//! verification still succeeds, and [`Model::set_mac`], the
//! `integrity_resign` task's re-MAC of a row whose MAC verified.

use axa::sqlx::Db;
use chrono::{DateTime, FixedOffset, SubsecRound as _};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use super::{ModelResult, to_i64, updated};
use crate::compliance::audit_chain::{self, ChainInput, ChainReport};

/// Postgres advisory-lock key serialising chain appends.
///
/// The chain head must be read and the successor inserted without another
/// writer interleaving, or two rows would claim the same predecessor and
/// verification would report a fork. `pg_advisory_xact_lock` is held to
/// the end of the enclosing transaction, so under the **`outbox`**
/// transport — where the audit row shares the entity mutation's
/// transaction — appends are fully serialised.
///
/// Under the `memory` transport the audit write is a best-effort side
/// channel on a pooled connection, where each statement is its own
/// implicit transaction and the lock is released immediately. Concurrent
/// writers can then fork the chain, which verification reports as a
/// `linkage` break. **A compliance deployment should run
/// `CASE_EVENT_TRANSPORT=outbox`**; this is stated in the entity
/// spec's honest limits rather than papered over.
const CHAIN_LOCK_KEY: i64 = 0x6D78_695F_6175_6469; // "mxi_audi"

/// One persisted audit row: one CRUD/read action on a case (who / what /
/// when + a snapshot), chained onto its predecessor.
///
/// Timestamps are `DateTime<FixedOffset>`, the type the `SeaORM` entity
/// used, so the audit endpoints serialize them exactly as before.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    /// Row creation timestamp: stamped by the writer, truncated to
    /// microseconds, and bound into the chain digest.
    pub created_at: DateTime<FixedOffset>,
    /// Equal to `created_at`; rows are never updated except by redaction
    /// and re-MACing, which leave it alone.
    pub updated_at: DateTime<FixedOffset>,
    /// Internal auto-increment primary key; the chain order.
    pub id: i64,
    /// The case pid the entry concerns (`Uuid::nil` for a collection
    /// access).
    pub entity_pid: Uuid,
    /// The action verb (`created`, `updated`, `read`, `erased`, …).
    pub action: String,
    /// The caller's `sub`, when a verified token was presented.
    pub actor: Option<String>,
    /// Snapshot of the record at the time of the action; `None` once
    /// redacted.
    pub snapshot: Option<serde_json::Value>,
    /// Hash of the preceding chain row; `None` for the genesis row and for
    /// rows written before the chain existed.
    pub prev_hash: Option<String>,
    /// SHA-3 digest of the preceding chain row.
    pub prev_hash_sha3: Option<String>,
    /// This row's content hash — the link every successor binds to.
    pub hash: Option<String>,
    /// This row's SHA-3 digest — the parallel chain's link.
    pub hash_sha3: Option<String>,
    /// HMAC over this row's pre-image, as `"<key id>:<hex>"`.
    pub mac: Option<String>,
    /// Request/processing context (purpose-of-use, disclosure recipient).
    pub context: Option<serde_json::Value>,
    /// Whether this access was an outward **disclosure** rather than an
    /// internal access — the HIPAA §164.528 accounting distinction.
    pub disclosure: bool,
    /// When the row's content was destroyed under GDPR Art. 17.
    pub redacted_at: Option<DateTime<FixedOffset>>,
}

impl Model {
    /// Record one audit entry. `actor` is the caller's `sub` (user `pid`)
    /// when a verified token was presented, else `None`.
    ///
    /// Takes a [`Db`], so it runs either on the pool (the best-effort
    /// side-channel path, `memory` transport) **or** on an open
    /// transaction — under the `outbox` transport the audit row is written
    /// in the *same* transaction as the entity mutation and its
    /// `event_outbox` row, so the three can never disagree
    /// (`agents/share/event-bus.md` §3).
    ///
    /// This is the mutation-path entry point: no request context and never
    /// a disclosure. Read paths use
    /// [`crate::compliance::disclosure::record_access`].
    ///
    /// # Errors
    ///
    /// When the insert fails.
    pub async fn record(
        db: impl Into<Db<'_>>,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
    ) -> ModelResult<Self> {
        Self::record_with_context(db, entity_pid, action, actor, snapshot, None, false).await
    }

    /// Record one audit entry with its request context and disclosure
    /// flag, chained onto the current head.
    ///
    /// The row's `created_at` is stamped here rather than by the database
    /// default, and **truncated to microseconds**, so the value hashed on
    /// write is byte-identical to the value Postgres returns on read (see
    /// [`crate::compliance::audit_chain`]).
    ///
    /// The advisory lock, the head read and the insert all run on one
    /// connection, so under a transaction the lock covers both.
    ///
    /// # Errors
    ///
    /// When taking the lock, reading the chain head or the insert fails.
    pub async fn record_with_context(
        db: impl Into<Db<'_>>,
        entity_pid: Uuid,
        action: &str,
        actor: Option<&str>,
        snapshot: Option<serde_json::Value>,
        context: Option<serde_json::Value>,
        disclosure: bool,
    ) -> ModelResult<Self> {
        let mut conn = db.into().acquire().await?;
        // Serialise the read-head/append pair (see `CHAIN_LOCK_KEY`).
        sqlx::query!("SELECT pg_advisory_xact_lock($1)", CHAIN_LOCK_KEY)
            .execute(&mut *conn)
            .await?;
        let (prev_hash, prev_hash_sha3) = Self::heads(&mut conn).await?;
        let created_at: DateTime<FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        // One input, re-pointed at each algorithm's own predecessor:
        // the digests cover byte-identical content while the chains stay
        // independent.
        let mut input = ChainInput {
            prev_hash: prev_hash.as_deref(),
            entity_pid,
            action,
            actor,
            created_at_micros: created_at.timestamp_micros(),
            snapshot: snapshot.as_ref(),
            context: context.as_ref(),
            disclosure,
        };
        let hash = audit_chain::row_hash(&input);
        input.prev_hash = prev_hash_sha3.as_deref();
        let hash_sha3 = audit_chain::row_hash_sha3(&input);
        // Keyed over the SHA-256 pre-image, so the MAC binds the same
        // chain position the primary digest does.
        input.prev_hash = prev_hash.as_deref();
        let mac = crate::compliance::mac::tag(
            crate::compliance::mac::Domain::AuditChain,
            &audit_chain::preimage(&input),
        );
        let row = sqlx::query_as!(
            Self,
            r#"INSERT INTO audit_logs
                   (created_at, updated_at, entity_pid, action, actor, snapshot,
                    prev_hash, hash, prev_hash_sha3, hash_sha3, mac,
                    context, disclosure, redacted_at)
               VALUES ($1, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NULL)
               RETURNING created_at AS "created_at: _", updated_at AS "updated_at: _",
                         id, entity_pid, action, actor, snapshot,
                         prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                         context, disclosure, redacted_at AS "redacted_at: _""#,
            created_at,
            entity_pid,
            action,
            actor,
            snapshot,
            prev_hash,
            Some(hash),
            prev_hash_sha3,
            Some(hash_sha3),
            mac,
            context,
            disclosure,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(row)
    }

    /// Both chain heads — `(SHA-256, SHA-3)` — from the same row.
    ///
    /// Read together so an append binds each algorithm's *own*
    /// predecessor. Binding the SHA-256 head into the SHA-3 digest would
    /// make the second chain's linkage rest on SHA-256's collision
    /// resistance, which is the dependency keeping two algorithms is
    /// meant to avoid.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn heads(conn: &mut PgConnection) -> ModelResult<(Option<String>, Option<String>)> {
        let last = sqlx::query!("SELECT hash, hash_sha3 FROM audit_logs ORDER BY id DESC LIMIT 1")
            .fetch_optional(conn)
            .await?;
        Ok(last.map_or((None, None), |row| (row.hash, row.hash_sha3)))
    }

    /// The current chain head: the most recent row's `hash`, or `None`
    /// when the trail is empty (or its last row predates the chain, in
    /// which case the next row starts a fresh run — see
    /// [`audit_chain::verify`]).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn head(db: impl Into<Db<'_>>) -> ModelResult<Option<String>> {
        let mut conn = db.into().acquire().await?;
        Ok(Self::heads(&mut conn).await?.0)
    }

    /// Most-recent audit entries, capped at `limit`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn recent(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               ORDER BY id DESC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Audit entries for one case, most-recent first.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn for_entity(db: &PgPool, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               WHERE entity_pid = $1
               ORDER BY id DESC"#,
            entity_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Disclosure entries for one case, most-recent first — the
    /// HIPAA §164.528 accounting of disclosures.
    ///
    /// Only rows the access context classified as an outward disclosure
    /// are returned (see [`crate::compliance::disclosure::AccessContext::is_disclosure`]);
    /// ordinary internal accesses are excluded, which is the whole point
    /// of the distinction.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn disclosures_for_entity(db: &PgPool, entity_pid: Uuid) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               WHERE entity_pid = $1 AND disclosure = true
               ORDER BY id DESC"#,
            entity_pid,
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// The newest `limit` rows in **ascending `id` order** — the shape
    /// [`audit_chain::verify`] expects.
    ///
    /// Verifying a suffix of the trail is sound: the run's first row has no
    /// predecessor to check against, and every row after it is fully
    /// checked.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn tail(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let mut rows = Self::recent(db, limit).await?;
        rows.reverse();
        Ok(rows)
    }

    /// Verify the newest `limit` rows of the chain.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn verify_chain(db: &PgPool, limit: u64) -> ModelResult<ChainReport> {
        Ok(audit_chain::verify(&Self::tail(db, limit).await?))
    }

    /// One row by its primary key, or `None`.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id(db: &PgPool, id: i64) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               WHERE id = $1"#,
            id,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// The newest row carrying a chain `hash` — the anchor a checkpoint
    /// witnesses — or `None` when no row is chained yet.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn newest_hashed(db: &PgPool) -> ModelResult<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               WHERE hash IS NOT NULL
               ORDER BY id DESC
               LIMIT 1"#,
        )
        .fetch_optional(db)
        .await?;
        Ok(row)
    }

    /// How many rows have an `id` at or before `id` — the count a
    /// checkpoint records, so a later deletion below the anchor shows.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn count_at_or_before(db: &PgPool, id: i64) -> ModelResult<u64> {
        let n = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM audit_logs WHERE id <= $1"#,
            id
        )
        .fetch_one(db)
        .await?;
        Ok(u64::try_from(n).unwrap_or(0))
    }

    /// The rows carrying a MAC, oldest first, capped at `limit` — the
    /// `integrity_resign` task's walk.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn with_mac_oldest_first(db: &PgPool, limit: u64) -> ModelResult<Vec<Self>> {
        let rows = sqlx::query_as!(
            Self,
            r#"SELECT created_at AS "created_at: _", updated_at AS "updated_at: _",
                      id, entity_pid, action, actor, snapshot,
                      prev_hash, prev_hash_sha3, hash, hash_sha3, mac,
                      context, disclosure, redacted_at AS "redacted_at: _"
               FROM audit_logs
               WHERE mac IS NOT NULL
               ORDER BY id ASC
               LIMIT $1"#,
            to_i64(limit),
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    /// Replace one row's MAC, and nothing else — the MAC is not part of
    /// the chain, so this cannot break linkage.
    ///
    /// # Errors
    ///
    /// When the update fails, including when the row is gone
    /// (`RowNotFound`, as `SeaORM`'s `RecordNotUpdated` was).
    pub async fn set_mac(db: &PgPool, id: i64, mac: Option<&str>) -> ModelResult<()> {
        let done = sqlx::query!("UPDATE audit_logs SET mac = $2 WHERE id = $1", id, mac)
            .execute(db)
            .await?;
        updated(done.rows_affected())
    }

    /// GDPR Art. 17: destroy the audit **content** held about one case
    /// while preserving the chain.
    ///
    /// Sets `snapshot = NULL` and stamps `redacted_at` on every not-yet
    /// redacted row for `entity_pid`. The row's `hash` and `prev_hash` are
    /// deliberately **left untouched**, so [`audit_chain::verify`] still
    /// checks linkage across the redacted row and the trail as a whole
    /// keeps verifying — the erasure is real, and the accountability
    /// record that *something happened, when, and by whom* survives.
    ///
    /// `actor` and `action` are retained on purpose: they are the
    /// controller's own accountability record under the Art. 17(3)(b)
    /// legal-obligation carve-out, and they identify the operator rather
    /// than the erased subject.
    ///
    /// Returns the number of rows redacted.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn redact_for_entity(db: impl Into<Db<'_>>, entity_pid: Uuid) -> ModelResult<u64> {
        let now: DateTime<FixedOffset> = chrono::Utc::now().trunc_subsecs(6).into();
        let mut conn = db.into().acquire().await?;
        let done = sqlx::query!(
            "UPDATE audit_logs SET snapshot = NULL, redacted_at = $2
             WHERE entity_pid = $1 AND redacted_at IS NULL",
            entity_pid,
            now,
        )
        .execute(&mut *conn)
        .await?;
        Ok(done.rows_affected())
    }
}
