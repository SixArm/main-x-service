//! Audit log repository: HIPAA-style write/query of the `audit_log` table.
//!
//! [`AuditLogRepository`] records who did what, when, and to which entity,
//! capturing old/new JSON snapshots plus request provenance (user id, IP,
//! user agent). The `log_create` / `log_update` / `log_delete` helpers are
//! thin wrappers over the private `log_action` insert. Query helpers back
//! the audit REST endpoints (per-entity, recent, per-user history).
//!
//! Every statement against `audit_log` is here, including the address
//! retention sweep ([`scrub_addresses`]), the checkpoint reads, and the
//! `integrity_resign` task's re-MAC.

use axa::sqlx::Db;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use uuid::Uuid;

use super::convert::offset_to_ts;
use super::models::{AuditLogRaw, audit_log};
use super::repositories::AuditContext;
use super::{not_updated, to_i64};
use crate::Result;
use crate::compliance::audit_chain;

/// Postgres advisory-lock key serialising chain appends.
///
/// The chain head must be read and its successor inserted without another
/// writer interleaving, or two rows would claim the same predecessor and
/// verification would report a fork.
const CHAIN_LOCK_KEY: i64 = 0x6D78_695F_7072_736E; // "mxi_prsn"

/// Reads and writes the `audit_log` table.
///
/// Holds a cloned [`PgPool`]; construct one per shared application state
/// and wrap in an `Arc` to share across handlers.
pub struct AuditLogRepository {
    /// The pool used for every audit query/insert.
    db: PgPool,
}

/// The `entity_type` spellings that mean "person", newest first in
/// preference order.
///
/// One list, because a per-entity audit query that filters on a single
/// spelling silently drops the rows written under another — and a silently
/// short audit answer is worse than an error, since nothing about the
/// response says it is incomplete.
///
/// - `"Person"` — the canonical spelling. Repository mutations have always
///   used it, and every writer uses it now.
/// - `"person"` — written by the read-auditing path between the audit
///   chain landing and 2026-07-26. A query for one spelling returned none
///   of the other's rows, so the per-entity audit endpoint silently
///   omitted every read, and an accounting of disclosures built on it
///   would have looked empty while disclosures were being recorded.
/// - `"patient"` — written by the `audit_patients_changes` database
///   trigger, from before the tables were renamed. The trigger is dropped
///   by `m20260726_000003_drop_audit_triggers`, but its rows remain and
///   are still part of the record.
///
/// Historical rows are **not** rewritten to the canonical spelling.
/// `entity_type` is bound into the audit chain's row digest, so an
/// `UPDATE` normalising it would make every affected chained row fail
/// verification — the chain would correctly report that someone had
/// edited the audit trail, because someone had. Tolerating the spelling on
/// read is the only option that keeps both the history and its integrity.
pub const ENTITY_TYPE_SPELLINGS: [&str; 3] = ["Person", "person", "patient"];

/// Expand `entity_type` to every spelling that means the same entity.
///
/// Only the canonical entity name expands; anything else (for example
/// `"PersonBulkExport"`, which the bulk pipeline audits under its own
/// type) is returned unchanged, so this cannot silently widen an unrelated
/// query.
#[must_use]
pub fn entity_type_spellings(entity_type: &str) -> Vec<&str> {
    if entity_type == "Person" {
        ENTITY_TYPE_SPELLINGS.to_vec()
    } else {
        vec![entity_type]
    }
}

impl AuditLogRepository {
    /// Wrap a database connection in an audit repository.
    #[must_use]
    pub fn new(db: PgPool) -> Self {
        Self { db }
    }

