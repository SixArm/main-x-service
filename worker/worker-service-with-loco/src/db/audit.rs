//! HIPAA-style audit logging into the `audit_log` table.
//!
//! [`AuditLogRepository`] records who changed what and when, capturing
//! old/new values as JSON plus actor metadata (user ID, IP, user agent). The
//! `log_create` / `log_update` / `log_delete` helpers funnel into a single
//! private `log_action`, and the `get_*` queries back the audit REST
//! endpoints.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use sea_orm::ConnectionTrait;

use crate::Result;
use crate::compliance::audit_chain;

/// Postgres advisory-lock key serialising chain appends.
const CHAIN_LOCK_KEY: i64 = 0x6D78_695F_776B_7272; // "mxi_wkrr"

/// Actor metadata recorded alongside an audit entry, borrowed for the duration
/// of the write. Groups the user/IP/user-agent triple so the `log_*` helpers
/// take a single argument rather than three.
#[derive(Debug, Clone, Copy, Default)]
pub struct AuditActor<'a> {
    /// Acting user's identifier.
    pub user_id: Option<&'a str>,
    /// Originating client IP address.
    pub ip_address: Option<&'a str>,
    /// Originating client user-agent string.
    pub user_agent: Option<&'a str>,
}

/// Repository that writes and queries entries in the `audit_log` table.
pub struct AuditLogRepository {
    /// `SeaORM` connection used for all audit reads and writes.
    db: DatabaseConnection,
}

/// The `entity_type` spellings that mean "worker", newest first in
/// preference order.
///
/// One list, because a per-entity audit query that filters on a single
/// spelling silently drops the rows written under another — and a silently
/// short audit answer is worse than an error, since nothing about the
/// response says it is incomplete.
///
/// - `"Worker"` — the canonical spelling. Repository mutations have always
///   used it, and every writer uses it now.
/// - `"worker"` — two writers used it: the read-auditing path between the
///   audit chain landing and 2026-07-26, and the `audit_workers_changes`
///   database trigger. A query for one spelling returned none of the
///   other's rows, so the per-entity audit endpoint silently omitted
///   every read, and an accounting of disclosures built on it would have
///   looked empty while disclosures were being recorded. The trigger is
///   dropped by `m20260726_000003_drop_audit_triggers`, but its rows
///   remain and are still part of the record.
///
/// Historical rows are **not** rewritten to the canonical spelling.
/// `entity_type` is bound into the audit chain's row digest, so an
/// `UPDATE` normalising it would make every affected chained row fail
/// verification — the chain would correctly report that someone had
/// edited the audit trail, because someone had. Tolerating the spelling on
/// read is the only option that keeps both the history and its integrity.
pub const ENTITY_TYPE_SPELLINGS: [&str; 2] = ["Worker", "worker"];

/// Expand `entity_type` to every spelling that means the same entity.
///
/// Only the canonical entity name expands; anything else (for example
/// `"WorkerBulkExport"`, which the bulk pipeline audits under its own
/// type) is returned unchanged, so this cannot silently widen an unrelated
/// query.
#[must_use]
pub fn entity_type_spellings(entity_type: &str) -> Vec<&str> {
    if entity_type == "Worker" {
        ENTITY_TYPE_SPELLINGS.to_vec()
    } else {
        vec![entity_type]
    }
}

