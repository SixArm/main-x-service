//! Audit log repository: HIPAA-style write/query of the `audit_log` table.
//!
//! [`AuditLogRepository`] records who did what, when, and to which entity,
//! capturing old/new JSON snapshots plus request provenance (user id, IP,
//! user agent). The `log_create` / `log_update` / `log_delete` helpers are
//! thin wrappers over the private `log_action` insert. Query helpers back
//! the audit REST endpoints (per-entity, recent, per-user history).

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::models::audit_log;
use super::repositories::AuditContext;
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
/// Holds a cloned [`DatabaseConnection`]; construct one per shared
/// application state and wrap in an `Arc` to share across handlers.
pub struct AuditLogRepository {
    /// The `SeaORM` connection used for every audit query/insert.
    db: DatabaseConnection,
}

impl AuditLogRepository {
    /// Wrap a database connection in an audit repository.
    #[must_use]
    pub fn new(db: DatabaseConnection) -> Self {
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

    /// SEC-B10: insert one audit row on an arbitrary connection `conn` —
    /// which may be a `&DatabaseTransaction`, so a caller can write the audit
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
    pub async fn log_action_on<C: ConnectionTrait>(
        &self,
        conn: &C,
        action: &str,
        entity_type: &str,
        entity_id: Uuid,
        old_values: Option<JsonValue>,
        new_values: Option<JsonValue>,
        ctx: &AuditContext,
    ) -> Result<()> {
        // Serialise the read-head/append pair so two concurrent writers
        // cannot claim the same predecessor and fork the chain. Held to
        // the end of the enclosing transaction, so an audit row written
        // inside a caller's transaction is fully serialised; on a pooled
        // connection each statement is its own implicit transaction and a
        // concurrent fork remains possible, which verification reports as
        // a `linkage` break rather than hiding.
        if conn.get_database_backend() == sea_orm::DatabaseBackend::Postgres {
            conn.execute(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                format!("SELECT pg_advisory_xact_lock({CHAIN_LOCK_KEY})"),
            ))
            .await?;
        }
        let prev_hash = Self::chain_head(conn).await?;

        let id = Uuid::new_v4();
        // Truncated to microseconds so the value hashed here is the value
        // Postgres returns (see `compliance::audit_chain`).
        let timestamp = audit_chain::trunc_micros(time::OffsetDateTime::now_utc());
        let hash = audit_chain::row_hash(&audit_chain::ChainInput {
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
            user_agent: ctx.user_agent.as_deref(),
            context: None,
            disclosure: false,
        });

        let new_audit = audit_log::ActiveModel {
            id: Set(id),
            timestamp: Set(timestamp),
            user_id: Set(ctx.user_id.clone()),
            action: Set(action.to_string()),
            entity_type: Set(entity_type.to_string()),
            entity_id: Set(entity_id),
            old_values: Set(old_values),
            new_values: Set(new_values),
            ip_address: Set(ctx.ip_address.clone()),
            user_agent: Set(ctx.user_agent.clone()),
            prev_hash: Set(prev_hash),
            hash: Set(Some(hash)),
            context: Set(None),
            disclosure: Set(false),
            redacted_at: Set(None),
            // `seq` is a BIGSERIAL: let Postgres assign the append order.
            seq: sea_orm::ActiveValue::NotSet,
        };

        new_audit.insert(conn).await?;

        Ok(())
    }

    /// The current chain head: the most recent row's `hash`, or `None`
    /// when the trail is empty (or its last row predates the chain, in
    /// which case the next row starts a fresh run — see
    /// [`audit_chain::verify`]).
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn chain_head<C: ConnectionTrait>(conn: &C) -> Result<Option<String>> {
        let last = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Seq)
            .one(conn)
            .await?;
        Ok(last.and_then(|row| row.hash))
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

    /// SEC-B10: transaction-scoped `UPDATE` audit — like [`log_update`], but
    /// inserted on `conn` (e.g. `&DatabaseTransaction`) so it commits
    /// atomically with the entity change.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_update_on<C: ConnectionTrait>(
        &self,
        conn: &C,
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
    /// inserted on `conn` (e.g. `&DatabaseTransaction`) so it commits
    /// atomically with the entity change.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the audit row insert fails.
    pub async fn log_delete_on<C: ConnectionTrait>(
        &self,
        conn: &C,
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
        let logs = audit_log::Entity::find()
            .filter(audit_log::Column::EntityType.eq(entity_type))
            .filter(audit_log::Column::EntityId.eq(entity_id))
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
    }

    /// Return the `limit` most recent audit rows system-wide.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Database`] if the query fails.
    pub async fn get_recent_logs(&self, limit: u64) -> Result<Vec<audit_log::Model>> {
        let logs = audit_log::Entity::find()
            .order_by_desc(audit_log::Column::Timestamp)
            .limit(limit)
            .all(&self.db)
            .await?;

        Ok(logs)
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
/// the person tables, so these pins stay green independently of the rest
/// of the schema.
#[cfg(test)]
mod chain_tests {
    use super::AuditLogRepository;
    use crate::compliance::audit_chain;
    use crate::db::repositories::AuditContext;
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

        db.execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "DELETE FROM audit_log WHERE seq = (SELECT MIN(seq) + 1 FROM audit_log)".to_string(),
        ))
        .await
        .expect("delete a row");

        let report = repo.verify_chain(1000).await.expect("verify");
        assert!(!report.verified, "a deleted row must break the chain");
        assert!(report.breaks.iter().any(|b| b.kind == "linkage"));
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