    /// Record a `CREATE`: stores `new_values`, with no prior snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            ctx,
        )
        .await
    }

    /// Record an `UPDATE`: stores both the prior and new JSON snapshots.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            ctx,
        )
        .await
    }

    /// Record a `DELETE`: stores the prior snapshot, with no new values.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            ctx,
        )
        .await
    }

    /// Record an `EXPORT`: stores the export `details` (actor, filter,
    /// format, masking profile, `include_soft_deleted`, row count) as the
    /// new-values snapshot, with no prior snapshot. Used for the bulk
    /// export compliance trail (`bulk-import-export.md` §8) — a bulk
    /// extract of personal data is itself an audited event.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_export(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        details: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action("EXPORT", entity_type, entity_id, None, Some(details), ctx)
            .await
    }

    /// Record a job-level **bulk import** audit row (SEC-B8): a bulk load of
    /// personal data is itself an audited event, distinct from the per-row
    /// create/update audit. `details` carries the reconciled job summary
    /// (counts, dry-run, actor), and `ctx` the acting operator.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_import(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        details: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action("IMPORT", entity_type, entity_id, None, Some(details), ctx)
            .await
    }

    /// Insert one audit row. Shared backend for the typed `log_*` helpers.
    ///
    /// Stamps a fresh UUID and the current UTC time; `old_values` /
    /// `new_values` are `None` for the side that does not apply.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action_on(
            &self.db,
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            ctx,
        )
        .await
    }

    /// SEC-B10: insert one audit row on `db` — the pool, or a caller's
    /// connection such as an open transaction, so a caller can write the audit
    /// **inside** the same transaction as the entity change and have both
    /// commit (or roll back) atomically. A crash after the entity commit can
    /// then no longer lose the audit row.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    // Mirrors the private `log_action` field list plus the target connection;
    // the audit row genuinely has this many independent columns.
    #[allow(clippy::too_many_arguments)]
    pub async fn log_action_on(
        &self,
        conn: impl Into<Db<'_>>,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_chained(
            conn,
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            ctx,
            None,
            false,
        )
        .await
    }

    /// Record one **read/disclosure** access (HIPAA §164.312(b),
    /// §164.528), carrying the caller's declared purpose-of-use context
    /// and whether the access was an outward disclosure.
    ///
    /// Separate from [`Self::log_action_on`] because the two differ in
    /// what they mean, not just in their arguments: a mutation records a
    /// change, an access records that data was *seen*. Only the latter
    /// can be a disclosure.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_access(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        action: &str,
        ctx: &AuditContext,
        access: &crate::compliance::disclosure::AccessContext,
    ) -> Result<()> {
        self.log_chained(
            &self.db,
            action,
            entity_type,
            entity_id,
            None,
            None,
            ctx,
            Some(access.to_json()),
            access.is_disclosure(),
        )
        .await
    }

    /// Redact every audit row about one entity: destroy the value
    /// snapshots and stamp `redacted_at`, **keeping `hash` and
    /// `prev_hash`**. Returns how many rows were redacted.
    ///
    /// Keeping the digests is the whole design. Deleting the rows would
    /// honour GDPR Art. 17 and destroy HIPAA §164.312(c) integrity;
    /// refusing the erasure would do the reverse. Redaction destroys the
    /// content while leaving each row's stored hash and linkage in place,
    /// so [`audit_chain::verify`] still checks across it and the chain as
    /// a whole keeps verifying. `redacted_at` is what tells a reader that
    /// the missing content was erased on purpose rather than lost.
    ///
    /// Already-redacted rows are skipped, so re-running an erasure does
    /// not restamp them with a later time and misreport when the data
    /// actually went.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the update fails.
    pub async fn redact_for_entity(&self, conn: impl Into<Db<'_>>, entity_id: Uuid) -> Result<u64> {
        let now = audit_chain::trunc_micros(time::OffsetDateTime::now_utc());
        let mut conn = conn.into().acquire().await?;
        let result = sqlx::query!(
            "UPDATE audit_log SET old_values = NULL, new_values = NULL, redacted_at = $2
             WHERE entity_id = $1 AND redacted_at IS NULL",
            entity_id,
            offset_to_ts(now),
        )
        .execute(&mut *conn)
        .await?;
        Ok(result.rows_affected())
    }

    /// Append the chained `erased` accountability row for a GDPR Art. 17
    /// erasure.
    ///
    /// Deliberately carries no value snapshot: it records *that* a record
    /// was erased, by whom, and when — the controller's own accountability
    /// record under the Art. 17(3)(b) legal-obligation carve-out — and
    /// nothing about the data subject. It is therefore never itself
    /// redacted.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the insert fails.
    pub async fn log_erasure(
        &self,
        conn: impl Into<Db<'_>>,
        entity_id: Uuid,
        actor: Option<&str>,
        client: Option<std::net::IpAddr>,
        context: JsonValue,
        disclosure: bool,
    ) -> Result<()> {
        let ctx = AuditContext {
            user_id: actor.map(ToString::to_string),
            ip_address: client.map(|address| address.to_string()),
            user_agent: None,
        };
        self.log_chained(
            conn,
            crate::compliance::erasure::ACTION_ERASED,
            "Person",
            entity_id,
            None,
            None,
            &ctx,
            Some(context),
            disclosure,
        )
        .await
    }

    /// The chained insert both write paths share.
    #[allow(clippy::too_many_arguments)]
    async fn log_chained(
        &self,
        conn: impl Into<Db<'_>>,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
        context: Option<JsonValue>,
        disclosure: bool,
    ) -> Result<()> {
        // Serialise the read-head/append pair so two concurrent writers
        // cannot claim the same predecessor and fork the chain. Held to
        // the end of the enclosing transaction, so an audit row written
        // inside a caller's transaction is fully serialised; on a pooled
        // connection each statement is its own implicit transaction and a
        // concurrent fork remains possible, which verification reports as
        // a `linkage` break rather than hiding. The lock, the head read and
        // the insert all run on one connection, so under a transaction the
        // lock covers both.
        let mut conn = conn.into().acquire().await?;
        sqlx::query!("SELECT pg_advisory_xact_lock($1)", CHAIN_LOCK_KEY)
            .execute(&mut *conn)
            .await?;
        // Both chain heads from the same row, so each algorithm binds
        // its *own* predecessor. Binding the SHA-256 head into the SHA-3
        // digest would make the second chain's linkage rest on SHA-256's
        // collision resistance — the dependency two algorithms exist to
        // avoid.
        let (prev_hash, prev_hash_sha3) = Self::chain_heads(&mut *conn).await?;

        let id = Uuid::new_v4();
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::audit_chain`).
        let timestamp = audit_chain::trunc_micros(time::OffsetDateTime::now_utc());
        let committed = ctx.ip_address.as_deref().map(audit_chain::commit_address);
        let (ip_salt, ip_commitment) = committed.unzip();
        let mut chain_input = audit_chain::ChainInput {
            prev_hash: prev_hash.as_deref(),
            id,
            timestamp_micros: audit_chain::micros(timestamp),
            user_id: ctx.user_id.as_deref(),
            action,
            entity_type,
            entity_id,
            old_values: old_values.as_ref(),
            new_values: new_values.as_ref(),
            ip_address: ctx.ip_address.as_deref(),
            ip_commitment: ip_commitment.as_deref(),
            user_agent: ctx.user_agent.as_deref(),
            context: context.as_ref(),
            disclosure,
        };
        let hash = audit_chain::row_hash(&chain_input);
        chain_input.prev_hash = prev_hash_sha3.as_deref();
        let hash_sha3 = audit_chain::row_hash_sha3(&chain_input);
        // Keyed over the SHA-256 pre-image, so the MAC binds the same
        // chain position the primary digest does.
        chain_input.prev_hash = prev_hash.as_deref();
        let mac = crate::compliance::mac::tag(
            crate::compliance::mac::Domain::AuditChain,
            &audit_chain::preimage(&chain_input),
        );

        // `seq` is a BIGSERIAL: Postgres assigns the append order.
        sqlx::query!(
            "INSERT INTO audit_log
                 (id, timestamp, user_id, action, entity_type, entity_id, old_values,
                  new_values, ip_address, user_agent, prev_hash, prev_hash_sha3, hash_sha3,
                  mac, hash, context, disclosure, redacted_at, ip_salt, ip_commitment)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                     $17, NULL, $18, $19)",
            id,
            offset_to_ts(timestamp),
            ctx.user_id,
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            ctx.ip_address,
            ctx.user_agent,
            prev_hash,
            prev_hash_sha3,
            hash_sha3,
            mac,
            hash,
            context,
            disclosure,
            ip_salt,
            ip_commitment,
        )
        .execute(&mut *conn)
        .await?;

        Ok(())
    }

    /// Both chain heads — `(SHA-256, SHA-3)` — from the same row.
    ///
    /// Read together so an append binds each algorithm's own predecessor.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_heads(conn: impl Into<Db<'_>>) -> Result<(Option<String>, Option<String>)> {
        let mut conn = conn.into().acquire().await?;
        let last = sqlx::query!("SELECT hash, hash_sha3 FROM audit_log ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut *conn)
            .await?;
        Ok(match last {
            Some(row) => (row.hash, row.hash_sha3),
            None => (None, None),
        })
    }

    /// The current chain head: the most recent row's `hash`, or `None`
    /// when the trail is empty (or its last row predates the chain, in
    /// which case the next row starts a fresh run — see
    /// [`audit_chain::verify`]).
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_head(conn: impl Into<Db<'_>>) -> Result<Option<String>> {
        Ok(Self::chain_heads(conn).await?.0)
    }

    /// The newest `limit` rows in **ascending `seq` order** — the shape
    /// [`audit_chain::verify`] expects.
    ///
    /// Verifying a suffix is sound: the run's first row has no predecessor
    /// to check against, and every row after it is fully checked.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_tail(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let mut rows: Vec<audit_log::Model> = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             ORDER BY seq DESC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
        rows.reverse();
        Ok(rows)
    }

    /// Verify the newest `limit` rows of the chain.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn verify_chain(&self, limit: u64) -> Result<audit_chain::ChainReport> {
        Ok(audit_chain::verify(&self.chain_tail(limit).await?))
    }

    /// SEC-B10: transaction-scoped `UPDATE` audit — like [`log_update`], but
    /// inserted on `conn` (e.g. an open transaction) so it commits
    /// atomically with the entity change.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_update_on(
        &self,
        conn: impl Into<Db<'_>>,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action_on(
            conn,
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            ctx,
        )
        .await
    }

    /// SEC-B10: transaction-scoped `DELETE` audit — like [`log_delete`], but
    /// inserted on `conn` (e.g. an open transaction) so it commits
    /// atomically with the entity change.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_delete_on(
        &self,
        conn: impl Into<Db<'_>>,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        ctx: &AuditContext,
    ) -> Result<()> {
        self.log_action_on(
            conn,
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            ctx,
        )
        .await
    }

    /// Return up to `limit` audit rows for one entity, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let spellings = entity_type_spellings(entity_type);
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             WHERE entity_type = ANY($1::text[]) AND entity_id = $2
             ORDER BY timestamp DESC
             LIMIT $3",
            &spellings as &[&str],
            entity_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Every audit row for one entity flagged as an outward
    /// **disclosure**, newest first — the HIPAA §164.528 accounting.
    ///
    /// Accepts every spelling in [`ENTITY_TYPE_SPELLINGS`], so the
    /// accounting cannot silently omit rows written under an older one.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn disclosures_for_entity(
        &self,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             WHERE entity_type = ANY($1::text[]) AND entity_id = $2 AND disclosure = true
             ORDER BY timestamp DESC
             LIMIT $3",
            &ENTITY_TYPE_SPELLINGS as &[&str],
            entity_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Return the `limit` most recent audit rows system-wide.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             ORDER BY timestamp DESC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Return up to `limit` audit rows for one user id, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn get_logs_by_user(
        &self,
        user_id: &str,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             WHERE user_id = $1
             ORDER BY timestamp DESC
             LIMIT $2",
            user_id,
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;

        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Every audit row about `entity_id`, in append (`seq`) order,
    /// optionally narrowed to one `entity_type` and one `action`. A
    /// ground-truth read for tests and operators.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn rows_for(
        &self,
        entity_type: Option<&str>,
        entity_id: Uuid,
        action: Option<&str>,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             WHERE entity_id = $1
               AND ($2::text IS NULL OR entity_type = $2)
               AND ($3::text IS NULL OR action = $3)
             ORDER BY seq",
            entity_id,
            entity_type,
            action,
        )
        .fetch_all(&self.db)
        .await?;
        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Up to `limit` rows carrying a MAC, oldest first — the
    /// `integrity_resign` task's walk.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn rows_with_mac(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = sqlx::query_as!(
            AuditLogRaw,
            "SELECT id, timestamp, user_id, action, entity_type, entity_id, old_values,
                    new_values, ip_address, user_agent, seq, prev_hash, prev_hash_sha3, hash,
                    hash_sha3, mac, context, disclosure, redacted_at, ip_salt, ip_commitment
             FROM audit_log
             WHERE mac IS NOT NULL
             ORDER BY seq ASC
             LIMIT $1",
            to_i64(limit),
        )
        .fetch_all(&self.db)
        .await?;
        Ok(logs.into_iter().map(Into::into).collect())
    }

    /// Replace one row's MAC, the only column `integrity_resign` writes.
    /// No matching row is an error, as it was under `SeaORM`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the update fails or matches
    /// no row.
    pub async fn set_mac(&self, id: Uuid, mac: Option<String>) -> Result<()> {
        let done = sqlx::query!("UPDATE audit_log SET mac = $2 WHERE id = $1", id, mac)
            .execute(&self.db)
            .await?;
        if done.rows_affected() == 0 {
            return Err(not_updated());
        }
        Ok(())
    }
}

/// The newest chained row, as `(seq, hash)`: the anchor a checkpoint
/// records. `None` when no row carries a hash.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn latest_chained_row(db: &PgPool) -> Result<Option<(i64, Option<String>)>> {
    let row = sqlx::query!(
        "SELECT seq, hash FROM audit_log WHERE hash IS NOT NULL ORDER BY seq DESC LIMIT 1"
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| (r.seq, r.hash)))
}

/// The hash of the row at `seq`: `None` when there is no such row, and
/// `Some(None)` when it carries no hash.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn hash_at(db: &PgPool, seq: i64) -> Result<Option<Option<String>>> {
    let row = sqlx::query!("SELECT hash FROM audit_log WHERE seq = $1 LIMIT 1", seq)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|r| r.hash))
}

/// How many rows sit at or before `seq`.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn count_at_or_before(db: &PgPool, seq: i64) -> Result<u64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM audit_log WHERE seq <= $1"#,
        seq
    )
    .fetch_one(db)
    .await?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// Remove the caller's address and its salt from rows older than `days`,
/// where the chain does not bind the address itself, returning how many
/// went ([`crate::compliance::address_retention`]). One statement, so
/// replicas running it together do no harm.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the update fails.
pub async fn scrub_addresses(db: &PgPool, days: i32) -> Result<u64> {
    let done = sqlx::query!(
        r#"UPDATE audit_log SET ip_address = NULL, ip_salt = NULL
           WHERE ip_address IS NOT NULL
             AND "timestamp" < now() - make_interval(days => $1)
             AND (ip_commitment IS NOT NULL OR hash IS NULL OR redacted_at IS NOT NULL)"#,
        days,
    )
    .execute(db)
    .await?;
    Ok(done.rows_affected())
}

/// How many rows past `days` still hold an address their chain hash
/// binds, so cannot be scrubbed.
///
/// # Errors
///
/// Returns [`crate::Error::Database`] if the query fails.
pub async fn count_unscrubbable_addresses(db: &PgPool, days: i32) -> Result<i64> {
    let n = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM audit_log
           WHERE ip_address IS NOT NULL
             AND "timestamp" < now() - make_interval(days => $1)
             AND ip_commitment IS NULL AND hash IS NOT NULL AND redacted_at IS NULL"#,
        days,
    )
    .fetch_one(db)
    .await?;
    Ok(n)
}

/// Database-backed pins for the tamper-evident audit chain
/// ([`crate::compliance::audit_chain`]).
///
/// These need a migrated `PostgreSQL` via `DATABASE_URL` and are
/// `#[ignore]`d, matching this crate's other DB tests. They exist because
/// the chain's riskiest property cannot be checked without a database: a
/// digest computed in Rust before an `INSERT` must still match after
/// Postgres has stored the snapshots as `jsonb` (which reorders object
/// keys) and returned `timestamp` as a `timestamptz`.
///
/// They touch only `audit_log`, deliberately — it has no foreign keys to
/// the person tables, so these pins stay green independently of the rest
/// of the schema.
#[cfg(test)]
mod chain_tests {
    /// The canonical name expands to every historical spelling, so a
    /// per-entity audit query cannot silently drop rows written under an
    /// older one.
    #[test]
    fn canonical_entity_type_expands_to_every_spelling() {
        let spellings = super::entity_type_spellings("Person");
        assert!(spellings.contains(&"Person"), "the canonical spelling");
        assert_eq!(
            spellings.len(),
            super::ENTITY_TYPE_SPELLINGS.len(),
            "the expansion must be the full list, not a subset"
        );
        // The read-auditing spelling and the pre-rename trigger spelling.
        assert!(spellings.contains(&"person"));
        assert!(spellings.contains(&"patient"));
    }

    /// Anything that is not the canonical entity name is returned
    /// unchanged, so this cannot widen an unrelated query — the bulk
    /// pipeline audits under its own `PersonBulkExport` type.
    #[test]
    fn other_entity_types_are_not_widened() {
        assert_eq!(
            super::entity_type_spellings("PersonBulkExport"),
            vec!["PersonBulkExport"]
        );
        assert_eq!(
            super::entity_type_spellings("organization"),
            vec!["organization"]
        );
        // Case matters: the lower-case spelling is a *legacy value*, not a
        // second canonical name, so asking for it must not expand.
        assert_eq!(super::entity_type_spellings("person"), vec!["person"]);
    }

    use super::AuditLogRepository;
    use crate::compliance::audit_chain;
    use crate::db::repositories::AuditContext;
    use serial_test::serial;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn connect() -> PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        PgPool::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Remove every audit row, so a run starts from an empty chain.
    async fn clear(db: &PgPool) {
        sqlx::query!("DELETE FROM audit_log")
            .execute(db)
            .await
            .expect("clear audit_log");
    }

    fn ctx() -> AuditContext {
        AuditContext {
            user_id: Some("alice".to_string()),
            ip_address: Some("203.0.113.7".to_string()),
            user_agent: Some("curl/8".to_string()),
        }
    }

    /// **The load-bearing pin.** Rows written through the repository
    /// verify after a full Postgres round-trip — `jsonb` key reordering
    /// and `timestamptz` conversion included.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn chain_survives_a_jsonb_round_trip() {
        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        let entity = Uuid::new_v4();

        // Deliberately key-disordered payloads: Postgres will reorder them.
        repo.log_create(
            "person",
            entity,
            serde_json::json!({ "z_last": 1, "a_first": 2, "nested": { "y": 1, "x": 2 } }),
            &ctx(),
        )
        .await
        .expect("log create");
        repo.log_update(
            "person",
            entity,
            serde_json::json!({ "a_first": 2 }),
            serde_json::json!({ "a_first": 3, "z_last": 1 }),
            &ctx(),
        )
        .await
        .expect("log update");
        repo.log_delete(
            "person",
            entity,
            serde_json::json!({ "a_first": 3 }),
            &ctx(),
        )
        .await
        .expect("log delete");

        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(report.verified, "chain must verify: {:?}", report.breaks);
        assert_eq!(report.rows, 3);
        assert_eq!(report.intact, 3);
        assert_eq!(report.unchained, 0, "every write must be chained");
        assert!(report.head.is_some());
    }

    /// Rewriting a row with raw SQL is reported as a `content` break — the
    /// property the chain exists to provide.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn tampering_with_a_row_breaks_verification() {
        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        let entity = Uuid::new_v4();
        repo.log_create(
            "person",
            entity,
            serde_json::json!({ "family_name": "Smith" }),
            &ctx(),
        )
        .await
        .expect("log create");
        assert!(repo.verify_chain(1000).await.expect("verify").verified);

        sqlx::query!(
            r#"UPDATE audit_log
               SET new_values = jsonb_set(new_values, '{family_name}', '"Tampered"')
               WHERE new_values IS NOT NULL"#
        )
        .execute(&db)
        .await
        .expect("tamper");

        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(!report.verified, "an edited row must break verification");
        assert!(report.breaks.iter().any(|b| b.kind == "content"));
        // Leave no deliberately-corrupted trail behind. These two tests
        // are the only ones that damage the chain on purpose, and the
        // database is shared with every other DB-gated target in the
        // crate — a tampered row left here surfaced later as a `content`
        // break in the integration suite's `/api/audit/verify` test,
        // which looked like a product defect and was not.
        clear(&db).await;
    }

    /// The sweep empties an old row's address and leaves the row verifying
    /// with its content still checked, which `redacted_at` would not.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn an_old_address_goes_and_the_chain_still_verifies() {
        use crate::compliance::address_retention::{scrub, unscrubbable};

        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        for _ in 0..2 {
            repo.log_create("person", Uuid::new_v4(), serde_json::json!({}), &ctx())
                .await
                .expect("log create");
        }

        assert_eq!(scrub(&db, 7).await.expect("scrub"), 0, "nothing is old yet");
        // A retention of minus one day puts the cutoff a day ahead, so it
        // takes every row. Zero would compare the app's clock, which stamps
        // the rows, with the database's, which sets the cutoff, and fail
        // whenever the database runs a few milliseconds behind.
        assert_eq!(scrub(&db, -1).await.expect("scrub"), 2);
        assert_eq!(
            scrub(&db, -1).await.expect("scrub"),
            0,
            "nothing left to do"
        );

        let rows = repo.chain_tail(10).await.expect("rows");
        assert!(
            rows.iter()
                .all(|row| row.ip_address.is_none() && row.ip_salt.is_none())
        );
        assert!(rows.iter().all(|row| row.ip_commitment.is_some()));
        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.intact, 2, "content was checked, not skipped");
        assert_eq!(report.redacted, 0);
        assert_eq!(unscrubbable(&db, -1).await.expect("count"), 0);
        clear(&db).await;
    }

    /// Deleting a row breaks its successor's linkage — the property an
    /// append-only convention alone cannot provide, and the reason the
    /// chain binds a predecessor at all.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn deleting_a_row_breaks_linkage() {
        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        let entity = Uuid::new_v4();
        for i in 0..3 {
            repo.log_create("person", entity, serde_json::json!({ "n": i }), &ctx())
                .await
                .expect("log create");
        }
        assert!(repo.verify_chain(1000).await.expect("verify").verified);

        sqlx::query!("DELETE FROM audit_log WHERE seq = (SELECT MIN(seq) + 1 FROM audit_log)")
            .execute(&db)
            .await
            .expect("delete a row");

        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(!report.verified, "a deleted row must break the chain");
        assert!(report.breaks.iter().any(|b| b.kind == "linkage"));
        // Leave no deliberately-corrupted trail behind. These two tests
        // are the only ones that damage the chain on purpose, and the
        // database is shared with every other DB-gated target in the
        // crate — a tampered row left here surfaced later as a `content`
        // break in the integration suite's `/api/audit/verify` test,
        // which looked like a product defect and was not.
        clear(&db).await;
    }

    /// A read/disclosure access is chained like any other row, and
    /// carries the caller's declared purpose and recipient in `context`
    /// with `disclosure` set — the §164.528 distinction, persisted.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn read_access_is_chained_and_flagged_as_a_disclosure() {
        use crate::compliance::disclosure::AccessContext;

        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        let entity = Uuid::new_v4();

        // A research read released to a named recipient: a disclosure.
        let outward = AccessContext::from_parts(Some("research"), Some("University"), None);
        repo.log_access("person", entity, "read", &ctx(), &outward)
            .await
            .expect("log access");
        // A care read with no recipient: an internal access.
        let internal = AccessContext::from_parts(Some("care"), None, None);
        repo.log_access("person", entity, "read", &ctx(), &internal)
            .await
            .expect("log access");

        let rows = repo.chain_tail(10).await.expect("tail");
        assert_eq!(rows.len(), 2);
        assert!(rows[0].disclosure, "a named recipient is a disclosure");
        assert!(!rows[1].disclosure, "a care read is an internal access");
        assert_eq!(
            rows[0]
                .context
                .as_ref()
                .and_then(|c| c["purpose_of_use"].as_str()),
            Some("research"),
            "the declared purpose is persisted"
        );

        // Chaining the access rows must not weaken the chain.
        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(report.verified, "{:?}", report.breaks);
        assert_eq!(report.intact, 2);
    }

    /// The stored timestamp round-trips at exactly microsecond precision,
    /// which is what makes the digest reproducible.
    #[tokio::test]
    #[serial]
    #[ignore = "requires a running PostgreSQL via DATABASE_URL"]
    async fn stored_timestamp_is_microsecond_exact() {
        let db = connect().await;
        clear(&db).await;
        let repo = AuditLogRepository::new(db.clone());
        repo.log_create("person", Uuid::new_v4(), serde_json::json!({}), &ctx())
            .await
            .expect("log create");
        let rows = repo.chain_tail(1).await.expect("tail");
        let stored = rows.first().expect("one row").timestamp;
        assert_eq!(
            audit_chain::trunc_micros(stored),
            stored,
            "Postgres returned sub-microsecond precision the writer did not truncate"
        );
    }
}