impl AuditLogRepository {
    /// Wraps an existing database connection in an audit repository.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// Records a `CREATE` action, storing `new_values` (no prior state).
    ///
    /// The `old_values` column is left null because a created record has no
    /// before-state; `new_values` is the JSON snapshot of the new record.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails (e.g. a
    /// database connectivity or constraint error).
    pub async fn log_create(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        new_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "CREATE",
            entity_type,
            entity_id,
            None,
            Some(new_values),
            actor,
        )
        .await
    }

    /// Records an `UPDATE` action, storing both `old_values` and `new_values`
    /// so the entry captures the full before/after diff.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_update(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        new_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "UPDATE",
            entity_type,
            entity_id,
            Some(old_values),
            Some(new_values),
            actor,
        )
        .await
    }

    /// Records a `DELETE` action, storing `old_values` (no new state).
    ///
    /// The `new_values` column is left null because a deleted record has no
    /// after-state; `old_values` preserves the record as it was before
    /// (soft-)deletion.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_delete(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        old_values: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action(
            "DELETE",
            entity_type,
            entity_id,
            Some(old_values),
            None,
            actor,
        )
        .await
    }

    /// Records an `EXPORT` action, storing the export `details` (actor,
    /// filter, format, masking profile, row count) as the new-values
    /// snapshot, with no prior snapshot. Used for the bulk export/read
    /// compliance trail (`bulk-import-export.md` §8, cross-service-linking
    /// §8) — a bulk extract of personal data is itself an audited event.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying audit-row insert fails.
    pub async fn log_export(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        details: JsonValue,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_action("EXPORT", entity_type, entity_id, None, Some(details), actor)
            .await
    }

    /// Inserts one audit row. Shared implementation behind the typed
    /// `log_create`/`log_update`/`log_delete` helpers; stamps a fresh UUID and
    /// the current UTC time so callers never supply identity or timing.
    ///
    /// # Errors
    ///
    /// Returns an error if the row insert fails.
    async fn log_action(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        actor: &AuditActor<'_>,
    ) -> Result<()> {
        self.log_chained(
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            actor,
            None,
            false,
        )
        .await
    }

    /// Record one **read/disclosure** access (HIPAA §164.312(b),
    /// §164.528), carrying the caller's declared purpose-of-use context
    /// and whether the access was an outward disclosure.
    ///
    /// Separate from the mutation path because the two differ in what
    /// they mean, not just in their arguments: a mutation records a
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
        actor: &AuditActor<'_>,
        access: &crate::compliance::disclosure::AccessContext,
    ) -> Result<()> {
        self.log_chained(
            action,
            entity_type,
            entity_id,
            None,
            None,
            actor,
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
    pub async fn redact_for_entity<C: ConnectionTrait>(
        &self,
        conn: &C,
        entity_id: Uuid,
    ) -> Result<u64> {
        let now = audit_chain::trunc_micros(time::OffsetDateTime::now_utc());
        let result = audit_log::Entity::update_many()
            .col_expr(
                audit_log::Column::OldValues,
                sea_orm::sea_query::Expr::value(sea_orm::Value::Json(None)),
            )
            .col_expr(
                audit_log::Column::NewValues,
                sea_orm::sea_query::Expr::value(sea_orm::Value::Json(None)),
            )
            .col_expr(
                audit_log::Column::RedactedAt,
                sea_orm::sea_query::Expr::value(now),
            )
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .filter(audit_log::Column::RedactedAt.is_null())
            .exec(conn)
            .await?;
        Ok(result.rows_affected)
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
    pub async fn log_erasure<C: ConnectionTrait>(
        &self,
        conn: &C,
        entity_id: Uuid,
        actor: Option<&str>,
        context: JsonValue,
        disclosure: bool,
    ) -> Result<()> {
        let audit_actor = AuditActor {
            user_id: actor,
            ip_address: None,
            user_agent: None,
        };
        self.log_chained_on(
            conn,
            crate::compliance::erasure::ACTION_ERASED,
            "Worker",
            entity_id,
            None,
            None,
            &audit_actor,
            Some(context),
            disclosure,
        )
        .await
    }

    /// The chained insert both write paths share.
    ///
    /// Every row binds its own content and its predecessor's hash, so the
    /// trail can prove it was not rewritten (HIPAA §164.312(c)); see
    /// [`crate::compliance::audit_chain`].
    #[allow(clippy::too_many_arguments)]
    async fn log_chained(
        &self,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        actor: &AuditActor<'_>,
        context: Option<JsonValue>,
        disclosure: bool,
    ) -> Result<()> {
        self.log_chained_on(
            &self.db,
            action,
            entity_type,
            entity_id,
            old_values,
            new_values,
            actor,
            context,
            disclosure,
        )
        .await
    }

    /// The connection-generic chained insert.
    ///
    /// Split out from [`Self::log_chained`] so an audit row can be written
    /// **inside a caller's transaction** — GDPR Art. 17 erasure needs the
    /// redaction sweep and its `erased` accountability row to commit
    /// together with the data destruction, and a row written on
    /// `self.db` would land outside that transaction and survive a
    /// rollback that undid the erasure it claims to record.
    #[allow(clippy::too_many_arguments)]
    async fn log_chained_on<C: ConnectionTrait>(
        &self,
        conn: &C,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        actor: &AuditActor<'_>,
        context: Option<JsonValue>,
        disclosure: bool,
    ) -> Result<()> {
        // Serialise the read-head/append pair so two concurrent writers
        // cannot claim the same predecessor and fork the chain.
        if conn.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            conn.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT pg_advisory_xact_lock({CHAIN_LOCK_KEY})"),
            ))
            .await?;
        }
        // Both chain heads from the same row, so each algorithm binds
        // its *own* predecessor. Binding the SHA-256 head into the BLAKE3
        // digest would make the second chain's linkage rest on SHA-256's
        // collision resistance — the dependency two algorithms exist to
        // avoid.
        let (prev_hash, prev_hash_sha3) = Self::chain_heads(conn).await?;

        let id = Uuid::new_v4();
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::audit_chain`).
        let timestamp = audit_chain::trunc_micros(time::OffsetDateTime::now_utc());
        let mut chain_input = audit_chain::ChainInput {
            prev_hash: prev_hash.as_deref(),
            id,
            timestamp_micros: audit_chain::micros(timestamp),
            user_id: actor.user_id,
            action,
            entity_type,
            entity_id,
            old_values: old_values.as_ref(),
            new_values: new_values.as_ref(),
            ip_address: actor.ip_address,
            user_agent: actor.user_agent,
            context: context.as_ref(),
            disclosure,
        };
        let hash = audit_chain::row_hash(&chain_input);
        chain_input.prev_hash = prev_hash_sha3.as_deref();
        let hash_sha3 = audit_chain::row_hash_sha3(&chain_input);

        let new_audit = audit_log::ActiveModel {
            id: Set(id),
            timestamp: Set(timestamp),
            user_id: Set(actor.user_id.map(String::from)),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(actor.ip_address.map(String::from)),
            user_agent: Set(actor.user_agent.map(String::from)),
            prev_hash: Set(prev_hash),
            prev_hash_sha3: Set(prev_hash_sha3),
            hash_sha3: Set(Some(hash_sha3)),
            hash: Set(Some(hash)),
            context: Set(context),
            disclosure: Set(disclosure),
            redacted_at: Set(None),
            // `seq` is a BIGSERIAL: let Postgres assign the append order.
            seq: sea_orm::ActiveValue::NotSet,
        };

        new_audit.insert(conn).await?;

        Ok(())
    }

    /// Both chain heads — `(SHA-256, BLAKE3)` — from the same row.
    ///
    /// Read together so an append binds each algorithm's own predecessor.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn chain_heads<C: ConnectionTrait>(
        conn: &C,
    ) -> Result<(Option<String>, Option<String>)> {
        let last = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Seq)
            .one(conn)
            .await?;
        Ok(match last {
            Some(row) => (row.hash, row.hash_sha3),
            None => (None, None),
        })
    }

    /// The current SHA-256 chain head: the most recent row's `hash`, or
    /// `None` when the trail is empty (or its last row predates the
    /// chain).
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_head(&self) -> Result<Option<String>> {
        let last = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Seq)
            .one(&self.db)
            .await?;
        Ok(last.and_then(|row| row.hash))
    }

    /// The newest `limit` rows in **ascending `seq` order** — the shape
    /// [`audit_chain::verify`] expects.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_tail(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let mut rows = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Seq)
            .limit(limit)
            .all(&self.db)
            .await?;
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

    /// Returns up to `limit` audit entries for one entity, newest first.
    ///
    /// Filters by both `entity_type` and `entity_id` so audit IDs are only
    /// unique within an entity type. Backs the per-record audit endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_logs_for_entity(
        &self,
        entity_type: &str,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::EntityType.is_in(entity_type_spellings(entity_type)))
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Every audit row for one entity flagged as an outward
    /// **disclosure**, newest first — the HIPAA §164.528 accounting.
    ///
    /// Accepts every spelling in [`ENTITY_TYPE_SPELLINGS`], so the
    /// accounting cannot silently omit rows written under an older one.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn disclosures_for_entity(
        &self,
        entity_id: Uuid,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::EntityType.is_in(ENTITY_TYPE_SPELLINGS))
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .filter(audit_log::Column::Disclosure.eq(true))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Returns the `limit` most recent audit entries across all entities,
    /// newest first. Backs the system-wide recent-activity endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Returns up to `limit` audit entries performed by `user_id`, newest
    /// first. Backs the per-user audit endpoint.
    ///
    /// # Errors
    ///
    /// Returns an error if the query fails.
    pub async fn get_logs_by_user(
        &self,
        user_id: &str,
        limit: u64,
    ) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::UserId.eq(user_id))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }
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
/// the worker tables, so these pins stay green independently of the rest
/// of the schema.
#[cfg(test)]
mod chain_tests {
    /// The canonical name expands to every historical spelling, so a
    /// per-entity audit query cannot silently drop rows written under an
    /// older one.
    #[test]
    fn canonical_entity_type_expands_to_every_spelling() {
        let spellings = super::entity_type_spellings("Worker");
        assert!(spellings.contains(&"Worker"), "the canonical spelling");
        assert_eq!(
            spellings.len(),
            super::ENTITY_TYPE_SPELLINGS.len(),
            "the expansion must be the full list, not a subset"
        );
        // Used by both the old read-auditing path and the trigger.
        assert!(spellings.contains(&"worker"));
    }

    /// Anything that is not the canonical entity name is returned
    /// unchanged, so this cannot widen an unrelated query — the bulk
    /// pipeline audits under its own `WorkerBulkExport` type.
    #[test]
    fn other_entity_types_are_not_widened() {
        assert_eq!(
            super::entity_type_spellings("WorkerBulkExport"),
            vec!["WorkerBulkExport"]
        );
        assert_eq!(
            super::entity_type_spellings("organization"),
            vec!["organization"]
        );
        // Case matters: the lower-case spelling is a *legacy value*, not a
        // second canonical name, so asking for it must not expand.
        assert_eq!(super::entity_type_spellings("worker"), vec!["worker"]);
    }

    use super::AuditLogRepository;
    use crate::compliance::audit_chain;
    use crate::db::audit::AuditActor;
    use sea_orm::{ConnectionTrait, Statement};
    use serial_test::serial;
    use uuid::Uuid;

    async fn connect() -> sea_orm::DatabaseConnection {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for DB tests");
        sea_orm::Database::connect(&url)
            .await
            .expect("connect to DATABASE_URL")
    }

    /// Remove every audit row, so a run starts from an empty chain.
    async fn clear(db: &sea_orm::DatabaseConnection) {
        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM audit_log".to_string(),
        ))
        .await
        .expect("clear audit_log");
    }

    fn ctx() -> AuditActor<'static> {
        AuditActor {
            user_id: Some("alice"),
            ip_address: Some("203.0.113.7"),
            user_agent: Some("curl/8"),
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
            "worker",
            entity,
            serde_json::json!({ "z_last": 1, "a_first": 2, "nested": { "y": 1, "x": 2 } }),
            &ctx(),
        )
        .await
        .expect("log create");
        repo.log_update(
            "worker",
            entity,
            serde_json::json!({ "a_first": 2 }),
            serde_json::json!({ "a_first": 3, "z_last": 1 }),
            &ctx(),
        )
        .await
        .expect("log update");
        repo.log_delete(
            "worker",
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
            "worker",
            entity,
            serde_json::json!({ "family_name": "Smith" }),
            &ctx(),
        )
        .await
        .expect("log create");
        assert!(repo.verify_chain(1000).await.expect("verify").verified);

        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            r#"UPDATE audit_log
               SET new_values = jsonb_set(new_values, '{family_name}', '"Tampered"')
               WHERE new_values IS NOT NULL"#
                .to_string(),
        ))
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
            repo.log_create("worker", entity, serde_json::json!({ "n": i }), &ctx())
                .await
                .expect("log create");
        }
        assert!(repo.verify_chain(1000).await.expect("verify").verified);

        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM audit_log WHERE seq = (SELECT MIN(seq) + 1 FROM audit_log)".to_string(),
        ))
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
        repo.log_access("worker", entity, "read", &ctx(), &outward)
            .await
            .expect("log access");
        // A care read with no recipient: an internal access.
        let internal = AccessContext::from_parts(Some("care"), None, None);
        repo.log_access("worker", entity, "read", &ctx(), &internal)
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
        repo.log_create("worker", Uuid::new_v4(), serde_json::json!({}), &ctx())
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
